//! Golden vectors for `rh_core::fixed` (SPR-010 T-02; `Prob32`: SPR-011 T-01; GOV-002 §2 tier 5).
//!
//! Replays every vector of `tests/golden/fixed_vectors.txt`, which `bin/gen_fixed_vectors.py`
//! generates from an exact rational reference (Python `fractions`, round half to even, then
//! saturation). `bin/check_all.sh` regenerates the file and fails if it differs, so the vectors
//! here are always the reference's.
//!
//! Refs: V-DET-19 (DET-3); `05.5` §2.1.
#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "test helpers: a malformed vector file must fail the test loudly"
)]

use std::collections::BTreeMap;

use rh_core::fixed::{Money, Prob32, Q8_8, Q16_16, draw_hits};

const VECTORS: &str = include_str!("golden/fixed_vectors.txt");

fn int(token: &str) -> i64 {
    token
        .parse()
        .unwrap_or_else(|_| panic!("not an integer: {token}"))
}

fn q8(token: &str) -> Q8_8 {
    Q8_8::from_bits(i16::try_from(int(token)).expect("a Q8_8 bit pattern"))
}

fn q16(token: &str) -> Q16_16 {
    Q16_16::from_bits(i32::try_from(int(token)).expect("a Q16_16 bit pattern"))
}

fn money(token: &str) -> Money {
    Money::from_minor(int(token))
}

/// Formats an optional result the way the vector file does.
fn opt<T>(value: Option<T>, bits: impl Fn(T) -> i64) -> String {
    value.map_or_else(|| "none".to_owned(), |v| bits(v).to_string())
}

fn uint(token: &str) -> u64 {
    token
        .parse()
        .unwrap_or_else(|_| panic!("not a u64: {token}"))
}

fn p32(token: &str) -> Prob32 {
    Prob32::from_bits(uint(token)).expect("a Prob32 bit pattern")
}

fn p32_bits(p: Prob32) -> i64 {
    i64::try_from(p.to_bits()).expect("Prob32 bits fit i64")
}

fn q8_bits(q: Q8_8) -> i64 {
    q.to_bits().into()
}

fn q16_bits(q: Q16_16) -> i64 {
    q.to_bits().into()
}

/// Computes one vector's result with the engine, formatted like its expected value.
fn compute(op: &str, args: &[&str]) -> String {
    match (op, args) {
        ("q8.add", [a, b]) => q8_bits(q8(a) + q8(b)).to_string(),
        ("q8.sub", [a, b]) => q8_bits(q8(a) - q8(b)).to_string(),
        ("q8.mul", [a, b]) => q8_bits(q8(a) * q8(b)).to_string(),
        ("q8.div", [a, b]) => opt(q8(a).checked_div(q8(b)), q8_bits),
        ("q8.neg", [a]) => q8_bits(-q8(a)).to_string(),
        ("q8.widen", [a]) => q16_bits(q8(a).widen()).to_string(),
        ("q8.ratio", [n, d]) => opt(Q8_8::from_ratio(int(n), int(d)), q8_bits),
        ("q16.add", [a, b]) => q16_bits(q16(a) + q16(b)).to_string(),
        ("q16.sub", [a, b]) => q16_bits(q16(a) - q16(b)).to_string(),
        ("q16.mul", [a, b]) => q16_bits(q16(a) * q16(b)).to_string(),
        ("q16.div", [a, b]) => opt(q16(a).checked_div(q16(b)), q16_bits),
        ("q16.neg", [a]) => q16_bits(-q16(a)).to_string(),
        ("q16.narrow", [a]) => q8_bits(q16(a).narrow_saturating()).to_string(),
        ("q16.ratio", [n, d]) => opt(Q16_16::from_ratio(int(n), int(d)), q16_bits),
        ("money.add", [a, b]) => (money(a) + money(b)).to_minor().to_string(),
        ("money.sub", [a, b]) => (money(a) - money(b)).to_minor().to_string(),
        ("money.neg", [a]) => (-money(a)).to_minor().to_string(),
        ("money.mul_int", [a, n]) => money(a).saturating_mul_int(int(n)).to_minor().to_string(),
        ("money.mul_rate", [a, r]) => money(a).mul_rate(q16(r)).to_minor().to_string(),
        ("money.div_int", [a, n]) => opt(money(a).checked_div_int(int(n)), Money::to_minor),
        ("p32.bits", [b]) => opt(Prob32::from_bits(uint(b)), p32_bits),
        ("p32.draw", [d, b]) => u8::from(draw_hits(uint(d), p32(b))).to_string(),
        ("p32.from_q16", [q]) => opt(Prob32::from_q16_16(q16(q)), p32_bits),
        ("p32.to_q16", [b]) => q16_bits(p32(b).to_q16_16()).to_string(),
        ("p32.ratio", [n, d]) => opt(Prob32::from_ratio(uint(n), uint(d)), p32_bits),
        ("draw", [d, p]) => {
            let draw = d
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("not a u64 draw: {d}"));
            u8::from(draw_hits(draw, q16(p))).to_string()
        }
        _ => panic!(
            "unknown vector `{op}` with {} inputs: regenerate or extend this test",
            args.len()
        ),
    }
}

#[test]
fn every_golden_vector_matches_the_exact_reference() {
    let mut per_op: BTreeMap<&str, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for (index, line) in VECTORS.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let (op, rest) = tokens.split_first().expect("an op");
        let (expected, args) = rest.split_last().expect("an expected value");
        *per_op.entry(op).or_default() += 1;
        let actual = compute(op, args);
        if actual != *expected {
            failures.push(format!("line {}: `{line}` gave {actual}", index + 1));
        }
    }
    assert!(
        failures.is_empty(),
        "{} vectors differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // Every operation of the API is covered, so a vector family cannot silently disappear.
    let ops = [
        "q8.add",
        "q8.sub",
        "q8.mul",
        "q8.div",
        "q8.neg",
        "q8.widen",
        "q8.ratio",
        "q16.add",
        "q16.sub",
        "q16.mul",
        "q16.div",
        "q16.neg",
        "q16.narrow",
        "q16.ratio",
        "money.add",
        "money.sub",
        "money.neg",
        "money.mul_int",
        "money.mul_rate",
        "money.div_int",
        "p32.bits",
        "p32.draw",
        "p32.from_q16",
        "p32.to_q16",
        "p32.ratio",
    ];
    for op in ops {
        assert!(
            per_op.get(op).copied().unwrap_or(0) >= 50,
            "too few `{op}` vectors: {per_op:?}"
        );
    }
}
