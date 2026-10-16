//! Keyframes (SPR-007 T-02; V-DET-05): Arrow IPC with zstd round-trips the whole plane-A state of
//! a life world (pending timers, queued inputs, epochs, held events); a loaded keyframe advances
//! ten years exactly like the unbroken run; the permanent 10-year cadence and the yearly LRU;
//! every way a keyframe can be wrong is refused, a lost pending timer included.
//!
//! REF: `docs/09` V-DET-05; `05.2` §6; SPR-007 T-02, ruling 2; Architect rulings 2026-10-16.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

mod life;
mod support;

use support::Stub;

/// The keyframe envelope's length: 8 bytes of magic, 16 of digest.
const ENVELOPE_LEN: usize = 24;

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, BinaryArray, RecordBatch, StringArray};
use arrow_ipc::CompressionType;
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::{FileWriter, IpcWriteOptions};
use life::{CALENDAR, DAY, entity, life_engine, life_engine_with, life_registered};
use rh_core::crash::Provenance;
use rh_core::engine::{Engine, MaterialInput};
use rh_core::time::SimTime;
use rh_snapshot::guard::PlaneAState;
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{
    History, KeyframeError, load_engine, read_keyframe, seal_keyframe, unseal_keyframe,
    write_keyframe,
};
use rh_snapshot::state::{decode_engine_tables, engine_tables};
use rh_snapshot::state_hash;
use rh_snapshot::store::{Admission, KeyframeStore};

const SEED: u64 = 13;
const YEAR: i64 = 365 * DAY;

fn history() -> History {
    History {
        provenance: Provenance {
            build: "test-build".to_owned(),
            registry_version: "0.1.0".to_owned(),
            manifest_hash: "life".to_owned(),
        },
        world_seed: SEED,
        branch_salt: 0,
    }
}

/// The journal position after the inputs `e` has applied (these tests have no journal file, so
/// the head is the genesis value).
fn anchor(e: &Engine) -> JournalPosition {
    position(u32::try_from(e.snapshot().applied_inputs).unwrap())
}

fn position(inputs: u32) -> JournalPosition {
    JournalPosition {
        timeline_id: 0,
        inputs,
        head: chain_genesis(0),
    }
}

/// The life world with three inputs, the later two still queued at day 50.
fn life() -> Engine {
    let mut e = life_engine(SEED, 48, 3, false, 1);
    e.set_provenance(history().provenance);
    for (day, step) in [(45, 5), (400, 7), (600, 3)] {
        let scope = (0..48).step_by(step).map(entity).collect();
        e.submit_input(
            SimTime::from_minutes(day * DAY + 3),
            MaterialInput { scope },
        )
        .unwrap();
    }
    e
}

fn run_to(mut e: Engine, end: i64) -> Engine {
    e.advance_until(SimTime::from_minutes(end), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    e
}

/// `state_hash` of the engine at every month boundary up to `end`.
fn monthly(e: &mut Engine, end: i64) -> Vec<(i64, u64)> {
    let mut out = Vec::new();
    while let Some(t) = e.next_instant().filter(|t| t.minutes() <= end) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        if CALENDAR.is_boundary(rh_core::Pulse::Month, t) {
            out.push((t.minutes(), state_hash(&e.snapshot())));
        }
    }
    out
}

fn load(bytes: &[u8]) -> Result<Engine, KeyframeError> {
    load_engine(bytes, CALENDAR, life_registered(), &history()).map(|(e, _)| e)
}

#[test]
fn a_keyframe_round_trips_the_whole_plane_a_state() {
    let engine = run_to(life(), 50 * DAY + 1);
    let state = engine.snapshot();
    assert!(!state.agenda.entries.is_empty(), "pending timers");
    assert!(!state.events.events.is_empty(), "held events");
    assert!(!state.inputs.is_empty() && !state.epochs.is_empty());
    let (bytes, header) = write_keyframe(&engine, anchor(&engine)).unwrap();
    assert_eq!(header.state_hash, state_hash(&state));
    assert_eq!(&bytes[..8], b"RHKF\0v1\n", "the envelope's magic");
    assert_eq!(
        &bytes[ENVELOPE_LEN..ENVELOPE_LEN + 6],
        b"ARROW1",
        "an Arrow IPC file"
    );
    let (read, read_header) = read_keyframe(&bytes, &life_registered()).unwrap();
    assert_eq!(read, state);
    assert_eq!(read_header, header);
    let loaded = load(&bytes).unwrap();
    assert_eq!(loaded.snapshot(), state);
    // The decoder inverts the encoder table by table.
    let tables = engine_tables(&state)
        .into_iter()
        .map(|(n, rows)| (n.to_owned(), rows))
        .collect();
    assert_eq!(
        decode_engine_tables(&tables, &life_registered(), state.tables.clone()).unwrap(),
        state
    );
}

