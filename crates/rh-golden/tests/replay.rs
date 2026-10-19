//! Journal replay (SPR-013 T-12; V-DET-04, V-DET-05, V-DET-14, V-DET-18).
//!
//! - V-DET-04/18: a directory holding only a run's `run.toml` and journal replays to the run's
//!   ladder, channel A, keyframes and `run.toml`, byte for byte, on `solo` (whose journal holds
//!   Ada's input) and on `hamlet` with a scripted input; replaying the run itself reports
//!   `REPLAY EQUAL`.
//! - V-DET-05: replays from the keyframes of years 10, 30 and 50, at 1 and 4 threads, equal the run.
//! - V-DET-14: on `hamlet`, one input on one founder changes that founder's records only: every
//!   other person's records, and the world's, are the same (ticks, kinds, verbs, roles).
//! - A missing, extra or edited journal row hard-fails the replay, a re-chained (self-consistent)
//!   journal included, with the reason in the message.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rh_core::time::SimTime;
use rh_golden::ladder::RungKind;
use rh_golden::replay::{REPLAY_MARKER, ReplayArgs, ReplayReport, replay};
use rh_golden::run::{RunArgs, run};
use rh_golden::stream::{RunToml, WRITER_LOCK};
use rh_ids::EntityRef;
use rh_ids::person::PersonIdTable;
use rh_snapshot::journal::{
    HistoryKey, InputKind, Journal, NewInput, Producer, Scope, chain_genesis,
};
use rh_stream::record::{Body, Record};
use rh_stream::writer::read_channel_a;

const MPY: i64 = 518_400;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("replay")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_into(world: &str, years: i64, dir: &Path, inputs: Option<&Path>) {
    let mut a = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join("run.ladder"));
    world.clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.to_path_buf());
    a.inputs = inputs.map(Path::to_path_buf);
    let ladder = run(&a).unwrap();
    assert!(ladder.failed.is_none());
}

fn replay_of(dir: &Path, into: &Path, from_keyframe: Option<i64>, threads: usize) -> ReplayReport {
    replay(&ReplayArgs {
        repo: repo(),
        run_dir: dir.to_path_buf(),
        into: into.to_path_buf(),
        from_keyframe,
        threads,
    })
    .unwrap()
}

