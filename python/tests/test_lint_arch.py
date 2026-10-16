"""Unit tests for bin/lint_arch.py: the crate-boundary law.

Refs: SPR-001 T-04; docs/09 V-INSP-03 (EVO-1), V-PLN-01 (DET-12); docs/13 §3; ADR-0024.
"""

from pathlib import Path

import pytest
from lint_arch import (
    EXIT_OK,
    EXIT_UNREADABLE,
    EXIT_VIOLATIONS,
    PLANE_A_EXTERNAL_DEPS,
    UNKEYED_RNG_CRATES,
    Crate,
    Edge,
    WorkspaceError,
    check_workspace,
    crates_from_metadata,
    main,
)

REPO = Path(__file__).resolve().parents[2]
ARCH_FIXTURES = REPO / "tests" / "lint_fixtures" / "arch"


def crate(
    name: str,
    plane: str | None = "A",
    kind: str | None = "machinery",
    deps: tuple[str, ...] = (),
    dep_kind: str = "normal",
) -> Crate:
    """Build a crate with edges of one dependency kind."""
    return Crate(name, plane, kind, tuple(Edge(d, dep_kind) for d in deps))


def found(crates: list[Crate]) -> list[tuple[str, str]]:
    """Return (rule, crate) for every violation."""
    return [(v.rule, v.crate) for v in check_workspace(crates)]


def engine(*extra: Crate) -> list[Crate]:
    """Return the workspace's crates (SPR-001, plus rh-manifest from SPR-004) and ``extra``."""
    return [
        crate("rh-ids"),
        crate("rh-core", kind="core", deps=("rh-ids",)),
        crate("rh-events"),
        crate("rh-registry"),
        crate("rh-contracts", kind="contract"),
        crate("rh-snapshot"),
        crate("rh-cli", plane="tool", kind="tool", deps=("rh-core", "rh-snapshot")),
        crate("rh-manifest", plane="tool", kind="tool"),
        *extra,
    ]


def test_a_clean_layout_passes() -> None:
    """Allowed edges of every kind produce no violation."""
    crates = engine(
        crate("rh-contract-economy", kind="contract"),
        crate(
            "rh-sys-economy",
            kind="system",
            deps=("rh-core", "rh-ids", "rh-events", "rh-registry", "rh-contract-economy"),
        ),
        crate("rh-sifter", plane="C", deps=("rh-events", "rh-snapshot")),
        crate("rh-observe", plane="B", deps=("rh-core",)),
        crate("rh-server", plane="tool", kind="tool", deps=("rh-sifter", "rh-observe")),
    )
    assert found(crates) == []


def test_a_system_depending_on_a_system_is_v_insp_03() -> None:
    """Systems never import each other."""
    crates = engine(
        crate("rh-sys-war", kind="system", deps=("rh-sys-polity",)),
        crate("rh-sys-polity", kind="system"),
    )
    assert found(crates) == [("V-INSP-03", "rh-sys-war")]


@pytest.mark.parametrize("target", ["rh-snapshot", "rh-cli"])
def test_a_system_depending_on_a_non_allowed_crate_is_v_insp_03(target: str) -> None:
    """Only rh-core, rh-ids, rh-events, rh-registry and contract crates are allowed."""
    crates = engine(crate("rh-sys-war", kind="system", deps=(target,)))
    assert ("V-INSP-03", "rh-sys-war") in found(crates)


@pytest.mark.parametrize(
    "target", ["rh-core", "rh-ids", "rh-events", "rh-registry", "rh-contracts"]
)
def test_a_system_may_depend_on_each_allowed_crate(target: str) -> None:
    """Each crate on the allow-list is accepted."""
    assert found(engine(crate("rh-sys-war", kind="system", deps=(target,)))) == []


@pytest.mark.parametrize("plane", ["B", "C"])
def test_plane_a_depending_on_plane_b_or_c_is_v_pln_01(plane: str) -> None:
    """Plane A never depends on observation or narrative crates."""
    crates = engine(crate("rh-view", plane=plane), crate("rh-minds", deps=("rh-view",)))
    assert found(crates) == [("V-PLN-01", "rh-minds")]


def test_plane_a_depending_on_a_tool_crate_breaks_the_closure() -> None:
    """A tool crate would launder planes B and C and unscanned APIs into plane A."""
    crates = engine(crate("rh-minds", deps=("rh-cli",)))
    assert found(crates) == [("plane-A-closure", "rh-minds")]