/// V-DET-05: write a keyframe, load it, advance ten years; equal to the continuous run, month by
/// month. Two cut points: day 50 (timers pending, events held, inputs queued) and year 1.
#[test]
fn v_det_05_a_loaded_keyframe_advances_ten_years_like_the_continuous_run() {
    for cut in [50 * DAY + 1, YEAR - 1] {
        let end = cut + 10 * YEAR;
        let mut continuous = run_to(life(), cut);
        let (bytes, _) = write_keyframe(&continuous, anchor(&continuous)).unwrap();
        let expected = monthly(&mut continuous, end);
        let mut loaded = load(&bytes).unwrap();
        let got = monthly(&mut loaded, end);
        assert_eq!(got.len(), 120, "ten years of months");
        assert_eq!(got, expected, "cut at minute {cut}");
        assert_eq!(loaded.snapshot(), continuous.snapshot());
    }
    println!("V-DET-05: two keyframes loaded and advanced 10 years, 120 monthly hashes equal");
}

/// The permanent 10-year cadence and the yearly LRU (V-DET-05's second half): years 0, 10, 20
/// are kept for good; the cache keeps the most recently used years; a seek to any year replays
/// from the nearest held keyframe and reaches the continuous run's hash.
#[test]
fn v_det_05_the_permanent_cadence_and_the_yearly_lru() {
    let mut store = KeyframeStore::new(10, 3);
    assert_eq!(KeyframeStore::default(), KeyframeStore::new(10, 8));
    let mut evicted = Vec::new();
    let mut frames = std::collections::BTreeMap::new();
    let mut e = life();
    let mut year_hashes = Vec::new();
    for year in 0..=24_i64 {
        e = run_to(e, year * YEAR - 1);
        year_hashes.push(state_hash(&e.snapshot()));
        match store.admit(year) {
            Admission::Permanent => assert_eq!(year % 10, 0),
            Admission::Cached { evicted: gone } => evicted.extend(gone),
        }
        let (bytes, _) = write_keyframe(&e, anchor(&e)).unwrap();
        frames.insert(year, bytes);
        for gone in &evicted {
            frames.remove(gone);
        }
    }
    assert_eq!(store.permanent().collect::<Vec<_>>(), [0, 10, 20]);
    assert_eq!(store.cached(), [22, 23, 24]);
    assert_eq!(evicted.len(), 25 - 3 - 3);
    assert!(
        evicted
            .iter()
            .all(|y| !store.holds(*y) && !frames.contains_key(y))
    );
    // Loading year 22 makes it the most recent; year 25's admission then evicts 23.
    assert!(store.touch(22));
    assert_eq!(store.admit(25), Admission::Cached { evicted: Some(23) });
    assert_eq!(store.nearest_at_or_before(19), Some(10));
    assert_eq!(store.nearest_at_or_before(23), Some(22));
    assert_eq!(store.nearest_at_or_before(-1), None);
    // Seek to year 17: replay from year 10's keyframe reaches the continuous hash.
    let from = store.nearest_at_or_before(17).unwrap();
    let seeked = run_to(load(&frames[&from]).unwrap(), 17 * YEAR - 1);
    assert_eq!(state_hash(&seeked.snapshot()), year_hashes[17]);
}

