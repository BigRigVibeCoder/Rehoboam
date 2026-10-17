//! The input journal: an append-only `SQLite` (WAL) file of push inputs, and its replay
//! (SPR-007 T-03).
//!
//! History is `f(engine_build, registry_version, manifest_hash, seed, input_journal)` (ADR-0006).
//! The journal holds the last argument and pins the first four: a journal file belongs to exactly
//! one [`HistoryKey`], written when it is created, and a replay under any other key fails.
//!
//! # Records (`05.9` IF-2)
//!
//! A [`JournaledInput`] is `timeline_id, tick, seq, class, kind, scope, producer, payload,
//! payload_hash`. Push is the only class (ADR-0034). The journal assigns:
//! - `seq`: the append order within the timeline, contiguous from 0. An input scheduled for a
//!   future tick is journaled when issued (`05.10` §2), so `seq` order is not tick order; inputs
//!   **apply** in `(tick, seq)` order.
//! - `payload_hash`: the XXH3-128 of the payload, canonical form ([`crate::hash::xxh3_128`]).
//!
//! The kind-specific arguments that IF-2 names (`ArgumentFrame`, `SpeechAct`, targets, specs) have
//! no Rust types yet; they travel in the payload, which the simulation validates before the input
//! is appended (IF-2 rules), and which the hash covers. [`InputKind`] keeps what the record itself
//! must show: the kind, the console mode and the verb id.
//!
//! # Storage
//!
//! Every table is `STRICT`, with only `INTEGER`, `TEXT` and `BLOB` columns, so `SQLite` can store no
//! `REAL` and its floating point never reaches plane A (ruling 3). `u64` values (the world seed,
//! entity ids) are stored as their two's-complement `i64`. Writes use WAL with `synchronous=FULL`.
//! Each append is one transaction that also advances the timeline's input count and its chain, so
//! a truncated, padded or edited journal is detected.
//!
//! # Integrity (VER-SPR-007-TEST GAP-3)
//!
//! Every row stores a **row digest**, `XXH3-128(D_ROW || fields)` over all its IF-2 fields
//! ([`row_digest`]), and a **chain** value. Per timeline, `chain(-1) = XXH3-128(D_CHAIN ||
//! u32(timeline_id))` and `chain(n) = XXH3-128(D_CHAIN || chain(n-1) || row_digest(n))` in seq order;
//! the timeline's row keeps the head, `chain(count-1)`. Replay recomputes both and hard-fails on any
//! difference, so an edited tick, scope or kind, or a last row dropped together with the count, no
//! longer replays silently. [`JournalPosition`] (count and head) is what a keyframe and a crash
//! artifact record as their journal position (T-02). The tags are `D_ROW =
//! "rehoboam/rh-snapshot/journal/v2/row"` and `D_CHAIN = ".../journal/v2/chain"`. This is
//! integrity against accident and naive edits, not a signature: a writer who recomputes every
//! digest can forge a journal. In particular, a tail truncated together with a forged count and head
//! (the head recomputed to the new last row) is consistent in itself; it is detected only against
//! an anchor kept elsewhere, the [`JournalPosition`] a keyframe records (T-02).
//!
//! # Replay
//!
//! [`Replay`] opens the file read-only and reads nothing else (`05.1` §6 item 8). It hard-fails,
//! with a [`JournalError`], on:
//! - a different history key;
//! - a **missing** input: a gap in `seq`, fewer rows than the count, or an input whose tick the
//!   replay passed without applying it;
//! - an **extra** input: a row past the count, or one the replay's [`expect`](Replay::expect)
//!   check did not ask for;
//! - a **mismatched** input: a payload whose hash differs, a record that does not decode, or an
//!   input that differs from the one [`expect`](Replay::expect) was given.
//!
//! There is no silent fallback.
//!
//! REF: `docs/05_architecture/05.9_interfaces_icd.md` IF-2; `05.1` §6 items 1 and 8; ADR-0006;
//! ADR-0034; `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-03, ruling 3.

use std::fmt;
use std::path::Path;

use rh_core::time::SimTime;
use rh_ids::{EntityKind, EntityRef};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};

use crate::hash::xxh3_128;

/// The journal schema's version, stored in the file.
pub const SCHEMA: &str = "rehoboam/rh-snapshot/journal/v2";

/// Domain tag of a row digest.
const D_ROW: &[u8] = b"rehoboam/rh-snapshot/journal/v2/row";
/// Domain tag of the chain (its genesis and every link).
const D_CHAIN: &[u8] = b"rehoboam/rh-snapshot/journal/v2/chain";

const DDL: &str = "
CREATE TABLE history (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    schema TEXT NOT NULL,
    engine_build TEXT NOT NULL,
    registry_version TEXT NOT NULL,
    manifest_hash BLOB NOT NULL,
    world_seed INTEGER NOT NULL
) STRICT;
CREATE TABLE timelines (
    timeline_id INTEGER PRIMARY KEY CHECK (timeline_id BETWEEN 0 AND 4294967295),
    input_count INTEGER NOT NULL CHECK (input_count >= 0),
    head_chain BLOB NOT NULL CHECK (length(head_chain) = 16)
) STRICT;
CREATE TABLE inputs (
    timeline_id INTEGER NOT NULL REFERENCES timelines (timeline_id),
    seq INTEGER NOT NULL CHECK (seq BETWEEN 0 AND 4294967295),
    tick INTEGER NOT NULL,
    class TEXT NOT NULL CHECK (class = 'push'),
    kind TEXT NOT NULL,
    kind_arg TEXT,
    scope TEXT NOT NULL,
    scope_id INTEGER,
    producer TEXT NOT NULL,
    job_id TEXT,
    model_id TEXT,
    weights_hash BLOB,
    provider_version TEXT,
    prompt_hash BLOB,
    payload BLOB NOT NULL,
    payload_hash BLOB NOT NULL CHECK (length(payload_hash) = 16),
    row_digest BLOB NOT NULL CHECK (length(row_digest) = 16),
    chain BLOB NOT NULL CHECK (length(chain) = 16),
    PRIMARY KEY (timeline_id, seq)
) STRICT;
";

