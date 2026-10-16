"""Dependency-graph lint for the Cargo workspace: the crate-boundary law (SPR-001 T-04).

Every workspace crate declares its plane (ADR-0024) and kind in ``[package.metadata.rehoboam]``.
This lint reads them from ``cargo metadata --no-deps`` (offline) and checks every edge between
workspace crates, of every dependency kind (normal, build and dev):

    metadata         Every crate declares plane and kind from the vocabularies below,
                     consistently: kinds core, contract and system are plane A; plane "tool"
                     goes with kind "tool". Labels cannot be chosen to pass the law: the seven
                     SPR-001 crates are pinned to their (plane, kind) by name; crates named
                     rh-sys-* are (A, system), and only they are kind "system"; kind
                     "contract" belongs only to rh-contracts and rh-contract-* crates. Edges are
                     judged with the pinned labels, not the declared ones.
    V-INSP-03        A system crate depends only on rh-core, rh-ids, rh-events, rh-registry and
                     contract crates, never on another system crate (docs/13 §3; EVO-1).
    V-PLN-01         A plane-A crate never depends on a plane-B or plane-C crate (ADR-0024 rule 1;
                     05.1 §6 item 7).
    plane-A-closure  A plane-A crate depends only on plane-A workspace crates. A tool crate is not
                     scanned by the forbidden-API gate and may itself reach planes B and C, so an
                     edge to one would launder both past the checks.
    rng-crate        A plane-A crate never depends on a crate that provides unkeyed or
                     entropy-seeded randomness (rand, getrandom, ...); the RNG building blocks
                     rand_core and rand_chacha are allowed only in rh-ids, which owns the keyed
                     RNG (ADR-0006; DET-2; DEF-1). Crates are matched by package name, so a
                     `package =` alias does not hide one. Dev-dependencies are exempt: test code
                     is not authoritative, and property testing (proptest, GOV-002 tier 3)
                     needs rand. The forbidden-API gate still scans src/ (V-DET-19).
    plane-A-external Every other third-party dependency of a plane-A crate (normal or build) is
                     on one reviewed list, PLANE_A_EXTERNAL_DEPS, with the crates allowed to use
                     it and a note on whether it can hand plane A a float. Clippy has no lint for
                     float < and >, so a float that plane-A code never names could only enter
                     through such a dependency (VER-SPR-010 GAP-3; BCK-002 A-20). A new
                     dependency turns the gate red until it is reviewed and listed.
                     Dev-dependencies are exempt, as above; the RNG crates are left to rng-crate.

Sources, wildcards and licences of third-party crates are governed by cargo-deny (deny.toml).

Usage:  python -I bin/lint_arch.py [--manifest-path Cargo.toml]
Exit:   0 clean, 1 violations, 2 the workspace could not be read.
Standard library only (GOV-003 §3).
"""

import argparse
import json
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass, replace
from pathlib import Path
from types import MappingProxyType

PLANES = frozenset({"A", "B", "C", "tool"})
KINDS = frozenset({"core", "machinery", "contract", "system", "tool"})
# REF: docs/13 §3 ("Crate boundaries are law"); docs/09 V-INSP-03. Contract crates (kind
# "contract") are allowed in addition to these names.
SYSTEM_ALLOWED_DEPS = frozenset({"rh-core", "rh-ids", "rh-events", "rh-registry"})
PLANE_A_KINDS = frozenset({"core", "contract", "system"})
SYSTEM_CRATE_PREFIX = "rh-sys-"
# REF: docs/05_architecture/05.0 §6; the SPR-001 audit (relabelling rh-snapshot as a contract crate
# let a system depend on it). The expected (plane, kind) of the crates SPR-001 created, by name.
PINNED_LABELS = MappingProxyType(
    {
        "rh-ids": ("A", "machinery"),
        "rh-core": ("A", "core"),
        "rh-events": ("A", "machinery"),
        "rh-registry": ("A", "machinery"),
        "rh-contracts": ("A", "contract"),
        "rh-snapshot": ("A", "machinery"),
        "rh-cli": ("tool", "tool"),
    }
)
SYSTEM_LABELS = ("A", "system")
CONTRACT_CRATE = "rh-contracts"
CONTRACT_CRATE_PREFIX = "rh-contract-"
OBSERVATION_AND_NARRATIVE = frozenset({"B", "C"})
# REF: ADR-0006 (keyed RNG only); DET-2; V-DET-19; DEF-1 (rand 0.9's thread-local functions passed
# every text gate). The single list of crates whose purpose is unkeyed or entropy-seeded randomness.
UNKEYED_RNG_CRATES = frozenset({"rand", "getrandom", "fastrand", "oorandom", "nanorand"})
# Building blocks of the keyed RNG (ChaCha8, SPR-002), allowed only in the crate that owns it.
KEYED_RNG_BUILDING_BLOCKS = frozenset({"rand_core", "rand_chacha"})
KEYED_RNG_OWNER = "rh-ids"
# REF: BCK-002 A-20; VER-SPR-010 GAP-3. The reviewed third-party dependencies of plane-A crates:
# package name -> (the plane-A crates that may use it, or None for any; the review note). The note
# says whether the crate can return a float to plane A. Keep it alphabetical; adding an entry is a
# review, and the reviewer names it in the PR.
ANY_PLANE_A_CRATE = None
PLANE_A_EXTERNAL_DEPS = MappingProxyType(
    {
        "libm": (
            frozenset({"rh-sample"}),
            "returns f32/f64: the transcendentals of the sampler float zone, the only "
            "plane-A crate allowed float arithmetic (SPR-003 ruling)",
        ),
        "serde": (
            ANY_PLANE_A_CRATE,
            "no floats of its own: the (de)serialisation traits and derives",
        ),
        "toml": (
            frozenset({"rh-registry"}),
            "can return f64 (toml::Value::Float); rh-registry reads only strings, arrays and "
            "tables from the system specs for the ownership lint (SPR-004)",
        ),
    }
)