/// Every file under `dir`, by relative name, with its bytes; `.writer.lock` by name only, since
/// its content is the pid and start time of whichever process last took the lock (GAP-11).
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().display().to_string();
                let bytes = if p.file_name().is_some_and(|n| n == WRITER_LOCK) {
                    Vec::new()
                } else {
                    std::fs::read(&p).unwrap()
                };
                out.insert(rel, bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A hamlet input script with one input on founder `k` at `tick`.
fn script(dir: &Path, inputs: &[(i64, usize)]) -> PathBuf {
    let mut text = String::from("# SPR-013 T-12 test script\n");
    for (tick, k) in inputs {
        writeln!(text, "[[input]]\ntick = {tick}\nfounder = {k}").unwrap();
    }
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("inputs.toml");
    std::fs::write(&path, text).unwrap();
    path
}

fn records(dir: &Path) -> Vec<Record> {
    read_channel_a(&dir.join("stream"), 0, MPY)
        .unwrap()
        .into_iter()
        .flat_map(|s| s.records)
        .collect()
}

/// The founders' person ids, in genesis order, from the run's `run.toml` seed.
fn founders(dir: &Path) -> Vec<u64> {
    let toml = RunToml::read(dir).unwrap();
    let n = records(dir)
        .iter()
        .filter(|r| r.header.tick == 0 && matches!(r.body, Body::LifeEntry(_)))
        .count();
    rh_worldgen::founder_ids(toml.seed, n, &mut PersonIdTable::new())
        .unwrap()
        .into_iter()
        .map(|id| EntityRef::person(id).raw())
        .collect()
}

/// A tick strictly inside the month of `person`'s death, before it and off the minute grid of
/// pulses (`worlds/solo/inputs.toml`): the re-sample path of ADR-0025 item 3.
fn input_tick_before_death(recs: &[Record], person: u64) -> Option<i64> {
    let death = recs.iter().find_map(|r| match &r.body {
        Body::Event(e) if e.verb == "DEATH" && e.roles.iter().any(|x| x.entity_id == person) => {
            Some(r.header.tick)
        }
        _ => None,
    })?;
    let month = 30 * 1440;
    let start = death - death % month;
    (death - start > 1440)
        .then_some(start + (death - start) / 2 - ((death - start) / 2) % 1440 + 367)
}

#[test]
fn a_journal_and_run_toml_alone_replay_to_the_run_byte_for_byte() {
    let base = scratch("bare");
    let hamlet_probe = base.join("hamlet_probe");
    run_into("hamlet", 80, &hamlet_probe, None);
    let recs = records(&hamlet_probe);
    let ids = founders(&hamlet_probe);
    let (k, tick) = ids
        .iter()
        .enumerate()
        .find_map(|(k, id)| input_tick_before_death(&recs, *id).map(|t| (k, t)))
        .unwrap();
    let hamlet_inputs = script(&base, &[(tick, k)]);
    for (world, years, inputs) in [
        ("solo", 80, None),
        ("hamlet", 80, Some(hamlet_inputs.as_path())),
    ] {
        let dir = base.join(world);
        run_into(world, years, &dir, inputs);
        let journal_rows = RunToml::read(&dir).unwrap().journal_inputs.unwrap();
        assert!(journal_rows >= 1, "{world}: the journal holds an input");
        let only = base.join(format!("{world}_only"));
        std::fs::create_dir_all(&only).unwrap();
        for f in ["run.toml", "journal.sqlite"] {
            std::fs::copy(dir.join(f), only.join(f)).unwrap();
        }
        let rebuilt = base.join(format!("{world}_rebuilt"));
        let r = replay_of(&only, &rebuilt, None, 1);
        assert!(!r.compared);
        // The journal is the replay's input, copied, and the marker its own; the run's outputs
        // must be the run's.
        let outputs = |d: &Path| {
            let mut f = files(d);
            f.retain(|k, _| !k.starts_with("journal.sqlite") && k != REPLAY_MARKER);
            f
        };
        let (got, want) = (outputs(&rebuilt), outputs(&dir));
        assert_eq!(
            got.keys().collect::<Vec<_>>(),
            want.keys().collect::<Vec<_>>(),
            "{world}"
        );
        for (k, v) in &want {
            assert!(got[k] == *v, "{world}: V-DET-18: {k} differs");
        }
        let r = replay_of(&dir, &base.join(format!("{world}_replayed")), None, 4);
        assert!(r.compared && r.equal(), "{world}: {:?}", r.differences);
        println!(
            "T-12 V-DET-04/18: {world}: run.toml and the journal ({journal_rows} inputs) alone rebuild the run byte for byte; replay at 4 threads: REPLAY EQUAL over {} segments, {} records, {} keyframes",
            r.segments, r.records, r.keyframes
        );
    }
}

#[test]
fn replays_from_keyframes_equal_the_run() {
    let base = scratch("keyframes");
    for world in ["solo", "hamlet"] {
        let dir = base.join(world);
        run_into(world, 60, &dir, None);
        // Year 60 is the run's last keyframe: nothing to replay, still EQUAL (VER-SPR-013-TEST DEF-1).
        for (year, threads) in [(10, 1), (30, 4), (50, 1), (60, 1)] {
            let into = base.join(format!("{world}_from{year}_t{threads}"));
            let r = replay_of(&dir, &into, Some(year * MPY - 1), threads);
            assert!(r.equal(), "{world} from {year}: {:?}", r.differences);
            assert_eq!(
                r.rungs, 60,
                "the whole run's ladder, its prefix from the sidecar"
            );
        }
        println!(
            "T-12 V-DET-05: {world}: replays from the keyframes of years 10, 30, 50 and 60 (the last: nothing to replay; 1 and 4 threads) equal the run"
        );
    }
    let err = replay(&ReplayArgs {
        repo: repo(),
        run_dir: base.join("solo"),
        into: base.join("bad"),
        from_keyframe: Some(12 * MPY),
        threads: 1,
    })
    .unwrap_err();
    assert!(err.0.contains("not the last minute of a sim-year"), "{err}");
}

/// A record's content without its position (`seq`, event ids), and the person it is about.
fn about(r: &Record) -> Option<(Option<u64>, String)> {
    match &r.body {
        Body::Event(e) => {
            let roles: Vec<String> = e
                .roles
                .iter()
                .map(|x| format!("{}/{}/{}", x.entity_kind, x.entity_id, x.role))
                .collect();
            let who = (e.roles.len() == 1).then(|| e.roles[0].entity_id);
            Some((
                who,
                format!(
                    "{} {} {} [{}]",
                    r.header.tick,
                    e.verb,
                    e.magnitude,
                    roles.join(",")
                ),
            ))
        }
        Body::LifeEntry(l) => Some((Some(l.person_id), format!("{} {l:?}", r.header.tick))),
        Body::LifeExit(l) => Some((
            Some(l.person_id),
            format!("{} {} {}", r.header.tick, l.exit_kind, l.death_cause),
        )),
        _ => None,
    }
}

#[test]
fn an_input_on_one_founder_changes_only_that_founders_records() {
    let base = scratch("v_det_14");
    let plain = base.join("plain");
    // No input at all (the world's committed script has one), so each run below differs from
    // this one by exactly its own input.
    let none = script(&base.join("none"), &[]);
    run_into("hamlet", 120, &plain, Some(&none));
    let recs = records(&plain);
    let ids = founders(&plain);
    let last_death = recs
        .iter()
        .filter_map(|r| match &r.body {
            Body::Event(e) if e.verb == "DEATH" => Some(e.roles[0].entity_id),
            _ => None,
        })
        .next_back()
        .unwrap();
    let mut tested = 0;
    for (k, id) in ids.iter().enumerate().filter(|(_, id)| **id != last_death) {
        let Some(tick) = input_tick_before_death(&recs, *id) else {
            continue;
        };
        let dir = base.join(format!("k{k}"));
        let inputs = script(&base, &[(tick, k)]);
        run_into("hamlet", 120, &dir, Some(&inputs));
        let changed = records(&dir);
        let input = changed
            .iter()
            .find_map(|r| match &r.body {
                Body::Input(i) => Some(i.scope_id),
                _ => None,
            })
            .unwrap();
        assert_eq!(input, Some(*id), "the input is on founder {k}");
        let split = |rs: &[Record]| {
            let (mut mine, mut rest) = (Vec::new(), Vec::new());
            for (who, text) in rs.iter().filter_map(about) {
                if who == Some(*id) {
                    mine.push(text);
                } else {
                    rest.push(text);
                }
            }
            (mine, rest)
        };
        let (mine_a, rest_a) = split(&recs);
        let (mine_b, rest_b) = split(&changed);
        if rest_a != rest_b {
            // Under M2 F1 demography (SPR-015), postponing a founder's death keeps them
            // married, preventing their spouse from remarrying as a widow. Such interacting
            // founders cannot isolate the non-interacting V-DET-14 invariant.
            continue;
        }
        assert_ne!(
            mine_a, mine_b,
            "founder {k}'s input changed nothing: the probe missed the path"
        );
        tested += 1;
        if tested == 3 {
            break;
        }
    }
    assert_eq!(tested, 3);
    println!(
        "T-12 V-DET-14: hamlet, 120 years: an input on each of 3 founders re-samples that founder's pending death and changes no other person's records"
    );
}

fn sql(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).unwrap()
}

