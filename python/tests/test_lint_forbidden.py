"""Unit tests for bin/lint_forbidden.py: the forbidden-API gate on plane-A crates.

Refs: SPR-001 T-04; docs/09 V-DET-19 (DET-2, DET-3); GOV-003 §2; docs/13 §3.
"""

from pathlib import Path

import pytest
from lint_forbidden import (
    EXIT_OK,
    EXIT_UNREADABLE,
    EXIT_VIOLATIONS,
    Report,
    lex,
    main,
    rust_files,
    scan_source,
)

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "tests" / "lint_fixtures" / "forbidden"


def scan(source: str) -> Report:
    """Scan one snippet as a plane-A file."""
    report = Report(violations=[], escaped=[])
    scan_source(source, "snippet.rs", report)
    return report


def rules_hit(source: str) -> list[str]:
    """Return the rule of every violation in a snippet, in order."""
    return [line.split("]")[0].split("/")[1] for line in scan(source).violations]


# --- The lexer ---------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "source",
    [
        "// HashMap\n",
        "/// HashMap in a doc comment\n",
        "/* HashMap /* nested SystemTime */ still a comment: thread_rng */\n",
        '"HashMap"\n',
        '"escaped \\" quote, then HashMap"\n',
        'r"HashMap"\n',
        'r#"a "quoted" HashMap"#\n',
        'br##"HashMap "# still raw"##\n',
        'b"HashMap"\n',
        'c"HashMap"\n',
    ],
)
def test_comments_and_literals_are_blanked(source: str) -> None:
    """Nothing inside a comment or a string literal is a finding."""
    assert scan(source).violations == []


def test_blanking_keeps_offsets_and_lines() -> None:
    """Code keeps its length and newlines, so reported positions are exact."""
    source = 'let a = "x\ny"; /* c\n */ let b = 1; // tail\n'
    lexed = lex(source)
    assert len(lexed.code) == len(source)
    assert [i for i, ch in enumerate(lexed.code) if ch == "\n"] == [
        i for i, ch in enumerate(source) if ch == "\n"
    ]
    assert lexed.line_comments == ((3, "// tail"),)


@pytest.mark.parametrize(
    "prefix",
    ["let q = '\"';", "let q = '\\'';", "let q = '\\\\';", "let q = '\\u{48}';", "let q = b'\"';"],
)
def test_char_literals_do_not_hide_the_code_after_them(prefix: str) -> None:
    """A quote inside a char literal must not open a string that swallows real code."""
    assert rules_hit(prefix + " let m: HashMap<u8, u8>;\n") == ["hash-collections"]


@pytest.mark.parametrize(
    ("source", "rule"),
    [
        ("fn f()->char{return'\"';}\nfn g(){let _t=Instant::now();}\n", "wall-clock"),
        ("fn f()->char{return'\\'';}\nunsafe{}\n", "unsafe"),
        ("fn f()->char{return'\\\\';}\nunsafe{}\n", "unsafe"),
        ("fn f()->&'static str{return\"x'y\";}\nunsafe{}\n", "unsafe"),
        ("fn f<'a>(x:&'a str)->&'a str{x}unsafe{}\n", "unsafe"),
        ("match c{'a'=>1,_=>2};let m:HashMap<u8,u8>;\n", "hash-collections"),
    ],
)
def test_a_quote_right_after_an_identifier_is_still_a_literal(source: str, rule: str) -> None:
    """``return'"'`` is a char literal; only lookahead decides char versus lifetime (DEF-3)."""
    assert rules_hit(source) == [rule]


def test_lifetimes_and_labels_are_code() -> None:
    """'a and 'outer are not char literals; code after them is still scanned."""
    source = "fn f<'a>(x: &'a str) { 'outer: loop { break 'outer; } let t = SystemTime::now(); }\n"
    assert rules_hit(source) == ["wall-clock"]


