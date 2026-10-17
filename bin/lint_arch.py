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

    unsafe-allow     GOV-003 §2's reviewed unsafe allow-list is UNSAFE_ALLOW_LIST (SPR-009 T-05). A
                     crate not on it keeps #![forbid(unsafe_code)]; a listed crate relaxes it to
                     deny only, with its other lint tables equal to the workspace's, and holds
                     `unsafe` or `allow(unsafe_code` only in its listed files.
    global-allocator The only #[global_allocator] is in ALLOCATOR_CRATE's bench targets (V-PERF-11;
                     SPR-009 ruling 5): the counting allocator never reaches a shipped binary.
    count-alloc      ALLOCATOR_FEATURES (count-alloc, arena-alloc) are enabled by no dependency
                     edge, are not default features, and are required by no target of
                     ALLOCATOR_CRATE but a bench.

    engine-purity    The engine only engines (ADR-0035 §6; docs/13 §3; V-INSP-05; BCK-002 A-14). No
                     engine crate (ENGINE_CRATES, every rh-sys-* and rh-contract-* crate, and
                     every plane-A crate) reaches a crate on ENGINE_DENY_LIST, the one deny-list
                     of media, HTTP-UI and LLM-client crates, through its normal or build
                     dependencies, directly or transitively; dev-dependencies are exempt. Members
                     are followed through their cargo-metadata edges, everything else through the
                     resolved graph in the workspace's Cargo.lock, read offline. A missing lock,
                     or a dependency the lock does not hold, is a violation (never fail open).

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
        # SPR-012: the first real system crate (M2.demography) and the worldgen stub (ruling 10).
        "rh-sys-demography": ("A", "system"),
        "rh-worldgen": ("A", "machinery"),
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
# The review note shared by the four Arrow crates of the keyframe format (SPR-007 T-02).
ARROW_NOTE = (
    "Arrow can hand plane A floats (Float16/32/64 arrays, types and builders; half::f16). The "
    "keyframe format stores only Utf8 and Binary columns: every row is the canonical integer "
    "encoding the state hash reads, and the P0 tables hold no float; the float-yielding names "
    "are refused by FLOAT_YIELDING_APIS. Exact pin, no default features. Transitive crates reach "
    "plane A only through this API: ahash and hashbrown (Arrow-internal maps, never iterated by "
    "plane-A code), getrandom (ahash's runtime seeding: no keyframe byte depends on it, and the "
    "state hash is checked on every read), arrow-data, arrow-select, arrow-cmp, chrono (temporal "
    "types, unused), half, flatbuffers, num-*, zstd. cargo-deny bans non-crates.io sources and "
    "wildcards across the whole graph"
)
PLANE_A_EXTERNAL_DEPS = MappingProxyType(
    {
        "arrow-array": (
            frozenset({"rh-snapshot"}),
            ARROW_NOTE,
            NO_DEFAULT_FEATURES,
            NO_FEATURES,
        ),
        "arrow-buffer": (
            frozenset({"rh-snapshot"}),
            ARROW_NOTE,
            NO_DEFAULT_FEATURES,
            NO_FEATURES,
        ),
        "arrow-ipc": (
            frozenset({"rh-snapshot"}),
            ARROW_NOTE
            + "; feature zstd only, whose C build (zstd-sys) takes CFLAGS=-ffp-contract=off from "
            ".cargo/config.toml (SPR-007 ruling 4; check_all checks the cc output)",
            NO_DEFAULT_FEATURES,
            frozenset({"zstd"}),
        ),
        "arrow-schema": (
            frozenset({"rh-snapshot"}),
            ARROW_NOTE,
            NO_DEFAULT_FEATURES,
            NO_FEATURES,
        ),
        "libm": (
            frozenset({"rh-sample"}),
            "returns f32/f64: the transcendentals of the sampler float zone, the only "
            "plane-A crate allowed float arithmetic (SPR-003 ruling); no features: `arch` would "
            "swap in architecture-specific routines (VER-SPR-003-TEST GAP-10)",
            NO_DEFAULT_FEATURES,
            NO_FEATURES,
        ),
        "rusqlite": (
            frozenset({"rh-snapshot"}),
            "can return f64 (FromSql for f64, Value::Real); the input journal stores no REAL: its "
            "tables are STRICT with INTEGER, TEXT and BLOB columns only, and rh-snapshot names no "
            "float type (SPR-007 ruling 3); feature bundled only, whose C build takes "
            "CFLAGS=-ffp-contract=off from .cargo/config.toml (ruling 4). Its transitive crates "
            "(libsqlite3-sys, bitflags, smallvec, fallible-iterator) reach plane A only through "
            "this API; cargo-deny bans non-crates.io sources and wildcards across the whole graph",
            NO_DEFAULT_FEATURES,
            frozenset({"bundled"}),
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
        "twox-hash": (
            frozenset({"rh-snapshot"}),
            "no floats: XXH3-64 for the plane hashes and XXH3-128 for the journal's payload hash "
            "(SPR-007 ruling 1); default features off (they add rand and std), features "
            "xxhash3_64 and xxhash3_128 only. Its API hands plane A only u64, u128 and bytes; with "
            "these features it has no transitive dependencies",
            NO_DEFAULT_FEATURES,
            frozenset({"xxhash3_64", "xxhash3_128"}),
        ),
    }
)

# REF: VER-SPR-007-TEST GAP-1 (Architect ruling, 2026-10-16). The API tokens through which a
# reviewed dependency can hand plane A a float without plane-A code naming a float type (a matched
# `ValueRef::Real(x)` and `x.to_bits()`, a comparison of two `Real`s). bin/lint_forbidden.py refuses
# them, as it refuses a float, in the source of the crates PLANE_A_EXTERNAL_DEPS allows to use the
# dependency; the "no floats" claim of a review note rests on this list. Keys are entries of
# PLANE_A_EXTERNAL_DEPS (python/tests checks it). Keep it alphabetical.
FLOAT_YIELDING_APIS = MappingProxyType(
    {
        # Each Arrow name under the crate that exports it, so a use is reported once; arrow-ipc
        # exports no float of its own (it reads and writes the arrays above). SPR-007 T-02.
        "arrow-array": (
            "Float16Array",
            "Float16Builder",
            "Float16Type",
            "Float32Array",
            "Float32Builder",
            "Float32Type",
            "Float64Array",
            "Float64Builder",
            "Float64Type",
        ),
        "arrow-buffer": ("f16", "half"),
        "arrow-schema": ("Float16", "Float32", "Float64"),
        "rusqlite": ("Real", "as_f64", "as_f64_or_null"),
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

# REF: GOV-003 §2 ("#![forbid(unsafe_code)], except crates on a reviewed allow-list"); SPR-009
# T-05, Architect ruling 2026-10-16. THE reviewed unsafe allow-list: crate -> the files, relative
# to the crate, that may hold `unsafe` and `allow(unsafe_code`. A listed crate declares its own
# [lints] equal to the workspace's but for `unsafe_code = "deny"`. Adding an entry is a review.
UNSAFE_ALLOW_LIST = MappingProxyType({"rh-bench": ("src/alloc_count.rs",)})
UNSAFE_RELAXED = "deny"
# V-PERF-11's counting allocator (SPR-009 ruling 5): compiled only behind this feature of this
# crate, which only that crate's bench targets enable, and the only #[global_allocator] in the
# workspace, declared in a bench target. So it can never reach a shipped binary.
ALLOCATOR_CRATE = "rh-bench"
ALLOCATOR_FEATURES = ("arena-alloc", "count-alloc")
ALLOCATOR_TARGET_DIR = "benches"
# Anywhere inside an attribute: `#[global_allocator]` and `#[cfg_attr(cond, global_allocator)]`
# alike (VER-SPR-009-TEST GAP-6).
GLOBAL_ALLOCATOR = re.compile(r"#\s*!?\s*\[[^\]]*\bglobal_allocator\b")
ALLOW_UNSAFE = re.compile(r"\ballow\s*\(\s*unsafe_code\b")
UNSAFE_TOKEN = re.compile(r"\bunsafe\b")

# REF: docs/13 §3 ("The engine only engines"); ADR-0035 §6; 05.0 §6; docs/09 V-INSP-05; EVO-4;
# BCK-002 A-14 (SPR-011 T-00). The engine crates: the union of the lists in docs/13 §3, 05.0 §6 and
# ADR-0035 §6, planned ones included, and SPR-011's new rh-stream. Every crate named rh-sys-* or
# rh-contract-*, and every crate whose effective plane is A, is an engine crate too, so a new
# plane-A crate cannot be left out of the rule by forgetting to list it. rh-bench and rh-golden are
# not engine crates, by design: they are tool crates that measure and replay the engine from outside
# (SPR-009, SPR-008) and no engine crate depends on them; leaving them off is not a gap.
ENGINE_CRATES = frozenset(
    {
        "rh-cli",
        "rh-contracts",
        "rh-core",
        "rh-events",
        "rh-ids",
        "rh-manifest",
        "rh-minds",
        "rh-registry",
        "rh-sample",
        "rh-server",
        "rh-sifter",
        "rh-snapshot",
        "rh-stream",
        "rh-worldgen",
    }
)
ENGINE_CRATE_PREFIXES = (SYSTEM_CRATE_PREFIX, CONTRACT_CRATE_PREFIX)
# THE engine-purity deny-list, defined here and nowhere else (python/tests checks that): package
# name -> what it is. docs/13 §3 names the categories (media, HTTP-UI, LLM client) and nine
# examples; the rest are the common crates of the same categories. Matched by package name over the
# transitive normal and build dependencies of every engine crate. Keep it alphabetical; adding an
# entry is a review, removing one is a C2 change (ADR-0035 §6).
ENGINE_DENY_LIST = MappingProxyType(
    {
        "actix-web": "HTTP UI (web server framework)",
        "async-openai": "LLM client",
        "axum": "HTTP UI (web server framework)",
        "candle-core": "LLM inference",
        "cpal": "media (audio I/O)",
        "gif": "media (image codec)",
        "hound": "media (audio codec)",
        "hyper": "HTTP (the base of web servers and clients not listed here)",
        "image": "media (images)",
        "jpeg-decoder": "media (image codec)",
        "kira": "media (audio engine)",
        "lewton": "media (audio codec)",
        "llama-cpp-2": "LLM inference",
        "minimp3": "media (audio codec)",
        "ollama-rs": "LLM client",
        "png": "media (image codec)",
        "reqwest": "HTTP client (LLM and web APIs)",
        "rocket": "HTTP UI (web server framework)",
        "rodio": "media (audio playback)",
        "symphonia": "media (audio codecs)",
        "tiktoken-rs": "LLM tokenizer",
        "tokenizers": "LLM tokenizer",
        "tower-http": "HTTP UI (web middleware)",
        "tts": "TTS (text to speech; docs/13 §3)",
        "ureq": "HTTP client (LLM and web APIs)",
        "warp": "HTTP UI (web server framework)",
    }
)
ENGINE_PURITY_RULE = "engine-purity"
LOCK_FILE = "Cargo.lock"

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


def _manifest(crate: Crate) -> dict[str, object]:
    """The crate's Cargo.toml as TOML."""
    if crate.directory is None:
        return {}
    path = crate.directory / "Cargo.toml"
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as err:
        raise WorkspaceError(f"cannot read {path}: {err}") from err


def workspace_lints(root: Path) -> dict[str, object]:
    """The root manifest's [workspace.lints] tables."""
    try:
        document = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as err:
        raise WorkspaceError(f"cannot read {root / 'Cargo.toml'}: {err}") from err
    workspace = document.get("workspace", {})
    lints = workspace.get("lints", {}) if isinstance(workspace, dict) else {}
    return dict(lints) if isinstance(lints, dict) else {}


def check_unsafe_lints(crate: Crate, inherited: dict[str, object]) -> list[Violation]:
    """GOV-003 §2: only an allow-listed crate relaxes unsafe_code, and only to deny.

    A listed crate's own lint tables must equal the workspace's but for that one key, so its
    other denies cannot drift away (SPR-009 T-05).
    """
    if crate.lints == INHERITED_LINTS:
        return []
    own = _manifest(crate).get("lints", {})
    own = own if isinstance(own, dict) else {}
    rust = own.get("rust", {})
    level = rust.get("unsafe_code") if isinstance(rust, dict) else None
    if level == "forbid":
        return []
    if crate.name not in UNSAFE_ALLOW_LIST:
        return [
            Violation(
                "unsafe-allow",
                crate.name,
                f"declares unsafe_code = {level!r}, but only crates on UNSAFE_ALLOW_LIST may relax "
                "#![forbid(unsafe_code)] (GOV-003 §2)",
            )
        ]
    expected = {k: dict(v) if isinstance(v, dict) else v for k, v in inherited.items()}
    rust_expected = expected.setdefault("rust", {})
    if isinstance(rust_expected, dict):
        rust_expected["unsafe_code"] = UNSAFE_RELAXED
    if own != expected:
        return [
            Violation(
                "unsafe-allow",
                crate.name,
                "its own [lints] must equal [workspace.lints] with only unsafe_code = "
                f'"{UNSAFE_RELAXED}" (GOV-003 §2; SPR-009 T-05)',
            )
        ]
    return []


def _sources(crate: Crate) -> list[Path]:
    """Every .rs file of the crate, outside the crate's own top-level `target/` directory.

    Only the first component below the crate's directory is tested, as lint_forbidden does: a
    checkout under a directory named `target` is scanned (SPR-009 review 4), and so is a source
    directory named `target` inside the crate, such as `src/target/` (VER-SPR-009-TEST GAP-21).
    """
    directory = crate.directory
    if directory is None:
        return []
    return sorted(
        p for p in directory.rglob("*.rs") if p.relative_to(directory).parts[0] != "target"
    )


def check_allocator_sources(crate: Crate) -> list[Violation]:
    """No #[global_allocator] but in the allocator crate's benches; unsafe only in listed files.

    The unsafe part applies to listed crates; others keep #![forbid(unsafe_code)] (SPR-009 T-05).
    """
    if crate.directory is None:
        return []
    allowed = {crate.directory / f for f in UNSAFE_ALLOW_LIST.get(crate.name, ())}
    found = []
    for path in _sources(crate):
        # Comment lines (doc comments included) name things; they declare nothing.
        text = "\n".join(
            line
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines()
            if not line.lstrip().startswith("//")
        )
        rel = path.relative_to(crate.directory).as_posix()
        in_bench = crate.name == ALLOCATOR_CRATE and rel.startswith(f"{ALLOCATOR_TARGET_DIR}/")
        if GLOBAL_ALLOCATOR.search(text) and not in_bench:
            found.append(
                Violation(
                    "global-allocator",
                    crate.name,
                    f"{rel} declares a #[global_allocator]; only {ALLOCATOR_CRATE}'s bench "
                    "targets may (V-PERF-11; SPR-009 ruling 5)",
                )
            )
        listed = crate.name in UNSAFE_ALLOW_LIST
        if (
            listed
            and path not in allowed
            and (ALLOW_UNSAFE.search(text) or UNSAFE_TOKEN.search(text))
        ):
            found.append(
                Violation(
                    "unsafe-allow",
                    crate.name,
                    f"{rel} holds unsafe code; the allow-list admits only "
                    f"{', '.join(UNSAFE_ALLOW_LIST[crate.name])} (GOV-003 §2)",
                )
            )
    return found


def check_allocator_feature(crate: Crate) -> list[Violation]:
    """The counting allocator's feature is enabled by the allocator crate's bench targets only."""
    found = [
        Violation(
            "count-alloc",
            crate.name,
            f"enables {ALLOCATOR_CRATE}/{feature} on its {e.kind} dependency; only "
            f"{ALLOCATOR_CRATE}'s bench targets may (V-PERF-11; SPR-009 ruling 5)",
        )
        for e in crate.edges
        for feature in ALLOCATOR_FEATURES
        if e.target == ALLOCATOR_CRATE and feature in e.features
    ]
    if crate.name != ALLOCATOR_CRATE:
        return found
    manifest = _manifest(crate)
    features = manifest.get("features", {})
    defaults = features.get("default", []) if isinstance(features, dict) else []
    for feature in ALLOCATOR_FEATURES:
        if isinstance(defaults, list) and feature in defaults:
            found.append(Violation("count-alloc", crate.name, f"{feature} is a default feature"))
    for table in ("bin", "lib", "test", "example"):
        targets = manifest.get(table, [])
        for target in targets if isinstance(targets, list) else [targets]:
            required = target.get("required-features", []) if isinstance(target, dict) else []
            for feature in set(ALLOCATOR_FEATURES) & set(required):
                found.append(
                    Violation(
                        "count-alloc",
                        crate.name,
                        f"a [{table}] target requires {feature}; only bench targets may",
                    )
                )
    return found


def check_allocator(crates: Sequence[Crate], root: Path) -> list[Violation]:
    """The unsafe allow-list and the counting allocator's containment (SPR-009 T-05)."""
    inherited = workspace_lints(root)
    found = []
    for crate in crates:
        found += check_unsafe_lints(crate, inherited)
        found += check_allocator_sources(crate)
        found += check_allocator_feature(crate)
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


@dataclass(frozen=True)
class LockPackage:
    """One `[[package]]` of a Cargo.lock: its name, version and resolved dependency entries."""

    name: str
    version: str
    # Raw entries: "name", "name version" or "name version (source)".
    dependencies: tuple[str, ...]


def read_lock(path: Path) -> tuple[LockPackage, ...] | None:
    """Return the packages of a Cargo.lock, or None when the file does not exist.

    The lock is read rather than a full `cargo metadata`, which would need every crate's source
    fetched: it holds the whole resolved graph, offline, and lists dev-dependencies only for
    workspace members, whose edges come from cargo metadata with their kinds instead.

    Raises:
        WorkspaceError: The file exists but cannot be read or does not have the expected shape.
    """
    if not path.is_file():
        return None
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as err:
        raise WorkspaceError(f"cannot read {path}: {err}") from err
    packages = []
    for raw in _as_list(document.get("package", []), f"{path} [[package]]"):
        package = _as_dict(raw, f"a [[package]] of {path}")
        name, version = package.get("name"), package.get("version")
        if not isinstance(name, str) or not isinstance(version, str):
            raise WorkspaceError(f"a [[package]] of {path} has no name or version: {package!r}")
        deps = _as_list(package.get("dependencies", []), f"the dependencies of {name} in {path}")
        if not all(isinstance(d, str) for d in deps):
            raise WorkspaceError(f"the dependencies of {name} in {path} are not strings")
        packages.append(LockPackage(name, version, tuple(str(d) for d in deps)))
    return tuple(packages)


def is_engine_crate(crate: Crate) -> bool:
    """Whether the engine-purity rule applies to a crate (ENGINE_CRATES; on its pinned labels)."""
    return (
        crate.name in ENGINE_CRATES
        or crate.name.startswith(ENGINE_CRATE_PREFIXES)
        or crate.plane == "A"
    )


# A node of the dependency walk: (package name, version). The version is None for a workspace
# member and "" for a dependency the lock does not hold.
_Node = tuple[str, str | None]
UNRESOLVED = ""


@dataclass(frozen=True)
class DependencyGraph:
    """The non-dev dependency graph: members from cargo metadata, the rest from Cargo.lock."""

    members: MappingProxyType[str, Crate]
    locked: MappingProxyType[str, tuple[LockPackage, ...]]

    @classmethod
    def of(cls, crates: Sequence[Crate], lock: Sequence[LockPackage]) -> "DependencyGraph":
        """Index the members by name and the locked packages by name."""
        by_name: dict[str, list[LockPackage]] = {}
        for package in lock:
            by_name.setdefault(package.name, []).append(package)
        return cls(
            MappingProxyType({crate.name: crate for crate in crates}),
            MappingProxyType({name: tuple(found) for name, found in by_name.items()}),
        )

    def _resolve(self, name: str, version: str | None = None) -> list[_Node]:
        """The nodes a dependency on ``name`` (at ``version``, when the lock names one) reaches."""
        if name in self.members:
            return [(name, None)]
        nodes: list[_Node] = [
            (name, p.version) for p in self.locked.get(name, ()) if version in (None, p.version)
        ]
        return nodes or [(name, UNRESOLVED)]

    def successors(self, node: _Node) -> list[_Node]:
        """A node's normal and build dependencies (a member's dev-dependencies are left out)."""
        name, version = node
        if version is None:
            member = self.members[name]
            edges = (*member.edges, *member.external)
            return [n for e in edges if e.kind != "dev" for n in self._resolve(e.target)]
        if version == UNRESOLVED:
            return []
        package = next(p for p in self.locked[name] if p.version == version)
        entries = (entry.split() for entry in package.dependencies)
        return [n for parts in entries for n in self._resolve(parts[0], *parts[1:2])]

    def walk(self, start: str) -> list[list[str]]:
        """Return the shortest path from ``start`` to every node it reaches, breadth first.

        A path ends at the node it reaches; an unresolved node's path ends in "".
        """
        first: _Node = (start, None)
        parent: dict[_Node, _Node | None] = {first: None}
        queue, order = [first], []
        while queue:
            node = queue.pop(0)
            order.append(node)
            for succ in self.successors(node):
                if succ not in parent:
                    parent[succ] = node
                    queue.append(succ)
        paths: list[list[str]] = []
        for node in order[1:]:
            path: list[str] = []
            at: _Node | None = node
            while at is not None:
                path.append(at[0])
                at = parent[at]
            paths.append([*reversed(path), *([UNRESOLVED] if node[1] == UNRESOLVED else [])])
        return paths


def check_engine_purity(
    crates: Sequence[Crate], lock: Sequence[LockPackage] | None
) -> list[Violation]:
    """Check that no engine crate reaches a deny-listed crate (V-INSP-05; BCK-002 A-14).

    The walk starts at an engine crate's normal and build edges (never its dev-dependencies: test
    code does not ship in the engine), follows workspace members through their own non-dev edges
    from cargo metadata, and every other crate through its resolved dependencies in Cargo.lock.
    A missing lock, or a dependency the lock does not hold, is a violation: the transitive graph
    would go unchecked (never fail open).
    """
    crates = [effective(crate) for crate in crates]
    engines = [crate for crate in crates if crate.declared_ok and is_engine_crate(crate)]
    if lock is None:
        return [
            Violation(
                ENGINE_PURITY_RULE,
                "workspace",
                f"no {LOCK_FILE} beside the workspace manifest: the transitive dependencies of "
                "the engine crates are not checked (V-INSP-05); generate and commit it",
            )
            for _ in engines[:1]
        ]
    graph = DependencyGraph.of(crates, lock)
    found: list[Violation] = []
    for crate in engines:
        reported: set[str] = set()
        for path in graph.walk(crate.name):
            if path[-1] == UNRESOLVED:
                found.append(
                    Violation(
                        ENGINE_PURITY_RULE,
                        crate.name,
                        f"its dependency {path[-2]} is not in {LOCK_FILE}, so what it depends on "
                        "is not checked (V-INSP-05); update the lock",
                    )
                )
            elif path[-1] in ENGINE_DENY_LIST and path[-1] not in reported:
                reported.add(path[-1])
                found.append(
                    Violation(
                        ENGINE_PURITY_RULE,
                        crate.name,
                        f"engine crate depends on {path[-1]} ({ENGINE_DENY_LIST[path[-1]]}) "
                        f"through {' -> '.join(path)}; the engine only engines: no media, "
                        "HTTP-UI or LLM-client crate among its normal or build dependencies, "
                        "direct or transitive (ADR-0035 §6; V-INSP-05)",
                    )
                )
    return found


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
    try:
        allocator = check_allocator(crates, root)
    except WorkspaceError as err:
        print(f"lint_arch: cannot read the workspace: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    try:
        lock = read_lock(root / LOCK_FILE)
    except WorkspaceError as err:
        print(f"lint_arch: cannot read the workspace: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    violations = (
        check_workspace(crates)
        + check_engine_purity(crates, lock)
        + check_clippy_configs(root, crates)
        + allocator
    )
    print(f"lint_arch: {args.manifest_path}")
    for crate in crates:
        print(describe(crate))
    for violation in violations:
        print(violation.render())
    edge_count = sum(len(crate.edges) for crate in crates)
    engines = sum(1 for crate in crates if is_engine_crate(effective(crate)))
    print(
        f"lint_arch: {len(crates)} crates, {edge_count} workspace edges, {engines} engine crates "
        f"checked against {len(ENGINE_DENY_LIST)} deny-listed crates over "
        f"{len(lock or ())} locked packages, {len(violations)} violations"
    )
    return EXIT_VIOLATIONS if violations else EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