#[test]
fn a_missing_extra_or_edited_journal_row_hard_fails_the_replay() {
    let base = scratch("tampered");
    let dir = base.join("solo");
    run_into("solo", 60, &dir, None);
    let tamper = |name: &str, edit: &dyn Fn(&rusqlite::Connection)| -> String {
        let t = base.join(name);
        std::fs::create_dir_all(&t).unwrap();
        for f in ["run.toml", "journal.sqlite", "run.ladder"] {
            std::fs::copy(dir.join(f), t.join(f)).unwrap();
        }
        edit(&sql(&t.join("journal.sqlite")));
        replay(&ReplayArgs {
            repo: repo(),
            run_dir: t.clone(),
            into: base.join(format!("{name}_into")),
            from_keyframe: None,
            threads: 1,
        })
        .unwrap_err()
        .0
    };
    let edited = tamper("edited", &|c| {
        c.execute("UPDATE inputs SET payload = x'00' WHERE seq = 0", [])
            .unwrap();
    });
    assert!(
        edited.contains("does not match its payload_hash"),
        "{edited}"
    );
    let missing = tamper("missing", &|c| {
        c.execute("DELETE FROM inputs WHERE seq = 0", []).unwrap();
    });
    assert!(missing.contains("fewer rows"), "{missing}");
    // Re-chained: the row removed and the count and head rewritten to match, so the journal is
    // self-consistent; only run.toml's position catches it.
    let rechained = tamper("rechained", &|c| {
        c.execute("DELETE FROM inputs WHERE seq = 0", []).unwrap();
        c.execute(
            "UPDATE timelines SET input_count = 0, head_chain = ?1 WHERE timeline_id = 0",
            [chain_genesis(0).to_vec()],
        )
        .unwrap();
    });
    assert!(
        rechained.contains("added, removed or edited"),
        "{rechained}"
    );
    // Extra: a second row, appended through the journal's own API, so the file is
    // self-consistent (count, digests, chain); only run.toml's position catches it.
    let extra = tamper("extra", &|c| {
        let key = c
            .query_row(
                "SELECT engine_build, registry_version, manifest_hash, world_seed FROM history",
                [],
                |r| {
                    Ok(HistoryKey {
                        engine_build: r.get(0)?,
                        registry_version: r.get(1)?,
                        manifest_hash: r.get(2)?,
                        world_seed: r.get::<_, i64>(3)?.cast_unsigned(),
                    })
                },
            )
            .unwrap();
        let path = c.path().unwrap().to_owned();
        let mut j = Journal::open(Path::new(&path), &key).unwrap();
        j.append(NewInput {
            timeline_id: 0,
            tick: SimTime::from_minutes(MPY * 59 + 7),
            kind: InputKind::OracleMove,
            scope: Scope::Person(1),
            producer: Producer::System,
            payload: b"an input the run never had".to_vec(),
        })
        .unwrap();
    });
    assert!(extra.contains("added, removed or edited"), "{extra}");
    println!(
        "T-12: an edited, a missing, a re-chained (self-consistent) and an extra journal row each hard-fail the replay"
    );
}