@pytest.mark.parametrize("dep_kind", ["dev", "build"])
def test_dev_and_build_dependencies_are_checked(dep_kind: str) -> None:
    """Every dependency kind counts, not only normal dependencies."""
    crates = engine(
        crate("rh-observe", plane="B"),
        crate("rh-minds", deps=("rh-observe",), dep_kind=dep_kind),
    )
    violations = check_workspace(crates)
    assert [(v.rule, v.crate) for v in violations] == [("V-PLN-01", "rh-minds")]
    assert f"as a {dep_kind}-dependency" in violations[0].message


def test_missing_metadata_is_reported_for_plane_and_kind() -> None:
    """Every workspace crate must declare both keys."""
    assert found(engine(crate("rh-new", plane=None, kind=None))) == [
        ("metadata", "rh-new"),
        ("metadata", "rh-new"),
    ]


@pytest.mark.parametrize(
    ("plane", "kind"),
    [("D", "machinery"), ("A", "service"), ("1", "core"), ("a", "core")],
)
def test_values_outside_the_vocabularies_are_reported(plane: str, kind: str) -> None:
    """Planes and kinds come from fixed vocabularies (case-sensitive)."""
    assert ("metadata", "rh-new") in found(engine(crate("rh-new", plane=plane, kind=kind)))


@pytest.mark.parametrize(
    ("name", "plane", "kind"),
    [
        ("rh-new", "B", "core"),
        ("rh-contract-new", "C", "contract"),
        ("rh-new", "tool", "machinery"),
        ("rh-new", "A", "tool"),
        ("rh-new", "B", "tool"),
    ],
)
def test_inconsistent_plane_and_kind_are_reported(name: str, plane: str, kind: str) -> None:
    """Core, contract and system crates are plane A; plane tool goes with kind tool."""
    assert found(engine(crate(name, plane=plane, kind=kind))) == [("metadata", name)]


PINNED = {
    "rh-ids": ("A", "machinery"),
    "rh-core": ("A", "core"),
    "rh-events": ("A", "machinery"),
    "rh-registry": ("A", "machinery"),
    "rh-contracts": ("A", "contract"),
    "rh-snapshot": ("A", "machinery"),
    "rh-cli": ("tool", "tool"),
    "rh-manifest": ("tool", "tool"),
}


@pytest.mark.parametrize("name", sorted(PINNED))
def test_each_pinned_crate_must_keep_its_labels(name: str) -> None:
    """The SPR-001 crates and rh-manifest cannot be relabelled, even to another valid one."""
    relabel = ("A", "core") if PINNED[name] != ("A", "core") else ("A", "machinery")
    crates = [crate(name, *relabel) if c.name == name else c for c in engine()]
    assert ("metadata", name) in found(crates)


def test_a_relabelled_crate_does_not_open_an_edge() -> None:
    """rh-snapshot declared as a contract crate: reported, and the system edge is still judged."""
    crates = [
        crate("rh-snapshot", kind="contract") if c.name == "rh-snapshot" else c for c in engine()
    ]
    crates.append(crate("rh-sys-war", kind="system", deps=("rh-snapshot",)))
    # Two label problems (the pin, and kind "contract" outside rh-contract-*), plus the edge.
    assert sorted(found(crates)) == [
        ("V-INSP-03", "rh-sys-war"),
        ("metadata", "rh-snapshot"),
        ("metadata", "rh-snapshot"),
    ]


@pytest.mark.parametrize(
    ("plane", "kind"), [("A", "machinery"), ("A", "contract"), ("B", "system")]
)
def test_system_named_crates_are_plane_a_systems(plane: str, kind: str) -> None:
    """rh-sys-* must be declared (A, system)."""
    assert ("metadata", "rh-sys-war") in found(engine(crate("rh-sys-war", plane, kind)))


@pytest.mark.parametrize(
    ("name", "ok"),
    [
        ("rh-contracts", True),
        ("rh-contract-economy", True),
        ("rh-shadow", False),
        ("rh-contractor", False),
    ],
)
def test_kind_contract_belongs_to_contract_named_crates(name: str, ok: bool) -> None:
    """Only rh-contracts and rh-contract-* may declare kind contract."""
    crates = [c for c in engine() if c.name != name] + [crate(name, kind="contract")]
    assert (found(crates) == []) is ok