/// The history tuple's fixed part: what a journal's inputs are replayed against (ADR-0006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryKey {
    /// The engine build.
    pub engine_build: String,
    /// The Registry version.
    pub registry_version: String,
    /// The manifest hash.
    pub manifest_hash: Vec<u8>,
    /// The world seed.
    pub world_seed: u64,
}

/// The input class. Push is the only one: pull inputs are retired (ADR-0034).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputClass {
    /// Initiated outside the simulation: player, Oracle, settings.
    Push,
}

/// The console mode of a `ConsoleMode` input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleMode {
    /// Cultivator mode.
    Cultivator,
    /// Oracle mode.
    Oracle,
}

/// The input kind (IF-2). Kind-specific arguments travel in the payload (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputKind {
    /// Sets the manifest.
    ManifestSet,
    /// Changes a setting.
    SettingChange,
    /// Switches the console mode.
    ConsoleMode(ConsoleMode),
    /// Moves the Oracle.
    OracleMove,
    /// An Oracle whisper (its argument frame is in the payload).
    OracleWhisper,
    /// An Oracle pierce (its target is in the payload).
    OraclePierce,
    /// An Oracle omen (its spec is in the payload).
    OracleOmen,
    /// An Oracle ward (its spec is in the payload).
    OracleWard,
    /// A player action with its Registry verb id (its arguments are in the payload).
    PlayerAction {
        /// The verb id.
        verb_id: String,
    },
    /// A speech act (the act is in the payload).
    SpeechAct,
    /// A what-if override at a fork's first tick (its spec is in the payload).
    ForkOverride,
}

impl InputKind {
    fn encode(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::ManifestSet => ("manifest_set", None),
            Self::SettingChange => ("setting_change", None),
            Self::ConsoleMode(ConsoleMode::Cultivator) => ("console_mode", Some("cultivator")),
            Self::ConsoleMode(ConsoleMode::Oracle) => ("console_mode", Some("oracle")),
            Self::OracleMove => ("oracle_move", None),
            Self::OracleWhisper => ("oracle_whisper", None),
            Self::OraclePierce => ("oracle_pierce", None),
            Self::OracleOmen => ("oracle_omen", None),
            Self::OracleWard => ("oracle_ward", None),
            Self::PlayerAction { verb_id } => ("player_action", Some(verb_id)),
            Self::SpeechAct => ("speech_act", None),
            Self::ForkOverride => ("fork_override", None),
        }
    }

    fn decode(kind: &str, arg: Option<String>) -> Option<Self> {
        let k = match (kind, arg.as_deref()) {
            ("manifest_set", None) => Self::ManifestSet,
            ("setting_change", None) => Self::SettingChange,
            ("console_mode", Some("cultivator")) => Self::ConsoleMode(ConsoleMode::Cultivator),
            ("console_mode", Some("oracle")) => Self::ConsoleMode(ConsoleMode::Oracle),
            ("oracle_move", None) => Self::OracleMove,
            ("oracle_whisper", None) => Self::OracleWhisper,
            ("oracle_pierce", None) => Self::OraclePierce,
            ("oracle_omen", None) => Self::OracleOmen,
            ("oracle_ward", None) => Self::OracleWard,
            ("player_action", Some(_)) => Self::PlayerAction {
                verb_id: arg.unwrap_or_default(),
            },
            ("speech_act", None) => Self::SpeechAct,
            ("fork_override", None) => Self::ForkOverride,
            _ => return None,
        };
        Some(k)
    }
}

/// The dependency scope of an input (`05.2` §3.2). Anything but `None` makes it material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Not material.
    None,
    /// A person.
    Person(u64),
    /// A household.
    Household(u64),
    /// A settlement.
    Settlement(u64),
    /// A region.
    Region(u64),
}

impl Scope {
    /// Whether the input is material (cancels and re-samples timers in its scope; ADR-0025).
    #[must_use]
    pub const fn is_material(self) -> bool {
        !matches!(self, Self::None)
    }

    /// The entity whose timers a material input in this scope cancels and re-samples (Architect
    /// ruling 3 on the T-02 plan): the [`EntityKind`] of the same name and the raw id. Not
    /// material, no entity. This is the one conversion between journal scopes and entities.
    #[must_use]
    pub const fn entity(self) -> Option<EntityRef> {
        let (kind, raw) = match self {
            Self::None => return None,
            Self::Person(id) => (EntityKind::Person, id),
            Self::Household(id) => (EntityKind::Household, id),
            Self::Settlement(id) => (EntityKind::Settlement, id),
            Self::Region(id) => (EntityKind::Region, id),
        };
        Some(EntityRef::new(kind, raw))
    }