EXIT_OK = 0
EXIT_VIOLATIONS = 1
EXIT_UNREADABLE = 2

METADATA_HINT = "declare it in the crate's Cargo.toml [package.metadata.rehoboam]"


class WorkspaceError(Exception):
    """The workspace metadata could not be read or understood."""


@dataclass(frozen=True, order=True)
class Edge:
    """A dependency of a workspace crate, named by the target's package name (never its alias)."""

    target: str
    kind: str  # "normal", "build" or "dev"


@dataclass(frozen=True)
class Crate:
    """A workspace crate: its declared plane and kind, and its dependency edges.

    ``edges`` go to other workspace crates; ``external`` to everything else (registry, git, or a
    path outside the workspace).
    """

    name: str
    plane: str | None
    kind: str | None
    edges: tuple[Edge, ...]
    external: tuple[Edge, ...] = ()

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


def _edges(
    package: dict[str, object], workspace: frozenset[str]
) -> tuple[tuple[Edge, ...], tuple[Edge, ...]]:
    """Return the package's (workspace, external) dependency edges, each sorted.

    ``name`` in cargo metadata is the dependency's package name; an alias (``package =``) appears
    only as ``rename``, which is ignored, so an alias cannot disguise a crate.
    """
    internal: set[Edge] = set()
    external: set[Edge] = set()
    for raw in _as_list(package.get("dependencies", []), "dependencies"):
        dep = _as_dict(raw, "a dependency")
        name, kind = dep.get("name"), dep.get("kind")
        if not isinstance(name, str):
            raise WorkspaceError(f"a dependency has no name: {dep!r}")
        edge = Edge(target=name, kind=kind if isinstance(kind, str) else "normal")
        # A workspace dependency is a path dependency (no registry or git source) on a member.
        is_member = name in workspace and dep.get("source") is None
        (internal if is_member else external).add(edge)
    return tuple(sorted(internal)), tuple(sorted(external))


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
    crates = []
    for package in in_workspace:
        internal, external = _edges(package, names)
        crates.append(
            Crate(
                name=str(package.get("name")),
                plane=_declared(package, "plane"),
                kind=_declared(package, "kind"),
                edges=internal,
                external=external,
            )
        )
    return sorted(crates, key=lambda crate: crate.name)


def _vocabulary_problems(crate: Crate) -> list[str]:
    """Return what is missing, unknown or inconsistent in a crate's declared plane and kind."""
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
    return problems


def _label_problems(crate: Crate) -> list[str]:
    """Return where a crate's labels contradict what its name requires."""
    problems: list[str] = []
    declared = (crate.plane, crate.kind)
    pinned = PINNED_LABELS.get(crate.name)
    if pinned is not None and declared != pinned:
        problems.append(
            f"declares (plane, kind) = {declared}, but {crate.name} is pinned to {pinned}"
        )
    is_system_name = crate.name.startswith(SYSTEM_CRATE_PREFIX)
    if is_system_name and declared != SYSTEM_LABELS:
        problems.append(f"crates named {SYSTEM_CRATE_PREFIX}* are {SYSTEM_LABELS}, not {declared}")
    if crate.kind == "system" and not is_system_name:
        problems.append(f'only crates named {SYSTEM_CRATE_PREFIX}* are kind "system"')
    is_contract_name = crate.name == CONTRACT_CRATE or crate.name.startswith(CONTRACT_CRATE_PREFIX)
    if crate.kind == "contract" and not is_contract_name:
        problems.append(
            f'kind "contract" belongs only to {CONTRACT_CRATE} and {CONTRACT_CRATE_PREFIX}* crates'
        )
    return problems


