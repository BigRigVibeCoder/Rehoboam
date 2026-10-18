//! The ledger row insert (SPR-014 T-05; `05.1` §2 rule 2, "Ledger rows enter by insert"; REV-SPR-015
//! R11), on random small worlds with two or three owners of `person.*` columns.
//!
//! `K1.ledger` owns `person.id`, `person.alive` and the life-course log. At Day/Vital it hands
//! `rh-core` new rows of its own `person.*` columns (`StepCtx::insert_person`) for a keyed draw of
//! newcomers and appends their life-course rows. `K2.wealth` owns `person.wealth`, declared with
//! `default = 7`; `K3.rank` (in the three-owner worlds) owns `person.rank`, with no default. At
//! Day/Polity of the same instant K2 reads the ledger: every newcomer is there, at its sorted
//! position, with wealth 7 and rank 0. The core's ledger invariants run at every `Bookkeeping`.

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, PersonId};

use super::delivery::SEEDS;
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const TAG: u64 = tag("rehoboam/rh-core/tests/spr014-insert/v1");
const WEALTH_DEFAULT: i64 = 7;

enum Ip {
    Born,
}

impl SpecPurpose for Ip {
    const ALL: &'static [Self] = &[Self::Born];
    fn name(&self) -> &'static str {
        "born"
    }
}

fn newcomer(day: i64, k: u64) -> u64 {
    h64_words(TAG, &[day.cast_unsigned(), k]) >> 4
}

fn ledger_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let day = c.now().minutes() / DAY;
    let mode = v.read("k1.mode")?[0];
    let mut born = Vec::new();
    for k in 0..3 {
        let draw = c.rng(&Ip::Born, PersonId::from_raw(newcomer(day, 99)), k)?;
        if draw % 2 == 0 {
            born.push(newcomer(day, k).cast_signed());
        }
    }
    if mode == 1 && day == 2 {
        // The planted duplicate: an id the ledger already holds.
        born.push(v.read("person.id")?[0]);
    }
    if mode == 2 && day == 2 {
        born.push(newcomer(day, 7).cast_signed()); // the planted short row's id
    }
    for &id in &born {
        if mode == 2 && day == 2 {
            c.insert_person(vec![("person.id", id)]); // the planted short row
        } else {
            c.insert_person(vec![("person.id", id), ("person.alive", 1)]);
        }
        v.write("life_course.person_id")?.push(id);
        v.write("life_course.exit_kind")?.push(0);
        v.write("life_course.exit_tick")?.push(i64::MAX);
        v.write("life_course.death_cause")?.push(0);
    }
    v.write("k1.born")?[0] += i64::try_from(born.len()).unwrap_or(0);
    Ok(())
}

fn wealth_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = v.read("person.id")?.len();
    let wealth = v.read("person.wealth")?.to_vec();
    let fail = |message: String| SimError::System {
        system: "K2.wealth",
        message,
    };
    if wealth.len() != ids {
        return Err(fail(format!(
            "{} wealth rows for {ids} persons",
            wealth.len()
        )));
    }
    // K2 never writes wealth: every row, the founders' and the newcomers', is the default.
    if let Some(w) = wealth.iter().find(|&&w| w != WEALTH_DEFAULT) {
        return Err(fail(format!("a wealth of {w}, not the default")));
    }
    let born = v.read("k1.born")?[0];
    let founders = v.read("k2.founders")?[0];
    if i64::try_from(ids).unwrap_or(0) != founders + born {
        return Err(fail(format!(
            "{ids} rows, not {founders} founders and {born} newcomers: an insert is not visible at Polity"
        )));
    }
    v.write("k2.seen")?[0] = i64::try_from(ids).unwrap_or(0);
    Ok(())
}

fn rank_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if v.read("person.rank")?.iter().any(|&r| r != 0) {
        return Err(SimError::System {
            system: "K3.rank",
            message: "a rank other than the type's zero".to_owned(),
        });
    }
    Ok(())
}

