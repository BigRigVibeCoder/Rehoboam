"""The uv project's package layout: flat python/rehoboam_*/ packages, declared one by one (PY-PKG).

python/pyproject.toml lists the repository's packages in [tool.uv.build-backend] module-name, and
`uv sync` installs them in editable mode: a .pth file in python/.venv naming python/, which the
site module processes at start-up, under -I as well (not under -S, which skips site). So
`python -I -m rehoboam_<name>` runs from the repository root with python/.venv's interpreter, which
is how SPR-027 T-05, SPR-028 T-02 and SPR-029 T-02 run their packages.

The editable install puts all of python/ on sys.path, so a package directory that is not declared
still imports locally and is missing only from a built wheel. These tests pin the declared list to
the directories, both ways, and the rest of the layout (each with a planted control):
- every python/rehoboam_*/ directory, and every directory of python/ holding an __init__.py, is
  declared; every declared package exists, with an __init__.py and a py.typed (PEP 561: mypy
  needs it for a file checked on its own that imports the package); the list is alphabetical,
  so a sprint adding a package changes one line; no module sits directly in python/;
- each declared package imports under `python -I -c` from this checkout, from the repository root
  and from elsewhere, and each `__main__` module runs under `python -I -m <module> --help`;
- ruff and mypy --strict cover every module of every declared package;
- rehoboam_tools.__version__ equals the distribution's version;
- the build backend is pinned exactly, to the uv release CI installs, so uv builds with its
  bundled backend and downloads nothing.

A sprint adding a package extends nothing here: every test is parametrized over the declared list
(python/README.md).

Refs: PY-PKG (REV-WAVE8 S6); GOV-003 §3; docs/05_architecture/05.0_overview.md §6.
"""

import re
import subprocess
import sys
import tomllib
from collections.abc import Callable, Sequence
from importlib import metadata
from pathlib import Path
from typing import Any

import pytest
import rehoboam_tools
import yaml  # type: ignore[import-untyped]  # PyYAML ships no stubs; adding types-PyYAML would change the lock
from mypy.main import process_options

REPO = Path(__file__).resolve().parents[2]
PYTHON_DIR = REPO / "python"
PYPROJECT = PYTHON_DIR / "pyproject.toml"
WORKFLOWS = REPO / ".github" / "workflows"
DISTRIBUTION = "rehoboam-tools"
PREFIX = "rehoboam_"
PACKAGE_NAME = re.compile(r"rehoboam_[a-z][a-z0-9_]*")
# Every regular package carries these (PEP 561's marker included).
MARKERS = ("__init__.py", "py.typed")
TOP_LEVEL_SUFFIXES = (".py", ".pyi")
BACKEND = "uv_build"
EXACT_PIN = re.compile(r"uv_build==(?P<version>\d+\.\d+\.\d+)")
SETUP_UV = "astral-sh/setup-uv@"
IMPORT_AND_SHOW_FILE = "import importlib, sys; print(importlib.import_module(sys.argv[1]).__file__)"


def pyproject() -> dict[str, Any]:
    """python/pyproject.toml, parsed."""
    with PYPROJECT.open("rb") as f:
        return tomllib.load(f)


def backend_settings() -> dict[str, Any]:
    """The [tool.uv.build-backend] table."""
    settings = pyproject()["tool"]["uv"]["build-backend"]
    assert isinstance(settings, dict)
    return settings


def declared() -> list[str]:
    """The packages [tool.uv.build-backend] module-name declares, in the file's order."""
    names = backend_settings()["module-name"]
    assert isinstance(names, list), "module-name must be a list, one package per line"
    return [str(n) for n in names]


def package_directories(root: Path) -> set[str]:
    """Directories of root that are packages: named rehoboam_*, or holding an __init__.py."""
    return {
        p.name
        for p in root.iterdir()
        if p.is_dir() and (p.name.startswith(PREFIX) or (p / "__init__.py").is_file())
    }


def list_problems(names: Sequence[str]) -> list[str]:
    """Why the declared list is not a sorted, duplicate-free list of rehoboam_<name> packages."""
    bad = [n for n in names if not PACKAGE_NAME.fullmatch(n)]
    found = [f"{n!r} is not a rehoboam_<name> package name" for n in bad]
    if list(names) != sorted(names):
        found.append(f"module-name is not alphabetical: {list(names)!r}")
    found.extend(f"{n} is declared twice" for n in sorted({n for n in names if names.count(n) > 1}))
    return found