def check_metadata(crate: Crate) -> list[Violation]:
    """Check that a crate declares a valid, consistent plane and kind that its name allows."""
    problems = _vocabulary_problems(crate) + _label_problems(crate)
    return [Violation("metadata", crate.name, f"{p}; {METADATA_HINT}") for p in problems]


def effective(crate: Crate) -> Crate:
    """Return the crate with its pinned labels, if it has any, in place of the declared ones."""
    pinned = PINNED_LABELS.get(crate.name)
    if pinned is None:
        return crate
    return replace(crate, plane=pinned[0], kind=pinned[1])


def _via(edge: Edge) -> str:
    """Return the phrase naming a non-normal dependency kind, or nothing for a normal one."""
    return "" if edge.kind == "normal" else f" (as a {edge.kind}-dependency)"


def check_rng_crates(crate: Crate) -> list[Violation]:
    """Check a plane-A crate's external edges against the RNG-crate rule (DEF-1)."""
    if crate.plane != "A":
        return []
    found: list[Violation] = []
    for edge in crate.external:
        # DECISION: dev-dependencies are exempt. Tests never run inside a simulation, and
        # proptest (GOV-002 tier 3) depends on rand. REF: VER-SPR-001 carry-over; SPR-010 report.
        if edge.kind == "dev":
            continue
        if edge.target in UNKEYED_RNG_CRATES:
            found.append(
                Violation(
                    "rng-crate",
                    crate.name,
                    f"plane-A crate depends on {edge.target}{_via(edge)}; the only RNG in plane A "
                    f"is the keyed one in {KEYED_RNG_OWNER} (ADR-0006)",
                )
            )
        elif edge.target in KEYED_RNG_BUILDING_BLOCKS and crate.name != KEYED_RNG_OWNER:
            found.append(
                Violation(
                    "rng-crate",
                    crate.name,
                    f"depends on {edge.target}{_via(edge)}; RNG building blocks are allowed only "
                    f"in {KEYED_RNG_OWNER}, which owns the keyed RNG (SPR-002)",
                )
            )
    return found


def check_external_deps(crate: Crate) -> list[Violation]:
    """Check a plane-A crate's third-party edges against the reviewed list (BCK-002 A-20)."""
    if crate.plane != "A":
        return []
    found: list[Violation] = []
    for edge in crate.external:
        # DECISION: dev-dependencies are exempt, as for rng-crate; RNG crates are judged by
        # rng-crate alone, so one edge is never reported twice. REF: BCK-002 A-20.
        if edge.kind == "dev" or edge.target in UNKEYED_RNG_CRATES | KEYED_RNG_BUILDING_BLOCKS:
            continue
        entry = PLANE_A_EXTERNAL_DEPS.get(edge.target)
        if entry is None:
            found.append(
                Violation(
                    "plane-A-external",
                    crate.name,
                    f"depends on {edge.target}{_via(edge)}, which is not on the reviewed list of "
                    "plane-A third-party dependencies; review it (can it hand plane A a float, "
                    "or nondeterminism?) and add it to PLANE_A_EXTERNAL_DEPS with a note "
                    "(BCK-002 A-20)",
                )
            )
            continue
        allowed, _note = entry
        if allowed is not None and crate.name not in allowed:
            users = ", ".join(sorted(allowed))
            found.append(
                Violation(
                    "plane-A-external",
                    crate.name,
                    f"depends on {edge.target}{_via(edge)}, which plane A may use only in "
                    f"{users} (PLANE_A_EXTERNAL_DEPS)",
                )
            )
    return found


def check_edge(source: Crate, edge: Edge, target: Crate) -> list[Violation]:
    """Check one workspace edge against V-INSP-03, V-PLN-01 and the plane-A closure."""
    found: list[Violation] = []
    via = _via(edge)
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

    Edges are judged with each crate's effective labels (pinned where pinned), so relabelling a
    crate cannot open an edge. Edges with an endpoint whose labels are still invalid are skipped:
    that crate is already reported, and its plane or kind cannot be trusted.
    """
    violations = [v for crate in crates for v in check_metadata(crate)]
    crates = [effective(crate) for crate in crates]
    by_name = {crate.name: crate for crate in crates}
    violations += [v for crate in crates if crate.declared_ok for v in check_rng_crates(crate)]
    violations += [v for crate in crates if crate.declared_ok for v in check_external_deps(crate)]
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
    """Return one forensic line per crate: its declaration and its edges."""

    def names(edges: tuple[Edge, ...]) -> str:
        listed = ", ".join(
            e.target if e.kind == "normal" else f"{e.target} ({e.kind})" for e in edges
        )
        return listed or "(none)"

    return (
        f"  {crate.name}: plane={crate.plane} kind={crate.kind} -> {names(crate.edges)}; "
        f"external: {names(crate.external)}"
    )


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
