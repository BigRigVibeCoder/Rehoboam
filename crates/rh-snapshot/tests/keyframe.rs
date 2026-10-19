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
use rh_snapshot::journal::{JournalPosition, Replay, chain_genesis};
use rh_snapshot::keyframe::{
    History, KeyframeError, load_engine, read_keyframe, seal_keyframe, unseal_keyframe,
    write_keyframe,
};
use rh_snapshot::resume::resume;
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
    position(u32::try_from(e.journal_position()).unwrap())
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
    // A keyframe of format v1 (SPR-007 to SPR-013) is refused by its format name (SPR-020 T-05;
    // R12 (d)): its events.held rows are SPR-006's events.
    let v1 = with_header(&bytes, "format", rh_snapshot::keyframe::KEYFRAME_FORMAT_V1);
    let err = load(&v1).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains("rehoboam/rh-snapshot/keyframe/v1 needs the v1 decoder"),
        "a v1 keyframe: {err}"
    );
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

/// VER-SPR-007-TEST GAP-7, as GAP-10 redefined it: the journal anchor must count exactly the
/// inputs the engine accepted (`rh-core/engine.meta`, inside `state_hash`), on write and on load,
/// so an edited count, the applied count, or an anchor past what was accepted is refused without
/// the journal.
#[test]
fn gap_7_the_journal_anchor_counts_exactly_the_accepted_inputs() {
    let engine = run_to(life(), 50 * DAY + 1);
    // Three inputs accepted at the start; one applied by day 50, two still queued.
    assert_eq!((engine.journal_position(), engine.applied_inputs()), (3, 1));
    // The writer refuses the applied count, and any other count.
    for inputs in [0, 1, 2, 4, 99] {
        assert!(matches!(
            write_keyframe(&engine, position(inputs)),
            Err(KeyframeError::Anchor(_))
        ));
    }
    let (bytes, _) = write_keyframe(&engine, position(3)).unwrap();
    // The loader refuses a header edited to another count.
    for forged in ["999", "0", "1", "2", "4"] {
        assert!(
            matches!(
                load(&with_header(&bytes, "journal_inputs", forged)),
                Err(KeyframeError::Anchor(_))
            ),
            "journal_inputs {forged}"
        );
    }
    // The faithful header still loads.
    assert!(load(&with_header(&bytes, "journal_inputs", "3")).is_ok());
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

/// The byte offset of a flatbuffer struct inside `base`, from the reference the verified accessor
/// hands out (structs in a vector are read in place).
fn offset_in<T>(base: &[u8], item: &T) -> usize {
    std::ptr::from_ref(item) as usize - base.as_ptr() as usize
}

/// Writes `value` little-endian into `width` bytes at `at`.
fn patch(bytes: &mut [u8], at: usize, width: usize, value: i64) {
    bytes[at..at + width].copy_from_slice(&value.to_le_bytes()[..width]);
}

/// Every forgeable length or offset field of a keyframe's Arrow body, as (byte position, width,
/// true value): each footer block's `offset`, `metaDataLength` and `bodyLength`, and in the first
/// block's message every buffer's `offset` and `length`, every node's `length` and `null_count`,
/// and the batch `length`. Located through the verified accessors.
fn forgeable_fields(body: &[u8]) -> Vec<(usize, usize, i64)> {
    let n = body.len();
    let footer_len =
        usize::try_from(i32::from_le_bytes(body[n - 10..n - 6].try_into().unwrap())).unwrap();
    let footer_start = n - 10 - footer_len;
    let footer = arrow_ipc::root_as_footer(&body[footer_start..n - 10]).unwrap();
    let blocks = footer.recordBatches().unwrap();
    // (byte position in the body, width, true value) of every field to forge.
    let mut fields: Vec<(usize, usize, i64)> = Vec::new();
    for block in blocks {
        let at = offset_in(body, block);
        fields.push((at, 8, block.offset()));
        fields.push((at + 8, 4, i64::from(block.metaDataLength())));
        fields.push((at + 16, 8, block.bodyLength()));
    }
    let first = blocks.get(0);
    let meta_at = usize::try_from(first.offset()).unwrap() + 8;
    let fb_len = usize::try_from(i32::from_le_bytes(
        body[meta_at - 4..meta_at].try_into().unwrap(),
    ))
    .unwrap();
    let message = arrow_ipc::root_as_message(&body[meta_at..meta_at + fb_len]).unwrap();
    let buffers = message.header_as_record_batch().unwrap().buffers().unwrap();
    assert_eq!(buffers.len(), 6);
    for buffer in buffers {
        let at = offset_in(body, buffer);
        fields.push((at, 8, buffer.offset()));
        fields.push((at + 8, 8, buffer.length()));
    }
    let batch = message.header_as_record_batch().unwrap();
    let nodes = batch.nodes().unwrap();
    assert_eq!(nodes.len(), 2);
    for node in nodes {
        let at = offset_in(body, node);
        fields.push((at, 8, node.length()));
        fields.push((at + 8, 8, node.null_count()));
    }
    // The batch length is a table field, not a struct: find it as the one 8-byte slot holding the
    // length that, when changed, changes what the accessor reads.
    let rows = batch.length();
    let slot = (meta_at..meta_at + fb_len - 7)
        .step_by(8)
        .find(|&at| {
            let mut f = body.to_vec();
            if i64::from_le_bytes(f[at..at + 8].try_into().unwrap()) != rows {
                return false;
            }
            patch(&mut f, at, 8, rows + 1);
            arrow_ipc::root_as_message(&f[meta_at..meta_at + fb_len])
                .ok()
                .and_then(|m| m.header_as_record_batch())
                .is_some_and(|b| b.length() == rows + 1)
        })
        .expect("the batch length's slot");
    fields.push((slot, 8, rows));
    fields
}

/// DEF-3b (Architect review of b9c3ede): multi-byte forgeries a bit-flip sweep cannot reach. Every
/// footer block's `offset`, `metaDataLength` and `bodyLength`, and every buffer `offset` and
/// `length`, node `length` and `null_count`, and the batch `length` of the first block's message,
/// set to 0, -1, -8, the extremes, the true value +- 8 and
/// the file length; each forgery re-sealed and loaded. Refused or equal state, never a panic.
#[test]
fn def_3b_forged_block_and_buffer_fields_are_refused_without_a_panic() {
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, header) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let body = unseal_keyframe(&bytes).unwrap().to_vec();
    let fields = forgeable_fields(&body);
    let n = body.len();
    // The located fields really are the ones the accessors read.
    for &(at, width, value) in &fields {
        let mut raw = [0_u8; 8];
        raw[..width].copy_from_slice(&body[at..at + width]);
        let read = if width == 4 {
            i64::from(i32::from_le_bytes(raw[..4].try_into().unwrap()))
        } else {
            i64::from_le_bytes(raw)
        };
        assert_eq!(read, value, "field at {at}");
    }
    let file_len = i64::try_from(n).unwrap();
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sweep = Sweep::default();
    for &(at, width, value) in &fields {
        let (min, max) = if width == 4 {
            (i64::from(i32::MIN), i64::from(i32::MAX))
        } else {
            (i64::MIN, i64::MAX)
        };
        for forged in [
            0,
            -1,
            -8,
            -16,
            -256,
            min,
            max,
            value + 8,
            value - 8,
            file_len,
        ] {
            if forged == value {
                continue;
            }
            let mut f = body.clone();
            patch(&mut f, at, width, forged);
            let sealed = seal_keyframe(&f);
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
    println!("DEF-3b: {} fields, {sweep:?}", fields.len());
    assert_eq!(sweep.panicked, 0, "{sweep:?}");
    assert_eq!(sweep.accepted_other, 0, "{sweep:?}");
    assert!(sweep.cases > 300, "{sweep:?}");
}

/// DEF-5 (VER-SPR-007-TEST round 2, found independently of the read-back): the life world with
/// `rows` people and seed `seed`, written (and read back) and loaded at days 10, 50, 100, 150 and
/// 200; each load equals the live state. Larger worlds have tombstoned timers out of key order,
/// which a decoder keeping row order lost (fixed in 24d2262).
fn round_trips_at_every_cut(rows: usize, seed: u64) {
    let mut e = life_engine(seed, rows, 3, false, 1);
    e.set_provenance(history().provenance);
    let mut expected = history();
    expected.world_seed = seed;
    for day in [10, 50, 100, 150, 200] {
        e.advance_until(SimTime::from_minutes(day * DAY + 1), |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        let (bytes, _) = write_keyframe(&e, anchor(&e))
            .unwrap_or_else(|err| panic!("rows {rows}, seed {seed}, day {day}: {err}"));
        let (loaded, _) = load_engine(&bytes, CALENDAR, life_registered(), &expected)
            .unwrap_or_else(|err| panic!("rows {rows}, seed {seed}, day {day}: {err}"));
        assert_eq!(
            loaded.snapshot(),
            e.snapshot(),
            "rows {rows}, seed {seed}, day {day}"
        );
    }
}

/// Rows 48, 150, 257, 1,000 and 5,000, three seeds each (about 7 s in a debug build, so all in the
/// PR tier).
#[test]
fn def_5_keyframes_round_trip_across_world_sizes() {
    for rows in [48, 150, 257, 1_000, 5_000] {
        for seed in [13, 99, 0x5EED] {
            round_trips_at_every_cut(rows, seed);
        }
    }
}

/// A fresh scratch directory for one test's journal.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("keyframe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn journal_key(seed: u64) -> rh_snapshot::journal::HistoryKey {
    rh_snapshot::journal::HistoryKey {
        engine_build: history().provenance.build,
        registry_version: history().provenance.registry_version,
        manifest_hash: history().provenance.manifest_hash.into_bytes(),
        world_seed: seed,
    }
}

/// The host's order (05.1 section 6): check the input against the engine's cursor, journal it,
/// then submit it, so journal (`seq`) order is acceptance order.
fn accept(
    e: &mut Engine,
    j: &mut rh_snapshot::journal::Journal,
    day: i64,
    row: usize,
    payload: &str,
) {
    use rh_snapshot::journal::{InputKind, NewInput, Producer, Scope};
    let at = SimTime::from_minutes(day * DAY + 3);
    assert!(e.snapshot().cursor.is_some_and(|c| at >= c), "a past input");
    let journaled = j
        .append(NewInput {
            timeline_id: 0,
            tick: at,
            kind: InputKind::OracleMove,
            scope: Scope::of_entity(entity(row)).unwrap(),
            producer: Producer::Player,
            payload: payload.as_bytes().to_vec(),
        })
        .unwrap();
    let input = rh_snapshot::resume::material_input(&journaled).unwrap();
    e.submit_input(at, input).unwrap();
}

/// VER-SPR-007-TEST GAP-10: the journal position counts the inputs the engine *accepted*. Input
/// seq 0 applies at day 100 and seq 1 at day 10; at day 50 one is applied and both are accepted.
/// The anchor covers both rows, so a journal whose seq 1 is forged fails `check_anchor`; an anchor
/// at the applied count (one row) would have covered seq 0 only, the unapplied one.
#[test]
fn gap_10_the_anchor_covers_every_accepted_input_in_journal_order() {
    let dir = scratch("gap10");
    let (a_path, b_path) = (dir.join("a.sqlite"), dir.join("b.sqlite"));
    let mut e = life_engine(SEED, 48, 3, false, 1);
    e.set_provenance(history().provenance);
    let mut a = rh_snapshot::journal::Journal::create(&a_path, &journal_key(SEED)).unwrap();
    accept(&mut e, &mut a, 100, 1, "late");
    accept(&mut e, &mut a, 10, 2, "early");
    let e = run_to(e, 50 * DAY + 1);
    assert_eq!((e.journal_position(), e.applied_inputs()), (2, 1));
    let anchor = a.position(0).unwrap();
    assert_eq!(anchor.inputs, 2);
    let (bytes, header) = write_keyframe(&e, anchor).unwrap();
    assert_eq!(header.journal, anchor);
    // Journal B: the same seq 0, a forged seq 1 (another entity).
    let mut b = rh_snapshot::journal::Journal::create(&b_path, &journal_key(SEED)).unwrap();
    let mut scratch_engine = life_engine(SEED, 48, 3, false, 1);
    accept(&mut scratch_engine, &mut b, 100, 1, "late");
    accept(&mut scratch_engine, &mut b, 10, 4, "early");
    drop((a, b));
    let ra = Replay::open(&a_path, &journal_key(SEED), 0).unwrap();
    let rb = Replay::open(&b_path, &journal_key(SEED), 0).unwrap();
    ra.check_anchor(&header.journal).unwrap();
    assert!(matches!(
        rb.check_anchor(&header.journal),
        Err(rh_snapshot::journal::JournalError::Mismatched { .. })
    ));
    // The applied-count anchor would not have seen the forgery.
    assert_eq!(ra.chain_after(1), rb.chain_after(1));
    // And the keyframe resumes through the true journal, refused through the forged one.
    assert!(resume(&bytes, CALENDAR, life_registered(), &history(), &ra).is_ok());
    assert!(matches!(
        resume(&bytes, CALENDAR, life_registered(), &history(), &rb),
        Err(rh_snapshot::resume::ResumeError::Journal(_))
    ));
}

/// The month-boundary `state_hash` ladder of `e` from now to `end`, and its final state.
fn ladder_to(e: &mut Engine, end: i64) -> (Vec<(i64, u64)>, rh_core::state::EngineState) {
    let ladder = monthly(e, end);
    (ladder, e.snapshot())
}

/// Resuming from a keyframe through the journal, the first real host loop (SPR-008's bisect
/// reuses it). In a 150-person life world (inputs land while people live and have timers to
/// re-sample), inputs are submitted out of tick order; at the day-20 keyframe two are still
/// queued; two more are journaled after it. The resumed run equals the continuous run from the
/// last submission on, over more than two years, and ends in the same state. Control:
/// re-submitting from the applied count instead re-submits an applied input (refused as past)
/// and a queued one (applied twice), and diverges.
#[test]
fn gap_10_a_keyframe_resumes_through_the_journal_like_the_continuous_run() {
    let dir = scratch("resume");
    let path = dir.join("journal.sqlite");
    // The input queued at the keyframe and doubled by the control targets a person who still has a
    // pending timer on day 90, so the double application re-samples a real timer. Found with a
    // scout run: nothing before day 90 depends on that input.
    let target = {
        let mut scout = life_engine(SEED, 150, 3, false, 1);
        let mut sj =
            rh_snapshot::journal::Journal::create(&dir.join("scout.sqlite"), &journal_key(SEED))
                .unwrap();
        accept(&mut scout, &mut sj, 120, 3, "a");
        accept(&mut scout, &mut sj, 15, 5, "b");
        let scout = run_to(scout, 90 * DAY);
        let s = scout.snapshot();
        (0..150)
            .find(|&row| {
                s.agenda
                    .entries
                    .iter()
                    .any(|(k, _)| k.entity == entity(row) && k.time.minutes() > 90 * DAY + 3)
            })
            .expect("someone has a timer after day 90")
    };
    let mut e = life_engine(SEED, 150, 3, false, 1);
    e.set_provenance(history().provenance);
    let mut j = rh_snapshot::journal::Journal::create(&path, &journal_key(SEED)).unwrap();
    // Out of tick order: 120, 15, 90.
    accept(&mut e, &mut j, 120, 3, "a");
    accept(&mut e, &mut j, 15, 5, "b");
    accept(&mut e, &mut j, 90, target, "c");
    let mut e = run_to(e, 20 * DAY + 1);
    assert_eq!((e.journal_position(), e.applied_inputs()), (3, 1));
    let applied_at_keyframe = 1_u32;
    let (bytes, header) = write_keyframe(&e, j.position(0).unwrap()).unwrap();
    // The continuous run journals two more inputs after the keyframe.
    e = run_to(e, 25 * DAY + 1);
    accept(&mut e, &mut j, 60, 9, "d");
    e = run_to(e, 40 * DAY + 1);
    accept(&mut e, &mut j, 150, 11, "e");
    drop(j);
    let alive: i64 = e.tables().get("vital.alive").unwrap().iter().sum();
    assert!(alive > 0, "people live while the inputs apply");
    let end = 40 * DAY + 2 * YEAR + 30 * DAY;
    let (continuous, final_state) = ladder_to(&mut e, end);
    assert!(continuous.len() >= 24, "more than two years of months");
    let replay = Replay::open(&path, &journal_key(SEED), 0).unwrap();
    assert_eq!(replay.position().inputs, 5);
    let (mut resumed, h) = resume(&bytes, CALENDAR, life_registered(), &history(), &replay)
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(h, header);
    assert_eq!(resumed.journal_position(), 5);
    resumed = run_to(resumed, 40 * DAY + 1);
    let (ladder, state) = ladder_to(&mut resumed, end);
    assert_eq!(ladder, continuous);
    assert_eq!(state, final_state);
    // Control: the applied-count position (1) re-submits rows from seq 1 on.
    let (mut wrong, _) = load_engine(&bytes, CALENDAR, life_registered(), &history()).unwrap();
    let mut rows: Vec<_> = replay
        .inputs()
        .iter()
        .filter(|i| i.seq >= applied_at_keyframe)
        .collect();
    rows.sort_by_key(|i| i.seq);
    // Seq 1 (day 15) was applied before the keyframe: refused as past. Seq 2 (day 90) is still
    // queued in the keyframe: accepted a second time.
    let mut refused = 0;
    for row in rows {
        let input = rh_snapshot::resume::material_input(row).unwrap();
        if wrong.submit_input(row.tick, input).is_err() {
            refused += 1;
        }
    }
    assert_eq!(refused, 1, "the applied input is refused as past");
    assert_eq!(
        wrong.journal_position(),
        6,
        "the queued day-90 input accepted twice"
    );
    wrong = run_to(wrong, 40 * DAY + 1);
    let mut good = resume(&bytes, CALENDAR, life_registered(), &history(), &replay)
        .unwrap()
        .0;
    good = run_to(good, 40 * DAY + 1);
    // The world itself diverges: some month's world tables differ, not only the counters.
    let mut differ = 0;
    while let Some(t) = good.next_instant().filter(|t| t.minutes() <= end) {
        good.advance_until(t, |_, _| {}).unwrap();
        wrong.advance_until(t, |_, _| {}).unwrap();
        if CALENDAR.is_boundary(rh_core::Pulse::Month, t)
            && rh_snapshot::tables_hash(good.tables()) != rh_snapshot::tables_hash(wrong.tables())
        {
            differ += 1;
        }
    }
    println!("GAP-10 control: the world tables differ in {differ} months");
    assert!(differ > 0, "the double-applied input changes the world");
    assert_ne!(state_hash(&wrong.snapshot()), state_hash(&final_state));
}

/// GAP-11 (Tester round 2b): pins the guard's uncompressed-size bounds (`ipc_guard::check_buffer`).
/// For each buffer of the first batch that carries a compression prefix, the declared uncompressed
/// length is forged to its bound + 1, to `ZSTD_MAX_RATIO` times the compressed bytes + 1, to
/// 2^40, to `i64::MAX` and to -2, and every forgery must be refused as `KeyframeError::Ipc` before
/// Arrow decodes. If a bound regresses, Arrow allocates the declared size and the allocation
/// aborts the test process: nextest reports that as a failed test, so a regression cannot pass
/// silently.
#[test]
fn gap_11_every_forged_uncompressed_length_is_refused_before_arrow_allocates() {
    use rh_snapshot::ipc_guard::{MAX_VALUES_BYTES, ZSTD_MAX_RATIO};
    let engine = run_to(life(), 50 * DAY + 1);
    let (bytes, _) = write_keyframe(&engine, anchor(&engine)).unwrap();
    let body = unseal_keyframe(&bytes).unwrap().to_vec();
    let n = body.len();
    let footer_len =
        usize::try_from(i32::from_le_bytes(body[n - 10..n - 6].try_into().unwrap())).unwrap();
    let footer = arrow_ipc::root_as_footer(&body[n - 10 - footer_len..n - 10]).unwrap();
    let block = footer.recordBatches().unwrap().get(0);
    let meta_at = usize::try_from(block.offset()).unwrap() + 8;
    let fb_len = usize::try_from(i32::from_le_bytes(
        body[meta_at - 4..meta_at].try_into().unwrap(),
    ))
    .unwrap();
    let body_at =
        usize::try_from(block.offset()).unwrap() + usize::try_from(block.metaDataLength()).unwrap();
    let message = arrow_ipc::root_as_message(&body[meta_at..meta_at + fb_len]).unwrap();
    let batch = message.header_as_record_batch().unwrap();
    let rows = usize::try_from(batch.length()).unwrap();
    let mut cases = 0;
    for (k, buffer) in batch.buffers().unwrap().iter().enumerate() {
        let len = usize::try_from(buffer.length()).unwrap();
        if len == 0 {
            continue; // No prefix: an empty buffer (a validity bitmap with no nulls).
        }
        let at = body_at + usize::try_from(buffer.offset()).unwrap();
        let bound = match k % 3 {
            0 => rows / 8 + 64,
            1 => 4 * (rows + 1),
            _ => MAX_VALUES_BYTES,
        };
        let ratio = (len - 8) * ZSTD_MAX_RATIO;
        let forged = [
            i64::try_from(bound + 1).unwrap(),
            i64::try_from(ratio + 1).unwrap(),
            1 << 40,
            i64::MAX,
            -2,
        ];
        for value in forged {
            let mut f = body.clone();
            patch(&mut f, at, 8, value);
            match load(&seal_keyframe(&f)) {
                Err(KeyframeError::Ipc(_)) => cases += 1,
                Err(e) => panic!("buffer {k} declaring {value}: refused late: {e}"),
                Ok(_) => panic!("buffer {k} declaring {value}: accepted"),
            }
        }
    }
    println!("GAP-11: {cases} forged uncompressed lengths refused before decoding");
    assert!(
        cases >= 20,
        "at least four prefixed buffers forged five ways: {cases}"
    );
}