    /// The scope of a material input on `entity`, the inverse of [`Scope::entity`]; the world as a
    /// whole is no scope (ADR-0025 scopes are a person, household, settlement or region).
    #[must_use]
    pub const fn of_entity(entity: EntityRef) -> Option<Self> {
        let raw = entity.raw();
        match entity.kind() {
            EntityKind::World => None,
            EntityKind::Person => Some(Self::Person(raw)),
            EntityKind::Household => Some(Self::Household(raw)),
            EntityKind::Settlement => Some(Self::Settlement(raw)),
            EntityKind::Region => Some(Self::Region(raw)),
        }
    }

    fn encode(self) -> (&'static str, Option<i64>) {
        match self {
            Self::None => ("none", None),
            Self::Person(id) => ("person", Some(id.cast_signed())),
            Self::Household(id) => ("household", Some(id.cast_signed())),
            Self::Settlement(id) => ("settlement", Some(id.cast_signed())),
            Self::Region(id) => ("region", Some(id.cast_signed())),
        }
    }

    fn decode(scope: &str, id: Option<i64>) -> Option<Self> {
        Some(match (scope, id.map(i64::cast_unsigned)) {
            ("none", None) => Self::None,
            ("person", Some(id)) => Self::Person(id),
            ("household", Some(id)) => Self::Household(id),
            ("settlement", Some(id)) => Self::Settlement(id),
            ("region", Some(id)) => Self::Region(id),
            _ => return None,
        })
    }
}

/// Who produced an input (IF-2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Producer {
    /// A player.
    Player,
    /// An LLM job: traceable to the gateway logs (ADR-0034). At least one of `weights_hash` and
    /// `provider_version` is present.
    LlmJob {
        /// The job id.
        job_id: String,
        /// The model id.
        model_id: String,
        /// The weights hash, for a local model.
        weights_hash: Option<Vec<u8>>,
        /// The provider version, for a hosted model.
        provider_version: Option<String>,
        /// The prompt hash.
        prompt_hash: Vec<u8>,
    },
    /// The system (settings, console).
    System,
}

/// One journaled input (`05.9` IF-2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournaledInput {
    /// The timeline.
    pub timeline_id: u32,
    /// The tick it applies at.
    pub tick: SimTime,
    /// Its append order within the timeline, from 0.
    pub seq: u32,
    /// Its class (push).
    pub class: InputClass,
    /// Its kind.
    pub kind: InputKind,
    /// Its dependency scope.
    pub scope: Scope,
    /// Its producer.
    pub producer: Producer,
    /// The validated payload.
    pub payload: Vec<u8>,
    /// XXH3-128 of the payload, canonical form.
    pub payload_hash: [u8; 16],
}

/// An input to append: the record without the fields the journal assigns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewInput {
    /// The timeline.
    pub timeline_id: u32,
    /// The tick it applies at.
    pub tick: SimTime,
    /// Its kind.
    pub kind: InputKind,
    /// Its dependency scope.
    pub scope: Scope,
    /// Its producer.
    pub producer: Producer,
    /// The validated payload.
    pub payload: Vec<u8>,
}

/// A journal or replay failure. Every replay failure is a hard failure (`05.1` §6 item 8).
#[derive(Debug)]
pub enum JournalError {
    /// `SQLite` failed.
    Sqlite(rusqlite::Error),
    /// The file is not a journal of this schema.
    Schema(String),
    /// The journal belongs to another history key.
    HistoryMismatch {
        /// The key in the file.
        journal: Box<HistoryKey>,
        /// The key the caller replays under.
        expected: Box<HistoryKey>,
    },
    /// An input that should be there is not.
    Missing {
        /// The timeline.
        timeline_id: u32,
        /// The missing input's seq.
        seq: u32,
        /// Why it counts as missing.
        reason: &'static str,
    },
    /// An input that should not be there is.
    Extra {
        /// The timeline.
        timeline_id: u32,
        /// The extra input's seq.
        seq: u32,
        /// Why it counts as extra.
        reason: &'static str,
    },
    /// An input differs from what it should be.
    Mismatched {
        /// The timeline.
        timeline_id: u32,
        /// The input's seq.
        seq: u32,
        /// What differs.
        reason: String,
    },
    /// An input was rejected before it was journaled (IF-2: rejected inputs are never journaled).
    Rejected(String),
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "journal: SQLite: {e}"),
            Self::Schema(m) => write!(f, "journal: not a {SCHEMA} journal: {m}"),
            Self::HistoryMismatch { journal, expected } => write!(
                f,
                "replay: the journal belongs to {journal:?}, not to {expected:?}"
            ),
            Self::Missing {
                timeline_id,
                seq,
                reason,
            } => write!(
                f,
                "replay: missing input (timeline {timeline_id}, seq {seq}): {reason}"
            ),
            Self::Extra {
                timeline_id,
                seq,
                reason,
            } => write!(
                f,
                "replay: extra input (timeline {timeline_id}, seq {seq}): {reason}"
            ),
            Self::Mismatched {
                timeline_id,
                seq,
                reason,
            } => write!(
                f,
                "replay: mismatched input (timeline {timeline_id}, seq {seq}): {reason}"
            ),
            Self::Rejected(m) => write!(f, "journal: input rejected: {m}"),
        }
    }
}

