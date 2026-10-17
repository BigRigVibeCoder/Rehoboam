"""Unit tests for bin/lint_forbidden.py: the forbidden-API gate on plane-A crates.

Refs: SPR-001 T-04; docs/09 V-DET-19 (DET-2, DET-3); GOV-003 §2; docs/13 §3.
"""

from pathlib import Path

import pytest
from lint_forbidden import (
    EXIT_OK,
    EXIT_UNREADABLE,
    EXIT_VIOLATIONS,
    FILE_IO_CRATES,
    FILE_IO_FILES,
    FLOAT_EXEMPT_CRATES,
    FloatScan,
    FloatScope,
    Report,
    declared_float_fields,
    expand_use,
    float_yielding_apis,
    lex,
    main,
    plane_a_crate_dirs,
    rust_files,
    scan_paths,
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
        ("let b = rand::random_bool(p);", "rand-random"),
        ("let b = random_ratio(1, 3);", "rand-random"),
        ("let n = rand::random_iter::<u8>().take(3).count();", "rand-random"),
        ("let r = SmallRng::seed_from_u64(1);", "std-rng"),
        ("let r = Generator::from_os_rng();", "std-rng"),
        ("getrandom::fill(&mut buf);", "std-rng"),
        ("let y = a.mul_add(b, c);", "mul-add"),
        ("let y = Real::mul_add(a, b, c);", "mul-add"),
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
    "float_type": ("float", "f32"),
    "float_literal": ("float", "0.5"),
    "float_suffix": ("float", "2e-3f64"),
    "float_conversion": ("float", "to_f64"),
    "float_escape_reason": ("float", "f64"),
    "float_undeclared_field": ("float", "f32"),
    "allow_disallowed": ("lint-allow", "allow(clippy::disallowed_methods"),
    "file_modified": ("file-clock", ".modified("),
    "process_command": ("process", "std::process"),
    "process_glob_std": ("process", "process::C"),
    "process_rename": ("process", "std::process"),
    "fs_group_rename": ("file-io", "std::fs"),
    "metadata_ext": ("file-clock", "MetadataExt"),
    "metadata_ufcs": ("file-clock", "Metadata::modified"),
    "mtime": ("file-clock", ".mtime("),
    "env_rename": ("env", "std::env"),
    "env_var": ("env", "std::env"),
    "env_current_dir": ("env", "std::env"),
    "env_current_exe": ("env", "std::env"),
    "env_temp_dir": ("env", "std::env"),
    "env_home_dir": ("env", "std::env"),
    "env_set_current_dir": ("env", "std::env"),
    "env_split_paths": ("env", "std::env"),
    "env_join_paths": ("env", "std::env"),
    "fs_renamed_std": ("file-io", "fs::r"),
    "fs_os_symlink": ("file-io", "fs::s"),
    "raw_ident_fs": ("file-io", "std::fs"),
    "std_rename": ("std-rename", "std as"),
    "std_rename_root": ("std-rename", "std as"),
    "std_rename_self": ("std-rename", "std as"),
    "extern_crate_std": ("std-rename", "extern crate std as"),
    "file_io": ("file-io", "std::fs"),
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
        parsed[name] = (rule, token.replace("~", " "))
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


# --- The float rule (SPR-010 T-04; DET-3) --------------------------------------------------------

FIELDS = frozenset({"region.soil_moisture"})


def float_texts(source: str, fields: frozenset[str] = frozenset()) -> list[str]:
    """Return the matched text of every float violation in a snippet."""
    report = Report(violations=[], escaped=[])
    scan_source(source, "snippet.rs", report, fields)
    return [
        v.split("`")[1] for v in report.violations if v.startswith("VIOLATION [V-DET-19/float]")
    ]


@pytest.mark.parametrize(
    ("snippet", "texts"),
    [
        ("let x: f32 = y;", ["f32"]),
        ("fn f(a: f64) -> f64 { a }", ["f64", "f64"]),
        ("let m = f64::MAX;", ["f64"]),
        ("let r = n as f64;", ["f64"]),
        ("let r = n.to_f64();", ["to_f64"]),
        ("let q = Q::from_f32(v);", ["from_f32"]),
        ("let p = 0.5;", ["0.5"]),
        ("if hunger > 1.25 { }", ["1.25"]),
        ("let e = 1e-3;", ["1e-3"]),
        ("let e = 2E9;", ["2E9"]),
        ("let s = 3f32;", ["3f32"]),
        ("let s = 1_000.5_f64;", ["1_000.5_f64"]),
        ("let t = 1.;", ["1."]),
        ("let range = 0.0..1.0;", ["0.0", "1.0"]),
        ("let r = 0..=2.5e1;", ["2.5e1"]),
    ],
)
def test_each_float_form_is_caught(snippet: str, texts: list[str]) -> None:
    """Float types, conversions named after them, and float literals are all findings."""
    assert float_texts(snippet + "\n") == texts


@pytest.mark.parametrize(
    "snippet",
    [
        "let x = t.0.1;",
        "for i in 0..5 {}",
        "let m = 1.max(2);",
        "let h = 0x1e5;",
        "let b = 0b1010_1010;",
        "let n = 1_000_u32;",
        "struct F { f32_field: u8 }",
        "let buf64 = [0u8; 64];",
        "let q = Q16_16::from_ratio(1, 2);",
        'let s = "0.5 f64";',
        "// a rate of 0.5 as f64",
    ],
)
def test_float_near_misses_are_not_flagged(snippet: str) -> None:
    """Tuple indices, ranges, method calls on integers, hex and identifiers are not floats."""
    assert scan(snippet + "\n").violations == []


def test_a_declared_float_field_escape_excuses_the_float() -> None:
    """`f32_field <column>` naming a declared column excuses the float and is counted."""
    source = "pub moisture: f32, // LINT-ALLOW(forbidden-api): f32_field region.soil_moisture\n"
    report = Report(violations=[], escaped=[])
    scan_source(source, "snippet.rs", report, FIELDS)
    assert report.violations == []
    assert len(report.escaped) == 1
    assert report.escaped[0].startswith("ESCAPE [V-DET-19/float] `f32`")


@pytest.mark.parametrize(
    ("reason", "message"),
    [
        ("needed for the heuristic", "the escape must read `f32_field <entity.column>`"),
        ("f32_field region.fertility", "`region.fertility` is not a column any system spec"),
        ("f32_field", "the escape must read `f32_field <entity.column>`"),
    ],
)
def test_any_other_escape_leaves_the_float_a_violation(reason: str, message: str) -> None:
    """A generic reason, an undeclared column, or a missing column never excuses a float."""
    report = Report(violations=[], escaped=[])
    scan_source(f"let x: f32 = y; // LINT-ALLOW(forbidden-api): {reason}\n", "s.rs", report, FIELDS)
    assert len(report.violations) == 1
    assert message in report.violations[0]
    assert report.escaped == []


def test_a_generic_escape_still_excuses_other_rules() -> None:
    """Only floats need the f32_field form; other rules keep the plain escape."""
    source = "let y = a.mul_add(b, c); // LINT-ALLOW(forbidden-api): a reviewed exception\n"
    assert scan(source).violations == []


def test_the_float_rule_skips_tests_benches_and_examples(tmp_path: Path) -> None:
    """Only a crate's top-level tests/, benches/ and examples/ are exempt, and only from floats."""
    for rel in ["src/lib.rs", "src/tests/mod.rs", "tests/a.rs", "benches/b.rs", "examples/c.rs"]:
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_text("let x: f64 = 0.5; let m: HashMap<u8, u8>;\n")
    report = scan_paths([tmp_path])
    floats = sorted({v.split(" at ")[1].split(":")[0] for v in report.violations if "/float]" in v})
    maps = [v for v in report.violations if "/hash-collections]" in v]
    root = tmp_path.as_posix()
    assert [f.removeprefix(root + "/") for f in floats] == ["src/lib.rs", "src/tests/mod.rs"]
    assert len(maps) == 5


def test_a_file_given_directly_is_always_checked_for_floats(tmp_path: Path) -> None:
    """--scan of a file applies the float rule whatever directory the file is in."""
    planted = tmp_path / "tests" / "planted.rs"
    planted.parent.mkdir()
    planted.write_text("let x: f32 = y;\n")
    assert len(scan_paths([planted]).violations) == 1


def test_declared_float_fields_reads_only_f32_field_columns(tmp_path: Path) -> None:
    """Columns of other types, and the template, declare no float field."""
    (tmp_path / "M1.a.odd.toml").write_text(
        '[[owns]]\ncolumn = "region.moisture"\ntype = "f32_field"\n'
        '[[owns]]\ncolumn = "region.fertility"\ntype = "q16_16"\n'
    )
    (tmp_path / "_template.odd.toml").write_text('[[owns]]\ncolumn = "x.y"\ntype = "f32_field"\n')
    (tmp_path / "notes.toml").write_text('[[owns]]\ncolumn = "z.w"\ntype = "f32_field"\n')
    assert declared_float_fields(tmp_path) == frozenset({"region.moisture"})


def test_the_float_field_fixture_passes_with_its_specs(capsys: pytest.CaptureFixture[str]) -> None:
    """The negative control: a declared f32_field escape is accepted and counted."""
    fixture = FIXTURES / "float_field"
    args = ["--scan", str(fixture / "declared.rs"), "--specs", str(fixture / "specs")]
    assert main(args) == EXIT_OK
    assert "0 violations; 1 escapes; 1 declared f32_field columns" in capsys.readouterr().out


def test_the_float_workspace_reports_src_and_not_tests(capsys: pytest.CaptureFixture[str]) -> None:
    """A float threshold in a plane-A crate's src/ fails; its tests/ statistic is exempt."""
    assert main(["--manifest-path", str(FIXTURES / "float_workspace" / "Cargo.toml")]) == 1
    out = capsys.readouterr().out
    violations = [line for line in out.splitlines() if line.startswith("VIOLATION")]
    assert len(violations) == 3
    assert all("fixture-sys/src/lib.rs" in v for v in violations)


def test_an_unreadable_spec_is_an_error(tmp_path: Path) -> None:
    """A spec that does not parse, or a missing spec directory, is exit 2, never a pass."""
    (tmp_path / "M1.bad.odd.toml").write_text("[[owns]\n")
    assert main(["--scan", str(FIXTURES / "planted" / "unsafe.rs"), "--specs", str(tmp_path)]) == 2
    missing = tmp_path / "missing"
    assert main(["--scan", str(FIXTURES / "planted" / "unsafe.rs"), "--specs", str(missing)]) == 2


# --- The sampler float zone (SPR-003 Architect ruling, 2026-10-16, item 2) ----------------------


def test_the_float_zone_exempts_rh_sample_and_nothing_else(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """rh-sample's floats pass; rh-sampler, named like it, is still reported (exact-name match)."""
    assert main(["--manifest-path", str(FIXTURES / "float_zone" / "Cargo.toml")]) == 1
    out = capsys.readouterr().out
    violations = [line for line in out.splitlines() if line.startswith("VIOLATION")]
    assert len(violations) == 3
    assert all("float_zone/rh-sampler/src/lib.rs" in v for v in violations)
    assert "float rule exempts rh-sample only" in out


def test_a_float_in_rh_sample_alone_passes(capsys: pytest.CaptureFixture[str]) -> None:
    """The negative control: the float zone crate with floats in src/ is clean."""
    assert main(["--manifest-path", str(FIXTURES / "float_zone_ok" / "Cargo.toml")]) == EXIT_OK
    assert "1 files scanned; 0 violations" in capsys.readouterr().out


def test_every_other_rule_still_applies_to_rh_sample(capsys: pytest.CaptureFixture[str]) -> None:
    """The exemption is the float rule only: rh-sample's mul_add is reported, its floats are not."""
    manifest = FIXTURES / "float_zone_other_rules" / "Cargo.toml"
    assert main(["--manifest-path", str(manifest)]) == EXIT_VIOLATIONS
    violations = [line for line in capsys.readouterr().out.splitlines() if "VIOLATION" in line]
    assert len(violations) == 1
    assert "[V-DET-19/mul-add] `mul_add` at" in violations[0]


def test_the_exemption_is_by_crate_root_not_by_path_text(tmp_path: Path) -> None:
    """scan_paths exempts only the roots it is given; --scan never exempts (it names no crate)."""
    zone, other = tmp_path / "rh-sample", tmp_path / "other"
    for root in (zone, other):
        (root / "src").mkdir(parents=True)
        (root / "src" / "lib.rs").write_text("fn f(x: f64) -> f64 { x }\n")
    report = scan_paths([zone, other], float_exempt=frozenset({zone}))
    assert len(report.violations) == 2
    assert all("other" in v for v in report.violations)
    assert main(["--scan", str(zone)]) == EXIT_VIOLATIONS


# --- The float zone's public API (SPR-003 ruling item 4; VER-SPR-003-TEST GAP-1) -----------------


def float_api_texts(source: str) -> list[str]:
    """Return the float-api violations of ``source`` scanned as the float zone's src/."""
    report = Report(violations=[], escaped=[])
    scan_source(source, "lib.rs", report, floats="zone")
    return [v for v in report.violations if "[V-DET-19/float-api]" in v]


@pytest.mark.parametrize(
    "source",
    [
        "pub fn f() -> f64 { 0.0 }",
        "pub fn f(\n    x: u32,\n    y: f32,\n) -> u32 { x }",
        "pub const fn f(x: f64) -> u32 { 0 }",
        "pub const X: f64 = 1.0;",
        "pub static X: f32 = 1.0;",
        "pub type T = f64;",
        "pub use libm::log;",
        "pub use libm;",
        "pub struct S { pub rate: f64 }",
        "pub struct S(pub f64);",
        "pub struct S<T = f64>(T);",
        "pub trait T: Into<f64> {}",
        "impl From<S> for f64 { fn from(s: S) -> f64 { 0.0 } }",
        "impl S { pub fn f(&self) -> Option<f64> { None } }",
        "type F = f64;\npub fn leak() -> F { 0.0 }",
        "type G = f64;\npub struct S(pub G);",
        "type Pair<T> = (T, f32);",
        "impl Tr for W { type Out = f64; }",
        "pub(crate) type F = f64;",
        "use core::primitive::f64 as Fl;\npub fn leak() -> Fl { 0.0 }",
        "use std::primitive::f32 as Small;",
        "use core::primitive::{f64 as Fl, u8};",
        "pub(crate) use core::primitive::f64 as Fl;",
    ],
)
def test_each_public_float_is_reported(source: str) -> None:
    """Signatures, consts, statics, aliases, re-exports, fields, headers and impl headers."""
    assert float_api_texts(source)


@pytest.mark.parametrize(
    "source",
    [
        "pub(crate) fn f() -> f64 { 0.0 }",
        "pub(super) struct S { pub(crate) rate: f64 }",
        "pub fn f(x: u32) -> u32 { let y: f64 = 1.0; x }",
        "pub struct S { rate: f64 }",
        "pub const X: u32 = 1; fn g() -> f64 { libm::log(1.0) }",
        "/// pub fn f() -> f64\npub fn f() -> u32 { 0 }",
        'pub fn f() -> &\'static str { "f64" }',
        "type Count = u32;\npub fn f() -> Count { 0 }",
        "trait T { type Out; }",
        "use libm::{exp, log};\npub fn f() -> u32 { 0 }",
        "use rh_core::fixed::Q16_16;",
    ],
)
def test_floats_behind_the_public_surface_are_not_reported(source: str) -> None:
    """Bodies, private fields, pub(crate) items, comments and strings are not the API."""
    assert float_api_texts(source) == []


def test_no_escape_excuses_a_public_float() -> None:
    """Not even a reasoned one: ruling item 4 has no exception."""
    source = "pub fn f() -> f64 { 0.0 } // LINT-ALLOW(forbidden-api): f32_field region.x\n"
    assert len(float_api_texts(source)) == 1


def test_the_float_api_fixture_reports_each_plant_and_not_internal(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Six plants, six violations; pub(crate) floats in internal.rs are not reported."""
    assert main(["--manifest-path", str(FIXTURES / "float_api" / "Cargo.toml")]) == 1
    out = capsys.readouterr().out.splitlines()
    violations = [line for line in out if line.startswith("VIOLATION")]
    files = sorted(v.split(" at ")[1].split(":")[0].rsplit("/", 1)[1] for v in violations)
    assert files == [
        "alias.rs",
        "field.rs",
        "fn_return.rs",
        "impl_from.rs",
        "import_alias.rs",
        "reexport.rs",
    ]
    assert all("[V-DET-19/float-api]" in v for v in violations)


def test_the_real_float_zone_exposes_no_float() -> None:
    """rh-sample's public API is integers and fixed point only (negative control)."""
    report = scan_paths(
        [REPO / "crates" / "rh-sample"], float_exempt=frozenset({REPO / "crates" / "rh-sample"})
    )
    assert report.violations == []


# --- The exemption set (VER-SPR-003-TEST GAP-3) -------------------------------------------------


def test_the_float_exemption_is_exactly_rh_sample() -> None:
    """Widening the exemption to another crate is a reviewed change: it must edit this test."""
    assert frozenset({"rh-sample"}) == FLOAT_EXEMPT_CRATES


def test_a_float_in_every_real_plane_a_crate_name_but_rh_sample_is_reported(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """A workspace of crates named like each real plane-A crate, each with a float in src/."""
    real = [name for name, _ in plane_a_crate_dirs(REPO / "Cargo.toml")]
    assert "rh-sample" in real
    assert len(real) > 1
    for name in real:
        crate = tmp_path / name
        (crate / "src").mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2024"\n'
            'publish = false\n\n[package.metadata.rehoboam]\nplane = "A"\nkind = "machinery"\n'
        )
        (crate / "src" / "lib.rs").write_text("fn half(x: f64) -> f64 {\n    x\n}\n")
    members = ", ".join(f'"{name}"' for name in real)
    (tmp_path / "Cargo.toml").write_text(f'[workspace]\nmembers = [{members}]\nresolver = "3"\n')
    assert main(["--manifest-path", str(tmp_path / "Cargo.toml")]) == EXIT_VIOLATIONS
    out = capsys.readouterr().out
    reported = {
        line.split(" at ")[1].split("/src/")[0].rsplit("/", 1)[-1]
        for line in out.splitlines()
        if line.startswith("VIOLATION [V-DET-19/float]")
    }
    assert reported == set(real) - {"rh-sample"}


# --- Allows of clippy's disallowed lists (VER-SPR-003-TEST GAP-5) -------------------------------


def lint_allow_texts(source: str) -> tuple[list[str], list[str]]:
    """Return the lint-allow violations and escapes of ``source`` scanned as plane-A code."""
    report = Report(violations=[], escaped=[])
    scan_source(source, "lib.rs", report)
    rule = "[V-DET-19/lint-allow]"
    return (
        [v for v in report.violations if rule in v],
        [e for e in report.escaped if rule in e],
    )


@pytest.mark.parametrize(
    "source",
    [
        "#[allow(clippy::disallowed_methods)]\nfn f() {}",
        '#[allow(clippy::disallowed_types, reason = "x")]\nfn f() {}',
        '#![allow(clippy::float_arithmetic, clippy::disallowed_methods, reason = "x")]',
        "#[expect(clippy::disallowed_methods)]\nfn f() {}",
        '#[cfg_attr(test, allow(clippy::disallowed_methods, reason = "x"))]\nfn f() {}',
        "#[allow(clippy::style)]\nfn f() {}",
        "#[allow(clippy::all)]\nfn f() {}",
        "#[allow(\n    clippy::panic,\n    clippy::disallowed_methods,\n)]\nfn f() {}",
        "#[allow(clippy :: disallowed_methods)]\nfn f() {}",
    ],
)
def test_each_allow_of_the_disallowed_lists_is_reported(source: str) -> None:
    """Item or crate level, allow or expect, the group or the lint, multi-line or spaced."""
    violations, _ = lint_allow_texts(source)
    assert len(violations) == 1


@pytest.mark.parametrize(
    "source",
    [
        '#![allow(clippy::float_arithmetic, reason = "the float zone")]',
        "#[allow(clippy::disallowed_script_idents)]\nfn f() {}",
        "#[allow(clippy::style_lint_name_that_is_longer)]\nfn f() {}",
        "// #[allow(clippy::disallowed_methods)]\nfn f() {}",
        'const S: &str = "allow(clippy::disallowed_methods)";',
    ],
)
def test_other_allows_are_not_reported(source: str) -> None:
    """Other lints, comments and strings are not an allow of the disallowed lists."""
    assert lint_allow_texts(source) == ([], [])


def test_a_reasoned_escape_counts_the_allow() -> None:
    """With a LINT-ALLOW and a reason, the allow is excused, counted and printed."""
    source = (
        "// LINT-ALLOW(forbidden-api): a vendored kernel needs std exp (ruling pending)\n"
        '#[allow(clippy::disallowed_methods, reason = "x")]\nfn f() {}'
    )
    violations, escaped = lint_allow_texts(source)
    assert violations == []
    assert len(escaped) == 1


# --- Float-yielding dependency APIs (VER-SPR-007-TEST GAP-1) ----------------------------------

FLOAT_DEP = FIXTURES / "float_dep"


def scan_with_deps(source: str, floats: str = "rule") -> Report:
    """Scan a snippet as a file of a crate allowed rusqlite."""
    report = Report(violations=[], escaped=[])
    rusqlite = float_yielding_apis()["rusqlite"][1]
    scope: FloatScope = "none" if floats == "none" else "rule"
    scan_source(source, "snippet.rs", report, floats=FloatScan(scope, (("rusqlite", rusqlite),)))
    return report


def test_the_float_yielding_apis_come_from_lint_arch() -> None:
    """The REAL accessors of rusqlite are listed, for the one crate allowed rusqlite."""
    apis = float_yielding_apis()
    users, pattern = apis["rusqlite"]
    assert users == frozenset({"rh-snapshot"})
    for token in ("Real", "as_f64", "as_f64_or_null"):
        assert pattern.fullmatch(token), token
    assert not pattern.search("RealTime is_real as_f64x")


@pytest.mark.parametrize(
    "source",
    [
        "match v { ValueRef::Real(x) => x.to_bits(), _ => 0 }",
        "matches!((a, b), (ValueRef::Real(x), ValueRef::Real(y)) if x < y)",
        "if let ValueRef::Real(x) = v { x.max(y).floor() as i64 } else { 0 }",
        "v.as_f64_or_null()",
    ],
)
def test_each_float_yielding_use_is_reported(source: str) -> None:
    """The Tester's snippets name no float type, yet each is a violation."""
    assert scan(source).violations == [] or "float" in scan(source).violations[0]
    report = scan_with_deps(source)
    assert any("[V-DET-19/float-dep]" in v for v in report.violations), report.violations


def test_float_yielding_apis_are_not_checked_in_tests_or_other_crates() -> None:
    """tests/ is float-free by the DECISION; a crate not allowed the dependency gets no list."""
    source = "match v { ValueRef::Real(x) => x.to_bits(), _ => 0 }"
    assert scan_with_deps(source, floats="none").violations == []
    assert scan(source).violations == []


def test_a_declared_float_field_escape_excuses_a_float_yielding_api() -> None:
    """The same escape as a float, and only that one."""
    line = "let m = ValueRef::Real(x); // LINT-ALLOW(forbidden-api): f32_field region.soil_moisture"
    report = Report(violations=[], escaped=[])
    rusqlite = float_yielding_apis()["rusqlite"][1]
    fields = frozenset({"region.soil_moisture"})
    scan_source(line, "s.rs", report, fields, floats=FloatScan("rule", (("rusqlite", rusqlite),)))
    assert report.violations == []
    assert len(report.escaped) == 1
    other = "let m = ValueRef::Real(x); // LINT-ALLOW(forbidden-api): it is fine"
    report = Report(violations=[], escaped=[])
    scan_source(other, "s.rs", report, floats=FloatScan("rule", (("rusqlite", rusqlite),)))
    assert any("float-dep" in v for v in report.violations)


def test_the_float_dep_fixture_reports_each_plant_and_not_tests(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Every planted file of float_dep/ is reported; its tests/ file is not."""
    assert main(["--manifest-path", str(FLOAT_DEP / "Cargo.toml")]) == EXIT_VIOLATIONS
    out = capsys.readouterr().out
    for name in ("accessor.rs", "arrow_float.rs", "real_bits.rs", "real_cmp.rs", "real_floor.rs"):
        assert "float-dep] `" in out and f"rh-snapshot/src/{name}" in out, name
    # SPR-013: parquet's float names in rh-stream, the one crate allowed parquet.
    stream = "tests/lint_fixtures/forbidden/float_dep/rh-stream/src"
    assert f"float-dep] `get_double` at {stream}/lib.rs" in out
    assert f"float-dep] `Float` at {stream}/field_float.rs" in out
    assert "tests/allowed.rs" not in out


# --- Literal include! is followed (BCK-002 A-27; SPR-008 T-00) ---------------------------------


def scan_file(path: Path, repo: Path) -> Report:
    """Scan one file on disk as plane-A code, resolving its include!s inside ``repo``."""
    report = Report(violations=[], escaped=[], repo=repo)
    scan_source(path.read_text(encoding="utf-8"), path, report)
    return report


def write(path: Path, text: str) -> Path:
    """Write ``text`` to ``path``, creating its directory."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


@pytest.mark.parametrize(
    "invocation",
    [
        'include!("gen/skel.rs");',
        'include! { r"gen/skel.rs" }',
        'include![r#"gen/skel.rs"#];',
        'include!( /* the skeleton */ "gen/skel.rs" );',
        'std::include!("gen/skel.rs");',
    ],
)
def test_a_literal_include_is_followed_and_its_target_scanned(
    tmp_path: Path, invocation: str
) -> None:
    """The include! itself is no finding; the included file's forbidden API is, at its own line."""
    write(tmp_path / "src" / "gen" / "skel.rs", "fn a() {}\nlet t = Instant::now();\n")
    report = scan_file(write(tmp_path / "src" / "lib.rs", invocation + "\n"), tmp_path)
    (violation,) = report.violations
    assert "[V-DET-19/wall-clock]" in violation and "skel.rs:2:" in violation
    assert (report.files, report.includes) == (2, 1)


def test_a_clean_included_file_passes(tmp_path: Path) -> None:
    """A literal include! of a clean file leaves nothing to report."""
    write(tmp_path / "gen.rs", "pub const X: u8 = 1;\n")
    report = scan_file(write(tmp_path / "lib.rs", 'include!("gen.rs");\n'), tmp_path)
    assert (report.violations, report.escaped, report.includes) == ([], [], 1)


@pytest.mark.parametrize(
    "invocation",
    [
        'include!(concat!("gen", ".rs"));',
        'include!(concat!(env!("OUT_DIR"), "/gen.rs"));',
        "include!($file);",
        'include!("gen\\x2ers");',
        'include!(b"gen.rs");',
        'include!("gen.rs" "x");',
    ],
)
def test_a_non_literal_include_stays_forbidden(tmp_path: Path, invocation: str) -> None:
    """Only one plain string literal is followed; anything else is the source-include finding."""
    write(tmp_path / "gen.rs", "pub const X: u8 = 1;\n")
    report = scan_file(write(tmp_path / "lib.rs", invocation + "\n"), tmp_path)
    assert [v.split("]")[0] for v in report.violations] == ["VIOLATION [V-DET-19/source-include"]
    assert report.includes == 0


def test_an_include_outside_the_repository_stays_forbidden(tmp_path: Path) -> None:
    """A path that leaves the repository is not followed, even when the file exists."""
    write(tmp_path / "outside.rs", "pub const X: u8 = 1;\n")
    repo = tmp_path / "repo"
    report = scan_file(write(repo / "lib.rs", 'include!("../outside.rs");\n'), repo)
    (violation,) = report.violations
    assert "[V-DET-19/source-include]" in violation and "outside the repository" in violation


def test_a_symlink_out_of_the_repository_stays_forbidden(tmp_path: Path) -> None:
    """The repository check is made on the resolved path, after following symlinks."""
    target = write(tmp_path / "outside.rs", "let t = Instant::now();\n")
    repo = tmp_path / "repo"
    (repo / "gen.rs").parent.mkdir(parents=True)
    (repo / "gen.rs").symlink_to(target)
    report = scan_file(write(repo / "lib.rs", 'include!("gen.rs");\n'), repo)
    (violation,) = report.violations
    assert "outside the repository" in violation


def test_an_include_of_a_missing_file_stays_forbidden(tmp_path: Path) -> None:
    """A literal that names no file cannot be scanned, so it is a violation."""
    report = scan_file(write(tmp_path / "lib.rs", 'include!("gone.rs");\n'), tmp_path)
    (violation,) = report.violations
    assert "names no file" in violation


def test_an_include_in_a_snippet_without_a_file_stays_forbidden() -> None:
    """With no file on disk there is nothing to resolve from."""
    assert rules_hit('include!("gen.rs");\n') == ["source-include"]


def test_a_nested_include_resolves_from_the_included_file(tmp_path: Path) -> None:
    """An include! inside an included file is followed from that file's own directory."""
    write(tmp_path / "gen" / "inner" / "deep.rs", "unsafe { x }\n")
    write(tmp_path / "gen" / "outer.rs", 'include!("inner/deep.rs");\n')
    report = scan_file(write(tmp_path / "lib.rs", 'include!("gen/outer.rs");\n'), tmp_path)
    (violation,) = report.violations
    assert "[V-DET-19/unsafe]" in violation and "deep.rs:1:" in violation
    assert report.includes == 2


def test_an_include_cycle_is_a_violation_not_a_hang(tmp_path: Path) -> None:
    """A file that includes itself, directly or through another, is reported once."""
    write(tmp_path / "a.rs", 'include!("b.rs");\n')
    write(tmp_path / "b.rs", 'include!("a.rs");\n')
    report = scan_file(tmp_path / "a.rs", tmp_path)
    assert len(report.violations) == 1 and "includes itself" in report.violations[0]


def test_the_included_file_keeps_the_includers_float_scope(tmp_path: Path) -> None:
    """A float in a file that src/ includes is a float violation, as if written in src/."""
    write(tmp_path / "gen.rs", "pub const X: f32 = 0.0;\n")
    report = scan_file(write(tmp_path / "lib.rs", 'include!("gen.rs");\n'), tmp_path)
    assert {v.split("]")[0].split("/")[1] for v in report.violations} == {"float"}


def test_an_escape_on_a_followed_include_covers_nothing(tmp_path: Path) -> None:
    """A followed include! is no finding, so a leftover escape on it must go."""
    write(tmp_path / "gen.rs", "pub const X: u8 = 1;\n")
    source = 'include!("gen.rs"); // LINT-ALLOW(forbidden-api): no longer needed\n'
    report = scan_file(write(tmp_path / "lib.rs", source), tmp_path)
    (violation,) = report.violations
    assert "[V-DET-19/escape]" in violation and "covers no forbidden API" in violation


def test_a_file_reached_twice_is_scanned_once(tmp_path: Path) -> None:
    """A file scanned directly and through an include! reports its findings once."""
    write(tmp_path / "src" / "gen.rs", "unsafe { x }\n")
    write(tmp_path / "src" / "lib.rs", 'include!("gen.rs");\n')
    report = scan_paths([tmp_path / "src"], repo=tmp_path)
    assert len(report.violations) == 1


def test_the_real_workspace_follows_the_real_specs_skeleton_includes() -> None:
    """rh-registry's real_specs.rs brings in its four skeletons by literal include!, scanned."""
    crates = plane_a_crate_dirs(REPO / "Cargo.toml")
    report = scan_paths(directory for _, directory in crates)
    generated = REPO / "crates" / "rh-core" / "tests" / "generated"
    for stem in ("S5_daily_life", "M2_demography", "S6_artifacts", "S7_vows"):
        target = (generated / f"{stem}.rs").resolve()
        assert any(f == target and scan.scope == "none" for f, scan in report.scanned), stem
    assert report.includes >= 4
    assert not any("real_specs.rs" in line for line in report.escaped)


# --- The include macro renamed, and data includes (VER-SPR-008 audit) --------------------------


@pytest.mark.parametrize(
    "snippet",
    [
        "use core::include as inc;",
        "use std::include;",
        "use core::{include as inc, line};",
    ],
)
def test_a_use_of_the_include_macro_is_forbidden(snippet: str) -> None:
    """A renamed include! (`inc!("/abs/outside.rs")`) would hide an include from the gate."""
    assert rules_hit(snippet + "\n") == ["source-include"]


def test_a_module_named_like_include_data_is_not_flagged() -> None:
    """`include_str` and `include_bytes` are other names; a `use` of them is no include!."""
    assert rules_hit("use crate::include_helpers::x;\n") == []


def test_an_in_repository_data_include_is_allowed_and_not_scanned(tmp_path: Path) -> None:
    """include_str!/include_bytes! of a file inside the repository: allowed; it is data."""
    write(tmp_path / "golden" / "table.csv", "HashMap,unsafe\n")
    source = (
        'let s = include_str!("golden/table.csv");\nlet b = include_bytes!("golden/table.csv");\n'
    )
    report = scan_file(write(tmp_path / "lib.rs", source), tmp_path)
    assert (report.violations, report.includes) == ([], 0)


@pytest.mark.parametrize(
    "invocation",
    [
        'include_str!("../outside.txt")',
        'include_bytes!("/etc/hostname")',
        'include_str!(concat!(env!("OUT_DIR"), "/x"))',
        'include_bytes!("missing.bin")',
    ],
)
def test_a_data_include_outside_or_unresolvable_is_forbidden(
    tmp_path: Path, invocation: str
) -> None:
    """Outside the repository, missing, or not a plain literal: a data-include violation."""
    write(tmp_path / "outside.txt", "x")
    repo = tmp_path / "repo"
    report = scan_file(write(repo / "lib.rs", f"let x = {invocation};\n"), repo)
    assert [v.split("]")[0] for v in report.violations] == ["VIOLATION [V-DET-19/data-include"]


def test_an_include_resolves_from_the_symlink_not_its_target(tmp_path: Path) -> None:
    """An include joins the literal to the symlinked file's own directory (VER-SPR-008 MIN-3)."""
    write(tmp_path / "gen.rs", "let t = Instant::now();\n")
    write(tmp_path / "real" / "gen.rs", "pub const X: u8 = 1;\n")
    write(tmp_path / "real" / "inner.rs", 'include!("gen.rs");\n')
    (tmp_path / "link.rs").symlink_to(tmp_path / "real" / "inner.rs")
    report = scan_file(tmp_path / "link.rs", tmp_path)
    (violation,) = report.violations
    assert "[V-DET-19/wall-clock]" in violation, "the compiled include is ./gen.rs, not real/gen.rs"


# --- File clocks, processes and file I/O (BCK-002 A-44; SPR-011 T-00) ----------------------------


@pytest.mark.parametrize(
    ("code", "rule"),
    [
        ("let t = meta.modified()?;", "file-clock"),
        ("let t = meta . accessed ();", "file-clock"),
        ("let t = std::fs::metadata(p)?.created()?;", "file-clock"),
        ("use std::process::{Child, Command};", "process"),
        ("use std::process::*;", "process"),
        ("let c = std :: process :: Command :: new(x);", "process"),
        # DEF-1: the module, in any form of use tree.
        ("let id = std::process::id();", "process"),
        ("std::process::exit(2);", "process"),
        ('use std::process as p; fn a() { p::Command::new("date"); }', "process"),
        ("use std::process::{self as p};", "process"),
        ("use std::process::{self}; fn a() { self::Command::new(x); }", "process"),
        ("use ::std::{io, process::{self as q}};", "process"),
        ('use std::{fs as f, io}; fn a() { f::read("x"); }', "file-io"),
        ("use std::{io, fs::{self as g}};", "file-io"),
        ("pub use std::{fs};", "file-io"),
        # DEF-2: every read of a file's timestamps.
        ("let _ = Metadata::modified(&m);", "file-clock"),
        ("let g = <Metadata>::accessed;", "file-clock"),
        ("use std::os::linux::fs::MetadataExt;", "file-clock"),
        ("let t = m.mtime() + m.ctime_nsec();", "file-clock"),
        ("let t = m.st_atime();", "file-clock"),
        ("let t = m.last_write_time();", "file-clock"),
        # A-4: the environment, in any import form.
        ('let s = std::env::var("EPOCHREALTIME");', "env"),
        ("let a: Vec<String> = std::env::args().collect();", "env"),
        ("use std::env as e;", "env"),
        ("use std::{io, env::{self as e}};", "env"),
        ("use std::env::vars_os;", "env"),
        ("use std::fs;", "file-io"),
        ("let f = fs::File::open(p);", "file-io"),
        ("let f = File::create(p);", "file-io"),
        ("let o = OpenOptions::new();", "file-io"),
    ],
)
def test_a_44_rules_catch_their_forms(code: str, rule: str) -> None:
    """Each A-44 form is reported under its own rule, in engine code."""
    report = Report(violations=[], escaped=[])
    scan_source(code + "\n", "snippet.rs", report)
    assert any(f"[V-DET-19/{rule}]" in v for v in report.violations), report.violations


@pytest.mark.parametrize(
    "code",
    [
        "use std::{io, path::Path};",
        "use std::os::unix::ffi::OsStrExt;",
        "let t = clock.datetime();",
        "let c = builder.args(&[1, 2]);",
        "let e = self.environment();",
        "let m = state.modified_rows;",
        "let r = ledger.created_at(t);",
        "let p = std::path::Path::new(x);",
    ],
)
def test_a_44_rules_leave_allowed_forms_alone(code: str) -> None:
    """Field names, paths and other std modules are not file clocks, processes or I/O."""
    report = Report(violations=[], escaped=[])
    scan_source(code + "\n", "snippet.rs", report)
    assert report.violations == []


def test_file_io_is_allowed_only_where_the_list_says() -> None:
    """The reviewed list of plane-A crates allowed file I/O (A-44): widening it is a review."""
    assert set(FILE_IO_CRATES) == {"rh-snapshot"}
    assert {name: path for name, (path, _) in FILE_IO_FILES.items()} == {
        "rh-stream": ("src", "seal.rs")
    }


def test_file_io_is_exempt_in_tests_and_in_listed_crates() -> None:
    """A test reads its golden file; a listed crate's src/ may do I/O; neither is reported."""
    cases: tuple[tuple[FloatScope, bool], ...] = (("none", False), ("rule", True), ("zone", True))
    for scope, allowed in cases:
        report = Report(violations=[], escaped=[])
        scan_source("std::fs::read(p);\n", "x.rs", report, floats=FloatScan(scope, (), allowed))
        assert report.violations == [], (scope, allowed)
    report = Report(violations=[], escaped=[])
    scan_source("std::fs::read(p);\n", "x.rs", report, floats=FloatScan("zone"))
    assert len(report.violations) == 1


def test_the_file_io_workspace_reports_src_only(capsys: pytest.CaptureFixture[str]) -> None:
    """Three lib.rs files are reported; tests/, rh-snapshot and rh-stream's seal.rs are not.

    rh-core, rh-registry (off FILE_IO_CRATES since A-51) and rh-stream outside its one allowed
    file (FILE_IO_FILES, SPR-013).
    """
    assert main(["--manifest-path", str(FIXTURES / "file_io_workspace" / "Cargo.toml")]) == 1
    out = capsys.readouterr().out
    violations = [line for line in out.splitlines() if line.startswith("VIOLATION")]
    assert len(violations) == 3
    assert "file_io_workspace/rh-core/src/lib.rs:4:5" in violations[0]
    assert "file_io_workspace/rh-registry/src/lib.rs:4:5" in violations[1]
    assert "file_io_workspace/rh-stream/src/lib.rs:5:5" in violations[2]
    assert "seal.rs" not in "".join(violations)
    assert "file I/O allowed in rh-snapshot only (A-44)" in out
    assert "in the file rh-stream/src/seal.rs (FILE_IO_FILES)" in out


def test_def_1_a_path_after_a_grouped_import_is_reported_on_its_own_line(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """The `fs::<item>` pattern is not dead weight behind the use-tree check (DEF-1 test gap)."""
    assert main(["--scan", str(FIXTURES / "fs_path_after_group.rs")]) == EXIT_VIOLATIONS
    lines = [v for v in capsys.readouterr().out.splitlines() if v.startswith("VIOLATION")]
    assert [line.split(" at ", 1)[1].split(";")[0][-4:] for line in lines] == [":4:1", "5:18"]
    assert "`fs::r`" in lines[1]


@pytest.mark.parametrize(
    ("tree", "paths"),
    [
        ("std::process as p", [("std", "process")]),
        ("std::process::{self as p}", [("std", "process")]),
        (
            "::std::{io, fs::{self as g, File}}",
            [("std", "io"), ("std", "fs"), ("std", "fs", "File")],
        ),
        ("std::{fs, io}", [("std", "fs"), ("std", "io")]),
        ("a::b::*", [("a", "b", "*")]),
    ],
)
def test_use_trees_expand_to_the_paths_they_import(tree: str, paths: list[tuple[str, ...]]) -> None:
    """Renames are dropped, `self` is its prefix, groups nest (DEF-1)."""
    assert expand_use(f" {tree};") == paths


def test_a_4_the_env_workspace_reports_src_only(capsys: pytest.CaptureFixture[str]) -> None:
    """rh-core's src/ read is reported; its tests/ override and a builder's `args` are not."""
    assert main(["--manifest-path", str(FIXTURES / "env_workspace" / "Cargo.toml")]) == 1
    lines = [v for v in capsys.readouterr().out.splitlines() if v.startswith("VIOLATION")]
    assert len(lines) == 1
    assert "env_workspace/rh-core/src/lib.rs:5:35" in lines[0]


@pytest.mark.parametrize(
    "source",
    [
        "fn f() { let _ = std::env::current_dir(); }",
        "fn f() { let _ = ::std::env::current_exe(); }",
        "fn f() { let _ = std :: env :: temp_dir(); }",
        "fn f() { let _ = std::env::home_dir(); }",
        'fn f() { let _ = std::env::set_current_dir("/"); }',
        'fn f() { let _ = std::env::split_paths("a"); }',
        'fn f() { let _ = std::env::join_paths(["a"]); }',
        "fn f() { let _ = std::env::set_var; }",
        "fn f() { let _ = env::current_dir(); }",
        "use std::env::{self, consts};",
        "use std::env::*;",
        "use std::{env::consts, env::temp_dir};",
    ],
)
def test_def_5_any_path_into_std_env_is_refused(source: str) -> None:
    """Every form of std::env but its constants is the env rule (DEF-5)."""
    assert "env" in rules_hit(source)


@pytest.mark.parametrize(
    "source",
    [
        "fn f() -> &'static str { std::env::consts::OS }",
        "use std::env::consts;",
        "use std::env::consts::{ARCH, FAMILY};",
        'fn f() -> &\'static str { env!("CARGO_PKG_NAME") }',
        'fn f() -> Option<&\'static str> { option_env!("X") }',
        "fn f(env: u8) -> u8 { env }",
        "fn f() { let _ = my_env::current_dir(); }",
    ],
)
def test_def_5_the_constants_and_the_compile_time_macros_pass(source: str) -> None:
    """std::env::consts is fixed by the build; env! and option_env! are compile-time (DEF-5)."""
    assert rules_hit(source) == []


@pytest.mark.parametrize(
    ("source", "rule"),
    [
        ("use std as s;\nuse s::process as p;\nfn f() -> u32 { p::id() }", "std-rename"),
        ("extern crate std as s;\nfn f() { s::process::exit(0) }", "std-rename"),
        ("use ::std as s;", "std-rename"),
        ("use std::{self as s, io};", "std-rename"),
        ("fn f() { let _ = r#std::r#process::id(); }", "process"),
        ("fn f() { let _ = r#std::r#env::current_dir(); }", "env"),
        ("use r#std::r#fs;", "file-io"),
        ('fn f() { let _ = s::fs::read("a"); }', "file-io"),
        ('fn f() { let _ = std::os::unix::fs::symlink("a", "b"); }', "file-io"),
    ],
)
def test_maj_1_min_2_renamed_std_raw_identifiers_and_nested_fs_are_refused(
    source: str, rule: str
) -> None:
    """A renamed std, a raw identifier or an `X::fs::` path reaches no module (MAJ-1, MIN-2)."""
    assert rule in rules_hit(source)


@pytest.mark.parametrize(
    "source",
    [
        "use std::collections::BTreeMap as Map;",
        "fn f(r#type: u8) -> u8 { r#type }",
        'fn f() -> &\'static str { r#"std::fs"# }',
    ],
)
def test_min_2_other_renames_and_raw_identifiers_pass(source: str) -> None:
    """Only std itself may not be renamed; a raw identifier or raw string is not a path (MIN-2)."""
    assert rules_hit(source) == []


