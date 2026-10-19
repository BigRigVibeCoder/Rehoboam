//! SPR-020 T-06: channel A's envelope tables from the event, on a stub world with the full
//! envelope (`specs/chronicle/envelope_v0.md` §1, §4, §5, §8; R6).
//!
//! Two stub systems emit every day: `Z1.stage` a `RAISE_WIND` (an event snapshot of the weather,
//! a place, a negative magnitude, each secrecy in turn) and a `MENTAL_BREAK` (actor, target, two
//! witnesses; `stress(actor)` and relational `trust(actor, target)` snapshots; a `motivated_by`
//! edge with its motive to the wind, a `contingent` edge, and a `triggered` edge to the wind of a
//! year before, a known id); `Z2.crowd` a `FLED`. The engine drains to the Chronicle Writer's path:
//! the admission bitset, `event_body`, the stream writer.
//!
//! - The rows the writer seals are the event's, with the typed Registry's dictionaries
//!   (`attr_id`, motive index); every record's canonical line round-trips through rh-stream's
//!   parser; the run is written to `<target>/tmp/rh-golden/envelope_world` for
//!   `specs/stream/check_segments.py`, which reads it with `DuckDB` (V-DATA-10 subset; a `check_all`
//!   gate).
//! - `cause_told`: with a test-only admission predicate that refuses the wind of every odd day,
//!   an edge to a refused cause is untold, and the refused events have no record; after a kill in
//!   the middle of a year and a resume (SPR-013 T-09's path: the writer resumed after the last
//!   sealed year, its bitset rebuilt from the sealed `event` tables), the segments are
//!   byte-identical to the unbroken run's; with every draft admitted, every edge is told.
//! - V-STR-01 on the stub world: two runs, and 1 against 4 threads, seal the same bytes.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "a test: a broken stub world must fail loudly, and day numbers are small"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_core::access::{AccessSpec, ColumnWrite, Plane};
use rh_core::engine::Engine;
use rh_core::ownership::{EmitDecl, SnapshotDecl, SpecEntry, SpecTable};
use rh_core::register::register;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, StepCtx, SystemDescriptor,
    SystemModel,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{DraftRef, EdgeKind, EventDraft, EventRef, RoleId, Secrecy};
use rh_ids::{BranchSalt, EntityRef, EventId, PersonId, PlaceId};
use rh_manifest::Registry;
use rh_stream::canonical;
use rh_stream::convert::event_body;
use rh_stream::record::Body;
use rh_stream::seal::{NoHook, channel_dir, read_segment, sealed_segments};
use rh_stream::segment::Provenance;
use rh_stream::writer::{StreamWriter, WriterConfig, read_channel_a};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const MONTH: i64 = 30 * DAY;
const PERSONS: usize = 9;
const STAGE: &str = "Z1.stage";
const CROWD: &str = "Z2.crowd";
const STRESS: &str = "stage.stress";
const WIND: &str = "RAISE_WIND";
const BREAK: &str = "MENTAL_BREAK";
const FLED: &str = "FLED";
/// Events per day: the wind and the break (Z1), then the flight (Z2), in system-id order.
const PER_DAY: u64 = 3;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn person(row: usize) -> EntityRef {
    EntityRef::person(PersonId::from_raw(row as u64))
}

fn day_of(now: SimTime) -> u64 {
    (now.minutes() / DAY) as u64
}

struct Stage;
struct Crowd;

fn descriptor(id: &'static str) -> SystemDescriptor {
    SystemDescriptor {
        id,
        fidelity: "F0",
        spec_version: "0.0.0",
    }
}

impl SystemModel for Stage {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        descriptor(STAGE)
    }
    fn access(&self) -> AccessSpec {
        AccessSpec {
            writes: vec![ColumnWrite {
                column: STRESS,
                plane: Plane::A,
            }],
            emits: vec![WIND, BREAK],
            ..AccessSpec::default()
        }
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Agenda,
        }]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let d = day_of(ctx.now());
        let (a, t) = ((d as usize) % PERSONS, (d as usize + 1) % PERSONS);
        let pre = view.read(STRESS)?[a];
        let secrecy = Secrecy::from_code((d % 4) as u8).unwrap_or_default();
        let wind = ctx.emit(
            EventDraft::new(WIND)
                .at(PlaceId::from_raw((d % 5) as u32))
                .magnitude(-(d as i32))
                .secrecy(secrecy)
                .event_snapshot("weather", (d % 7) as i32)
                .with_payload(vec![d as i64]),
        )?;
        let mut brk = EventDraft::new(BREAK)
            .role(RoleId::TARGET, person(t))
            .witness(person((a + 3) % PERSONS))
            .witness(person((a + 2) % PERSONS))
            .role(RoleId::ACTOR, person(a))
            .relational_snapshot("trust", person(a), person(t), -((d % 50) as i32))
            .snapshot("stress", person(a), pre as i32)
            .motivated_by(EventRef::Draft(wind), "GRIEF_FOR", 300, 64)
            .motivated_by(EventRef::Draft(wind), "FEAR_OF", -100, 32)
            .caused_by(EventRef::Draft(wind), EdgeKind::Contingent);
        if d >= 360 {
            // The wind of a year before: a known id (each day emits PER_DAY events, in order).
            let before = EventId::from_raw((d - 360) * PER_DAY);
            brk = brk.caused_by(EventRef::Known(before), EdgeKind::Triggered);
        }
        let _: DraftRef = ctx.emit(brk)?;
        view.write(STRESS)?[a] = (pre + 777) % (100 * 256);
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