/// Rewrites a keyframe with `edit` applied to its batches' (table, row) pairs, zstd-compressed
/// as `write_keyframe` writes (the loader refuses any other IPC layout, DEF-3).
fn rewrite(bytes: &[u8], edit: impl Fn(&mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
    let zstd = IpcWriteOptions::default()
        .try_with_compression(Some(CompressionType::ZSTD))
        .unwrap();
    rewrite_with(bytes, zstd, edit)
}

/// [`rewrite`] with explicit Arrow write options.
fn rewrite_with(
    bytes: &[u8],
    options: IpcWriteOptions,
    edit: impl Fn(&mut Vec<(String, Vec<u8>)>),
) -> Vec<u8> {
    let reader = FileReader::try_new(Cursor::new(unseal_keyframe(bytes).unwrap()), None).unwrap();
    let schema = reader.schema();
    let mut pairs = Vec::new();
    for batch in reader {
        let batch = batch.unwrap();
        let names = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let rows = batch
            .column(1)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();
        for k in 0..batch.num_rows() {
            pairs.push((names.value(k).to_owned(), rows.value(k).to_vec()));
        }
    }
    edit(&mut pairs);
    let mut w = FileWriter::try_new_with_options(Vec::new(), &schema, options).unwrap();
    let names: ArrayRef = Arc::new(StringArray::from_iter_values(
        pairs.iter().map(|p| p.0.as_str()),
    ));
    let rows: ArrayRef = Arc::new(BinaryArray::from_iter_values(
        pairs.iter().map(|p| p.1.as_slice()),
    ));
    w.write(&RecordBatch::try_new(schema.clone(), vec![names, rows]).unwrap())
        .unwrap();
    w.finish().unwrap();
    seal_keyframe(&w.into_inner().unwrap())
}

#[test]
fn every_wrong_keyframe_is_refused() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, anchor(&engine)).unwrap();
    // The faithful rewrite still loads: the edits below are what the checks catch.
    assert!(load(&rewrite(&bytes, |_| {})).is_ok());
    // The same rows, uncompressed: not the keyframe's IPC layout, refused before Arrow decodes.
    let plain = rewrite_with(&bytes, IpcWriteOptions::default(), |_| {});
    assert!(matches!(load(&plain), Err(KeyframeError::Ipc(_))));
    // A lost pending timer (and the header's count lowered to match) no longer hashes equal.
    let lost = rewrite(&bytes, |pairs| {
        let k = pairs
            .iter()
            .position(|p| p.0 == "rh-core/agenda.timers")
            .unwrap();
        pairs.remove(k);
        for p in pairs.iter_mut().filter(|p| p.0 == "rehoboam:keyframe") {
            let line = String::from_utf8(p.1.clone()).unwrap();
            if let Some(n) = line.strip_prefix("table\trh-core/agenda.timers\t") {
                let n: usize = n.parse().unwrap();
                p.1 = format!("table\trh-core/agenda.timers\t{}", n - 1).into_bytes();
            }
        }
    });
    assert!(matches!(
        load(&lost),
        Err(KeyframeError::HashMismatch { .. })
    ));
    // An altered world row.
    let altered = rewrite(&bytes, |pairs| {
        let p = pairs.iter_mut().find(|p| p.0 == "vital.alive").unwrap();
        p.1[0] ^= 1;
    });
    assert!(matches!(
        load(&altered),
        Err(KeyframeError::HashMismatch { .. })
    ));
    // A row count that disagrees with the header.
    let short = rewrite(&bytes, |pairs| {
        let k = pairs.iter().position(|p| p.0 == "vital.alive").unwrap();
        pairs.remove(k);
    });
    assert!(matches!(load(&short), Err(KeyframeError::Format(_))));
    // Corrupt bytes.
    let mut corrupt = bytes.clone();
    let mid = corrupt.len() / 2;
    corrupt[mid] ^= 0xFF;
    assert!(matches!(load(&corrupt), Err(KeyframeError::Format(_))));
    assert!(matches!(
        load(&bytes[..bytes.len() / 2]),
        Err(KeyframeError::Format(_))
    ));
    assert!(matches!(
        load(&bytes[ENVELOPE_LEN..]),
        Err(KeyframeError::Format(_))
    ));
    // Every single flipped byte, envelope and body alike, is refused without a panic.
    for k in (0..bytes.len()).step_by(bytes.len() / 64 + 1) {
        let mut one = bytes.clone();
        one[k] ^= 0x01;
        assert!(load(&one).is_err(), "byte {k} flipped");
    }
    // Another history.
    let mut other = history();
    other.world_seed += 1;
    assert!(matches!(
        load_engine(&bytes, CALENDAR, life_registered(), &other),
        Err(KeyframeError::History("world_seed"))
    ));
    // Another timeline's journal position.
    let mut wrong = position(1);
    wrong.timeline_id = 4;
    assert!(matches!(
        write_keyframe(&engine, wrong),
        Err(KeyframeError::Anchor(_))
    ));
}

