//! T-04's timings, reported and never gated (SPR-017; D12 expects under a millisecond per program
//! evaluation): the seven Registry laws on `fan4_pruned` (583 candidates), and `primogeniture` on a
//! 4,000-member lineage. Run on demand, in release:
//! `cargo test -p rh-manifest --release --test effect_timing -- --ignored --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::float_arithmetic,
    reason = "a wall-clock report of a tool-plane test, never a gate"
)]

use std::fmt::Write as _;
use std::time::Instant;

use rh_manifest::effect_cases::Golden;
use rh_registry::effect::succession::{CompiledLaw, LawText};
use rh_registry::effect::{Diagnostics, EvalOptions, Mutant, Value};

const LAWS: [&str; 7] = [
    "primogeniture",
    "ultimogeniture",
    "seniority",
    "partition",
    "elective",
    "tanistry",
    "sortition",
];

fn golden_with(extra: &str) -> Golden {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/registry/effect_language_golden_v0.toml"
    );
    let mut text = std::fs::read_to_string(path).unwrap();
    // Worlds go before the cases: TOML tables after `[[case]]` would land inside the last case.
    let at = text.find("\n[[case]]").unwrap();
    text.insert_str(at, extra);
    Golden::parse(&text).unwrap()
}

/// A tree of 4,001 persons: the root, 16 sons, 256 grandsons and 3,728 great-grandsons spread over
/// the grandsons; `lineage(2, 3)` is every person but 2, so 4,000 members, all alive.
fn four_thousand() -> String {
    let mut out = String::from("\n[worlds.big]\nnow = 0\ndays_per_year = 360\n");
    let mut id = 1_u64;
    let mut rows = vec![(1_u64, 0_u64, -3_i64)];
    let mut level = vec![1_u64];
    for (g, count) in [(-2_i64, 16_usize), (-1, 256), (0, 3728)] {
        let mut next = Vec::new();
        for i in 0..count {
            id += 1;
            rows.push((id, level[i % level.len()], g));
            next.push(id);
        }
        level = next;
    }
    for (p, father, g) in rows {
        let birth = g * 10_800 * 1440;
        let _ = write!(
            out,
            "[[worlds.big.person]]\nid = {p}\nalive = true\nsex = \"male\"\nbirth_tick = {birth}\nmother = 0\nfather = {father}\n"
        );
        let _ = write!(
            out,
            "[[worlds.big.life_course]]\nid = {}\nperson_id = {p}\nmother = 0\nfather = {father}\nbirth_tick = {birth}\nsex = \"male\"\n",
            100_000 + p
        );
    }
    out
}

fn time_law(g: &Golden, world: &str, law: &str) -> (usize, f64, f64) {
    let texts = &g.laws[law];
    let started = Instant::now();
    let compiled = CompiledLaw::compile(
        &g.env,
        "M7.dynasty",
        LawText {
            candidates: &texts[0],
            eligible: &texts[1],
            order: &texts[2],
        },
    )
    .unwrap();
    let compile_ms = started.elapsed().as_secs_f64() * 1e3;
    let w = g.world(&g.env, world, Mutant::None).unwrap();
    let host = w.host(&g.env);
    let person = g.env.kind("person").unwrap();
    let title = g.env.kind("title").unwrap();
    let bindings = [Value::Ent(person, 2), Value::Ent(title, 100), Value::Int(0)];
    let started = Instant::now();
    let line = compiled.line(
        &g.env,
        &host,
        &bindings,
        &mut Diagnostics::default(),
        EvalOptions::default(),
    );
    let line_ms = started.elapsed().as_secs_f64() * 1e3;
    (line.map_or(0, |l| l.len()), compile_ms, line_ms)
}

#[test]
#[ignore = "a timing report, run on demand"]
fn report_law_timings() {
    let g = golden_with(&four_thousand());
    for law in LAWS {
        let (n, c, l) = time_law(&g, "fan4_pruned", law);
        println!(
            "timing fan4_pruned {law:<15} line of {n:>4}: compile {c:7.3} ms, line {l:9.3} ms"
        );
    }
    let (n, c, l) = time_law(&g, "big", "primogeniture");
    println!(
        "timing big (4,000 members) primogeniture line of {n}: compile {c:.3} ms, line {l:.3} ms, {:.4} ms per candidate",
        l / 4000.0
    );
}