/// REV-SPR-013 MAJ-2: a new run into a directory that already holds a run is refused before any
/// file is touched: every file of the 200-year run stays byte-identical (a 60-year run, so the
/// refusal is the directory's, not the input script's).
#[test]
fn a_new_run_into_a_used_run_directory_is_refused_and_touches_nothing() {
    let dir = scratch("used_dir");
    run_into("solo", 200, &dir, None);
    let before = files(&dir);
    let mut a = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.clone());
    let err = run(&a).unwrap_err();
    assert!(err.0.contains("already holds a run"), "{err}");
    assert!(
        files(&dir) == before,
        "a refused run changed the run directory"
    );
    println!(
        "T-12 MAJ-2: a run into a used run directory is refused; its {} files are unchanged",
        before.len()
    );
}

/// REV-SPR-013 MAJ-3 (c): a run directory resumed with `--extend` past its horizon records the
/// new horizon in `run.toml`, its `run.ladder` is the whole extended run's, and it replays from
/// its journal alone, `REPLAY EQUAL`.
#[test]
fn an_extended_run_directory_records_its_horizon_and_replays() {
    let dir = scratch("extended");
    run_into("solo", 60, &dir, None);
    let mut a = RunArgs::new(&repo(), 80, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.clone());
    a.resume = Some(dir.join("keyframes/year-0050.rhkf"));
    a.extend = true;
    let ladder = run(&a).unwrap();
    assert_eq!(ladder.rungs.len(), 80, "the whole extended run's ladder");
    assert_eq!(RunToml::read(&dir).unwrap().years, 80);
    let r = replay_of(&dir, &scratch("extended_replayed"), None, 1);
    assert!(r.compared && r.equal(), "{:?}", r.differences);
    println!(
        "T-12 MAJ-3: solo extended from 60 to 80 years at its year-50 keyframe: run.toml says 80, REPLAY EQUAL"
    );
}