def layout_problems(root: Path, names: Sequence[str]) -> list[str]:
    """Why root (python/) and the declared names do not match; empty if they do."""
    on_disk = package_directories(root)
    found = list_problems(names)
    found.extend(
        f"{root.name}/{n}/ is not declared in [tool.uv.build-backend] module-name"
        for n in sorted(on_disk - set(names))
    )
    found.extend(
        f"{n} is declared but {root.name}/{n}/ does not exist" for n in sorted(set(names) - on_disk)
    )
    found.extend(
        f"{root.name}/{n}/ has no {marker}"
        for n in sorted(on_disk & set(names))
        for marker in MARKERS
        if not (root / n / marker).is_file()
    )
    found.extend(
        f"{root.name}/{p.name} is a top-level module, importable everywhere through the editable "
        "install"
        for p in sorted(root.iterdir())
        if p.is_file() and p.suffix in TOP_LEVEL_SUFFIXES
    )
    return found


def main_modules() -> list[str]:
    """Every module of a declared package that `python -m` can run (a package with a __main__)."""
    found: list[str] = []
    for name in declared():
        for main in sorted((PYTHON_DIR / name).rglob("__main__.py")):
            found.append(".".join(main.parent.relative_to(PYTHON_DIR).parts))
    return found


def run_python(*args: str, cwd: Path) -> subprocess.CompletedProcess[str]:
    """This interpreter (python/.venv's under check_all) with args, from cwd."""
    return subprocess.run(
        [sys.executable, *args], cwd=cwd, capture_output=True, text=True, check=False
    )


# --- The declared list and the directories ---------------------------------------------------


def test_the_backend_reads_flat_packages_from_python_itself() -> None:
    """module-root is python/ itself (05.0 §6's layout), and no namespace mode hides the list."""
    settings = backend_settings()
    assert settings.get("module-root") == ""
    assert "namespace" not in settings


def test_the_declared_packages_equal_the_package_directories() -> None:
    """Every package directory is declared and every declared package exists, marked and typed."""
    assert declared(), "module-name declares no package"
    assert layout_problems(PYTHON_DIR, declared()) == []


def _package(root: Path, name: str) -> None:
    """Plant a complete package root/name/."""
    (root / name).mkdir()
    for marker in MARKERS:
        (root / name / marker).write_text("", encoding="utf-8")


def _undeclared(root: Path) -> list[str]:
    _package(root, "rehoboam_b")
    return ["rehoboam_a"]


def _missing(root: Path) -> list[str]:
    return ["rehoboam_a", "rehoboam_c"]


def _untyped(root: Path) -> list[str]:
    (root / "rehoboam_a" / "py.typed").unlink()
    return ["rehoboam_a"]


def _not_a_package(root: Path) -> list[str]:
    (root / "rehoboam_d").mkdir()
    return ["rehoboam_a", "rehoboam_d"]


def _foreign_package(root: Path) -> list[str]:
    (root / "helpers").mkdir()
    (root / "helpers" / "__init__.py").write_text("", encoding="utf-8")
    return ["rehoboam_a"]


def _top_level_module(root: Path) -> list[str]:
    (root / "util.py").write_text("", encoding="utf-8")
    return ["rehoboam_a"]


def _unsorted(root: Path) -> list[str]:
    _package(root, "rehoboam_z")
    return ["rehoboam_z", "rehoboam_a"]


def _bad_name(root: Path) -> list[str]:
    _package(root, "rehoboam_Calib")
    return ["rehoboam_Calib", "rehoboam_a"]


@pytest.mark.parametrize(
    ("plant", "expected"),
    [
        (_undeclared, "python/rehoboam_b/ is not declared"),
        (_missing, "rehoboam_c is declared but python/rehoboam_c/ does not exist"),
        (_untyped, "python/rehoboam_a/ has no py.typed"),
        (_not_a_package, "python/rehoboam_d/ has no __init__.py"),
        (_foreign_package, "python/helpers/ is not declared"),
        (_top_level_module, "python/util.py is a top-level module"),
        (_unsorted, "module-name is not alphabetical"),
        (_bad_name, "'rehoboam_Calib' is not a rehoboam_<name> package name"),
    ],
)
def test_layout_control(tmp_path: Path, plant: Callable[[Path], list[str]], expected: str) -> None:
    """Positive controls: each planted mistake is reported, and the clean layout is not."""
    root = tmp_path / "python"
    root.mkdir()
    _package(root, "rehoboam_a")
    (root / "tests").mkdir()
    assert layout_problems(root, ["rehoboam_a"]) == []
    problems = layout_problems(root, plant(root))
    assert any(expected in p for p in problems), problems


# --- Importable under python -I, from this checkout --------------------------------------------