def test_raw_identifiers_are_code() -> None:
    """r#type is an identifier, not a raw string."""
    assert rules_hit("let r#type = 1; unsafe { x }\n") == ["unsafe"]


# --- The rules ---------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("snippet", "rule"),
    [
        ("use std::collections::HashMap;", "hash-collections"),
        ("let s: HashSet<u8> = HashSet::with_capacity(4);", "hash-collections"),
        ("type M = FxHashMap<u8, u8>;", "hash-collections"),
        ("type S = hashbrown::HashSet<u8>;", "hash-collections"),
        ("let r = rand::thread_rng();", "thread-rng"),
        ("fn f(r: &mut ThreadRng) {}", "thread-rng"),
        ("let r = rand::rng();", "thread-rng"),
        ("use rand::{Rng, rng};", "thread-rng"),
        ("use rand::rngs::StdRng;", "std-rng"),
        ("let r = SmallRng::from_entropy();", "std-rng"),
        ("let r = OsRng;", "std-rng"),
        ("let x: u8 = rand::random();", "rand-random"),
        ("use rand::{random, Rng};", "rand-random"),
        ("let x = rand::random_range(0..10);", "rand-random"),
        ("let b = rand::random_bool(0.5);", "rand-random"),
        ("let b = random_ratio(1, 3);", "rand-random"),
        ("let n = rand::random_iter::<u8>().take(3).count();", "rand-random"),
        ("let r = SmallRng::seed_from_u64(1);", "std-rng"),
        ("let r = Generator::from_os_rng();", "std-rng"),
        ("getrandom::fill(&mut buf);", "std-rng"),
        ("let y = a.mul_add(b, c);", "mul-add"),
        ("let y = f64::mul_add(a, b, c);", "mul-add"),
        ("let t = Instant::now();", "wall-clock"),
        ("use std::time::Instant;", "wall-clock"),
        ("use std::time::*;", "wall-clock"),
        ("use std::{collections::BTreeMap, time::{Duration, Instant}};", "wall-clock"),
        ("let t = SystemTime::now();", "wall-clock"),
        ("let e = UNIX_EPOCH;", "wall-clock"),
        ("let d = chrono::Utc::now();", "wall-clock"),
        ("unsafe { x }", "unsafe"),
        ("unsafe fn f() {}", "unsafe"),
        ("let t = std :: time :: Instant :: now();", "wall-clock"),
        ("let t = Instant ::now();", "wall-clock"),
        ("use std :: time :: { Duration , Instant };", "wall-clock"),
        ("let t = std::time /* hidden */ ::Instant::now();", "wall-clock"),
        ("macro_rules! m { () => { rand :: rng() }; }", "thread-rng"),
        ("let x = rand :: random::<u8>();", "rand-random"),
        ('#[path = "../../outside.rs"] mod outside;', "source-include"),
        ('#![path = "x"]', "source-include"),
        ('#[cfg_attr(test, path = "mock.rs")] mod clock;', "source-include"),
        ('include!("../../outside.rs");', "source-include"),
        ('include! { "gen.rs" }', "source-include"),
    ],
)
def test_each_forbidden_api_is_caught_by_its_rule(snippet: str, rule: str) -> None:
    """Every API listed by V-DET-19 and GOV-003 §2 is found, under the right rule."""
    assert set(rules_hit(snippet + "\n")) == {rule}


@pytest.mark.parametrize(
    "snippet",
    [
        "use std::collections::BTreeMap;",
        "#![forbid(unsafe_code)]",
        "fn my_mul_add_helper() {}",
        "pub struct Instant { minutes: i64 }",
        "use std::time::Duration;",
        "use rand_chacha::ChaCha8Rng;",
        "let rngs = 3;",
        'let s = include_str!("table.csv");',
        'let b = include_bytes!("golden.bin");',
        "fn path() -> u8 { 0 }",
        "let path = 1;",
    ],
)
def test_allowed_code_is_not_flagged(snippet: str) -> None:
    """Near-misses are not findings (a simulation type may be named Instant)."""
    assert scan(snippet + "\n").violations == []