@pytest.mark.parametrize(("name", "kind"), [("rh-sys-war", "machinery"), ("rh-war", "system")])
def test_system_names_and_system_kind_go_together(name: str, kind: str) -> None:
    """A system cannot dodge the boundary law by declaring another kind, nor the reverse."""
    assert found(engine(crate(name, kind=kind))) == [("metadata", name)]


def test_edges_from_crates_with_invalid_metadata_are_not_judged() -> None:
    """The metadata violation is reported once; its edges are not guessed at."""
    crates = engine(crate("rh-new", plane=None, kind=None, deps=("rh-cli",)))
    assert [rule for rule, _ in found(crates)] == ["metadata", "metadata"]


def with_external(base: Crate, *deps: str, dep_kind: str = "normal") -> Crate:
    """Return ``base`` with third-party dependencies added."""
    return Crate(
        base.name, base.plane, base.kind, base.edges, tuple(Edge(d, dep_kind) for d in deps)
    )


@pytest.mark.parametrize("rng", sorted(UNKEYED_RNG_CRATES))
def test_plane_a_may_not_depend_on_an_unkeyed_rng_crate(rng: str) -> None:
    """Every crate on the single list is refused in plane A (DEF-1)."""
    crates = engine(with_external(crate("rh-minds"), rng))
    assert found(crates) == [("rng-crate", "rh-minds")]


def test_the_rng_rule_covers_rh_ids_and_build_dependencies() -> None:
    """rh-ids may not take rand either, and a build-dependency still counts."""
    crates = engine(with_external(crate("rh-minds"), "rand", dep_kind="build"))
    crates = [with_external(c, "getrandom") if c.name == "rh-ids" else c for c in crates]
    assert sorted(found(crates)) == [("rng-crate", "rh-ids"), ("rng-crate", "rh-minds")]


def test_dev_dependencies_may_use_rand_for_property_tests() -> None:
    """Test code is not authoritative: proptest and rand are allowed as dev-dependencies."""
    crates = engine(with_external(crate("rh-minds"), "rand", "rand_chacha", dep_kind="dev"))
    assert found(crates) == []


@pytest.mark.parametrize("block", ["rand_core", "rand_chacha"])
def test_rng_building_blocks_are_allowed_only_in_rh_ids(block: str) -> None:
    """rand_core and rand_chacha serve the keyed RNG in rh-ids and nowhere else in plane A."""
    allowed = [with_external(c, block) if c.name == "rh-ids" else c for c in engine()]
    assert found(allowed) == []
    assert found(engine(with_external(crate("rh-minds"), block))) == [("rng-crate", "rh-minds")]


def test_crates_outside_plane_a_may_use_any_rng() -> None:
    """The rule guards plane A; a tool or plane-C crate may use rand."""
    crates = engine(
        with_external(crate("rh-bench", plane="tool", kind="tool"), "rand"),
        with_external(crate("rh-sifter", plane="C"), "rand", "fastrand"),
    )
    assert found(crates) == []


def test_plane_a_may_not_take_an_unreviewed_third_party_crate() -> None:
    """A dependency missing from the reviewed list is refused until it is reviewed (A-20)."""
    crates = engine(with_external(crate("rh-minds"), "itoa"))
    assert found(crates) == [("plane-A-external", "rh-minds")]


def test_a_reviewed_crate_is_allowed_where_the_list_allows_it() -> None:
    """Any plane-A crate may use serde; only rh-registry toml; only rh-sample libm."""
    crates = engine(
        with_external(crate("rh-minds"), "serde"),
        with_external(crate("rh-sample"), "libm"),
    )
    crates = [with_external(c, "serde", "toml") if c.name == "rh-registry" else c for c in crates]
    assert found(crates) == []


@pytest.mark.parametrize("dep", ["libm", "toml"])
def test_a_scoped_crate_is_refused_elsewhere_in_plane_a(dep: str) -> None:
    """The float-bearing crates stay where they were reviewed (the sampler float zone; SPR-004)."""
    assert found(engine(with_external(crate("rh-minds"), dep))) == [
        ("plane-A-external", "rh-minds")
    ]


def test_the_external_rule_skips_dev_dependencies_tools_and_rng_crates() -> None:
    """Tests may use anything; tool crates are not plane A; RNG crates are rng-crate's alone."""
    crates = engine(
        with_external(crate("rh-minds"), "itoa", dep_kind="dev"),
        with_external(crate("rh-bench", plane="tool", kind="tool"), "itoa"),
        with_external(crate("rh-dice"), "rand"),
    )
    assert found(crates) == [("rng-crate", "rh-dice")]


