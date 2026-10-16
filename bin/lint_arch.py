"""Dependency-graph lint for the Cargo workspace: the crate-boundary law (SPR-001 T-04).

Every workspace crate declares its plane (ADR-0024) and kind in ``[package.metadata.rehoboam]``.
This lint reads them from ``cargo metadata --no-deps`` (offline) and checks every edge between
workspace crates, of every dependency kind (normal, build and dev):

    metadata         Every crate declares plane and kind from the vocabularies below,
                     consistently: kinds core, contract and system are plane A; plane "tool"
                     goes with kind "tool". Labels cannot be chosen to pass the law: the seven
                     SPR-001 crates, rh-manifest (SPR-004) and rh-sample (SPR-003) are pinned to
                     their (plane, kind) by name; crates named rh-sys-* are (A, system), and only
                     they are kind "system"; kind "contract" belongs only to rh-contracts and
                     rh-contract-* crates. Edges are judged with the pinned labels, not the
                     declared ones.
    V-INSP-03        A system crate depends only on rh-core, rh-ids, rh-events, rh-registry,
                     rh-sample and contract crates, never on another system crate (docs/13 §3;
                     EVO-1; rh-sample by the SPR-003 Architect ruling).
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
                     Every such dependency, the RNG building blocks included, is pinned exactly
                     (`=x.y.z`), and an entry may also require `default-features = false`
                     (libm does: the portable software routines only), read from cargo
                     metadata's `req` and `uses_default_features` (VER-SPR-003-TEST GAP-4).
                     Dev-dependencies are exempt, as above; the RNG crates are otherwise left to
                     rng-crate.

    lints            Every plane-A crate takes exactly `[lints] workspace = true` and no
                     `[lints.*]` table of its own: an own table (for example `[lints.clippy]
                     disallowed_methods = "allow"`) would switch off the workspace denies, the
                     std-transcendental ban and float_arithmetic among them, for that crate
                     without a trace (VER-SPR-003-TEST GAP-9). cargo metadata does not carry
                     lints, so each crate's Cargo.toml is read with tomllib. Tool crates may
                     differ; every crate's declaration is printed.

    clippy-config    The workspace root's clippy.toml is the only clippy configuration in the
                     repository: clippy reads the nearest clippy.toml or .clippy.toml, so a copy
                     in a crate (the root list minus `f64::ln`, say) would quietly replace the
                     disallowed lists for that crate (VER-SPR-003-TEST GAP-12). Every file of
                     either name under the workspace root is reported, except the root's own
                     clippy.toml and the lint fixtures listed in CLIPPY_CONFIG_FIXTURES. Two
                     searches: clippy's own lookup, from each member's manifest directory up to
                     the root, following symlinks (GAP-13); and a crawl of the whole tree that
                     skips only the root's target/ and .git/. A workspace member whose manifest
                     directory is a symlink is reported too: nothing needs one, and it can hide
                     a config from the crawl. CLIPPY_CONF_DIR, which points clippy at another
                     configuration, must not be set.

Sources, wildcards and licences of third-party crates are governed by cargo-deny (deny.toml).

Usage:  python -I bin/lint_arch.py [--manifest-path Cargo.toml]
Exit:   0 clean, 1 violations, 2 the workspace could not be read.
Standard library only (GOV-003 §3).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from collections.abc import Sequence
from dataclasses import dataclass, field, replace
from pathlib import Path
from types import MappingProxyType

PLANES = frozenset({"A", "B", "C", "tool"})
KINDS = frozenset({"core", "machinery", "contract", "system", "tool"})
# REF: docs/13 §3 ("Crate boundaries are law"); docs/09 V-INSP-03. Contract crates (kind
# "contract") are allowed in addition to these names. rh-sample, the vendored samplers, joined by
# the SPR-003 Architect ruling (2026-10-16), item 1.
SYSTEM_ALLOWED_DEPS = frozenset({"rh-core", "rh-ids", "rh-events", "rh-registry", "rh-sample"})
PLANE_A_KINDS = frozenset({"core", "contract", "system"})
SYSTEM_CRATE_PREFIX = "rh-sys-"
# REF: docs/05_architecture/05.0 §6; the SPR-001 audit (relabelling rh-snapshot as a contract crate
# let a system depend on it). The expected (plane, kind) of the crates SPR-001 created, by name, and
# of rh-sample (SPR-003 ruling item 1): bin/lint_forbidden.py exempts that name from its float rule,
# so relabelling the crate out of plane A, where nothing would scan it, must fail here.
PINNED_LABELS = MappingProxyType(
    {
        "rh-ids": ("A", "machinery"),
        "rh-core": ("A", "core"),
        "rh-events": ("A", "machinery"),
        "rh-registry": ("A", "machinery"),
        "rh-contracts": ("A", "contract"),
        "rh-snapshot": ("A", "machinery"),
        "rh-cli": ("tool", "tool"),
        # SPR-004 ruling A: the load-time front end reproduces the reference's float checks.
        "rh-manifest": ("tool", "tool"),
        "rh-sample": ("A", "machinery"),
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
# package name -> (the plane-A crates that may use it, or None for any; the review note; whether
# its default features must be off; the features it may enable, or None for any). The note says
# whether the crate can return a float to plane A. Keep it alphabetical; adding an entry is a
# review, and the reviewer names it in the PR.
ANY_PLANE_A_CRATE = None
NO_DEFAULT_FEATURES = True
DEFAULT_FEATURES_ALLOWED = False
ANY_FEATURES = None
NO_FEATURES: frozenset[str] = frozenset()
# An exact version requirement: `=x.y.z`, optionally with a pre-release or build suffix.
EXACT_REQUIREMENT = re.compile(r"=\s*\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.+-]+)?")
PLANE_A_EXTERNAL_DEPS = MappingProxyType(
    {
        "libm": (
            frozenset({"rh-sample"}),
            "returns f32/f64: the transcendentals of the sampler float zone, the only "
            "plane-A crate allowed float arithmetic (SPR-003 ruling); no features: `arch` would "
            "swap in architecture-specific routines (VER-SPR-003-TEST GAP-10)",
            NO_DEFAULT_FEATURES,
            NO_FEATURES,
        ),
        "serde": (
            ANY_PLANE_A_CRATE,
            "no floats of its own: the (de)serialisation traits and derives",
            DEFAULT_FEATURES_ALLOWED,
            ANY_FEATURES,
        ),
        "toml": (
            frozenset({"rh-registry"}),
            "can return f64 (toml::Value::Float); rh-registry reads only strings, arrays and "
            "tables from the system specs for the ownership lint (SPR-004)",
            DEFAULT_FEATURES_ALLOWED,
            ANY_FEATURES,
        ),
    }
)

# Rule clippy-config: the file names clippy reads, the one allowed place, and the lint fixtures that
# plant a crate-local config on purpose (paths relative to the workspace root). Keep this list tiny.
CLIPPY_CONFIG_NAMES = frozenset({"clippy.toml", ".clippy.toml"})
ROOT_CLIPPY_CONFIG = "clippy.toml"
CLIPPY_CONFIG_FIXTURES = frozenset(
    {
        "tests/lint_fixtures/clippy_config/crate_local/crates/rh-sample/clippy.toml",
        "tests/lint_fixtures/clippy_config/symlink_target/rh-sample/clippy.toml",
    }
)
# The only directories the crawl skips, and only at the workspace root (GAP-13).
SKIPPED_ROOT_DIRS = frozenset({"target", ".git"})

# The [lints] declaration of a crate that takes the workspace's lints and nothing else (rule lints).
INHERITED_LINTS = ("workspace",)

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
    # Third-party edges only, from cargo metadata (None when the document does not say). They do
    # not take part in equality or ordering: an edge is its target and kind.
    req: str | None = field(default=None, compare=False)
    default_features: bool | None = field(default=None, compare=False)
    features: tuple[str, ...] = field(default=(), compare=False)
    platform: str | None = field(default=None, compare=False)  # a `[target.'cfg(…)']` table


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
    # The crate's own [lints] keys, as read from its Cargo.toml: ("workspace",) for exactly
    # `[lints] workspace = true`, which every plane-A crate must declare.
    lints: tuple[str, ...] = INHERITED_LINTS
    # The manifest's directory as cargo metadata gives it (not resolved, so a symlink shows).
    directory: Path | None = None

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

    Workspace edges are deduplicated by (target, kind). Third-party edges are not: every entry of
    the metadata is kept, so a second entry for the same crate (a `[target.…]` table with a loose
    requirement or other features) is judged on its own (VER-SPR-003-TEST GAP-10).
    """
    internal: set[Edge] = set()
    external: list[Edge] = []
    for raw in _as_list(package.get("dependencies", []), "dependencies"):
        dep = _as_dict(raw, "a dependency")
        name, kind = dep.get("name"), dep.get("kind")
        if not isinstance(name, str):
            raise WorkspaceError(f"a dependency has no name: {dep!r}")
        req, defaults = dep.get("req"), dep.get("uses_default_features")
        features, platform = dep.get("features"), dep.get("target")
        edge = Edge(
            target=name,
            kind=kind if isinstance(kind, str) else "normal",
            req=req if isinstance(req, str) else None,
            default_features=defaults if isinstance(defaults, bool) else None,
            features=tuple(sorted(str(f) for f in features)) if isinstance(features, list) else (),
            platform=platform if isinstance(platform, str) else None,
        )
        # A workspace dependency is a path dependency (no registry or git source) on a member.
        if name in workspace and dep.get("source") is None:
            internal.add(edge)
        else:
            external.append(edge)
    external.sort(key=lambda e: (e.target, e.kind, e.platform or "", e.req or ""))
    return tuple(sorted(internal)), tuple(external)


