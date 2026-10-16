//! T-04: the `AccessSpec` conflict checker, and its agreement with the ownership lint (SPR-005).
//!
//! Acceptance: two systems writing one column fail registration. Ruling 3: the checker uses the
//! same single-writer rule as the ownership lint (one owner per column, reads declared, no plane-B
//! or plane-C read from plane A); the lint reads the specs, the checker the registered code.
//!
//! Refs: SPR-005 T-04, ruling 3; `05.1` §2 rules 1 and 4; ADR-0024.

#![allow(
    clippy::panic,
    clippy::many_single_char_names,
    reason = "test helpers fail loudly; stub systems read best as a, b, c"
)]

mod support;

use rh_core::access::{Lag, Plane};
use rh_core::register::{RegistrationError, register, system_id};
use rh_core::{Phase, Pulse};
use rh_registry::ownership::{Declaration, Source, single_writer};
use support::{Stub, agreeing_ownership, ownership};

fn refused(stubs: Vec<Stub>, specs_from: &[Stub]) -> RegistrationError {
    let t = agreeing_ownership(specs_from);
    match register(stubs.into_iter().map(Stub::boxed).collect(), &t) {
        Ok(r) => panic!("registration should fail; order {:?}", r.ids()),
        Err(e) => e,
    }
}

fn rules(e: &RegistrationError) -> Vec<&'static str> {
    e.violations.iter().map(|v| v.rule).collect()
}

/// The acceptance case: each spec owns its column, but the code of two systems writes the same one.
#[test]
fn two_systems_writing_one_column_fail_registration() {
    let a = Stub::new("M2.demography")
        .writes("person.alive")
        .at(Pulse::Month, Phase::Vital);
    let b = Stub::new("S5.daily_life")
        .writes("encounter_log.day_index")
        .at(Pulse::Day, Phase::Activity);
    let b_writes_alive = b.clone().writes("person.alive");
    let e = refused(vec![a.clone(), b_writes_alive], &[a, b]);
    let messages: Vec<String> = e.violations.iter().map(ToString::to_string).collect();
    // The lint's check (the spec gives person.alive to M2) and the code-vs-code rule both fire.
    assert_eq!(
        messages,
        [
            "VIOLATION [single-writer] S5.daily_life: writes person.alive, which M2.demography owns; request the change with a lifecycle command (05.1 §2)",
            "VIOLATION [single-writer] S5.daily_life: writes person.alive, which M2.demography also writes (05.1 §2)",
        ]
    );
}

/// Code-vs-code catches a double writer even where the specs say nothing about the column.
#[test]
fn two_writers_of_a_column_no_spec_owns_are_refused_by_the_code_rule() {
    let a = Stub::new("X1.a").writes("thing.x");
    let b = Stub::new("X2.b").writes("thing.x");
    let e = refused(vec![a.clone(), b.clone()], &[a, Stub::new("X2.b")]);
    assert!(
        e.violations.iter().any(|v| v.to_string()
            == "VIOLATION [single-writer] X2.b: writes thing.x, which X1.a also writes (05.1 §2)"),
        "{e}"
    );
}

#[test]
fn registration_order_does_not_matter() {
    let a = Stub::new("X1.a")
        .writes("a.x")
        .at(Pulse::Day, Phase::Production);
    let b = Stub::new("X2.b")
        .reads("a.x", "X1.a", Lag::SameInstant)
        .writes("b.x")
        .at(Pulse::Day, Phase::Production);
    let t = agreeing_ownership(&[a.clone(), b.clone()]);
    let one =
        register(vec![a.clone().boxed(), b.clone().boxed()], &t).unwrap_or_else(|e| panic!("{e}"));
    let two = register(vec![b.boxed(), a.boxed()], &t).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(one.ids(), two.ids());
}