def test_every_reviewed_entry_has_a_note() -> None:
    """The list is a review record: every entry says whether it can return a float."""
    for name, (allowed, note) in PLANE_A_EXTERNAL_DEPS.items():
        assert note.strip(), name
        assert allowed is None or allowed, name


def test_an_alias_does_not_hide_an_rng_crate() -> None:
    """The metadata names the package, not the alias; `dice = { package = "rand" }` is rand."""
    document = {
        "workspace_members": ["path+file:///rh-core#0.0.0"],
        "packages": [
            _package(
                "rh-core",
                [{"name": "rand", "rename": "dice", "source": "registry+https://x", "kind": None}],
                {"rehoboam": {"plane": "A", "kind": "core"}},
            )
        ],
    }
    assert found(crates_from_metadata(document)) == [("rng-crate", "rh-core")]


def _package(
    name: str, deps: list[dict[str, object]], meta: dict[str, object] | None
) -> dict[str, object]:
    """Return a package entry shaped like ``cargo metadata`` output."""
    return {
        "id": f"path+file:///{name}#0.0.0",
        "name": name,
        "dependencies": deps,
        "metadata": meta,
    }


def test_crates_from_metadata_keeps_only_edges_between_workspace_crates() -> None:
    """Third-party and non-member dependencies are not workspace edges."""
    decl: dict[str, object] = {"rehoboam": {"plane": "A", "kind": "core"}}
    document = {
        "workspace_members": ["path+file:///rh-core#0.0.0", "path+file:///rh-ids#0.0.0"],
        "packages": [
            _package(
                "rh-core",
                [
                    {"name": "rh-ids", "source": None, "kind": None},
                    {"name": "rh-ids", "source": None, "kind": "dev"},
                    {"name": "libm", "source": "registry+https://x", "kind": None},
                ],
                decl,
            ),
            _package("rh-ids", [], {"rehoboam": {"plane": "A", "kind": 7}}),
        ],
    }
    crates = crates_from_metadata(document)
    assert [c.name for c in crates] == ["rh-core", "rh-ids"]
    assert crates[0].edges == (Edge("rh-ids", "dev"), Edge("rh-ids", "normal"))
    assert crates[0].external == (Edge("libm", "normal"),)
    assert (crates[0].plane, crates[0].kind) == ("A", "core")
    assert crates[1].kind == "7"


@pytest.mark.parametrize(
    "document", [[], {"packages": []}, {"workspace_members": [], "packages": 3}]
)
def test_malformed_metadata_is_an_error(document: object) -> None:
    """A document of the wrong shape raises instead of passing silently."""
    with pytest.raises(WorkspaceError):
        crates_from_metadata(document)


def test_main_passes_on_the_repository_workspace() -> None:
    """The real workspace obeys the law (negative control)."""
    assert main(["--manifest-path", str(REPO / "Cargo.toml")]) == EXIT_OK


@pytest.mark.parametrize(
    ("fixture", "rule"),
    [
        ("system_to_system", "V-INSP-03"),
        ("system_to_disallowed", "V-INSP-03"),
        ("plane_a_to_b", "V-PLN-01"),
        ("plane_a_to_c", "V-PLN-01"),
        ("missing_metadata", "metadata"),
        ("system_build_dependency", "V-INSP-03"),
        ("system_package_alias", "V-INSP-03"),
        ("relabelled_snapshot", "metadata"),
        ("contract_name", "metadata"),
        ("plane_a_rng_crate", "rng-crate"),
        ("rng_building_block_outside_ids", "rng-crate"),
        ("plane_a_unlisted_dep", "plane-A-external"),
        ("plane_a_scoped_dep_outside", "plane-A-external"),
    ],
)
def test_main_catches_each_fixture(
    fixture: str, rule: str, capsys: pytest.CaptureFixture[str]
) -> None:
    """Each positive-control fixture fails with its own rule."""
    status = main(["--manifest-path", str(ARCH_FIXTURES / fixture / "Cargo.toml")])
    assert status == EXIT_VIOLATIONS
    assert f"VIOLATION [{rule}]" in capsys.readouterr().out


def test_main_reports_an_unreadable_workspace(tmp_path: Path) -> None:
    """A missing manifest is an error (exit 2), not a pass."""
    assert main(["--manifest-path", str(tmp_path / "Cargo.toml")]) == EXIT_UNREADABLE