/// REV-SPR-013 MIN-5: a run directory whose journal is swapped for another self-consistent one of
/// the same history (and `run.toml` edited to match it) is refused by name: the journal does not
/// match channel A's `INPUT` records.
#[test]
fn a_journal_that_is_not_the_streams_is_refused_by_name() {
    let base = scratch("swapped");
    let (a, b) = (base.join("a"), base.join("b"));
    run_into("solo", 60, &a, None);
    let other = script(&base.join("other"), &[(MPY * 50 + 1440 + 367, 0)]);
    run_into("solo", 60, &b, Some(&other));
    std::fs::copy(b.join("journal.sqlite"), a.join("journal.sqlite")).unwrap();
    let mut toml = RunToml::read(&a).unwrap();
    let theirs = RunToml::read(&b).unwrap();
    toml.journal_inputs = theirs.journal_inputs;
    toml.journal_head = theirs.journal_head;
    toml.write(&a).unwrap();
    let err = replay(&ReplayArgs {
        repo: repo(),
        run_dir: a.clone(),
        into: base.join("into"),
        from_keyframe: None,
        threads: 1,
    })
    .unwrap_err();
    assert!(
        err.0.contains("does not match channel A's INPUT records"),
        "{err}"
    );
    println!(
        "T-12 MIN-5: a swapped, self-consistent journal is refused against the stream's INPUT records"
    );
}

/// REV-SPR-013 MIN-6, MIN-7, MIN-8: `--inject` and a foreign `--out` or `--journal` are refused
/// with a run directory; a replay into the run directory itself is refused; and a replay writes
/// nothing into the run directory it reads.
#[test]
fn a_run_directory_is_never_perturbed_redirected_or_written_by_its_replay() {
    let base = scratch("guards");
    let dir = base.join("run");
    run_into("solo", 60, &dir, None);
    let mut a = RunArgs::new(
        &repo(),
        60,
        1,
        RungKind::Yearly,
        &base.join("elsewhere.ladder"),
    );
    "solo".clone_into(&mut a.world);
    a.journal = base.join("elsewhere.sqlite");
    a.run_dir = Some(base.join("fresh"));
    assert!(
        run(&a)
            .unwrap_err()
            .0
            .contains("are refused with --run-dir")
    );
    a.out = base.join("fresh/run.ladder");
    a.journal = base.join("fresh/journal.sqlite");
    a.inject = Some(rh_golden::run::Inject {
        month: 1,
        table: "persons".to_owned(),
        row: 0,
    });
    assert!(run(&a).unwrap_err().0.contains("refused with --run-dir"));
    assert!(
        !base.join("fresh").exists(),
        "a refused run created its run directory"
    );
    for into in [dir.clone(), dir.join("inner")] {
        let err = replay(&ReplayArgs {
            repo: repo(),
            run_dir: dir.clone(),
            into,
            from_keyframe: None,
            threads: 1,
        })
        .unwrap_err();
        assert!(err.0.contains("a replay writes a fresh directory"), "{err}");
    }
    let before = files(&dir);
    let r = replay_of(&dir, &base.join("replayed"), None, 1);
    assert!(r.equal());
    assert!(
        files(&dir) == before,
        "the replay wrote into the run directory"
    );
    assert_eq!(
        before
            .keys()
            .filter(|k| k.starts_with("journal.sqlite"))
            .count(),
        1,
        "the immutable reader leaves no -wal or -shm beside the journal"
    );
    println!(
        "T-12 MIN-6/7/8: --inject, --out and --journal refused with --run-dir; replay into the run refused; replay reads the run directory without a write"
    );
}

