//! The authored-number boundary against its exact reference (SPR-011 T-04; `05.5` §3.2).
//!
//! Replays every vector of `tests/golden/decimal_vectors.txt`, which `bin/gen_decimal_vectors.py`
//! generates from Python's `decimal` (random literals over the YAML, Python and TOML grammars,
//! halfway cases included); `bin/check_all.sh` regenerates the file and fails if it differs. Each
//! literal must round to the same bits, half to even, at 16 and 32 fraction bits.
#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "test helpers: a malformed vector file must fail the test loudly"
)]

use std::collections::BTreeMap;

use rh_manifest::exact::{Decimal, DecimalError};

const VECTORS: &str = include_str!("golden/decimal_vectors.txt");

fn parse(grammar: &str, literal: &str) -> Result<Decimal, DecimalError> {
    match grammar {
        "yaml" => Decimal::parse_yaml_float(literal),
        "python" => Decimal::parse_python_float(&literal.replace('~', " ")),
        "toml" => Decimal::parse_toml_float(literal),
        other => panic!("unknown grammar {other}"),
    }
}

#[test]
fn every_literal_rounds_as_the_decimal_reference_does() {
    let mut per_grammar: BTreeMap<&str, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for (index, line) in VECTORS.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let [grammar, bits, literal, expected] = tokens[..] else {
            panic!(
                "line {}: `{line}` is not <grammar> <bits> <literal> <expected>",
                index + 1
            );
        };
        let frac_bits: u32 = bits.parse().expect("frac bits");
        *per_grammar.entry(grammar).or_default() += 1;
        let actual = match parse(grammar, literal) {
            Err(DecimalError::NotFinite(_)) => "notfinite".to_owned(),
            Err(e) => format!("error: {e}"),
            Ok(d) => d
                .to_fixed(frac_bits)
                .map_or_else(|| "none".to_owned(), |v| v.to_string()),
        };
        if actual != expected {
            failures.push(format!("line {}: `{line}` gave {actual}", index + 1));
        }
    }
    assert!(
        failures.is_empty(),
        "{} vectors differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    for grammar in ["yaml", "python", "toml"] {
        assert!(
            per_grammar.get(grammar).copied().unwrap_or(0) >= 800,
            "too few `{grammar}` vectors: {per_grammar:?}"
        );
    }
}

#[test]
fn the_vectors_hold_halfway_cases_on_both_sides() {
    // A tie line ends in 5 and its expected value is even; the generator also writes the literal
    // one digit above and below it. Guard that the file keeps them.
    let ties = VECTORS
        .lines()
        .filter(|l| {
            l.starts_with("yaml 16 ") && l.split(' ').nth(2).is_some_and(|t| t.ends_with('5'))
        })
        .count();
    assert!(ties >= 20, "only {ties} tie literals");
}