impl std::error::Error for JournalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for JournalError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

/// The journal, open for appending.
pub struct Journal {
    conn: Connection,
    key: HistoryKey,
}

impl Journal {
    /// Creates a new journal file for `key`. Fails if the file exists.
    ///
    /// # Errors
    /// [`JournalError::Schema`] if the file exists; [`JournalError::Sqlite`] on I/O failure.
    pub fn create(path: &Path, key: &HistoryKey) -> Result<Self, JournalError> {
        if path.exists() {
            return Err(JournalError::Schema(format!(
                "{} exists; a journal is created once",
                path.display()
            )));
        }
        let mut conn = Connection::open(path)?;
        set_write_pragmas(&conn)?;
        let tx = conn.transaction()?;
        tx.execute_batch(DDL)?;
        tx.execute(
            "INSERT INTO history (id, schema, engine_build, registry_version, manifest_hash, \
             world_seed) VALUES (0, ?1, ?2, ?3, ?4, ?5)",
            params![
                SCHEMA,
                key.engine_build,
                key.registry_version,
                key.manifest_hash,
                key.world_seed.cast_signed()
            ],
        )?;
        tx.commit()?;
        Ok(Self {
            conn,
            key: key.clone(),
        })
    }

    /// Opens an existing journal for appending, checking its history key.
    ///
    /// # Errors
    /// [`JournalError::HistoryMismatch`] for another key; [`JournalError::Schema`] if the file is
    /// not a journal; [`JournalError::Sqlite`] on I/O failure.
    pub fn open(path: &Path, key: &HistoryKey) -> Result<Self, JournalError> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        set_write_pragmas(&conn)?;
        check_key(&conn, key)?;
        Ok(Self {
            conn,
            key: key.clone(),
        })
    }

    /// The history key.
    #[must_use]
    pub fn key(&self) -> &HistoryKey {
        &self.key
    }

    /// The writer connection's `journal_mode` and `synchronous` (`"wal"` and 2, FULL, on every
    /// open journal; read back for tests and diagnostics).
    ///
    /// # Errors
    /// [`JournalError::Sqlite`] if `SQLite` cannot report them.
    pub fn durability(&self) -> Result<(String, i64), JournalError> {
        durability_of(&self.conn)
    }

    /// Appends an already validated input, assigning its `seq` and `payload_hash`, and returns
    /// the journaled record. The append and the timeline's count advance in one transaction.
    ///
    /// # Errors
    /// [`JournalError::Rejected`] for a record IF-2 does not allow (an LLM producer with neither
    /// a weights hash nor a provider version; a timeline past 2^32 inputs); nothing is journaled.
    /// [`JournalError::Sqlite`] on I/O failure.
    pub fn append(&mut self, input: NewInput) -> Result<JournaledInput, JournalError> {
        validate_producer(&input.producer)?;
        let tx = self.conn.transaction()?;
        let head: Option<(i64, Vec<u8>)> = tx
            .query_row(
                "SELECT input_count, head_chain FROM timelines WHERE timeline_id = ?1",
                [i64::from(input.timeline_id)],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (count, head) = match head {
            Some((count, head)) => (count, chain_bytes(&head, input.timeline_id, count)?),
            None => (0, chain_genesis(input.timeline_id)),
        };
        let seq = u32::try_from(count)
            .map_err(|_| JournalError::Rejected("the timeline has 2^32 inputs".to_owned()))?;
        let record = JournaledInput {
            timeline_id: input.timeline_id,
            tick: input.tick,
            seq,
            class: InputClass::Push,
            payload_hash: xxh3_128(&input.payload),
            kind: input.kind,
            scope: input.scope,
            producer: input.producer,
            payload: input.payload,
        };
        let digest = row_digest(&record);
        let chain = chain_next(&head, &digest);
        tx.execute(
            "INSERT INTO timelines (timeline_id, input_count, head_chain) VALUES (?1, ?2, ?3) \
             ON CONFLICT (timeline_id) DO UPDATE SET input_count = excluded.input_count, \
             head_chain = excluded.head_chain",
            params![i64::from(record.timeline_id), count + 1, chain.as_slice()],
        )?;
        insert(&tx, &record, &digest, &chain)?;
        tx.commit()?;
        Ok(record)
    }
}

impl Journal {
    /// The timeline's journal position: its input count and chain head.
    ///
    /// # Errors
    /// [`JournalError::Sqlite`] on I/O failure; [`JournalError::Mismatched`] for a malformed head.
    pub fn position(&self, timeline_id: u32) -> Result<JournalPosition, JournalError> {
        position_of(&self.conn, timeline_id)
    }
}