impl SystemModel for Crowd {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        descriptor(CROWD)
    }
    fn access(&self) -> AccessSpec {
        AccessSpec {
            emits: vec![FLED],
            ..AccessSpec::default()
        }
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Agenda,
        }]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let d = day_of(ctx.now());
        ctx.emit(EventDraft::new(FLED).role(RoleId::ACTOR, person((d as usize * 5) % PERSONS)))?;
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn names(n: &[&str]) -> Vec<String> {
    n.iter().map(|x| (*x).to_owned()).collect()
}

fn snap(attr: &str, of: &str, partner: Option<&str>, from: &str) -> SnapshotDecl {
    SnapshotDecl {
        attr: attr.to_owned(),
        of: of.to_owned(),
        partner: partner.map(str::to_owned),
        from: from.to_owned(),
        ty: from.starts_with("algorithm.").then(|| "q8_8".to_owned()),
    }
}

fn registry() -> Registry {
    Registry::load(&repo().join("specs/manifest/registry_v0.toml")).unwrap()
}

fn spec_table() -> SpecTable {
    let text = std::fs::read_to_string(repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let mut stage = SpecEntry {
        owns: vec![(STRESS.to_owned(), "A".to_owned())],
        schedule: vec![("Day".to_owned(), "Agenda".to_owned())],
        emits: names(&[WIND, BREAK]),
        ..SpecEntry::default()
    };
    stage.envelope.insert(
        WIND.to_owned(),
        EmitDecl {
            snapshots: vec![snap("weather", "event", None, "algorithm.sky")],
            ..EmitDecl::default()
        },
    );
    stage.envelope.insert(
        BREAK.to_owned(),
        EmitDecl {
            roles: names(&["actor", "target", "witness"]),
            motives: names(&["GRIEF_FOR", "FEAR_OF"]),
            snapshots: vec![
                snap("stress", "actor", None, STRESS),
                snap("trust", "actor", Some("target"), "algorithm.trust"),
            ],
        },
    );
    let mut crowd = SpecEntry {
        schedule: vec![("Day".to_owned(), "Agenda".to_owned())],
        emits: names(&[FLED]),
        ..SpecEntry::default()
    };
    crowd.envelope.insert(
        FLED.to_owned(),
        EmitDecl {
            roles: names(&["actor"]),
            ..EmitDecl::default()
        },
    );
    let mut owners = BTreeMap::new();
    owners.insert(
        STRESS.to_owned(),
        rh_core::ownership::Owner {
            system: STAGE.to_owned(),
            plane: "A".to_owned(),
        },
    );
    SpecTable {
        owners,
        specs: [(STAGE.to_owned(), stage), (CROWD.to_owned(), crowd)]
            .into_iter()
            .collect(),
        registry: rh_registry::ownership::parse_registry_table("registry_v0.toml", &text).unwrap(),
        enum_codes: BTreeMap::new(),
    }
}

/// The test-only admission predicate: the wind of every odd day is refused.
fn odd_winds_refused(d: &EventDraft) -> bool {
    !(d.kind == WIND && d.payload.first().is_some_and(|day| day % 2 == 1))
}

fn engine(threads: usize, refuse: bool) -> Engine {
    let registered = register(vec![Box::new(Stage), Box::new(Crowd)], &spec_table())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert(STRESS, (0..PERSONS as i64).map(|r| r * 1_000).collect());
    let mut e = Engine::new(
        CALENDAR,
        41,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    e.set_admission(refuse.then_some(odd_winds_refused as fn(&EventDraft) -> bool));
    e
}

fn config(root: &Path) -> WriterConfig {
    WriterConfig {
        root: root.to_path_buf(),
        timeline_id: 0,
        minutes_per_year: CALENDAR.minutes_per_year(),
        provenance: Provenance {
            engine_build: "rh-golden-envelope-world".to_owned(),
            manifest_hash: vec![0x5A; 32],
            world_seed: 41,
            built_systems: names(&[STAGE, CROWD]),
            not_built: Vec::new(),
        },
        dictionaries: rh_golden::stream::dictionaries(&registry()).unwrap(),
    }
}

/// The month stops from `from` (exclusive) to `years` years.
fn stops(from: i64, years: i64) -> impl Iterator<Item = i64> {
    let end = years * CALENDAR.minutes_per_year();
    (1..)
        .map(|k| k * MONTH - 1)
        .skip_while(move |&t| t <= from)
        .take_while(move |&t| t < end)
}

/// Advances `e` to `t` and hands its drained events to `w` as records, through the writer's
/// admission bitset (the Chronicle Writer's path).
fn stop(e: &mut Engine, w: &mut StreamWriter, t: i64) {
    e.advance_until(SimTime::from_minutes(t), |_, _| {})
        .unwrap_or_else(|err| panic!("{t}: {err}"));
    let dictionaries = config(Path::new(".")).dictionaries;
    let mut batch = Vec::new();
    for ev in e.drain_events() {
        if w.admissions_mut().admits(&ev) {
            let body = event_body(&ev, &dictionaries, w.admissions()).unwrap();
            batch.push((ev.tick, body));
        }
    }
    w.push(t, batch, &mut NoHook).unwrap();
}

/// A whole run of `years` into `root`.
fn run(root: &Path, years: i64, threads: usize, refuse: bool) {
    let _ = std::fs::remove_dir_all(root);
    let mut e = engine(threads, refuse);
    let mut w = StreamWriter::create(config(root)).unwrap();
    for t in stops(-1, years) {
        stop(&mut e, &mut w, t);
    }
    w.finish(&mut NoHook).unwrap();
}

fn files_of(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for (year, dir) in
        sealed_segments(&channel_dir(root, 0, rh_stream::record::Channel::A)).unwrap()
    {
        for (name, bytes) in read_segment(&dir).unwrap() {
            out.insert(format!("y{year}/{name}"), bytes);
        }
    }
    out
}

/// A directory under Cargo's per-target temporary directory (`target/tmp/rh-golden/<name>`).
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("rh-golden")
        .join(name);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    dir
}

/// The rows are the event's; every canonical line round-trips; the fixture for `DuckDB` is written.
#[test]
fn the_envelope_world_writes_the_events_envelope() {
    let dir = scratch("envelope_world");
    let root = dir.join("stream");
    run(&root, 3, 1, true);
    let segs = read_channel_a(&root, 0, CALENDAR.minutes_per_year()).unwrap();
    let reg = registry();
    let (stress, trust, weather) = (
        reg.attr_id("stress").unwrap(),
        reg.attr_id("trust").unwrap(),
        reg.attr_id("weather").unwrap(),
    );
    let (mut breaks, mut winds, mut untold, mut told, mut lines) = (0, 0, 0, 0, 0);
    for r in segs.iter().flat_map(|s| &s.records) {
        let line = canonical::render(r).unwrap();
        assert_eq!(&canonical::parse(&line).unwrap(), r, "round trip: {line}");
        lines += 1;
        let Body::Event(e) = &r.body else { continue };
        match e.verb.as_str() {
            BREAK => {
                breaks += 1;
                let roles: Vec<u8> = e.roles.iter().map(|x| x.role).collect();
                assert_eq!(roles, [0, 1, 2, 2], "actor, target, two witnesses in order");
                let attrs: Vec<u16> = e.snapshots.iter().map(|x| x.attr_id).collect();
                assert_eq!(attrs, [stress, trust], "by attr_id");
                assert!(e.snapshots[1].partner_id.is_some());
                let motivated = e.causes.iter().find(|c| c.kind == 3).unwrap();
                // GRIEF_FOR +300 and FEAR_OF -100 to the wind coalesce to one motivated_by edge,
                // weight +200, motive GRIEF_FOR (index 6), evidence the larger margin.
                assert_eq!(
                    (motivated.motive, motivated.weight, motivated.evidence),
                    (reg.motive_index("GRIEF_FOR"), 200, 64)
                );
                for c in &e.causes {
                    if c.cause_told {
                        told += 1;
                    } else {
                        untold += 1;
                    }
                }
            }
            WIND => {
                winds += 1;
                assert_eq!(e.snapshots[0].attr_id, weather);
                assert_eq!(
                    e.snapshots[0].entity_kind, 0,
                    "an event snapshot is of the world"
                );
                assert!(e.place_id.is_some() && e.magnitude <= 0);
            }
            _ => {}
        }
    }
    assert_eq!(breaks, 3 * 360);
    assert_eq!(winds, 3 * 180, "the odd days' winds are refused: no record");
    assert!(untold > 0 && told > 0, "{untold} untold, {told} told");
    println!(
        "T-06 envelope world: {} segments, {lines} canonical lines round-trip; {breaks} breaks, \
         {winds} winds; {told} told and {untold} untold edges in {}",
        segs.len(),
        dir.display()
    );
}

/// `cause_told` follows admission exactly, survives a kill and a resume byte for byte, and is
/// true on every edge when every draft is admitted.
#[test]
fn cause_told_follows_admission_across_a_resume() {
    // An edge is told exactly when its cause has a record.
    let root = scratch("envelope_told").join("stream");
    run(&root, 2, 1, true);
    let segs = read_channel_a(&root, 0, CALENDAR.minutes_per_year()).unwrap();
    let recorded: std::collections::BTreeSet<u64> = segs
        .iter()
        .flat_map(|s| &s.records)
        .filter_map(|r| match &r.body {
            Body::Event(e) => Some(e.event_id),
            _ => None,
        })
        .collect();
    let mut cross_year_untold = 0;
    for r in segs.iter().flat_map(|s| &s.records) {
        if let Body::Event(e) = &r.body {
            for c in &e.causes {
                assert_eq!(c.cause_told, recorded.contains(&c.cause_id), "edge {e:?}");
                if !c.cause_told && c.kind == 1 {
                    cross_year_untold += 1;
                }
            }
        }
    }
    assert!(cross_year_untold > 0, "an untold edge to a year before");
    // Kill in the middle of year 2 (year 1 sealed), resume after year 1, finish year 3.
    let unbroken = scratch("envelope_unbroken").join("stream");
    run(&unbroken, 3, 1, true);
    let broken = scratch("envelope_broken").join("stream");
    let _ = std::fs::remove_dir_all(&broken);
    let mpy = CALENDAR.minutes_per_year();
    {
        let mut e = engine(1, true);
        let mut w = StreamWriter::create(config(&broken)).unwrap();
        for t in stops(-1, 3).take_while(|&t| t < mpy + 5 * MONTH) {
            stop(&mut e, &mut w, t);
        }
        drop(w); // kill -9: the open year is lost, the sealed year stays
    }
    let resume_at = 2 * mpy - 1;
    let mut w = StreamWriter::resume(config(&broken), mpy - 1).unwrap();
    // The probe: the resumed bitset was rebuilt from the sealed year (day 0's wind told, day 1's
    // refused), so year 2's edges to year 1 depend on it.
    assert!(w.admissions().told(0) && !w.admissions().told(PER_DAY));
    let mut e = engine(1, true);
    e.advance_until(SimTime::from_minutes(mpy - 1), |_, _| {})
        .unwrap();
    let _ = e.drain_events();
    for t in stops(mpy - 1, 3) {
        stop(&mut e, &mut w, t);
    }
    w.finish(&mut NoHook).unwrap();
    assert!(resume_at > 0);
    assert_eq!(files_of(&broken), files_of(&unbroken), "resumed bytes");
    // Every draft admitted: every edge told.
    let all = scratch("envelope_all_told").join("stream");
    run(&all, 2, 1, false);
    for r in read_channel_a(&all, 0, mpy)
        .unwrap()
        .iter()
        .flat_map(|s| &s.records)
    {
        if let Body::Event(e) = &r.body {
            assert!(e.causes.iter().all(|c| c.cause_told), "{e:?}");
        }
    }
    println!(
        "T-06 cause_told: {cross_year_untold} untold edges to refused causes of a year before; \
         a resumed writer seals the unbroken run's bytes; every edge told when all are admitted"
    );
}

/// V-STR-01 on the stub world: two runs, and 1 against 4 threads, seal byte-identical segments.
#[test]
fn v_str_01_the_envelope_world_is_byte_identical() {
    let one = scratch("envelope_t1").join("stream");
    let again = scratch("envelope_t1b").join("stream");
    let four = scratch("envelope_t4").join("stream");
    run(&one, 2, 1, true);
    run(&again, 2, 1, true);
    run(&four, 2, 4, true);
    let reference = files_of(&one);
    assert_eq!(reference.len(), 2 * 16, "two segments of sixteen files");
    assert_eq!(files_of(&again), reference, "two runs");
    assert_eq!(files_of(&four), reference, "1 against 4 threads");
}