/// Rewrites one header line `key<TAB>value` of a keyframe.
fn with_header(bytes: &[u8], key: &'static str, value: &'static str) -> Vec<u8> {
    rewrite(bytes, |pairs| {
        let prefix = format!("{key}\t");
        let line = pairs
            .iter_mut()
            .find(|p| p.0 == "rehoboam:keyframe" && p.1.starts_with(prefix.as_bytes()))
            .unwrap();
        line.1 = format!("{prefix}{value}").into_bytes();
    })
}

/// VER-SPR-007-TEST DEF-4: the keyframe's history tuple is the engine's own. The writer takes no
/// history from its caller, so an engine of seed 99 and salt 5 cannot be stamped as seed 13 and
/// salt 0 (or the reverse), and the loader refuses a keyframe whose tuple differs from the one
/// the caller expects.
#[test]
fn def_4_the_history_tuple_comes_from_the_engine() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, header) = write_keyframe(&engine, anchor(&engine)).unwrap();
    assert_eq!(header.history, history());
    let mut other = life_engine(99, 48, 3, false, 1);
    other.set_provenance(history().provenance);
    let other = run_to(other, 50 * DAY + 1);
    let (other_bytes, other_header) = write_keyframe(&other, anchor(&other)).unwrap();
    assert_eq!(other_header.history.world_seed, 99);
    // The seed-99 keyframe is refused under the seed-13 history, and loads under its own.
    assert!(matches!(
        load(&other_bytes),
        Err(KeyframeError::History("world_seed"))
    ));
    let mut expected = history();
    expected.world_seed = 99;
    let (mut loaded, _) =
        load_engine(&other_bytes, CALENDAR, life_registered(), &expected).unwrap();
    // And the loaded engine draws as the original does: equal a year on, instant by instant.
    let mut original = other;
    let end = 50 * DAY + YEAR;
    assert_eq!(monthly(&mut loaded, end), monthly(&mut original, end));
    assert_eq!(
        state_hash(&loaded.snapshot()),
        state_hash(&original.snapshot())
    );
    // The seed-13 keyframe still loads under the seed-13 history.
    assert!(load(&bytes).is_ok());
}

/// VER-SPR-007-TEST GAP-8: each of the five history fields is checked on load.
#[test]
fn gap_8_every_history_field_is_checked_on_load() {
    type Edit = fn(&mut History);
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let edits: [(&str, Edit); 5] = [
        ("build", |h| h.provenance.build.push('x')),
        ("registry_version", |h| {
            h.provenance.registry_version.push('x');
        }),
        ("manifest_hash", |h| h.provenance.manifest_hash.push('x')),
        ("world_seed", |h| h.world_seed ^= 1),
        ("branch_salt", |h| h.branch_salt ^= 1),
    ];
    for (field, edit) in edits {
        let mut expected = history();
        edit(&mut expected);
        match load_engine(&bytes, CALENDAR, life_registered(), &expected) {
            Err(KeyframeError::History(f)) => assert_eq!(f, field),
            Err(e) => panic!("{field}: refused for another reason: {e}"),
            Ok(_) => panic!("{field}: accepted"),
        }
    }
    assert!(load(&bytes).is_ok());
}

/// VER-SPR-007-TEST GAP-7: the journal anchor must count exactly the inputs the engine applied
/// (`rh-core/engine.meta`, inside `state_hash`), on write and on load, so an edited count or an
/// anchor past what was applied is refused without the journal.
#[test]
fn gap_7_the_journal_anchor_counts_exactly_the_applied_inputs() {
    let engine = run_to(life(), 50 * DAY + 1);
    assert_eq!(engine.snapshot().applied_inputs, 1);
    // The writer refuses an anchor of 99 (or 0) over one applied input.
    for inputs in [0, 2, 99] {
        assert!(matches!(
            write_keyframe(&engine, position(inputs)),
            Err(KeyframeError::Anchor(_))
        ));
    }
    let (bytes, _) = write_keyframe(&engine, position(1)).unwrap();
    // The loader refuses a header edited to another count.
    for forged in ["999", "0", "2"] {
        assert!(
            matches!(
                load(&with_header(&bytes, "journal_inputs", forged)),
                Err(KeyframeError::Anchor(_))
            ),
            "journal_inputs {forged}"
        );
    }
    // The faithful header still loads.
    assert!(load(&with_header(&bytes, "journal_inputs", "1")).is_ok());
}