fn position_of(conn: &Connection, timeline_id: u32) -> Result<JournalPosition, JournalError> {
    let row: Option<(i64, Vec<u8>)> = conn
        .query_row(
            "SELECT input_count, head_chain FROM timelines WHERE timeline_id = ?1",
            [i64::from(timeline_id)],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (count, head) = match row {
        Some((count, head)) => (count, chain_bytes(&head, timeline_id, count)?),
        None => (0, chain_genesis(timeline_id)),
    };
    Ok(JournalPosition {
        timeline_id,
        inputs: u32::try_from(count).map_err(|_| JournalError::Mismatched {
            timeline_id,
            seq: u32::MAX,
            reason: format!("input count {count}"),
        })?,
        head,
    })
}

fn set_write_pragmas(conn: &Connection) -> Result<(), JournalError> {
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if mode != "wal" {
        return Err(JournalError::Schema(format!(
            "WAL unavailable (got {mode})"
        )));
    }
    conn.execute_batch("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")?;
    let (_, synchronous) = durability_of(conn)?;
    if synchronous != SYNCHRONOUS_FULL {
        return Err(JournalError::Schema(format!(
            "synchronous=FULL did not take (got {synchronous})"
        )));
    }
    Ok(())
}

/// `SQLite`'s `PRAGMA synchronous` value for FULL.
const SYNCHRONOUS_FULL: i64 = 2;

fn durability_of(conn: &Connection) -> Result<(String, i64), JournalError> {
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
    let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
    Ok((mode, synchronous))
}

fn validate_producer(p: &Producer) -> Result<(), JournalError> {
    if let Producer::LlmJob {
        weights_hash: None,
        provider_version: None,
        ..
    } = p
    {
        return Err(JournalError::Rejected(
            "an LLM-produced input carries a weights hash or a provider version (IF-2)".to_owned(),
        ));
    }
    Ok(())
}

/// The record's fields as the journal stores them: kind and argument, scope and id, producer.
type Encoded<'a> = (
    (&'static str, Option<&'a str>),
    (&'static str, Option<i64>),
    (
        &'static str,
        Option<&'a str>,
        Option<&'a str>,
        Option<&'a [u8]>,
        Option<&'a str>,
        Option<&'a [u8]>,
    ),
);

fn encode(r: &JournaledInput) -> Encoded<'_> {
    let producer = match &r.producer {
        Producer::Player => ("player", None, None, None, None, None),
        Producer::System => ("system", None, None, None, None, None),
        Producer::LlmJob {
            job_id,
            model_id,
            weights_hash,
            provider_version,
            prompt_hash,
        } => (
            "llm_job",
            Some(job_id.as_str()),
            Some(model_id.as_str()),
            weights_hash.as_deref(),
            provider_version.as_deref(),
            Some(prompt_hash.as_slice()),
        ),
    };
    (r.kind.encode(), r.scope.encode(), producer)
}

/// How the World Stream's `INPUT` record spells a journal row (`specs/stream/channel_a_v0.sql`
/// `input`; SPR-013 T-09): the kind as the journal stores it, with its argument after a `:`
/// (`player_action:<verb>`), the scope name and id, and the producer name (with the job id after a
/// `:` for an LLM job). The digest that ties the record to the row is [`row_digest`].
#[must_use]
pub fn stream_fields(r: &JournaledInput) -> (String, String, Option<u64>, String) {
    let ((kind, arg), (scope, id), producer) = encode(r);
    let kind = arg.map_or_else(|| kind.to_owned(), |a| format!("{kind}:{a}"));
    let producer = match producer {
        (name, Some(job), ..) => format!("{name}:{job}"),
        (name, ..) => name.to_owned(),
    };
    (kind, scope.to_owned(), id.map(i64::cast_unsigned), producer)
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    let len = u32::try_from(b.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(b);
}

fn put_opt(out: &mut Vec<u8>, b: Option<&[u8]>) {
    match b {
        None => out.push(0),
        Some(b) => {
            out.push(1);
            put_bytes(out, b);
        }
    }
}

/// A row's digest: `XXH3-128(D_ROW || ...)` over every IF-2 field in record order, little-endian
/// integers, `u32` length-prefixed strings and blobs, and `u8` 0 or 1 before each optional field:
/// `u32 timeline_id, i64 tick, u32 seq, class, kind, kind_arg?, scope, scope_id? (i64), producer,
/// job_id?, model_id?, weights_hash?, provider_version?, prompt_hash?, payload_hash` (the payload
/// enters through its hash).
#[must_use]
pub fn row_digest(r: &JournaledInput) -> [u8; 16] {
    let ((kind, kind_arg), (scope, scope_id), producer) = encode(r);
    let (producer, job_id, model_id, weights_hash, provider_version, prompt_hash) = producer;
    let mut b = Vec::with_capacity(160);
    b.extend_from_slice(D_ROW);
    b.extend_from_slice(&r.timeline_id.to_le_bytes());
    b.extend_from_slice(&r.tick.minutes().to_le_bytes());
    b.extend_from_slice(&r.seq.to_le_bytes());
    put_bytes(&mut b, b"push");
    put_bytes(&mut b, kind.as_bytes());
    put_opt(&mut b, kind_arg.map(str::as_bytes));
    put_bytes(&mut b, scope.as_bytes());
    let id = scope_id.map(i64::to_le_bytes);
    put_opt(&mut b, id.as_ref().map(<[u8; 8]>::as_slice));
    put_bytes(&mut b, producer.as_bytes());
    put_opt(&mut b, job_id.map(str::as_bytes));
    put_opt(&mut b, model_id.map(str::as_bytes));
    put_opt(&mut b, weights_hash);
    put_opt(&mut b, provider_version.map(str::as_bytes));
    put_opt(&mut b, prompt_hash);
    b.extend_from_slice(&r.payload_hash);
    xxh3_128(&b)
}

/// The chain value before a timeline's first input.
#[must_use]
pub fn chain_genesis(timeline_id: u32) -> [u8; 16] {
    let mut b = D_CHAIN.to_vec();
    b.extend_from_slice(&timeline_id.to_le_bytes());
    xxh3_128(&b)
}

/// The next chain value: `XXH3-128(D_CHAIN || previous || row digest)`.
#[must_use]
pub fn chain_next(previous: &[u8; 16], digest: &[u8; 16]) -> [u8; 16] {
    let mut b = D_CHAIN.to_vec();
    b.extend_from_slice(previous);
    b.extend_from_slice(digest);
    xxh3_128(&b)
}

fn chain_bytes(b: &[u8], timeline_id: u32, at: i64) -> Result<[u8; 16], JournalError> {
    b.try_into().map_err(|_| JournalError::Mismatched {
        timeline_id,
        seq: u32::try_from(at).unwrap_or(u32::MAX),
        reason: "a chain value is not 16 bytes".to_owned(),
    })
}

/// Where a timeline's journal stands: its input count and its chain head (`chain(count - 1)`, or
/// the genesis value for an empty timeline). A keyframe and a crash artifact record it (T-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalPosition {
    /// The timeline.
    pub timeline_id: u32,
    /// The number of inputs journaled.
    pub inputs: u32,
    /// The chain head after them.
    pub head: [u8; 16],
}