pub fn stubs(three: bool) -> Vec<Stub> {
    let mut stubs = vec![
        Stub::new("K1.ledger")
            .writes("person.id")
            .writes("person.alive")
            .writes("life_course.person_id")
            .writes("life_course.exit_kind")
            .writes("life_course.exit_tick")
            .writes("life_course.death_cause")
            .writes("k1.born")
            .writes("k1.mode")
            .purposes_of::<Ip>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(ledger_step),
        Stub::new("K2.wealth")
            .writes("person.wealth")
            .writes("k2.seen")
            .writes("k2.founders")
            .default_of("person.wealth", WEALTH_DEFAULT)
            .reads("person.id", "K1.ledger", Lag::SameInstant)
            .reads("k1.born", "K1.ledger", Lag::SameInstant)
            .at(Pulse::Day, Phase::Polity)
            .stepping(wealth_step),
    ];
    if three {
        stubs.push(
            Stub::new("K3.rank")
                .writes("person.rank")
                .at(Pulse::Day, Phase::Agenda)
                .stepping(rank_step),
        );
    }
    stubs
}

pub fn engine(seed: u64, founders: usize, three: bool, mode: i64) -> Engine {
    let stubs = stubs(three);
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut ids: Vec<i64> = (0..founders)
        .map(|k| (h64_words(TAG ^ seed, &[k as u64]) >> 4).cast_signed())
        .collect();
    ids.sort_unstable_by_key(|v| v.cast_unsigned());
    let mut t = WorldTables::new();
    t.insert("person.id", ids.clone());
    t.insert("person.alive", vec![1; founders]);
    t.insert("person.wealth", vec![WEALTH_DEFAULT; founders]);
    if three {
        t.insert("person.rank", vec![0; founders]);
    }
    t.insert("life_course.person_id", ids);
    t.insert("life_course.exit_kind", vec![0; founders]);
    t.insert("life_course.exit_tick", vec![i64::MAX; founders]);
    t.insert("life_course.death_cause", vec![0; founders]);
    t.insert("k1.born", vec![0]);
    t.insert("k1.mode", vec![mode]);
    t.insert("k2.seen", vec![0]);
    t.insert("k2.founders", vec![i64::try_from(founders).unwrap_or(0)]);
    Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    )
}

/// The row insert: on worlds of 2, 9 and 40 founders, two and three owners, eight seeds, the
/// ledger stays sorted and aligned through every `Bookkeeping` (the core's invariants), the next
/// phase of the instant reads every new row, and other owners' columns get their default or 0.
#[test]
fn rows_enter_by_insert_sorted_aligned_and_visible_the_same_instant() {
    let mut inserted = 0;
    for founders in [2, 9, 40] {
        for three in [false, true] {
            for seed in SEEDS {
                let mut e = engine(seed, founders, three, 0);
                e.advance_until(SimTime::from_minutes(30 * DAY - 1), |_, _| {})
                    .unwrap_or_else(|err| panic!("{founders} {three} {seed}: {err}"));
                let t = e.tables();
                let ids: Vec<u64> = t
                    .get("person.id")
                    .unwrap()
                    .iter()
                    .map(|v| v.cast_unsigned())
                    .collect();
                assert!(ids.windows(2).all(|w| w[0] < w[1]), "sorted");
                let born = t.get("k1.born").unwrap()[0];
                assert_eq!(ids.len(), founders + usize::try_from(born).unwrap());
                assert_eq!(
                    t.get("k2.seen").unwrap()[0],
                    i64::try_from(ids.len()).unwrap()
                );
                for column in ["person.alive", "person.wealth"] {
                    assert_eq!(t.get(column).unwrap().len(), ids.len(), "{column} aligned");
                }
                inserted += born;
            }
        }
    }
    assert!(inserted > 0, "the property inserted no row");
    println!(
        "row insert: {inserted} rows inserted over 48 worlds, sorted, aligned and read the same instant"
    );
}

/// A duplicate id is `SimError::Ledger`; so is a row that does not name each of the inserter's
/// `person.*` columns once.
#[test]
fn a_duplicate_id_and_a_short_row_are_refused() {
    let err = engine(1, 9, false, 1)
        .advance_until(SimTime::from_minutes(5 * DAY - 1), |_, _| {})
        .expect_err("a duplicate id must be refused");
    let text = err.to_string();
    assert!(
        text.starts_with("K1.ledger: ledger: inserted person ")
            && text.ends_with(", whose id the ledger already holds"),
        "{text}"
    );
    let err = engine(1, 9, false, 2)
        .advance_until(SimTime::from_minutes(5 * DAY - 1), |_, _| {})
        .expect_err("a short row must be refused");
    assert!(
        err.to_string()
            .starts_with("K1.ledger: ledger: inserted a row of [\"person.id\"], not each of its own person.* columns"),
        "{err}"
    );
}