#[test]
fn a_poisoned_engine_has_no_keyframe() {
    use rh_core::system::SimError;
    let mut stubs = vec![
        support::Stub::new("X1.bad")
            .writes("bad.n")
            .at(rh_core::Pulse::Day, rh_core::Phase::Vital)
            .stepping(|_, _, _, _| {
                Err(SimError::System {
                    system: "X1.bad",
                    message: "always".to_owned(),
                })
            }),
    ];
    let specs = support::agreeing_specs(&stubs);
    let registered =
        rh_core::register::register(stubs.drain(..).map(support::Stub::boxed).collect(), &specs)
            .unwrap();
    let mut e = Engine::new(
        CALENDAR,
        1,
        rh_ids::BranchSalt::from_raw(0),
        registered,
        rh_core::world::WorldTables::new(),
        SimTime::EPOCH,
    );
    assert!(
        e.advance_until(SimTime::from_minutes(DAY), |_, _| {})
            .is_err()
    );
    assert!(matches!(
        write_keyframe(&e, anchor(&e)),
        Err(KeyframeError::Poisoned(_))
    ));
}

/// VER-SPR-007-TEST GAP-4: the guard's hash of an engine covers its engine-held state: two engines
/// with equal tables and different agendas hash differently.
#[test]
fn the_guard_sees_engine_held_state() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let mut state = read_keyframe(&bytes, &life_registered()).unwrap().0;
    let tables_only = rh_snapshot::tables_hash(&state.tables);
    state.agenda.entries.pop();
    state
        .agenda
        .tombstones
        .retain(|t| state.agenda.entries.iter().any(|(k, _)| k.id() == *t));
    let fewer = Engine::restore(
        CALENDAR,
        SEED,
        rh_ids::BranchSalt::from_raw(0),
        life_registered(),
        history().provenance,
        state,
    )
    .unwrap();
    assert_eq!(rh_snapshot::tables_hash(fewer.tables()), tables_only);
    assert_ne!(fewer.plane_a_hash(), engine.plane_a_hash());
}

/// The note system's step: one `NOTE` event at the month boundaries of days 180..211, carrying
/// `payload`. It writes nothing, so it changes no table.
fn note_step(
    payload: i64,
    c: &mut rh_core::system::StepCtx<'_>,
) -> Result<(), rh_core::system::SimError> {
    if (180 * DAY..211 * DAY).contains(&c.now().minutes()) {
        c.emit(rh_events::EventDraft::new("NOTE").with_payload(vec![payload]))?;
    }
    Ok(())
}

fn note_one(
    _: &Stub,
    _: rh_core::system::PulseBinding,
    _: &mut rh_core::world::SystemView<'_>,
    c: &mut rh_core::system::StepCtx<'_>,
) -> Result<(), rh_core::system::SimError> {
    note_step(1, c)
}

fn note_two(
    _: &Stub,
    _: rh_core::system::PulseBinding,
    _: &mut rh_core::world::SystemView<'_>,
    c: &mut rh_core::system::StepCtx<'_>,
) -> Result<(), rh_core::system::SimError> {
    note_step(2, c)
}

