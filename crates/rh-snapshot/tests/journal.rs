//! The input journal (SPR-007 T-03): round trip; a replay reads only the journal; every way a
//! journal can disagree with a run is a hard failure; `SQLite` stores no `REAL`.
//!
//! REF: `05.9` IF-2; `05.1` §6 item 8; `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-03,
//! ruling 3.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use rh_core::time::SimTime;
use rh_snapshot::hash::xxh3_128;
use rh_snapshot::journal::{
    ConsoleMode, HistoryKey, InputClass, InputKind, Journal, JournalError, JournaledInput,
    NewInput, Producer, Replay, Scope, chain_genesis, chain_next, row_digest,
};
use rusqlite::Connection;

/// A fresh, empty directory under cargo's per-test temporary directory.
fn fresh_dir(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "journal-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn key() -> HistoryKey {
    HistoryKey {
        engine_build: "rh-0.0.0+test".to_owned(),
        registry_version: "0.1.0".to_owned(),
        manifest_hash: vec![0xAB; 16],
        world_seed: u64::MAX - 6,
    }
}

fn llm() -> Producer {
    Producer::LlmJob {
        job_id: "job-17".to_owned(),
        model_id: "test-model".to_owned(),
        weights_hash: None,
        provider_version: Some("2026-10-01".to_owned()),
        prompt_hash: vec![7; 16],
    }
}

/// One input of every kind, with every scope and producer, some scheduled out of tick order.
fn inputs() -> Vec<NewInput> {
    let t = SimTime::from_minutes;
    let kinds = [
        (t(0), InputKind::ManifestSet, Scope::None, Producer::System),
        (
            t(0),
            InputKind::ConsoleMode(ConsoleMode::Oracle),
            Scope::None,
            Producer::Player,
        ),
        (
            t(1440),
            InputKind::SettingChange,
            Scope::None,
            Producer::System,
        ),
        // Scheduled for a later tick when issued: journaled now, applied later.
        (
            t(90_000),
            InputKind::OracleOmen,
            Scope::Region(3),
            Producer::Player,
        ),
        (
            t(1440),
            InputKind::OracleMove,
            Scope::None,
            Producer::Player,
        ),
        (
            t(2880),
            InputKind::OracleWhisper,
            Scope::Person(u64::MAX),
            llm(),
        ),
        (
            t(2880),
            InputKind::OraclePierce,
            Scope::Person(42),
            Producer::Player,
        ),
        (
            t(4320),
            InputKind::OracleWard,
            Scope::Settlement(9),
            Producer::Player,
        ),
        (
            t(4320),
            InputKind::PlayerAction {
                verb_id: "TAKE".to_owned(),
            },
            Scope::Household(5),
            Producer::Player,
        ),
        (t(5000), InputKind::SpeechAct, Scope::Person(1), llm()),
        (
            t(-60),
            InputKind::ForkOverride,
            Scope::Person(2),
            Producer::Player,
        ),
        (
            t(5000),
            InputKind::ConsoleMode(ConsoleMode::Cultivator),
            Scope::None,
            Producer::Player,
        ),
    ];
    kinds
        .into_iter()
        .enumerate()
        .map(|(i, (tick, kind, scope, producer))| NewInput {
            timeline_id: 0,
            tick,
            kind,
            scope,
            producer,
            payload: format!("payload-{i}").into_bytes(),
        })
        .collect()
}

/// A journal at `dir/journal.sqlite` holding [`inputs`], and the records the appends returned.
fn written(dir: &std::path::Path) -> (PathBuf, Vec<JournaledInput>) {
    let path = dir.join("journal.sqlite");
    let mut j = Journal::create(&path, &key()).unwrap();
    let records = inputs().into_iter().map(|i| j.append(i).unwrap()).collect();
    (path, records)
}