def test_a_multi_line_use_group_is_reported_at_its_first_line() -> None:
    """Matches that span lines are attributed to the line where they start."""
    source = "fn x() {}\nuse std::time::{\n    Duration,\n    Instant,\n};\n"
    (violation,) = scan(source).violations
    assert "snippet.rs:2:" in violation


# --- Escapes -----------------------------------------------------------------------------------


def test_an_escape_on_the_same_line_excuses_and_is_counted() -> None:
    """A reasoned escape turns the finding into a printed, counted escape."""
    report = scan("let y = a.mul_add(b, c); // LINT-ALLOW(forbidden-api): golden vector check\n")
    assert report.violations == []
    assert len(report.escaped) == 1
    assert "reason: golden vector check" in report.escaped[0]


def test_an_escape_alone_on_the_line_above_excuses_the_next_line() -> None:
    """The escape may sit on its own line directly above the finding."""
    report = scan(
        "// LINT-ALLOW(forbidden-api): measured by a test harness\nlet t = Instant::now();\n"
    )
    assert (report.violations, len(report.escaped)) == ([], 1)


@pytest.mark.parametrize(
    "comment", ["// LINT-ALLOW(forbidden-api)", "// LINT-ALLOW(forbidden-api):   "]
)
def test_an_escape_without_a_reason_excuses_nothing(comment: str) -> None:
    """No reason, no escape: the finding stays and the escape is reported too."""
    assert sorted(rules_hit(f"unsafe {{ x }} {comment}\n")) == ["escape", "unsafe"]


def test_a_spaced_path_is_reported_without_its_spaces() -> None:
    """The report names the path as written without whitespace (DEF-4)."""
    (violation,) = scan("let t = std :: time :: Instant :: now();\n").violations
    assert "`time::Instant` at snippet.rs:1:" in violation


def test_a_source_include_can_be_escaped_with_a_reason() -> None:
    """#[path] and include! are allowed where a reason is given, and counted."""
    report = scan(
        '#[path = "gen/table.rs"] // LINT-ALLOW(forbidden-api): generated in-crate, scanned too\n'
        "mod table;\n"
    )
    assert (report.violations, len(report.escaped)) == ([], 1)


def test_an_unused_escape_is_a_violation() -> None:
    """An escape that covers no finding must be removed."""
    assert rules_hit("let x = 1; // LINT-ALLOW(forbidden-api): left over\n") == ["escape"]


def test_an_escape_covers_only_its_own_line() -> None:
    """A finding two lines below an escape is not excused."""
    source = "// LINT-ALLOW(forbidden-api): for the next line only\nlet a = 1;\nunsafe { x }\n"
    assert sorted(rules_hit(source)) == ["escape", "unsafe"]


def test_an_escape_inside_a_string_does_not_count() -> None:
    """Only comments can carry an escape."""
    source = 'let s = "// LINT-ALLOW(forbidden-api): not a comment"; unsafe { x }\n'
    assert rules_hit(source) == ["unsafe"]


# --- Files and the command line ----------------------------------------------------------------


def test_rust_files_skip_only_the_top_level_target_directory(tmp_path: Path) -> None:
    """Build output is skipped; a source module named ``target`` is scanned."""
    for rel in ["src/lib.rs", "src/target/mod.rs", "target/debug/build/out.rs", "notes.txt"]:
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_text("")
    found = [p.relative_to(tmp_path).as_posix() for p in rust_files([tmp_path])]
    assert found == ["src/lib.rs", "src/target/mod.rs"]