/// The life world run to the end of year 1 with the note system stepping `step`, every event
/// drained, so no held event differs.
fn noted_year(step: support::StepFn) -> Engine {
    let note = Stub::new("X6.note")
        .emits("NOTE")
        .at(rh_core::Pulse::Month, rh_core::Phase::Agenda)
        .stepping(step);
    let mut e = life_engine_with(vec![note], SEED, 48, 3, false, 1);
    e.advance_until(SimTime::from_minutes(YEAR), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    let notes: Vec<_> = e
        .drain_events()
        .into_iter()
        .filter(|ev| ev.kind == "NOTE")
        .collect();
    assert!(!notes.is_empty(), "the note was emitted mid-year");
    e
}

/// Architect clarification on T-02 (2026-10-16): `rh-core/events.meta` carries the running
/// event-log hash (SPR-006 G4), so a transient divergence that emitted different events and then
/// converged still changes `state_hash`, drained or not, and a keyframe keeps that difference.
#[test]
fn a_converged_run_that_emitted_a_different_event_hashes_differently() {
    let one = noted_year(note_one);
    let two = noted_year(note_two);
    let (s1, s2) = (one.snapshot(), two.snapshot());
    assert_eq!(s1.tables, s2.tables, "the tables converged");
    assert!(
        s1.events.events.is_empty() && s2.events.events.is_empty(),
        "nothing held"
    );
    assert_eq!(
        s1.events.next_seq, s2.events.next_seq,
        "the same number of events"
    );
    assert_eq!(
        rh_snapshot::tables_hash(&s1.tables),
        rh_snapshot::tables_hash(&s2.tables)
    );
    assert_ne!(s1.events.running, s2.events.running);
    assert_ne!(
        state_hash(&s1),
        state_hash(&s2),
        "the running event-log hash is in state_hash"
    );
    let (bytes, _) = write_keyframe(&one, anchor(&one)).unwrap();
    let (read, _) = read_keyframe(&bytes, one.registered()).unwrap();
    assert_eq!(read.events.running, s1.events.running);
    assert_eq!(state_hash(&read), state_hash(&s1));
}

/// When the host drains events, `state_hash` drains nothing (VER-SPR-007-TEST round 2 GAP-6,
/// Architect ruling): the undrained events are carried output, outside the hash.
#[derive(Clone, Copy, Debug)]
enum Drain {
    Never,
    EveryInstant,
    Monthly,
    /// At pseudo-random instants (a fixed LCG, so the test is deterministic).
    Random(u64),
}

/// The life world (seed 13) run two years one instant at a time, draining per `drain`: the
/// `state_hash` at every month boundary, and the largest number of events held at a rung.
fn drained_ladder(drain: Drain) -> (Vec<(i64, u64)>, usize) {
    let mut e = life_engine(13, 48, 3, false, 1);
    let mut lcg = match drain {
        Drain::Random(seed) => seed,
        _ => 0,
    };
    let (mut ladder, mut most_held) = (Vec::new(), 0);
    while let Some(t) = e.next_instant().filter(|t| t.minutes() <= 2 * YEAR) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        let month = CALENDAR.is_boundary(rh_core::Pulse::Month, t);
        if month {
            let state = e.snapshot();
            most_held = most_held.max(state.events.events.len());
            ladder.push((t.minutes(), state_hash(&state)));
        }
        let now = match drain {
            Drain::Never => false,
            Drain::EveryInstant => true,
            Drain::Monthly => month,
            Drain::Random(_) => {
                lcg = lcg
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                lcg >> 61 == 0
            }
        };
        if now {
            e.drain_events();
        }
    }
    (ladder, most_held)
}

#[test]
fn state_hash_does_not_depend_on_when_the_host_drains() {
    let (never, held) = drained_ladder(Drain::Never);
    assert_eq!(
        never.len(),
        25,
        "the month rungs of two years, both ends included"
    );
    assert!(held > 0, "without draining, events are held at the rungs");
    for drain in [
        Drain::EveryInstant,
        Drain::Monthly,
        Drain::Random(1),
        Drain::Random(0x5EED),
    ] {
        let (ladder, _) = drained_ladder(drain);
        assert_eq!(ladder, never, "{drain:?}");
    }
}

/// A keyframe written with events held keeps them on load, and the loaded engine hashes equal.
#[test]
fn a_keyframe_keeps_the_held_events_outside_the_hash() {
    let engine = run_to(life(), 50 * DAY + 1);
    let state = engine.snapshot();
    assert!(!state.events.events.is_empty(), "events are held");
    let (bytes, header) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let mut loaded = load(&bytes).unwrap();
    assert_eq!(loaded.snapshot().events.events, state.events.events);
    assert_eq!(state_hash(&loaded.snapshot()), header.state_hash);
    // Draining the loaded engine changes no hash: the held events were never in it.
    let drained = loaded.drain_events();
    assert_eq!(drained, state.events.events);
    assert_eq!(state_hash(&loaded.snapshot()), header.state_hash);
    // The carried table is in the keyframe: altering a held row is still refused.
    let altered = rewrite(&bytes, |pairs| {
        let p = pairs
            .iter_mut()
            .find(|p| p.0 == rh_snapshot::state::EVENTS_HELD)
            .unwrap();
        let last = p.1.len() - 1;
        p.1[last] ^= 1;
    });
    assert!(load(&altered).is_err());
}