#[test]
fn undeclared_reads_and_wrong_owners_are_refused() {
    let a = Stub::new("X1.a").writes("a.x");
    let spec_b = Stub::new("X2.b").writes("b.x");
    // Code reads a column its spec does not list: the lint's undeclared-read.
    let b = spec_b.clone().reads("a.x", "X1.a", Lag::Lagged);
    let e = refused(vec![a.clone(), b], &[a.clone(), spec_b.clone()]);
    assert!(rules(&e).contains(&"undeclared-read"), "{e}");
    // Code reads a column naming the wrong owner: the shared single-writer rule.
    let c = Stub::new("X3.c").reads("a.x", "X2.b", Lag::Lagged);
    let e = refused(vec![a.clone(), spec_b.clone(), c.clone()], &[a, spec_b, c]);
    assert!(
        e.violations
            .iter()
            .any(|v| v.message == "reads a.x from X2.b, but X1.a owns it"),
        "{e}"
    );
}

#[test]
fn a_plane_a_system_reading_a_plane_b_column_is_refused() {
    let pins = Stub::new("X1.pins").writes_in("region.pins", Plane::B);
    let reader = Stub::new("X2.reader")
        .writes("r.x")
        .reads("region.pins", "X1.pins", Lag::Lagged);
    let e = refused(vec![pins.clone(), reader.clone()], &[pins, reader]);
    let plane: Vec<String> = e
        .violations
        .iter()
        .filter(|v| v.rule == "plane")
        .map(ToString::to_string)
        .collect();
    // Both the lint's check and the code rule name the leak.
    assert_eq!(plane.len(), 2, "{e}");
    assert!(plane.iter().all(|m| m == "VIOLATION [plane] X2.reader: a plane-A system reads plane-B column region.pins (ADR-0024)"));
}

#[test]
fn a_system_without_a_spec_and_a_double_registration_are_refused() {
    let a = Stub::new("X1.a");
    let e = refused(vec![a.clone(), a.clone()], &[a]);
    assert!(
        e.violations
            .iter()
            .any(|v| v.message == "is registered twice"),
        "{e}"
    );
    let ghost = Stub::new("X9.ghost");
    let e = refused(vec![ghost], &[]);
    assert_eq!(rules(&e), ["registry"]);
}

/// Ruling 3: on every real spec, the lint's rule and the checker's rule are the same function and
/// agree: the specs' declarations pass both, and a planted second owner fails both.
#[test]
fn the_checker_and_the_lint_share_one_rule() {
    let t = support::real_ownership();
    assert_eq!(t.problems, vec![], "the real specs are clean");
    let declarations: Vec<Declaration> = t.specs.values().map(Declaration::from_spec).collect();
    let (spec_owners, spec_problems) = single_writer(&declarations, Source::Spec);
    let (code_owners, code_problems) = single_writer(&declarations, Source::Code);
    assert_eq!(spec_owners, t.owners);
    assert_eq!(code_owners, t.owners);
    assert!(spec_problems.is_empty() && code_problems.is_empty());
    let mut planted = declarations.clone();
    planted[0]
        .writes
        .push(("person.alive".to_owned(), "A".to_owned()));
    planted.rotate_left(1); // the rightful owner (M2) comes first
    let (_, spec_problems) = single_writer(&planted, Source::Spec);
    let (_, code_problems) = single_writer(&planted, Source::Code);
    assert_eq!(spec_problems.len(), 1);
    assert_eq!(code_problems.len(), 1);
    assert_eq!(spec_problems[0].rule, code_problems[0].rule);
    assert_eq!(spec_problems[0].system, code_problems[0].system);
}

#[test]
fn system_ids_are_name_derived_and_distinct_for_the_registry() {
    let ids = rh_registry::ownership::registry_system_ids(
        &support::repo().join("specs/manifest/registry_v0.toml"),
    )
    .unwrap();
    let raw: std::collections::BTreeSet<u32> =
        ids.iter().map(|id| system_id(id).to_raw()).collect();
    assert_eq!(
        raw.len(),
        ids.len(),
        "no two Registry systems share an RNG system_id"
    );
    assert_eq!(system_id("M2.demography"), system_id("M2.demography"));
    let _ = ownership(&[]);
}
