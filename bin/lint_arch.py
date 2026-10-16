"""Dependency-graph lint for the Cargo workspace: the crate-boundary law (SPR-001 T-04).

Every workspace crate declares its plane (ADR-0024) and kind in ``[package.metadata.rehoboam]``.
This lint reads them from ``cargo metadata --no-deps`` (offline) and checks every edge between
workspace crates, of every dependency kind (normal, build and dev):

    metadata         Every crate declares plane and kind from the vocabularies below,
                     consistently: kinds core, contract and system are plane A; plane "tool"
                     goes with kind "tool"; crates named rh-sys-* are kind "system", and only
                     they.
    V-INSP-03        A system crate depends only on rh-core, rh-ids, rh-events, rh-registry and
                     contract crates, never on another system crate (docs/13 §3; EVO-1).
    V-PLN-01         A plane-A crate never depends on a plane-B or plane-C crate (ADR-0024 rule 1;
                     05.1 §6 item 7).
    plane-A-closure  A plane-A crate depends only on plane-A workspace crates. A tool crate is not
                     scanned by the forbidden-API gate and may itself reach planes B and C, so an
                     edge to one would launder both past the checks.

Edges to third-party crates are governed by cargo-deny (deny.toml), not by this lint.

Usage:  python -I bin/lint_arch.py [--manifest-path Cargo.toml]
Exit:   0 clean, 1 violations, 2 the workspace could not be read.
Standard library only (GOV-003 §3).
"""

import argparse
import json
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

PLANES = frozenset({"A", "B", "C", "tool"})
KINDS = frozenset({"core", "machinery", "contract", "system", "tool"})
# REF: docs/13 §3 ("Crate boundaries are law"); docs/09 V-INSP-03. Contract crates (kind
# "contract") are allowed in addition to these names.
SYSTEM_ALLOWED_DEPS = frozenset({"rh-core", "rh-ids", "rh-events", "rh-registry"})
PLANE_A_KINDS = frozenset({"core", "contract", "system"})
SYSTEM_CRATE_PREFIX = "rh-sys-"
OBSERVATION_AND_NARRATIVE = frozenset({"B", "C"})

EXIT_OK = 0
EXIT_VIOLATIONS = 1
EXIT_UNREADABLE = 2

METADATA_HINT = "declare it in the crate's Cargo.toml [package.metadata.rehoboam]"


class WorkspaceError(Exception):
    """The workspace metadata could not be read or understood."""


@dataclass(frozen=True, order=True)
class Edge:
    """A dependency of one workspace crate on another."""

    target: str
    kind: str  # "normal", "build" or "dev"


@dataclass(frozen=True)
class Crate:
    """A workspace crate with its declared plane, kind and workspace-internal edges."""

    name: str
    plane: str | None
    kind: str | None
    edges: tuple[Edge, ...]

    @property
    def declared_ok(self) -> bool:
        """Whether plane and kind are both from their vocabularies."""
        return self.plane in PLANES and self.kind in KINDS


@dataclass(frozen=True)
class Violation:
    """One broken rule, attributed to the crate that breaks it."""

    rule: str
    crate: str
    message: str

    def render(self) -> str:
        """Return the line printed for this violation."""
        return f"VIOLATION [{self.rule}] {self.crate}: {self.message}"


def _as_dict(value: object, what: str) -> dict[str, object]:
    """Return ``value`` as a JSON object, or raise WorkspaceError naming ``what``."""
    if not isinstance(value, dict):
        raise WorkspaceError(f"{what} is not a JSON object")
    return {str(key): item for key, item in value.items()}


def _as_list(value: object, what: str) -> list[object]:
    """Return ``value`` as a JSON array, or raise WorkspaceError naming ``what``."""
    if not isinstance(value, list):
        raise WorkspaceError(f"{what} is not a JSON array")
    return list(value)


def _declared(package: dict[str, object], key: str) -> str | None:
    """Return ``[package.metadata.rehoboam].<key>`` as text, or None when it is absent."""
    metadata = package.get("metadata")
    section = metadata.get("rehoboam") if isinstance(metadata, dict) else None
    value = section.get(key) if isinstance(section, dict) else None
    if value is None:
        return None
    return value if isinstance(value, str) else repr(value)


def _workspace_edges(package: dict[str, object], workspace: frozenset[str]) -> tuple[Edge, ...]:
    """Return the package's dependencies on other workspace crates, sorted."""
    edges: set[Edge] = set()
    for raw in _as_list(package.get("dependencies", []), "dependencies"):
        dep = _as_dict(raw, "a dependency")
        name = dep.get("name")
        # A workspace dependency is a path dependency (no registry or git source) on a member.
        if isinstance(name, str) and name in workspace and dep.get("source") is None:
            kind = dep.get("kind")
            edges.add(Edge(target=name, kind=kind if isinstance(kind, str) else "normal"))
    return tuple(sorted(edges))


def crates_from_metadata(document: object) -> list[Crate]:
    """Build the workspace crates from ``cargo metadata --format-version 1`` output.

    Args:
        document: The parsed JSON document.

    Returns:
        The workspace members, sorted by name.

    Raises:
        WorkspaceError: The document does not have the expected shape.
    """
    root = _as_dict(document, "cargo metadata output")
    members = {str(m) for m in _as_list(root.get("workspace_members"), "workspace_members")}
    packages = [_as_dict(p, "a package") for p in _as_list(root.get("packages"), "packages")]
    in_workspace = [p for p in packages if p.get("id") in members]
    names = frozenset(str(p.get("name")) for p in in_workspace)
    crates = [
        Crate(
            name=str(p.get("name")),
            plane=_declared(p, "plane"),
            kind=_declared(p, "kind"),
            edges=_workspace_edges(p, names),
        )
        for p in in_workspace
    ]
    return sorted(crates, key=lambda crate: crate.name)