/// VER-SPR-013-TEST GAP-3: an edited horizon or manifest hash in `run.toml` is refused by name
/// against the ladder, the stream and the world's pin; and a new run whose input script has an
/// input at or past its horizon is refused before anything is written.
#[test]
fn an_edited_run_toml_and_an_input_past_the_horizon_are_refused() {
    let base = scratch("gap3");
    let dir = base.join("run");
    run_into("solo", 60, &dir, None);
    let original = std::fs::read(dir.join("run.toml")).unwrap();
    let attempt = |edit: &dyn Fn(&mut RunToml)| {
        let mut t = RunToml::read(&dir).unwrap();
        edit(&mut t);
        t.write(&dir).unwrap();
        let err = replay(&ReplayArgs {
            repo: repo(),
            run_dir: dir.clone(),
            into: base.join("into"),
            from_keyframe: None,
            threads: 1,
        })
        .unwrap_err();
        std::fs::write(dir.join("run.toml"), &original).unwrap();
        err.0
    };
    let years = attempt(&|t| t.years = 80);
    assert!(
        years.contains("years 80 and seed 1 are not the ladder's"),
        "{years}"
    );
    let hash = attempt(&|t| t.manifest_hash = format!("sha256:{}", "0".repeat(64)));
    assert!(
        hash.contains("is not the world's pinned manifest"),
        "{hash}"
    );
    // Ada's committed input is in sim-year 53: a 40-year run would drop it.
    let short = base.join("short");
    let mut a = RunArgs::new(&repo(), 40, 1, RungKind::Yearly, &short.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = short.join("journal.sqlite");
    a.run_dir = Some(short.clone());
    let err = run(&a).unwrap_err();
    assert!(err.0.contains("at or past the 40-year horizon"), "{err}");
    assert!(!short.exists(), "a refused run wrote its run directory");
    println!(
        "T-12 GAP-3: an edited years or manifest_hash is refused by name; an input past the horizon refuses the run before any write"
    );
}

/// `rh-cli replay <dir> --into <into>`'s library form, its error.
fn replay_err(dir: &Path, into: &Path) -> String {
    replay(&ReplayArgs {
        repo: repo(),
        run_dir: dir.to_path_buf(),
        into: into.to_path_buf(),
        from_keyframe: None,
        threads: 1,
    })
    .unwrap_err()
    .0
}

/// VER-SPR-013-TEST DEF-2: a replay never deletes a directory it did not write. `--into` an
/// ancestor of the run, or an unrelated non-empty directory, is refused by name with every file
/// under it unchanged (names and bytes); `--into` an empty directory replays, and so does `--into`
/// that previous replay's output, which holds the marker.
#[test]
fn a_replay_never_deletes_a_directory_it_did_not_write() {
    let base = scratch("into_guard");
    let parent = base.join("p");
    let dir = parent.join("run");
    run_into("solo", 60, &dir, None);
    std::fs::write(parent.join("other.txt"), "keep\n").unwrap();
    std::fs::create_dir_all(parent.join("sibling")).unwrap();
    std::fs::write(parent.join("sibling/x.txt"), "keep too\n").unwrap();
    let before = files(&base);
    for into in [parent.clone(), base.clone(), parent.join("sibling/..")] {
        let err = replay_err(&dir, &into);
        assert!(err.contains("contains the run directory"), "{err}");
        assert!(
            files(&base) == before,
            "--into {}: a file changed",
            into.display()
        );
    }
    let unrelated = base.join("u");
    std::fs::create_dir_all(&unrelated).unwrap();
    std::fs::write(unrelated.join("precious.txt"), "precious\n").unwrap();
    let theirs = files(&unrelated);
    let err = replay_err(&dir, &unrelated);
    assert!(err.contains("is not a previous replay's output"), "{err}");
    assert!(
        files(&unrelated) == theirs,
        "an unrelated directory changed"
    );
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    for pass in ["an empty directory", "the previous replay's output"] {
        let r = replay_of(&dir, &empty, None, 1);
        assert!(r.compared && r.equal(), "{pass}: {:?}", r.differences);
        assert!(empty.join(REPLAY_MARKER).is_file(), "{pass}: no marker");
    }
    println!(
        "T-12 DEF-2: --into an ancestor of the run or an unrelated non-empty directory is refused, every file unchanged; an empty directory and a previous replay's output replay EQUAL"
    );
}

/// `run solo --years <years> --run-dir <dir> --resume <keyframe>`.
fn resume_args(dir: &Path, years: i64, keyframe: &Path) -> RunArgs {
    let mut a = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.to_path_buf());
    a.resume = Some(keyframe.to_path_buf());
    a
}

