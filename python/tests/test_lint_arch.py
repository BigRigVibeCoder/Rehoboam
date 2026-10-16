"""Unit tests for bin/lint_arch.py: the crate-boundary law.

Refs: SPR-001 T-04; docs/09 V-INSP-03 (EVO-1), V-PLN-01 (DET-12); docs/13 §3; ADR-0024.
"""

from pathlib import Path

import pytest
from lint_arch import (
    EXIT_OK,
    EXIT_UNREADABLE,
    EXIT_VIOLATIONS,
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
    """Return the SPR-001 workspace's crates plus ``extra``."""
    return [
        crate("rh-ids"),
        crate("rh-core", kind="core", deps=("rh-ids",)),
        crate("rh-events"),
        crate("rh-registry"),
        crate("rh-contracts", kind="contract"),
        crate("rh-snapshot"),
        crate("rh-cli", plane="tool", kind="tool", deps=("rh-core", "rh-snapshot")),
        *extra,
    ]


def test_a_clean_layout_passes() -> None:
    """Allowed edges of every kind produce no violation."""
    crates = engine(
        crate("rh-contracts-economy", kind="contract"),
        crate(
            "rh-sys-economy",
            kind="system",
            deps=("rh-core", "rh-ids", "rh-events", "rh-registry", "rh-contracts-economy"),
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
    ("plane", "kind"),
    [("B", "core"), ("C", "contract"), ("tool", "machinery"), ("A", "tool"), ("B", "tool")],
)
def test_inconsistent_plane_and_kind_are_reported(plane: str, kind: str) -> None:
    """Core, contract and system crates are plane A; plane tool goes with kind tool."""
    assert found(engine(crate("rh-new", plane=plane, kind=kind))) == [("metadata", "rh-new")]


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


def test_the_rng_rule_covers_dev_dependencies_and_rh_ids() -> None:
    """rh-ids may not take rand either, and a dev-dependency still counts."""
    crates = engine(with_external(crate("rh-minds"), "rand", dep_kind="dev"))
    crates = [with_external(c, "getrandom") if c.name == "rh-ids" else c for c in crates]
    assert sorted(found(crates)) == [("rng-crate", "rh-ids"), ("rng-crate", "rh-minds")]


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
        ("plane_a_rng_crate", "rng-crate"),
        ("rng_building_block_outside_ids", "rng-crate"),
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