def check_metadata(crate: Crate) -> list[Violation]:
    """Check that a crate declares a valid, consistent plane and kind."""
    problems: list[str] = []
    if crate.plane not in PLANES:
        problems.append(f"plane = {crate.plane!r} is missing or not one of {sorted(PLANES)}")
    if crate.kind not in KINDS:
        problems.append(f"kind = {crate.kind!r} is missing or not one of {sorted(KINDS)}")
    if crate.declared_ok:
        if crate.kind in PLANE_A_KINDS and crate.plane != "A":
            problems.append(f'kind "{crate.kind}" must be plane "A", not "{crate.plane}"')
        if (crate.plane == "tool") != (crate.kind == "tool"):
            problems.append('plane "tool" and kind "tool" must be declared together')
    if crate.name.startswith(SYSTEM_CRATE_PREFIX) != (crate.kind == "system"):
        problems.append(f'crates named {SYSTEM_CRATE_PREFIX}* are kind "system", and only they')
    return [Violation("metadata", crate.name, f"{p}; {METADATA_HINT}") for p in problems]


def check_edge(source: Crate, edge: Edge, target: Crate) -> list[Violation]:
    """Check one workspace edge against V-INSP-03, V-PLN-01 and the plane-A closure."""
    found: list[Violation] = []
    via = "" if edge.kind == "normal" else f" (as a {edge.kind}-dependency)"
    if source.kind == "system" and target.kind == "system":
        found.append(
            Violation(
                "V-INSP-03",
                source.name,
                f"depends on system crate {target.name}{via}; systems never import each other, "
                "they share data through contract crates, lifecycle commands and events "
                "(docs/13 §3)",
            )
        )
    elif source.kind == "system" and not (
        target.name in SYSTEM_ALLOWED_DEPS or target.kind == "contract"
    ):
        allowed = ", ".join(sorted(SYSTEM_ALLOWED_DEPS))
        found.append(
            Violation(
                "V-INSP-03",
                source.name,
                f"depends on {target.name}{via}; a system crate may depend only on {allowed} "
                "and contract crates (docs/13 §3)",
            )
        )
    if source.plane == "A" and target.plane in OBSERVATION_AND_NARRATIVE:
        found.append(
            Violation(
                "V-PLN-01",
                source.name,
                f"plane-A crate depends on plane-{target.plane} crate {target.name}{via}; "
                "plane A never reads planes B or C (ADR-0024 rule 1)",
            )
        )
    elif source.plane == "A" and target.plane != "A":
        found.append(
            Violation(
                "plane-A-closure",
                source.name,
                f'plane-A crate depends on plane-"{target.plane}" crate {target.name}{via}; '
                "plane-A crates depend only on plane-A workspace crates",
            )
        )
    return found


def check_workspace(crates: Sequence[Crate]) -> list[Violation]:
    """Check every crate's metadata and every workspace edge.

    Edges with an endpoint whose metadata is invalid are skipped: that crate is already reported,
    and its plane or kind cannot be trusted.
    """
    by_name = {crate.name: crate for crate in crates}
    violations = [v for crate in crates for v in check_metadata(crate)]
    for crate in crates:
        for edge in crate.edges:
            target = by_name.get(edge.target)
            if target is None or not (crate.declared_ok and target.declared_ok):
                continue
            violations.extend(check_edge(crate, edge, target))
    return violations


def load_metadata(manifest_path: Path) -> object:
    """Run ``cargo metadata --no-deps --offline`` on a manifest and return the parsed JSON.

    Raises:
        WorkspaceError: cargo is missing, fails, or prints something that is not JSON.
    """
    command = [
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--offline",
        "--manifest-path",
        str(manifest_path),
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
    except FileNotFoundError as err:
        raise WorkspaceError("cargo is not on PATH (RUN-001 §2)") from err
    if result.returncode != 0:
        raise WorkspaceError(f"cargo metadata failed:\n{result.stderr.strip()}")
    try:
        document: object = json.loads(result.stdout)
    except json.JSONDecodeError as err:
        raise WorkspaceError(f"cargo metadata printed invalid JSON: {err}") from err
    return document


def describe(crate: Crate) -> str:
    """Return one forensic line per crate: its declaration and its workspace edges."""
    edges = ", ".join(
        edge.target if edge.kind == "normal" else f"{edge.target} ({edge.kind})"
        for edge in crate.edges
    )
    return f"  {crate.name}: plane={crate.plane} kind={crate.kind} -> {edges or '(none)'}"


def main(argv: Sequence[str] | None = None) -> int:
    """Run the lint and return the process exit status."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument(
        "--manifest-path",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "Cargo.toml",
        help="workspace Cargo.toml to check (default: the repository's)",
    )
    args = parser.parse_args(argv)
    try:
        crates = crates_from_metadata(load_metadata(args.manifest_path))
    except WorkspaceError as err:
        print(f"lint_arch: cannot read the workspace: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    violations = check_workspace(crates)
    print(f"lint_arch: {args.manifest_path}")
    for crate in crates:
        print(describe(crate))
    for violation in violations:
        print(violation.render())
    edge_count = sum(len(crate.edges) for crate in crates)
    print(
        f"lint_arch: {len(crates)} crates, {edge_count} workspace edges, "
        f"{len(violations)} violations"
    )
    return EXIT_VIOLATIONS if violations else EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