def read_lints(manifest: Path) -> tuple[str, ...]:
    """Return a crate's own [lints] declaration: its keys, `workspace` only when it is true.

    Raises:
        WorkspaceError: The manifest cannot be read or parsed.
    """
    try:
        document = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as err:
        raise WorkspaceError(f"cannot read {manifest}: {err}") from err
    lints = document.get("lints", {})
    if not isinstance(lints, dict):
        return (f"lints = {lints!r}",)
    return tuple(
        sorted(
            key if key != "workspace" or value is True else f"workspace = {value!r}"
            for key, value in lints.items()
        )
    )


def check_lints(crate: Crate) -> list[Violation]:
    """Check that a plane-A crate takes exactly the workspace's lints (VER-SPR-003-TEST GAP-9)."""
    if crate.plane != "A" or crate.lints == INHERITED_LINTS:
        return []
    own = [key for key in crate.lints if key != "workspace"]
    if own:
        tables = ", ".join(f"[lints.{key}]" for key in own)
        problem = f"declares its own lint table(s) {tables}"
    else:
        problem = "does not declare `[lints] workspace = true`"
    return [
        Violation(
            "lints",
            crate.name,
            f"{problem}; a plane-A crate takes exactly `[lints] workspace = true` and no lint "
            "table of its own, or the workspace denies (the std transcendentals, floats, "
            "unwrap) stop applying to it (VER-SPR-003-TEST GAP-9)",
        )
    ]


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
        manifest = package.get("manifest_path")
        if not isinstance(manifest, str):
            raise WorkspaceError(f"package {package.get('name')!r} has no manifest_path")
        crates.append(
            Crate(
                name=str(package.get("name")),
                plane=_declared(package, "plane"),
                kind=_declared(package, "kind"),
                edges=internal,
                external=external,
                lints=read_lints(Path(manifest)),
                directory=Path(manifest).parent,
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
    """Return the phrase naming a non-normal dependency kind and a target table, if any."""
    kind = "" if edge.kind == "normal" else f" (as a {edge.kind}-dependency)"
    return kind + ("" if edge.platform is None else f" (for target {edge.platform})")


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
        if edge.kind == "dev":
            continue
        # VER-SPR-003-TEST GAP-4: a loosened requirement could move plane A's results with a
        # `cargo update`; only an exact pin keeps the reviewed version the built one.
        if edge.req is not None and not EXACT_REQUIREMENT.fullmatch(edge.req.strip()):
            found.append(
                Violation(
                    "plane-A-external",
                    crate.name,
                    f"requires {edge.target}{_via(edge)} as `{edge.req}`; a plane-A third-party "
                    "dependency is pinned exactly, `=x.y.z` (VER-SPR-003-TEST GAP-4)",
                )
            )
        # DECISION: RNG crates are otherwise judged by rng-crate alone, so one edge is never
        # reported twice. REF: BCK-002 A-20.
        if edge.target in UNKEYED_RNG_CRATES | KEYED_RNG_BUILDING_BLOCKS:
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
        allowed, _note, no_defaults, features = entry
        extra = [] if features is None else sorted(set(edge.features) - features)
        if features is not None and extra:
            found.append(
                Violation(
                    "plane-A-external",
                    crate.name,
                    f"enables feature(s) {', '.join(extra)} of {edge.target}{_via(edge)}; the "
                    f"review allows {', '.join(sorted(features)) or 'none'} "
                    "(PLANE_A_EXTERNAL_DEPS; VER-SPR-003-TEST GAP-10)",
                )
            )
        if no_defaults and edge.default_features:
            found.append(
                Violation(
                    "plane-A-external",
                    crate.name,
                    f"uses {edge.target}{_via(edge)} with its default features; plane A takes "
                    f"{edge.target} with `default-features = false` (PLANE_A_EXTERNAL_DEPS; "
                    "VER-SPR-003-TEST GAP-4)",
                )
            )
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
    violations += [v for crate in crates if crate.declared_ok for v in check_lints(crate)]
    for crate in crates:
        for edge in crate.edges:
            target = by_name.get(edge.target)
            if target is None or not (crate.declared_ok and target.declared_ok):
                continue
            violations.extend(check_edge(crate, edge, target))
    return violations


def clippy_configs(root: Path) -> list[str]:
    """Return every clippy.toml or .clippy.toml under ``root``, relative and sorted.

    Only the root's own target/ and .git/ are skipped; symlinked directories are not entered
    (os.walk's default), which is why ``member_clippy_configs`` exists.
    """
    found: list[str] = []
    for directory, subdirectories, files in os.walk(root):
        if Path(directory) == root:
            subdirectories[:] = [d for d in subdirectories if d not in SKIPPED_ROOT_DIRS]
        subdirectories.sort()
        found += [
            (Path(directory) / name).relative_to(root).as_posix()
            for name in files
            if name in CLIPPY_CONFIG_NAMES
        ]
    return sorted(found)


def member_clippy_configs(root: Path, crates: Sequence[Crate]) -> list[str]:
    """Return the clippy configs clippy itself would find for each member, relative and sorted.

    Clippy looks in the crate's manifest directory and then each parent. This walks from each
    member's directory (unresolved, so through any symlink) up to ``root``, testing both names
    with Path.exists(), which follows symlinks.
    """
    found: set[str] = set()
    for crate in crates:
        if crate.directory is None:
            continue
        directory = crate.directory
        while True:
            for name in CLIPPY_CONFIG_NAMES:
                if (directory / name).exists():
                    found.add(Path(os.path.relpath(directory / name, root)).as_posix())
            if directory in (root, directory.parent):
                break
            directory = directory.parent
    return sorted(found)


def check_clippy_configs(root: Path, crates: Sequence[Crate] = ()) -> list[Violation]:
    """Check that the root clippy.toml is the only clippy config (VER-SPR-003-TEST GAP-12, 13)."""
    allowed = CLIPPY_CONFIG_FIXTURES | {ROOT_CLIPPY_CONFIG}
    found = [
        Violation(
            "clippy-config",
            crate.name,
            f"its manifest directory {crate.directory} is a symlink; a workspace member lives "
            "in the tree, where every check can see it (VER-SPR-003-TEST GAP-13)",
        )
        for crate in crates
        if crate.directory is not None and crate.directory.is_symlink()
    ]
    if os.environ.get("CLIPPY_CONF_DIR"):
        found.append(
            Violation(
                "clippy-config",
                "CLIPPY_CONF_DIR",
                f"is set to {os.environ['CLIPPY_CONF_DIR']!r}; clippy would read its configuration "
                "there instead of the workspace root's clippy.toml (VER-SPR-003-TEST GAP-12)",
            )
        )
    return found + [
        Violation(
            "clippy-config",
            path,
            "a clippy configuration other than the workspace root's clippy.toml; clippy reads the "
            "nearest one, so it would replace the disallowed lists (the std transcendentals, "
            "HashMap, Instant, mul_add) for the crates below it (VER-SPR-003-TEST GAP-12)",
        )
        for path in sorted(set(clippy_configs(root)) | set(member_clippy_configs(root, crates)))
        if path not in allowed
    ]


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
        f"external: {names(crate.external)}; lints: {', '.join(crate.lints) or '(none)'}"
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
    root = args.manifest_path.absolute().parent
    violations = check_workspace(crates) + check_clippy_configs(root, crates)
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