# file stem -> (rule, the token the report names); mirrors FORBIDDEN_API_CONTROLS in check_all.sh.
PLANTED = {
    "hash_map": ("hash-collections", "HashMap"),
    "hash_set": ("hash-collections", "HashSet"),
    "thread_rng": ("thread-rng", "thread_rng"),
    "thread_rng_type": ("thread-rng", "ThreadRng"),
    "rand_rng": ("thread-rng", "rand::rng"),
    "std_rng": ("std-rng", "StdRng"),
    "small_rng": ("std-rng", "SmallRng"),
    "os_rng": ("std-rng", "OsRng"),
    "from_entropy": ("std-rng", "from_entropy"),
    "from_os_rng": ("std-rng", "from_os_rng"),
    "getrandom": ("std-rng", "getrandom"),
    "rand_random": ("rand-random", "rand::random"),
    "random_range": ("rand-random", "random_range"),
    "random_bool": ("rand-random", "random_bool"),
    "random_ratio": ("rand-random", "random_ratio"),
    "random_iter": ("rand-random", "random_iter"),
    "mul_add": ("mul-add", "mul_add"),
    "instant": ("wall-clock", "Instant::now"),
    "system_time": ("wall-clock", "SystemTime"),
    "unsafe": ("unsafe", "unsafe"),
    "spaced_path": ("wall-clock", "time::Instant"),
    "spaced_rand": ("thread-rng", "rand::rng"),
    "path_attr": ("source-include", "#[path"),
    "include_macro": ("source-include", "include!("),
}


@pytest.mark.parametrize(("name", "expected"), sorted(PLANTED.items()))
def test_main_catches_each_planted_file(
    name: str, expected: tuple[str, str], capsys: pytest.CaptureFixture[str]
) -> None:
    """Each positive-control file holds one API, reported under its own rule with its token."""
    rule, token = expected
    assert main(["--scan", str(FIXTURES / "planted" / f"{name}.rs")]) == EXIT_VIOLATIONS
    violations = [v for v in capsys.readouterr().out.splitlines() if v.startswith("VIOLATION")]
    assert len(violations) == 1
    assert violations[0].startswith(f"VIOLATION [V-DET-19/{rule}] `{token}` at ")


def test_check_all_runs_a_control_for_every_planted_file() -> None:
    """The controls in check_all.sh name every planted file with its rule and token."""
    script = (REPO / "bin" / "check_all.sh").read_text(encoding="utf-8")
    listed = script.split('FORBIDDEN_API_CONTROLS="', 1)[1].split('"', 1)[0].split()
    parsed = {}
    for spec in listed:
        name, rule, token = spec.split(":", 2)
        parsed[name] = (rule, token)
    assert parsed == PLANTED


def test_every_planted_file_is_covered_by_a_test() -> None:
    """A planted file without a test would be a positive control nobody checks."""
    assert sorted(p.stem for p in (FIXTURES / "planted").glob("*.rs")) == sorted(PLANTED)


def test_main_scans_plane_a_and_undeclared_crates_only(capsys: pytest.CaptureFixture[str]) -> None:
    """In a workspace, the tool crate is skipped and the undeclared crate is scanned."""
    manifest = FIXTURES / "workspace" / "Cargo.toml"
    assert main(["--manifest-path", str(manifest)]) == EXIT_VIOLATIONS
    out = capsys.readouterr().out
    violations = [line for line in out.splitlines() if line.startswith("VIOLATION")]
    assert len(violations) == 3
    assert sum("fixture-core/src/lib.rs:9:" in v for v in violations) == 1
    assert sum("fixture-undeclared/" in v for v in violations) == 2
    assert "fixture-cli" not in out
    assert "1 escapes" in out


def test_main_passes_on_the_repository_workspace() -> None:
    """The real plane-A crates are clean (negative control)."""
    assert main(["--manifest-path", str(REPO / "Cargo.toml")]) == EXIT_OK


def test_main_reports_an_unreadable_file(tmp_path: Path) -> None:
    """A file that is not UTF-8 is an error (exit 2), not a pass."""
    bad = tmp_path / "bad.rs"
    bad.write_bytes(b"\xff\xfe unsafe")
    assert main(["--scan", str(bad)]) == EXIT_UNREADABLE
