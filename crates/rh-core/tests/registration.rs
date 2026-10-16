//! T-04: the `AccessSpec` conflict checker, and its agreement with the ownership lint (SPR-005).
//!
//! Acceptance: two systems writing one column fail registration. Ruling 3: the checker uses the
//! same single-writer rule as the ownership lint (one owner per column, reads declared, no plane-B
//! or plane-C read from plane A); the lint reads the specs, the checker the registered code.
//!
//! Refs: SPR-005 T-04, T-05 (registration errors), ruling 3; `05.1` §2 rules 1 and 4; ADR-0024.

#![allow(
    clippy::panic,
    clippy::many_single_char_names,
    reason = "test helpers fail loudly; stub systems read best as a, b, c"
)]

mod support;

use rh_core::access::{Lag, Plane};
use rh_core::register::{RegistrationError, register};
use rh_core::{Phase, Pulse};
use support::{Stub, agreeing_specs};

fn refused(stubs: Vec<Stub>, specs_from: &[Stub]) -> RegistrationError {
    let t = agreeing_specs(specs_from);
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
    let t = agreeing_specs(&[a.clone(), b.clone()]);
    let one =
        register(vec![a.clone().boxed(), b.clone().boxed()], &t).unwrap_or_else(|e| panic!("{e}"));
    let two = register(vec![b.boxed(), a.boxed()], &t).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(one.ids(), two.ids());
    assert_eq!(one.schedule(), two.schedule());
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

#[test]
fn a_same_instant_read_against_phase_order_needs_a_lag() {
    let w = Stub::new("X1.w")
        .writes("w.x")
        .at(Pulse::Month, Phase::Polity);
    let r = Stub::new("X2.r")
        .reads("w.x", "X1.w", Lag::SameInstant)
        .writes("r.x")
        .at(Pulse::Day, Phase::Production);
    let e = refused(vec![w.clone(), r.clone()], &[w.clone(), r.clone()]);
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [phase-order] X2.r: reads w.x at same_instant in phase Production, but X1.w writes it in the later phase Polity; declare a lag (05.1 §2 rule 4)"
        ]
    );
    // Declaring a lag resolves it.
    let lagged = Stub::new("X2.r")
        .reads("w.x", "X1.w", Lag::PreviousPulse)
        .writes("r.x")
        .at(Pulse::Day, Phase::Production);
    let t = agreeing_specs(&[w.clone(), lagged.clone()]);
    assert!(register(vec![w.boxed(), lagged.boxed()], &t).is_ok());
}

#[test]
fn a_same_instant_cycle_within_a_phase_needs_a_lag() {
    let a = Stub::new("X1.a")
        .writes("a.x")
        .reads("b.x", "X2.b", Lag::SameInstant)
        .at(Pulse::Day, Phase::Exchange);
    let b = Stub::new("X2.b")
        .writes("b.x")
        .reads("a.x", "X1.a", Lag::SameInstant)
        .at(Pulse::Day, Phase::Exchange);
    let e = refused(vec![a.clone(), b.clone()], &[a.clone(), b]);
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [schedule-cycle] X1.a: in phase Exchange, X1.a, X2.b read each other's columns at same_instant in a cycle; declare a lag on one read (05.1 §2 rule 4)"
        ]
    );
    let b_lagged = Stub::new("X2.b")
        .writes("b.x")
        .reads("a.x", "X1.a", Lag::Lagged)
        .at(Pulse::Day, Phase::Exchange);
    let t = agreeing_specs(&[a.clone(), b_lagged.clone()]);
    let r = register(vec![a.boxed(), b_lagged.boxed()], &t).unwrap_or_else(|e| panic!("{e}"));
    // b reads a lagged, a reads b at the same instant: b first.
    assert_eq!(r.order(Phase::Exchange), ["X2.b", "X1.a"]);
}

/// `05.1` §2 rule 4 (v1.4): levels, then Registry id. A diamond plus a long chain in one phase
/// tells the rule apart from the smallest-ready-id Kahn order, which would run the whole chain
/// first (c1, c2, c3, c4, then the diamond). If someone reverts to Kahn, this fails.
#[test]
fn the_order_is_levels_then_id_not_the_smallest_ready_id() {
    use Lag::SameInstant;
    let at = |s: Stub| s.at(Pulse::Day, Phase::Production);
    let stubs = vec![
        // The chain X1 -> X2 -> X3 -> X4.
        at(Stub::new("X1.c1").writes("c.1")),
        at(Stub::new("X2.c2")
            .writes("c.2")
            .reads("c.1", "X1.c1", SameInstant)),
        at(Stub::new("X3.c3")
            .writes("c.3")
            .reads("c.2", "X2.c2", SameInstant)),
        at(Stub::new("X4.c4")
            .writes("c.4")
            .reads("c.3", "X3.c3", SameInstant)),
        // The diamond X5 -> {X6, X7} -> X8.
        at(Stub::new("X5.top").writes("d.top")),
        at(Stub::new("X6.left")
            .writes("d.left")
            .reads("d.top", "X5.top", SameInstant)),
        at(Stub::new("X7.right")
            .writes("d.right")
            .reads("d.top", "X5.top", SameInstant)),
        at(Stub::new("X8.bottom")
            .writes("d.bottom")
            .reads("d.left", "X6.left", SameInstant)
            .reads("d.right", "X7.right", SameInstant)),
    ];
    let t = agreeing_specs(&stubs);
    let r = register(stubs.into_iter().map(Stub::boxed).collect(), &t)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        r.order(Phase::Production),
        [
            "X1.c1",
            "X5.top",
            "X2.c2",
            "X6.left",
            "X7.right",
            "X3.c3",
            "X8.bottom",
            "X4.c4"
        ]
    );
    let kahn = [
        "X1.c1",
        "X2.c2",
        "X3.c3",
        "X4.c4",
        "X5.top",
        "X6.left",
        "X7.right",
        "X8.bottom",
    ];
    assert_ne!(r.order(Phase::Production), kahn);
}
