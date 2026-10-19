//! V-WC-10's mutation tests (SPR-017 T-05): each planted mutant of the evaluator fails its named
//! sentinel cases with the wrong value it predicts, and the unmutated evaluator passes every case.
//!
//! Run with `--features mutant-effect`; a build without the feature ignores every mutant, which
//! `a_build_without_the_feature_ignores_mutants` checks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use rh_manifest::effect_cases::{Golden, check_case};
use rh_registry::effect::{EvalOptions, Mutant};

fn golden() -> Golden {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/registry/effect_language_golden_v0.toml"
    );
    let text = std::fs::read_to_string(path).unwrap();
    Golden::parse(&text).unwrap()
}

fn failures(golden: &Golden, mutant: Mutant) -> Vec<String> {
    let opts = EvalOptions { mutant };
    golden
        .cases
        .iter()
        .filter_map(|c| check_case(golden, c, opts).err())
        .collect()
}

/// The sentinels of T-05's table: the case, and the outcome the mutant predicts for it (its text,
/// and for the two short-circuit cases the `missing_reads` the skipped side no longer counts).
const SENTINELS: &[(Mutant, &str, &str, Option<u64>)] = &[
    (
        Mutant::LogByRowId,
        "succession_primogeniture",
        "line:[]",
        None,
    ),
    (
        Mutant::ShortCircuit,
        "and_evaluates_both_sides",
        "bool:false",
        Some(0),
    ),
    (
        Mutant::ShortCircuit,
        "or_evaluates_both_sides",
        "bool:true",
        Some(0),
    ),
    (
        Mutant::LineageHeightTwo,
        "lineage_reaches_great_grandparents",
        "relation:[25, 26, 12, 10, 18]",
        None,
    ),
    (
        Mutant::BoundBeforePrune,
        "lineage_fan_pruned_before_the_bound",
        "invariant:E_TRUNCATED",
        None,
    ),
    (
        Mutant::SilentCut,
        "lineage_fan_over_the_bound_is_truncated",
        "int:4096",
        None,
    ),
    (
        Mutant::NeutralIsZero,
        "neutral_fixed_value_is_the_declared_one",
        "q8_8:0",
        None,
    ),
    (
        Mutant::NeutralIsZero,
        "neutral_enum_value_is_the_declared_one",
        "enum:region_conditions.normal",
        None,
    ),
    (
        Mutant::MissingEnumOrdinalZero,
        "read_enum_of_none_is_the_declared_default",
        "enum:claim_status.active",
        None,
    ),
    (
        Mutant::MulHalfUp,
        "q16_mul_tie_to_even_zero",
        "q16_16:1",
        None,
    ),
    (
        Mutant::MulHalfUp,
        "q16_mul_tie_to_even_negative",
        "q16_16:-1",
        None,
    ),
    (
        Mutant::MotherFirst,
        "common_of_a_brother_is_the_father_first",
        "person:4",
        None,
    ),
    (
        Mutant::LedgerPerStep,
        "succession_sortition_evaluated_twice_in_one_step",
        "invariant:E_RNG_REUSE",
        None,
    ),
    // The twins 16 and 27 swap.
    (
        Mutant::TieBreakDescending,
        "succession_primogeniture",
        "line:[10, 18, 14, 26, 12, 9, 24, 27, 16, 20, 8, 21, 5, 19, 17, 6]",
        None,
    ),
    (
        Mutant::TieBreakDescending,
        "succession_seniority",
        "line:[5, 9, 19, 10, 27, 16, 20, 14, 24]",
        None,
    ),
    // Kell (10) no longer precedes his daughter Thora (18).
    (
        Mutant::PrefixLast,
        "succession_primogeniture",
        "line:[18, 10, 14, 26, 12, 24, 16, 27, 20, 9, 21, 8, 19, 17, 5, 6]",
        None,
    ),
];

#[test]
#[cfg_attr(
    not(feature = "mutant-effect"),
    ignore = "needs --features mutant-effect"
)]
fn mutants_are_killed() {
    if !Mutant::enabled() {
        // Needs --features mutant-effect (run by bin/check_all.sh with the feature flag).
        return;
    }
    let golden = golden();
    assert_eq!(
        failures(&golden, Mutant::None),
        Vec::<String>::new(),
        "Mutant::None must pass all {}",
        golden.cases.len()
    );
    for mutant in Mutant::ALL {
        let failed = failures(&golden, mutant);
        let sentinels: Vec<_> = SENTINELS.iter().filter(|s| s.0 == mutant).collect();
        assert!(!sentinels.is_empty(), "{mutant:?} has no sentinel");
        for &&(_, id, predicted, missing) in &sentinels {
            assert!(
                failed.iter().any(|f| f.starts_with(&format!("FAIL {id}:"))),
                "{mutant:?} must fail {id}; it failed {failed:#?}"
            );
            let case = golden
                .cases
                .iter()
                .find(|c| c["id"].as_str() == Some(id))
                .unwrap();
            let outcome = golden.run_case(case, EvalOptions { mutant }).unwrap();
            assert_eq!(
                outcome.text, predicted,
                "{mutant:?} on {id}: not the predicted wrong value"
            );
            if let Some(m) = missing {
                assert_eq!(
                    outcome.diags.map(|d| d.missing_reads),
                    Some(m),
                    "{mutant:?} on {id}: missing_reads"
                );
            }
        }
        println!(
            "mutant {mutant:?}: killed ({} failing cases, sentinels {:?})",
            failed.len(),
            sentinels.iter().map(|s| s.1).collect::<Vec<_>>()
        );
    }
}

#[test]
#[cfg_attr(
    feature = "mutant-effect",
    ignore = "a mutant-effect build honours mutants"
)]
fn a_build_without_the_feature_ignores_mutants() {
    assert!(!Mutant::enabled());
    let golden = golden();
    for mutant in [Mutant::PrefixLast, Mutant::LogByRowId, Mutant::MulHalfUp] {
        assert_eq!(
            failures(&golden, mutant),
            Vec::<String>::new(),
            "{mutant:?} changed a result without the feature"
        );
    }
}