fn insert(
    tx: &Transaction<'_>,
    r: &JournaledInput,
    digest: &[u8; 16],
    chain: &[u8; 16],
) -> Result<(), JournalError> {
    let ((kind, kind_arg), (scope, scope_id), producer) = encode(r);
    let (producer, job_id, model_id, weights_hash, provider_version, prompt_hash) = producer;
    tx.execute(
        "INSERT INTO inputs (timeline_id, seq, tick, class, kind, kind_arg, scope, scope_id, \
         producer, job_id, model_id, weights_hash, provider_version, prompt_hash, payload, \
         payload_hash, row_digest, chain) VALUES (?1, ?2, ?3, 'push', ?4, ?5, ?6, ?7, ?8, ?9, \
         ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            i64::from(r.timeline_id),
            i64::from(r.seq),
            r.tick.minutes(),
            kind,
            kind_arg,
            scope,
            scope_id,
            producer,
            job_id,
            model_id,
            weights_hash,
            provider_version,
            prompt_hash,
            r.payload,
            r.payload_hash.as_slice(),
            digest.as_slice(),
            chain.as_slice(),
        ],
    )?;
    Ok(())
}

fn read_key(conn: &Connection) -> Result<HistoryKey, JournalError> {
    let row = conn
        .query_row(
            "SELECT schema, engine_build, registry_version, manifest_hash, world_seed \
             FROM history WHERE id = 0",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    HistoryKey {
                        engine_build: r.get(1)?,
                        registry_version: r.get(2)?,
                        manifest_hash: r.get(3)?,
                        world_seed: r.get::<_, i64>(4)?.cast_unsigned(),
                    },
                ))
            },
        )
        .map_err(|e| JournalError::Schema(format!("no history row: {e}")))?;
    if row.0 != SCHEMA {
        return Err(JournalError::Schema(format!("schema {:?}", row.0)));
    }
    Ok(row.1)
}

fn check_key(conn: &Connection, expected: &HistoryKey) -> Result<(), JournalError> {
    let journal = read_key(conn)?;
    if &journal != expected {
        return Err(JournalError::HistoryMismatch {
            journal: Box::new(journal),
            expected: Box::new(expected.clone()),
        });
    }
    Ok(())
}

/// The raw columns of one `inputs` row.
type RawRow = (
    i64,
    i64,
    String,
    String,
    Option<String>,
    String,
    Option<i64>,
    (String, Option<String>, Option<String>),
    (Option<Vec<u8>>, Option<String>, Option<Vec<u8>>),
    Vec<u8>,
    Vec<u8>,
);

fn decode(timeline_id: u32, raw: RawRow) -> Result<JournaledInput, JournalError> {
    let (seq, tick, class, kind, kind_arg, scope, scope_id, p, h, payload, payload_hash) = raw;
    let seq = u32::try_from(seq).map_err(|_| JournalError::Schema(format!("seq {seq}")))?;
    let bad = |reason: String| JournalError::Mismatched {
        timeline_id,
        seq,
        reason,
    };
    if class != "push" {
        return Err(bad(format!("class {class:?}")));
    }
    let kind = InputKind::decode(&kind, kind_arg).ok_or_else(|| bad(format!("kind {kind:?}")))?;
    let scope = Scope::decode(&scope, scope_id).ok_or_else(|| bad(format!("scope {scope:?}")))?;
    let (producer, job_id, model_id) = p;
    let (weights_hash, provider_version, prompt_hash) = h;
    let producer = match (producer.as_str(), job_id, model_id, prompt_hash) {
        ("player", None, None, None) => Producer::Player,
        ("system", None, None, None) => Producer::System,
        ("llm_job", Some(job_id), Some(model_id), Some(prompt_hash)) => Producer::LlmJob {
            job_id,
            model_id,
            weights_hash,
            provider_version,
            prompt_hash,
        },
        _ => return Err(bad(format!("producer {producer:?}"))),
    };
    validate_producer(&producer).map_err(|e| bad(e.to_string()))?;
    let payload_hash: [u8; 16] = payload_hash
        .as_slice()
        .try_into()
        .map_err(|_| bad("payload_hash is not 16 bytes".to_owned()))?;
    if xxh3_128(&payload) != payload_hash {
        return Err(bad("the payload does not match its payload_hash".to_owned()));
    }
    Ok(JournaledInput {
        timeline_id,
        tick: SimTime::from_minutes(tick),
        seq,
        class: InputClass::Push,
        kind,
        scope,
        producer,
        payload,
        payload_hash,
    })
}