@pytest.mark.parametrize("name", declared())
@pytest.mark.parametrize("where", ["repository root", "elsewhere"])
def test_declared_package_imports_under_isolated_mode(
    name: str, where: str, tmp_path: Path
) -> None:
    """`python -I -c 'import <package>'` finds this checkout's package, from any directory."""
    cwd = REPO if where == "repository root" else tmp_path
    done = run_python("-I", "-c", IMPORT_AND_SHOW_FILE, name, cwd=cwd)
    assert done.returncode == 0, f"{done.stderr}\nRun `uv sync --frozen --project python`."
    assert Path(done.stdout.strip()) == PYTHON_DIR / name / "__init__.py"


@pytest.mark.parametrize("module", main_modules())
def test_main_module_runs_under_isolated_mode(module: str) -> None:
    """`python -I -m <module> --help` succeeds from the repository root."""
    done = run_python("-I", "-m", module, "--help", cwd=REPO)
    assert done.returncode == 0, done.stderr


def test_rehoboam_tools_prints_its_version_under_isolated_mode() -> None:
    """The entry point every package uses, end to end: `python -I -m rehoboam_tools`."""
    done = run_python("-I", "-m", "rehoboam_tools", cwd=REPO)
    assert done.returncode == 0, done.stderr
    assert done.stdout == f"rehoboam_tools {rehoboam_tools.__version__}\n"


def test_rehoboam_tools_refuses_an_unknown_argument() -> None:
    """A usage error is status 2, with nothing on stdout."""
    done = run_python("-I", "-m", "rehoboam_tools", "--no-such-option", cwd=REPO)
    assert done.returncode == 2
    assert done.stdout == ""
    assert "unrecognized arguments: --no-such-option" in done.stderr


def test_without_site_the_packages_are_not_importable() -> None:
    """-S skips site, so the editable .pth is never read: run the packages with -I, never -S."""
    done = run_python("-I", "-S", "-c", "import rehoboam_tools", cwd=REPO)
    assert done.returncode != 0
    assert "No module named 'rehoboam_tools'" in done.stderr


def test_the_version_constant_equals_the_distribution_version() -> None:
    """rehoboam_tools.__version__, the installed metadata and pyproject.toml agree."""
    assert rehoboam_tools.__version__ == pyproject()["project"]["version"]
    assert rehoboam_tools.__version__ == metadata.version(DISTRIBUTION)


# --- Lint and type coverage --------------------------------------------------------------------


def test_ruff_checks_every_module_of_every_declared_package() -> None:
    """`ruff check .` with the repository's configuration includes each package's modules."""
    args = ["check", "--config", "python/pyproject.toml", "--no-cache", "--show-files", "."]
    done = run_python("-I", "-m", "ruff", *args, cwd=REPO)
    assert done.returncode == 0, done.stderr
    shown = {Path(line) for line in done.stdout.splitlines() if line.strip()}
    for name in declared():
        modules = sorted((PYTHON_DIR / name).rglob("*.py"))
        assert modules
        assert [m for m in modules if m not in shown] == []


def test_mypy_checks_every_module_of_every_declared_package_strictly(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """The source list mypy builds from python/pyproject.toml holds each package, strictly typed."""
    monkeypatch.chdir(REPO)
    sources, options = process_options(["--config-file", "python/pyproject.toml"])
    checked = {s.module: REPO / s.path for s in sources if s.path is not None}
    for name in declared():
        assert checked.get(name) == PYTHON_DIR / name / "__init__.py"
        for module in [m for m in checked if m == name or m.startswith(f"{name}.")]:
            module_options = options.clone_for_module(module)
            assert not module_options.ignore_errors, module
            assert module_options.disallow_untyped_defs, module


# --- The build backend's pin ---------------------------------------------------------------------


def setup_uv_versions() -> dict[str, str]:
    """The uv version every setup-uv step of every workflow installs, by workflow and step."""
    found: dict[str, str] = {}
    for path in sorted(WORKFLOWS.glob("*.yml")):
        flow = yaml.safe_load(path.read_text(encoding="utf-8"))
        for job_name, job in flow.get("jobs", {}).items():
            for step in job.get("steps", []):
                if str(step.get("uses", "")).startswith(SETUP_UV):
                    with_ = step.get("with") or {}
                    found[f"{path.name}:{job_name}"] = str(with_.get("version"))
    return found


def test_the_build_backend_is_pinned_to_the_uv_release_ci_installs() -> None:
    """An exact uv_build pin, equal to every setup-uv version: CI builds with uv's own backend."""
    build_system = pyproject()["build-system"]
    assert build_system["build-backend"] == BACKEND
    requires = build_system["requires"]
    assert len(requires) == 1, requires
    pin = EXACT_PIN.fullmatch(requires[0])
    assert pin is not None, f"{requires[0]!r} is not an exact uv_build==X.Y.Z pin"
    versions = setup_uv_versions()
    assert versions, "no workflow installs uv with astral-sh/setup-uv"
    assert {k: v for k, v in versions.items() if v != pin["version"]} == {}