/// What a sweep of forged keyframes did.
#[derive(Debug, Default, PartialEq, Eq)]
struct Sweep {
    cases: usize,
    /// Loaded, and hashing equal to the original (a flip Arrow ignores, such as padding).
    accepted_equal: usize,
    refused: usize,
    /// Loaded with another state: must never happen.
    accepted_other: usize,
    /// The loader panicked (caught here or mapped to `KeyframeError::Panicked`): must never happen.
    panicked: usize,
}

/// DEF-3 (VER-SPR-007-TEST round 2): flips every bit of every `stride`-th byte of the Arrow body
/// of a day-50 keyframe, re-seals the envelope so the digest matches, and loads each forgery.
/// Ports the Tester's `ver/forge.py` sweep in-process: an abort would end the test process.
fn forge_sweep(stride: usize) -> Sweep {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, header) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let body = unseal_keyframe(&bytes).unwrap().to_vec();
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sweep = Sweep::default();
    for k in (0..body.len()).step_by(stride) {
        for bit in 0..8 {
            let mut forged = body.clone();
            forged[k] ^= 1 << bit;
            let sealed = seal_keyframe(&forged);
            sweep.cases += 1;
            match std::panic::catch_unwind(|| load(&sealed)) {
                Ok(Ok(e)) if state_hash(&e.snapshot()) == header.state_hash => {
                    sweep.accepted_equal += 1;
                }
                Ok(Ok(_)) => sweep.accepted_other += 1,
                Ok(Err(KeyframeError::Panicked(_))) | Err(_) => sweep.panicked += 1,
                Ok(Err(_)) => sweep.refused += 1,
            }
        }
    }
    std::panic::set_hook(quiet);
    sweep
}

fn assert_sweep(sweep: &Sweep, min_cases: usize) {
    assert_eq!(sweep.panicked, 0, "{sweep:?}");
    assert_eq!(sweep.accepted_other, 0, "{sweep:?}");
    assert!(sweep.cases >= min_cases, "{sweep:?}");
    assert!(sweep.refused > sweep.cases / 2, "{sweep:?}");
}

/// PR tier: every bit of every 29th byte of the Arrow body.
#[test]
fn def_3_no_forged_arrow_body_panics_or_aborts_the_loader() {
    let sweep = forge_sweep(29);
    println!("DEF-3: {sweep:?}");
    assert_sweep(&sweep, 3_000);
}

/// Nightly tier: every bit of every byte (about 94,000 loads, a few minutes in a debug build).
#[test]
#[ignore = "nightly tier: the full DEF-3 sweep"]
fn def_3_full_sweep_every_bit_of_the_arrow_body() {
    let sweep = forge_sweep(1);
    println!("DEF-3 full: {sweep:?}");
    assert_sweep(&sweep, 90_000);
}

/// The engine tables decode tombstones in id order even when the tombstoned timers' key order
/// differs (found by the DEF-3 read-back on write: the seed-99 life world at day 50 has
/// tombstones 5 and 7 with timer 7 due first, and a decoder that kept row order produced a state
/// that hashed differently and that `Engine::restore` refused).
#[test]
fn tombstones_decode_in_id_order_whatever_their_key_order() {
    let mut e = life_engine(99, 48, 3, false, 1);
    e.set_provenance(history().provenance);
    let e = run_to(e, 50 * DAY + 1);
    let s = e.snapshot();
    let in_key_order: Vec<u64> = s
        .agenda
        .entries
        .iter()
        .map(|(k, _)| k.seq)
        .filter(|seq| {
            s.agenda
                .tombstones
                .contains(&rh_core::agenda::TimerId::from_raw(*seq))
        })
        .collect();
    assert!(
        in_key_order.windows(2).any(|w| w[0] > w[1]),
        "the case needs tombstones out of key order: {in_key_order:?}"
    );
    let tables = engine_tables(&s)
        .into_iter()
        .map(|(n, r)| (n.to_owned(), r))
        .collect();
    let decoded = decode_engine_tables(&tables, &life_registered(), s.tables.clone()).unwrap();
    assert_eq!(decoded, s);
    assert_eq!(state_hash(&decoded), state_hash(&s));
    assert!(write_keyframe(&e, anchor(&e)).is_ok());
}