/// A replay of one timeline: the journal's inputs in application order, `(tick, seq)`.
///
/// It reads only the journal file, opened read-only. Ask it for each tick's inputs with
/// [`due`](Self::due), in non-decreasing tick order, and close it with
/// [`finish`](Self::finish); every way the journal can disagree with the run is a hard error.
#[derive(Debug)]
pub struct Replay {
    timeline_id: u32,
    inputs: Vec<JournaledInput>,
    next: usize,
    last_tick: Option<SimTime>,
    /// `chains[n]` is the chain value after the first `n` inputs in seq order (`chains[0]` is the
    /// genesis value).
    chains: Vec<[u8; 16]>,
    position: JournalPosition,
}

/// Recomputes every row digest and the chain of `inputs` (seq order) against the stored values and
/// the timeline's head; returns the chain values from the genesis on.
fn verify_chain(
    conn: &Connection,
    timeline_id: u32,
    inputs: &[JournaledInput],
    position: &JournalPosition,
) -> Result<Vec<[u8; 16]>, JournalError> {
    let mut stmt =
        conn.prepare("SELECT row_digest, chain FROM inputs WHERE timeline_id = ?1 ORDER BY seq")?;
    let stored = stmt
        .query_map([i64::from(timeline_id)], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut chains = vec![chain_genesis(timeline_id)];
    for (input, (digest, chain)) in inputs.iter().zip(&stored) {
        let bad = |reason: &str| JournalError::Mismatched {
            timeline_id,
            seq: input.seq,
            reason: reason.to_owned(),
        };
        let expected = row_digest(input);
        if digest.as_slice() != expected {
            return Err(bad("the row digest does not match the row's fields"));
        }
        let previous = chains.last().copied().unwrap_or_default();
        let next = chain_next(&previous, &expected);
        if chain.as_slice() != next {
            return Err(bad("the chain breaks at this row"));
        }
        chains.push(next);
    }
    if chains.last() != Some(&position.head) {
        return Err(JournalError::Missing {
            timeline_id,
            seq: position.inputs,
            reason: "the timeline's head chain does not match its last row: inputs were removed \
                     or the head was altered",
        });
    }
    Ok(chains)
}

impl Replay {
    /// The journal position the replay was opened at (the timeline's count and chain head).
    #[must_use]
    pub fn position(&self) -> JournalPosition {
        self.position
    }

    /// The chain value after the first `n` inputs in seq order, `None` past the end: the journal
    /// position a keyframe taken after `n` journaled inputs records.
    #[must_use]
    pub fn chain_after(&self, n: usize) -> Option<[u8; 16]> {
        self.chains.get(n).copied()
    }

    /// Checks a keyframe's journal anchor against this journal (VER-SPR-007-TEST GAP-7): the
    /// journal holds at least `anchor.inputs` inputs of the anchor's timeline, and its chain after
    /// that many is `anchor.head`. Call it wherever a keyframe is loaded together with a journal,
    /// before replaying the journal from the keyframe. It catches a journal whose tail was
    /// truncated together with a forged count and head (which [`Replay::open`] alone cannot: the
    /// file is self-consistent), and one re-grown with other inputs after such a truncation.
    ///
    /// # Errors
    /// [`JournalError::Mismatched`] for another timeline or another chain at the anchor;
    /// [`JournalError::Missing`] if the journal is shorter than the anchor.
    pub fn check_anchor(&self, anchor: &JournalPosition) -> Result<(), JournalError> {
        if anchor.timeline_id != self.timeline_id {
            return Err(JournalError::Mismatched {
                timeline_id: self.timeline_id,
                seq: anchor.inputs,
                reason: format!("the anchor is timeline {}'s", anchor.timeline_id),
            });
        }
        let at = usize::try_from(anchor.inputs).unwrap_or(usize::MAX);
        match self.chain_after(at) {
            None => Err(JournalError::Missing {
                timeline_id: self.timeline_id,
                seq: self.position.inputs,
                reason: "the journal ends before the keyframe's anchor: inputs were removed",
            }),
            Some(head) if head != anchor.head => Err(JournalError::Mismatched {
                timeline_id: self.timeline_id,
                seq: anchor.inputs,
                reason: "the journal's chain at the keyframe's anchor differs: the inputs before \
                         the keyframe are not the ones it was taken after"
                    .to_owned(),
            }),
            Some(_) => Ok(()),
        }
    }

    /// Opens `path` read-only and loads `timeline_id`'s inputs, checking the history key, the
    /// input count, the `seq` sequence, every payload hash, every row digest and the chain up to
    /// the timeline's head.
    ///
    /// # Errors
    /// [`JournalError::HistoryMismatch`], [`JournalError::Missing`], [`JournalError::Extra`],
    /// [`JournalError::Mismatched`], [`JournalError::Schema`] or [`JournalError::Sqlite`].
    pub fn open(path: &Path, key: &HistoryKey, timeline_id: u32) -> Result<Self, JournalError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        check_key(&conn, key)?;
        let position = position_of(&conn, timeline_id)?;
        let count = i64::from(position.inputs);
        let mut stmt = conn.prepare(
            "SELECT seq, tick, class, kind, kind_arg, scope, scope_id, producer, job_id, \
             model_id, weights_hash, provider_version, prompt_hash, payload, payload_hash \
             FROM inputs WHERE timeline_id = ?1 ORDER BY seq",
        )?;
        let rows = stmt.query_map([i64::from(timeline_id)], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                (r.get(7)?, r.get(8)?, r.get(9)?),
                (r.get(10)?, r.get(11)?, r.get(12)?),
                r.get(13)?,
                r.get(14)?,
            ))
        })?;
        let mut inputs = Vec::new();
        for raw in rows {
            let raw: RawRow = raw?;
            let expected = i64::try_from(inputs.len()).unwrap_or(i64::MAX);
            let seq = raw.0;
            if seq >= count {
                return Err(JournalError::Extra {
                    timeline_id,
                    seq: u32::try_from(seq).unwrap_or(u32::MAX),
                    reason: "past the timeline's input count",
                });
            }
            if seq != expected {
                return Err(JournalError::Missing {
                    timeline_id,
                    seq: u32::try_from(expected).unwrap_or(u32::MAX),
                    reason: "a gap in seq",
                });
            }
            inputs.push(decode(timeline_id, raw)?);
        }
        if i64::try_from(inputs.len()).unwrap_or(i64::MAX) < count {
            return Err(JournalError::Missing {
                timeline_id,
                seq: u32::try_from(inputs.len()).unwrap_or(u32::MAX),
                reason: "fewer rows than the timeline's input count",
            });
        }
        let chains = verify_chain(&conn, timeline_id, &inputs, &position)?;
        inputs.sort_by_key(|i| (i.tick, i.seq));
        Ok(Self {
            timeline_id,
            inputs,
            next: 0,
            last_tick: None,
            chains,
            position,
        })
    }

    /// Every input of the timeline, in application order.
    #[must_use]
    pub fn inputs(&self) -> &[JournaledInput] {
        &self.inputs
    }

    /// The inputs that apply at `tick`, in seq order, marked applied.
    ///
    /// # Errors
    /// [`JournalError::Missing`] if an input of an earlier tick was never applied (the replay
    /// passed its tick); [`JournalError::Mismatched`] if `tick` goes backwards.
    pub fn due(&mut self, tick: SimTime) -> Result<&[JournaledInput], JournalError> {
        if let Some(last) = self.last_tick
            && tick < last
        {
            return Err(JournalError::Mismatched {
                timeline_id: self.timeline_id,
                seq: self.inputs.get(self.next).map_or(u32::MAX, |i| i.seq),
                reason: format!("replay asked for tick {tick:?} after {last:?}"),
            });
        }
        self.last_tick = Some(tick);
        if let Some(skipped) = self.inputs.get(self.next).filter(|i| i.tick < tick) {
            return Err(JournalError::Missing {
                timeline_id: self.timeline_id,
                seq: skipped.seq,
                reason: "its tick passed without it being applied",
            });
        }
        let start = self.next;
        while self.inputs.get(self.next).is_some_and(|i| i.tick == tick) {
            self.next += 1;
        }
        Ok(&self.inputs[start..self.next])
    }

    /// Checks that the next input in application order is exactly `live`, and marks it applied:
    /// for a run that produces its inputs again (a re-recorded session) and must agree with the
    /// journal.
    ///
    /// # Errors
    /// [`JournalError::Missing`] if the journal's next input sorts before `live` (one the run
    /// lacks); [`JournalError::Extra`] if `live` sorts before the journal's next input, or the
    /// journal has no more (one the journal lacks); [`JournalError::Mismatched`] if the record at
    /// the same `(tick, seq)` differs.
    pub fn expect(&mut self, live: &JournaledInput) -> Result<(), JournalError> {
        let Some(next) = self.inputs.get(self.next) else {
            return Err(JournalError::Extra {
                timeline_id: self.timeline_id,
                seq: live.seq,
                reason: "the run has an input the journal does not",
            });
        };
        if (next.tick, next.seq) < (live.tick, live.seq) {
            return Err(JournalError::Missing {
                timeline_id: self.timeline_id,
                seq: next.seq,
                reason: "the journal has an input the run does not",
            });
        }
        if (live.tick, live.seq) < (next.tick, next.seq) {
            return Err(JournalError::Extra {
                timeline_id: self.timeline_id,
                seq: live.seq,
                reason: "the run has an input, before the journal's next, that the journal does not",
            });
        }
        if next != live {
            return Err(JournalError::Mismatched {
                timeline_id: self.timeline_id,
                seq: next.seq,
                reason: format!("journal {next:?}, run {live:?}"),
            });
        }
        self.next += 1;
        self.last_tick = Some(live.tick);
        Ok(())
    }

    /// Ends a replay that ran through `end`: every input at or before `end` must have been
    /// applied. Inputs scheduled after `end` remain for a later replay.
    ///
    /// # Errors
    /// [`JournalError::Missing`] for the first input at or before `end` that was not applied.
    pub fn finish(self, end: SimTime) -> Result<(), JournalError> {
        match self.inputs.get(self.next) {
            Some(i) if i.tick <= end => Err(JournalError::Missing {
                timeline_id: self.timeline_id,
                seq: i.seq,
                reason: "the replay ended without applying it",
            }),
            _ => Ok(()),
        }
    }
}