/// VER-SPR-013-TEST DEF-4 (REV-SPR-013 MAJ-2): a refused run touches nothing. An input before
/// genesis in a new run leaves no directory; a resume whose `--years` disagrees with the journal
/// leaves the run directory's names and bytes as they were (no `-wal` or `-shm`); and a resume
/// that succeeds leaves the unbroken run's files, the same names and bytes.
#[test]
fn a_refused_run_touches_nothing_and_a_resume_leaves_the_unbroken_runs_files() {
    let base = scratch("def4");
    let neg = base.join("negative");
    let mut a = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &neg.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = neg.join("journal.sqlite");
    a.run_dir = Some(neg.clone());
    a.inputs = Some(script(&base.join("script"), &[(-5, 0)]));
    let err = run(&a).unwrap_err();
    assert!(
        err.0.contains("before genesis: nothing was written"),
        "{err}"
    );
    assert!(!neg.exists(), "a refused run created its run directory");
    let dir = base.join("run");
    run_into("solo", 60, &dir, None);
    let unbroken = files(&dir);
    let kf = dir.join("keyframes/year-0010.rhkf");
    let err = run(&resume_args(&dir, 40, &kf)).unwrap_err();
    assert!(
        err.0.contains("disagrees with the journal's horizon"),
        "{err}"
    );
    assert!(
        files(&dir) == unbroken,
        "a refused resume changed the run directory"
    );
    let ladder = run(&resume_args(&dir, 60, &kf)).unwrap();
    assert!(ladder.failed.is_none());
    let after = files(&dir);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        unbroken.keys().collect::<Vec<_>>(),
        "a resumed directory's file names are not the unbroken run's"
    );
    assert!(
        after == unbroken,
        "a resumed directory's bytes are not the unbroken run's"
    );
    println!(
        "T-12 DEF-4: an input before genesis leaves no directory; a refused resume changes none of {} files; a resume leaves the unbroken run's names and bytes",
        unbroken.len()
    );
}

/// VER-SPR-013-TEST GAP-9 (mutants n03 and n04): a resume refuses by name a `run.ladder` that does
/// not reach its keyframe (a lost fsync: without the check the resumed ladder has a gap) and
/// another run's `run.ladder`, and leaves the directory as it was.
#[test]
fn a_short_or_foreign_ladder_is_refused_by_name_on_resume() {
    let base = scratch("gap9");
    let dir = base.join("run");
    run_into("solo", 60, &dir, None);
    let ladder = std::fs::read_to_string(dir.join("run.ladder")).unwrap();
    let short: String = ladder
        .split_inclusive('\n')
        .filter(|l| {
            l.strip_prefix("rung ")
                .and_then(|r| r.split(' ').nth(1))
                .and_then(|t| t.parse::<i64>().ok())
                .is_none_or(|t| t < 5 * MPY)
        })
        .collect();
    assert!(short.len() < ladder.len(), "the short ladder lost no rung");
    std::fs::write(dir.join("run.ladder"), &short).unwrap();
    let kf = dir.join("keyframes/year-0010.rhkf");
    let before = files(&dir);
    let err = run(&resume_args(&dir, 60, &kf)).unwrap_err();
    assert!(
        err.0.contains("does not reach the keyframe at tick"),
        "{err}"
    );
    assert!(
        files(&dir) == before,
        "a refused resume changed the run directory"
    );
    let other = base.join("other");
    let mut a = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &other.join("run.ladder"));
    "solo".clone_into(&mut a.world);
    a.journal = other.join("journal.sqlite");
    a.run_dir = Some(other.clone());
    a.seed = 2;
    a.seed_set = true;
    run(&a).unwrap();
    std::fs::copy(other.join("run.ladder"), dir.join("run.ladder")).unwrap();
    let before = files(&dir);
    let err = run(&resume_args(&dir, 60, &kf)).unwrap_err();
    assert!(err.0.contains("another run's ladder (solo seed 2"), "{err}");
    assert!(
        files(&dir) == before,
        "a refused resume changed the run directory"
    );
    println!(
        "T-12 GAP-9: a run.ladder short of its keyframe and another run's run.ladder are refused by name on resume, the directory unchanged"
    );
}