#[test]
fn the_round_trip_returns_every_record_in_application_order() {
    let dir = fresh_dir("round-trip");
    let (path, records) = written(&dir);
    for (k, r) in records.iter().enumerate() {
        assert_eq!(
            r.seq,
            u32::try_from(k).unwrap(),
            "seq is append order from 0"
        );
        assert_eq!(r.class, InputClass::Push);
        assert_eq!(r.payload_hash, xxh3_128(&r.payload));
    }
    let replay = Replay::open(&path, &key(), 0).unwrap();
    let mut expected = records.clone();
    expected.sort_by_key(|r| (r.tick, r.seq));
    assert_eq!(replay.inputs(), expected.as_slice());
    // Another timeline in the same file is independent and starts at seq 0.
    let mut j = Journal::open(&path, &key()).unwrap();
    let mut other = inputs().remove(0);
    other.timeline_id = 7;
    assert_eq!(j.append(other).unwrap().seq, 0);
    assert_eq!(Replay::open(&path, &key(), 7).unwrap().inputs().len(), 1);
    assert_eq!(
        Replay::open(&path, &key(), 0).unwrap().inputs(),
        expected.as_slice()
    );
    assert!(Replay::open(&path, &key(), 9).unwrap().inputs().is_empty());
}

#[test]
fn a_replay_reads_only_the_journal_file() {
    let dir = fresh_dir("only-journal");
    let (path, records) = written(&dir);
    // Closing the writer checkpoints the WAL; the journal is then one file. Copy it alone to an
    // empty directory and replay it there, read-only: nothing else is consulted.
    let copy_dir = fresh_dir("only-journal-copy");
    let copy = copy_dir.join("copied.sqlite");
    std::fs::copy(&path, &copy).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let mut replay = Replay::open(&copy, &key(), 0).unwrap();
    let mut applied = Vec::new();
    let mut ticks: Vec<SimTime> = records.iter().map(|r| r.tick).collect();
    ticks.sort();
    ticks.dedup();
    for t in ticks {
        applied.extend(replay.due(t).unwrap().iter().cloned());
    }
    replay.finish(SimTime::from_minutes(i64::MAX)).unwrap();
    assert_eq!(applied.len(), records.len());
    // The replay connection is read-only: it cannot write the journal it reads.
    let conn =
        Connection::open_with_flags(&copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(conn.execute("DELETE FROM inputs", []).is_err());
}

#[test]
fn the_writer_uses_wal_and_creates_a_journal_once() {
    let dir = fresh_dir("wal");
    let path = dir.join("j.sqlite");
    let _j = Journal::create(&path, &key()).unwrap();
    let conn = Connection::open(&path).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert!(matches!(
        Journal::create(&path, &key()),
        Err(JournalError::Schema(_))
    ));
}

#[test]
fn another_history_key_hard_fails() {
    let dir = fresh_dir("history");
    let (path, _) = written(&dir);
    let variants = [
        HistoryKey {
            engine_build: "other".to_owned(),
            ..key()
        },
        HistoryKey {
            registry_version: "0.1.1".to_owned(),
            ..key()
        },
        HistoryKey {
            manifest_hash: vec![0xAB; 15],
            ..key()
        },
        HistoryKey {
            world_seed: 1,
            ..key()
        },
    ];
    for k in variants {
        assert!(matches!(
            Replay::open(&path, &k, 0),
            Err(JournalError::HistoryMismatch { .. })
        ));
        assert!(matches!(
            Journal::open(&path, &k),
            Err(JournalError::HistoryMismatch { .. })
        ));
    }
}

/// Runs `sql` on the journal behind the journal's back (a corrupted or tampered file).
fn tamper(path: &std::path::Path, sql: &str) {
    Connection::open(path).unwrap().execute_batch(sql).unwrap();
}

#[test]
fn a_missing_input_hard_fails() {
    // A gap in seq.
    let dir = fresh_dir("missing-gap");
    let (path, _) = written(&dir);
    tamper(&path, "DELETE FROM inputs WHERE seq = 4");
    assert!(matches!(
        Replay::open(&path, &key(), 0),
        Err(JournalError::Missing { seq: 4, .. })
    ));
    // A truncated tail: fewer rows than the count.
    let dir = fresh_dir("missing-tail");
    let (path, _) = written(&dir);
    tamper(&path, "DELETE FROM inputs WHERE seq = 11");
    assert!(matches!(
        Replay::open(&path, &key(), 0),
        Err(JournalError::Missing { seq: 11, .. })
    ));
}

#[test]
fn an_extra_input_hard_fails() {
    let dir = fresh_dir("extra");
    let (path, _) = written(&dir);
    tamper(
        &path,
        "INSERT INTO inputs SELECT timeline_id, 12, tick, class, kind, kind_arg, scope, scope_id, \
         producer, job_id, model_id, weights_hash, provider_version, prompt_hash, payload, \
         payload_hash, row_digest, chain FROM inputs WHERE seq = 0",
    );
    assert!(matches!(
        Replay::open(&path, &key(), 0),
        Err(JournalError::Extra { seq: 12, .. })
    ));
}

#[test]
fn a_mismatched_input_hard_fails() {
    for (name, sql) in [
        ("payload", "UPDATE inputs SET payload = X'00' WHERE seq = 3"),
        (
            "hash",
            "UPDATE inputs SET payload_hash = zeroblob(16) WHERE seq = 3",
        ),
        (
            "kind",
            "UPDATE inputs SET kind = 'oracle_dance' WHERE seq = 3",
        ),
        ("scope", "UPDATE inputs SET scope_id = NULL WHERE seq = 3"),
        (
            "producer",
            "UPDATE inputs SET provider_version = NULL WHERE seq = 5",
        ),
    ] {
        let dir = fresh_dir(name);
        let (path, _) = written(&dir);
        tamper(&path, sql);
        let err = Replay::open(&path, &key(), 0).unwrap_err();
        assert!(
            matches!(err, JournalError::Mismatched { .. }),
            "{name}: {err}"
        );
    }
}

#[test]
fn the_replay_hard_fails_on_a_skipped_tick_an_early_end_or_a_backward_tick() {
    let dir = fresh_dir("due");
    let (path, _) = written(&dir);
    let open = || Replay::open(&path, &key(), 0).unwrap();
    // The first tick is -60 (the fork override); asking for tick 0 first skips it.
    let mut r = open();
    assert!(matches!(
        r.due(SimTime::from_minutes(0)),
        Err(JournalError::Missing { seq: 10, .. })
    ));
    // Ending before an input at or before the end was applied.
    let mut r = open();
    r.due(SimTime::from_minutes(-60)).unwrap();
    assert!(matches!(
        r.finish(SimTime::from_minutes(0)),
        Err(JournalError::Missing { .. })
    ));
    // Inputs scheduled after the end stay for later.
    let mut r = open();
    for t in [-60, 0, 1440, 2880, 4320, 5000] {
        assert!(!r.due(SimTime::from_minutes(t)).unwrap().is_empty());
    }
    r.finish(SimTime::from_minutes(89_999)).unwrap();
    // A tick going backwards.
    let mut r = open();
    r.due(SimTime::from_minutes(-60)).unwrap();
    r.due(SimTime::from_minutes(0)).unwrap();
    assert!(matches!(
        r.due(SimTime::from_minutes(-1)),
        Err(JournalError::Mismatched { .. })
    ));
}

#[test]
fn expect_compares_a_run_with_the_journal() {
    let dir = fresh_dir("expect");
    let (path, records) = written(&dir);
    let mut ordered = records.clone();
    ordered.sort_by_key(|r| (r.tick, r.seq));
    // The same inputs: accepted.
    let mut r = Replay::open(&path, &key(), 0).unwrap();
    for i in &ordered {
        r.expect(i).unwrap();
    }
    r.finish(SimTime::from_minutes(i64::MAX)).unwrap();
    // One more input than the journal has.
    let mut extra = ordered.last().unwrap().clone();
    extra.seq += 1;
    assert!(matches!(
        r_after(&path, &ordered).expect(&extra),
        Err(JournalError::Extra { .. })
    ));
    // The run lacks the journal's first input.
    let mut r = Replay::open(&path, &key(), 0).unwrap();
    assert!(matches!(
        r.expect(&ordered[1]),
        Err(JournalError::Missing { .. })
    ));
    // OBS-1: a run input that sorts before the journal's next one is extra, not mismatched.
    let mut early = ordered[0].clone();
    early.tick = SimTime::from_minutes(early.tick.minutes() - 1);
    let mut r = Replay::open(&path, &key(), 0).unwrap();
    assert!(matches!(r.expect(&early), Err(JournalError::Extra { .. })));
    // A different payload at the same place.
    let mut changed = ordered[0].clone();
    changed.payload.push(0);
    changed.payload_hash = xxh3_128(&changed.payload);
    let mut r = Replay::open(&path, &key(), 0).unwrap();
    assert!(matches!(
        r.expect(&changed),
        Err(JournalError::Mismatched { .. })
    ));
}

/// A replay that has already accepted every input of `ordered`.
fn r_after(path: &std::path::Path, ordered: &[JournaledInput]) -> Replay {
    let mut r = Replay::open(path, &key(), 0).unwrap();
    for i in ordered {
        r.expect(i).unwrap();
    }
    r
}

#[test]
fn a_rejected_input_is_never_journaled() {
    let dir = fresh_dir("rejected");
    let path = dir.join("j.sqlite");
    let mut j = Journal::create(&path, &key()).unwrap();
    let bad = NewInput {
        producer: Producer::LlmJob {
            job_id: "j".to_owned(),
            model_id: "m".to_owned(),
            weights_hash: None,
            provider_version: None,
            prompt_hash: vec![1],
        },
        ..inputs().remove(0)
    };
    assert!(matches!(j.append(bad), Err(JournalError::Rejected(_))));
    assert!(Replay::open(&path, &key(), 0).unwrap().inputs().is_empty());
    let ok = j.append(inputs().remove(0)).unwrap();
    assert_eq!(ok.seq, 0);
}

#[test]
fn sqlite_stores_no_real_values() {
    let dir = fresh_dir("no-real");
    let (path, _) = written(&dir);
    let conn = Connection::open(&path).unwrap();
    // STRICT tables refuse a REAL in an INTEGER or BLOB column...
    for sql in [
        "UPDATE inputs SET tick = 1.5 WHERE seq = 0",
        "UPDATE inputs SET payload = 2.5 WHERE seq = 0",
        "UPDATE history SET world_seed = 3.25",
        "UPDATE timelines SET input_count = 12.5",
    ] {
        assert!(conn.execute(sql, []).is_err(), "{sql}");
    }
    // ...and a TEXT column stores the number's text, never a REAL.
    conn.execute("UPDATE inputs SET kind_arg = 0.5 WHERE seq = 1", [])
        .unwrap();
    let ty: String = conn
        .query_row(
            "SELECT typeof(kind_arg) FROM inputs WHERE seq = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ty, "text");
    // Every declared column type is INTEGER, TEXT or BLOB, and every table is STRICT.
    let mut stmt = conn
        .prepare(
            "SELECT m.name, p.type FROM sqlite_schema m, pragma_table_info(m.name) p \
             WHERE m.type = 'table'",
        )
        .unwrap();
    let types: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!types.is_empty());
    for (table, ty) in &types {
        assert!(
            ["INTEGER", "TEXT", "BLOB"].contains(&ty.as_str()),
            "{table}: {ty}"
        );
    }
    let strict: i64 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_list WHERE schema = 'main' AND type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND strict = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(strict, 3);
}

/// VER-SPR-007-TEST GAP-3: every field edit the decoder accepts is caught by the row digest, and
/// a dropped last row is caught by the chain head even with the count lowered to match.
#[test]
fn the_row_digest_and_the_chain_catch_every_silent_tamper() {
    for (name, sql) in [
        ("tick", "UPDATE inputs SET tick = tick + 1 WHERE seq = 3"),
        ("scope", "UPDATE inputs SET scope_id = 4 WHERE seq = 3"),
        (
            "scope-kind",
            "UPDATE inputs SET scope = 'household' WHERE seq = 3",
        ),
        (
            "kind",
            "UPDATE inputs SET kind = 'oracle_ward' WHERE seq = 3",
        ),
        ("verb", "UPDATE inputs SET kind_arg = 'GIVE' WHERE seq = 8"),
        (
            "producer",
            "UPDATE inputs SET producer = 'system' WHERE seq = 3",
        ),
        (
            "model",
            "UPDATE inputs SET model_id = 'other' WHERE seq = 5",
        ),
        (
            "payload-and-hash",
            "UPDATE inputs SET payload = X'00', \
            payload_hash = X'a6cd5e9392000f6ac44bdff4074eecdb' WHERE seq = 3",
        ),
        (
            "digest",
            "UPDATE inputs SET row_digest = zeroblob(16) WHERE seq = 3",
        ),
        (
            "chain",
            "UPDATE inputs SET chain = zeroblob(16) WHERE seq = 3",
        ),
    ] {
        let dir = fresh_dir(name);
        let (path, _) = written(&dir);
        tamper(&path, sql);
        let err = Replay::open(&path, &key(), 0).unwrap_err();
        assert!(
            matches!(err, JournalError::Mismatched { .. }),
            "{name}: {err}"
        );
    }
    // The last row dropped together with the count: the head no longer matches.
    let dir = fresh_dir("drop-last");
    let (path, _) = written(&dir);
    tamper(
        &path,
        "DELETE FROM inputs WHERE seq = 11; UPDATE timelines SET input_count = 11",
    );
    let err = Replay::open(&path, &key(), 0).unwrap_err();
    assert!(
        matches!(err, JournalError::Missing { seq: 11, .. }),
        "{err}"
    );
    // The head itself altered.
    let dir = fresh_dir("head");
    let (path, _) = written(&dir);
    tamper(&path, "UPDATE timelines SET head_chain = zeroblob(16)");
    assert!(matches!(
        Replay::open(&path, &key(), 0),
        Err(JournalError::Missing { .. })
    ));
}

#[test]
fn the_journal_position_is_the_count_and_the_chain_head() {
    let dir = fresh_dir("position");
    let path = dir.join("j.sqlite");
    let mut j = Journal::create(&path, &key()).unwrap();
    let empty = j.position(0).unwrap();
    assert_eq!((empty.inputs, empty.head), (0, chain_genesis(0)));
    assert_ne!(chain_genesis(0), chain_genesis(1));
    let mut head = chain_genesis(0);
    let mut records = Vec::new();
    for input in inputs() {
        let r = j.append(input).unwrap();
        head = chain_next(&head, &row_digest(&r));
        records.push(r);
        let p = j.position(0).unwrap();
        assert_eq!(p.inputs as usize, records.len());
        assert_eq!(p.head, head);
    }
    drop(j);
    let replay = Replay::open(&path, &key(), 0).unwrap();
    assert_eq!(replay.position().head, head);
    assert_eq!(replay.position().inputs, 12);
    assert_eq!(replay.chain_after(0), Some(chain_genesis(0)));
    assert_eq!(replay.chain_after(12), Some(head));
    assert_eq!(replay.chain_after(13), None);
    // Every field enters the digest: changing any one changes it.
    let r = records[5].clone();
    let base = row_digest(&r);
    let mut t = r.clone();
    t.tick = SimTime::from_minutes(t.tick.minutes() + 1);
    assert_ne!(row_digest(&t), base);
    let mut t = r.clone();
    t.seq += 1;
    assert_ne!(row_digest(&t), base);
    let mut t = r.clone();
    t.timeline_id = 1;
    assert_ne!(row_digest(&t), base);
    let mut t = r.clone();
    t.payload_hash[0] ^= 1;
    assert_ne!(row_digest(&t), base);
    let mut t = r;
    t.scope = Scope::Household(u64::MAX);
    assert_ne!(row_digest(&t), base);
}
