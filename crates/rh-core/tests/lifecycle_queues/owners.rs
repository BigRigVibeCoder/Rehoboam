//! V-INV-04's absent-owner rows on stub systems (SPR-014 T-04; `05.1` §2 rule 2, "Absent owners";
//! REV-P1a NEW-4; REV-EVO-001 R4, R12).
//!
//! `J1.read` reads two columns of `Z9.market`, which the world does not register: `price.level`
//! as a rate read (`q16_16`, neutral 1.5) and `mood.kind` as a mode read (an enum whose neutral is
//! code 2, not 0). Both are dropped and reported: each enumerates as an empty table and a keyed
//! lookup returns the declared neutral, never the type's zero. J1 also sends `Sell` to Z9 every
//! day: each submission is accepted, discarded and counted under `dropped{J1.read, Sell,
//! absent_owner}`. An identity read of Z9 refuses the world by name.

use rh_core::access::Lag;
use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{DropKind, DropReason, PulseBinding, SimError, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::BranchSalt;

use super::delivery::person;
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
/// 1.5 in `Q16_16`.
const PRICE_NEUTRAL: i64 = 3 << 15;
/// An enum neutral that is not code 0.
const MOOD_NEUTRAL: i64 = 2;

fn read_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    assert!(
        v.read("price.level")?.is_empty(),
        "a dropped read enumerates as empty"
    );
    assert!(v.read("mood.kind")?.is_empty());
    let seen = [
        v.value("price.level", 0)?.unwrap_or(-1),
        v.value("mood.kind", 41)?.unwrap_or(-1),
    ];
    v.write("read.seen")?.extend(seen);
    c.submit(Command {
        kind: "Sell",
        person: person(0),
        args: vec![Arg::Int(1)],
        cause: EventRef::None,
    })
}

fn reader() -> Stub {
    Stub::new("J1.read")
        .writes("read.seen")
        .reads("price.level", "Z9.market", Lag::Lagged)
        .reads("mood.kind", "Z9.market", Lag::Lagged)
        .sends("Sell", "Z9.market")
        .at(Pulse::Day, Phase::Agenda)
        .stepping(read_step)
}

/// V-INV-04: a rate and a mode read of an absent owner give their typed neutrals (a `q16_16` and
/// an enum whose neutral is code 2); a send to it is discarded and counted.
#[test]
fn v_inv_04_absent_owner_reads_give_their_neutrals_and_sends_are_discarded() {
    let j1 = reader()
        .uses("price.level", "rate", Some(PRICE_NEUTRAL))
        .uses("mood.kind", "mode", Some(MOOD_NEUTRAL));
    let specs = agreeing_specs(std::slice::from_ref(&j1));
    let registered = register(vec![j1.boxed()], &specs).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        registered.reports(),
        [
            "J1.read: reads price.level from Z9.market, which is not registered: a rate read, dropped; its neutral is 98304",
            "J1.read: reads mood.kind from Z9.market, which is not registered: a mode read, dropped; its neutral is 2",
            "J1.read: submits command Sell to Z9.market, which is not registered: each submission is discarded and counted under dropped{J1.read, Sell, absent_owner} (05.1 §2 rule 2)",
        ]
    );
    // No lag snapshot for a dropped read (NEW-6).
    assert!(registered.lagged_reads().is_empty());
    let mut tables = WorldTables::new();
    tables.insert("read.seen", Vec::new());
    let mut e = Engine::new(
        CALENDAR,
        3,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(5 * DAY - 1), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    let seen = e.tables().get("read.seen").unwrap();
    assert_eq!(seen.len(), 10, "five days, two lookups each");
    assert!(
        seen.chunks(2).all(|c| c == [PRICE_NEUTRAL, MOOD_NEUTRAL]),
        "{seen:?}"
    );
    assert_eq!(e.commands().len(), 0, "never queued");
    assert_eq!(
        e.dropped().get(&(
            "J1.read",
            DropKind::Command("Sell"),
            DropReason::AbsentOwner
        )),
        Some(&5)
    );
    assert!(e.tables().get("price.level").is_none(), "never created");
}

/// An identity read of an absent owner is refused by name (rule `absent-owner`), and so is a rate
/// read with no typed neutral (the type's zero is never a fallback).
#[test]
fn an_identity_read_of_an_absent_owner_is_refused_by_name() {
    for (j1, want) in [
        (
            reader().uses("mood.kind", "mode", Some(MOOD_NEUTRAL)),
            "VIOLATION [absent-owner] J1.read: reads price.level as an identity read from Z9.market, which is not registered: a world that registers J1.read registers Z9.market (05.1 §2 rule 2)",
        ),
        (
            reader().uses("price.level", "rate", None).uses(
                "mood.kind",
                "mode",
                Some(MOOD_NEUTRAL),
            ),
            "VIOLATION [absent-owner] J1.read: reads price.level as a rate read from Z9.market, which is not registered, and its spec gives no typed neutral (05.1 §2 rule 2: the type's zero is never a fallback)",
        ),
    ] {
        let specs = agreeing_specs(std::slice::from_ref(&j1));
        let Err(e) = register(vec![j1.boxed()], &specs) else {
            panic!("must be refused: {want}");
        };
        let shown: Vec<String> = e.violations.iter().map(ToString::to_string).collect();
        assert_eq!(shown, [want]);
    }
}
