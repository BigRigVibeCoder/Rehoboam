-- World Stream v0, channel A: the sealed-segment schema (DuckDB dialect; the files are Parquet with the same columns).
-- Spec: SPR-011 rulings 11, 12, 13 and 17; 05.10 §4, §5, §7; 05.9 IF-0; 05.2 §1, §4; ADR-0007, ADR-0024, ADR-0025, ADR-0030.
-- Self-test: python -I specs/stream/selftest_channel_a.py (builds these tables, checks the rules below, writes and reads a
-- synthetic run's segments).
--
-- Status: v0, for P1.0 (SPR-013, T-09). A C2 contract: a change to a column, a code or the order is a new format version
-- (`segment.format`), announced like a Registry change.
--
-- ============================================================================================ rules
-- R1  Plane A only. Channel A carries authoritative records; plane B never enters the stream; channel C (sift matches)
--     is a separate directory tree and never interleaves with these records (05.10 §4-§5).
-- R2  No float column, anywhere, including list elements (FLOAT, REAL, DOUBLE): every quantity is an integer, a
--     fixed-point integer, a hash or an identifier. The self-test refuses a float column in this file.
-- R3  Record header. Every record row starts with (timeline_id UINTEGER, seq UBIGINT, tick BIGINT, kind USMALLINT):
--       timeline_id  0 = canonical; forks get their own ids and their own streams (05.10 §5);
--       seq          the record's position in the timeline's channel A, contiguous from 0 across all its segments;
--       tick         the instant (SimTime, i64 minutes) the record belongs to; never earlier than any record before it;
--       kind         the record kind code (record_kind below), constant per table.
--     The channel is the segment's (the directory and `segment.channel`), not repeated in rows. Child rows (event_role,
--     causal_edge) are not records: they hang off their EVENT by (timeline_id, event_id) and carry no seq.
-- R4  Order within an instant (SPR-011 ruling 13). The records of one tick are written in this order, and seq follows it:
--       1. INPUT        in journal seq order (05.1 §6 item 6; the journal's acceptance order);
--       2. EVENT        in EventId order (the timeline's event counter; EventId order is emission order, ADR-0025);
--       3. LIFE_ENTRY and LIFE_EXIT, by person_id, an entry before an exit of the same person;
--       4. STEP_DIGEST  (at most one per tick);
--       5. KEYFRAME     (at most one per tick).
--     Across ticks, records are in tick order. The order key is therefore (tick, class, k) with class INPUT 0, EVENT 1,
--     LIFE 2, STEP_DIGEST 3, KEYFRAME 4, and k = journal_seq, event_id, (person_id, 0 entry | 1 exit), 0, 0.
--     Genesis writes the founders' LIFE_ENTRY records at tick 0 in PersonId order, before any later record of tick 0
--     of class > LIFE (05.1 §7; ruling 13).
-- R5  Segments. One directory per (timeline, channel, sim-year), relative to the run directory:
--       stream/t<timeline_id, 10 digits>/A/y<sim_year, 6 digits>/<table>.parquet
--     e.g. stream/t0000000000/A/y000012/event.parquet. sim_year = floor(tick / minutes_per_year) of the world calendar
--     (518,400 or 525,600; 05.1 §4.1); channel A has no record before tick 0, so sim_year >= 0. A sealed segment holds one
--     file per table below, every one of them, empty or not, plus the dictionaries (verb, act_class, record_kind) and
--     its `segment` row. Sealing is atomic: each file is written to a temporary name, fsynced, renamed; the directory
--     is complete when segment.parquet exists, which is written last (SPR-011 ruling 13; T-09).
-- R6  Self-describing. Every segment carries the Registry version it was written under and the verb, act-class and
--     record-kind dictionaries in force, so a segment stays readable after the Registry grows: verb_id is the verb's
--     index in the Registry's [verbs] sorted by name (ruling 11), act_classes bit i is the Registry's [act_classes].ids[i].
-- R7  Byte identity (V-STR-01). For the same build, Registry, manifest, seed and journal, every file of every segment is
--     byte-identical, at any pace and thread count; `segment.canonical_digest` (the digest of the records' canonical
--     text form in seq order, T-09) survives a Parquet library upgrade.

-- ============================================================================================ dictionaries
CREATE TABLE IF NOT EXISTS record_kind (          -- the kind codes of R3; the same in every segment of format v0
    kind  USMALLINT NOT NULL PRIMARY KEY,
    name  VARCHAR   NOT NULL UNIQUE,
    class UTINYINT  NOT NULL                       -- the R4 class rank
);
INSERT OR IGNORE INTO record_kind VALUES
    (1, 'INPUT', 0),
    (2, 'EVENT', 1),
    (3, 'LIFE_ENTRY', 2),
    (4, 'LIFE_EXIT', 2),
    (5, 'STEP_DIGEST', 3),
    (6, 'KEYFRAME', 4);
-- Reserved for later formats (05.9 IF-0): DECISION_TRACE, ENCOUNTER. 0 is never a kind.

CREATE TABLE IF NOT EXISTS verb (                 -- the Registry's [verbs] at sealing (R6)
    verb_id     USMALLINT NOT NULL PRIMARY KEY,    -- index in [verbs] sorted by name
    name        VARCHAR   NOT NULL UNIQUE,
    act_classes UBIGINT   NOT NULL,                -- bitset over act_class.act_class_id
    tellability UTINYINT  NOT NULL,                -- Registry prior 0..255 (ruling 11)
    emitted_by  VARCHAR   NOT NULL,
    aggregate   BOOLEAN   NOT NULL
);

CREATE TABLE IF NOT EXISTS act_class (            -- the Registry's [act_classes].ids at sealing, in list order (R6)
    act_class_id UTINYINT NOT NULL PRIMARY KEY,    -- bit position in act_classes bitsets (< 64)
    name         VARCHAR  NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS segment (              -- one row per sealed segment; written last (R5)
    timeline_id      UINTEGER NOT NULL,
    channel          VARCHAR  NOT NULL CHECK (channel = 'A'),
    sim_year         INTEGER  NOT NULL CHECK (sim_year >= 0),
    format           VARCHAR  NOT NULL CHECK (format = 'rehoboam/stream/channel-a/v0'),
    first_seq        UBIGINT  NOT NULL,
    last_seq         UBIGINT  NOT NULL,            -- first_seq + record_count - 1
    record_count     UBIGINT  NOT NULL CHECK (record_count > 0),
    first_tick       BIGINT   NOT NULL,
    last_tick        BIGINT   NOT NULL,
    engine_build     VARCHAR  NOT NULL,            -- the history tuple (05.1 §6 item 1) ...
    registry_version VARCHAR  NOT NULL,
    manifest_hash    BLOB     NOT NULL,
    world_seed       UBIGINT  NOT NULL,            -- ... with the journal, which the INPUT records mirror
    built_systems    VARCHAR[] NOT NULL,           -- SPR-011 ruling 3: the systems this build ran ...
    not_built        VARCHAR[] NOT NULL,           -- ... and the manifest's systems it skipped
    canonical_digest BLOB     NOT NULL,            -- R7; its algorithm and width are T-09's (SPR-011 ruling 13)
    PRIMARY KEY (timeline_id, channel, sim_year)
);

-- ============================================================================================ records
CREATE TABLE IF NOT EXISTS input (                -- kind 1 INPUT: a journaled push input, at its applied tick (05.10 §4)
    timeline_id  UINTEGER NOT NULL,
    seq          UBIGINT  NOT NULL,
    tick         BIGINT   NOT NULL,
    kind         USMALLINT NOT NULL CHECK (kind = 1),
    journal_seq  UBIGINT  NOT NULL,                -- the journal row (rh-snapshot journal.rs `inputs`, SPR-007)
    class        UTINYINT NOT NULL CHECK (class = 0),   -- 0 push; the only class (ADR-0034)
    input_kind   VARCHAR  NOT NULL,                -- the journal row's kind
    scope        VARCHAR  NOT NULL,                -- the journal row's scope and scope id: one entity (05.1 §6 item 6)
    scope_id     UBIGINT,
    producer     VARCHAR  NOT NULL,
    payload_hash BLOB     NOT NULL,                -- the journal row's payload hash (16 bytes)
    row_digest   BLOB     NOT NULL,                -- the journal row's digest (16 bytes): links the stream to the journal
    PRIMARY KEY (timeline_id, seq)
);

CREATE TABLE IF NOT EXISTS event (                -- kind 2 EVENT (schema.sql event, plus the header and the payload)
    timeline_id  UINTEGER  NOT NULL,
    seq          UBIGINT   NOT NULL,
    tick         BIGINT    NOT NULL,               -- the emission tick
    kind         USMALLINT NOT NULL CHECK (kind = 2),
    event_id     UBIGINT   NOT NULL,               -- the timeline's event counter (SPR-006; ruling 12), not (day << 24) | seq
    phase        UTINYINT  NOT NULL,               -- 05.1 §4.2 phase index of the emission
    verb_id      USMALLINT NOT NULL,               -- verb.verb_id
    act_classes  UBIGINT   NOT NULL,               -- the verb's act classes, bit i = [act_classes].ids[i]; 64 classes at most (34 today)
    place_id     UINTEGER,                         -- NULL until places exist (P1.1a)
    magnitude    INTEGER   NOT NULL DEFAULT 0,
    tellability  UTINYINT  NOT NULL,               -- the verb's Registry prior at emission
    secrecy      UTINYINT  NOT NULL,               -- 0 public, 1 witnessed, 2 faction, 3 secret
    origin       UTINYINT  NOT NULL,               -- 0 sim, 1 player, 3 oracle, 4 director (schema.sql)
    payload      BIGINT[]  NOT NULL,               -- the event's integer payload as emitted; meaning per verb (DEATH: [cause])
    PRIMARY KEY (timeline_id, seq)
);

CREATE TABLE IF NOT EXISTS event_role (           -- child rows of EVENT: who plays which role
    timeline_id  UINTEGER NOT NULL,
    event_id     UBIGINT  NOT NULL,
    ordinal      UTINYINT NOT NULL,                -- position among the event's subjects (emission order), contiguous from 0
    entity_kind  UTINYINT NOT NULL,                -- rh-ids EntityKind: 0 world, 1 person, 2 household, 3 settlement, 4 region
    entity_id    UBIGINT  NOT NULL,
    role         UTINYINT NOT NULL,                -- Registry [event_roles] index: 0 actor, 1 target, 2 witness, 3 instrument
    PRIMARY KEY (timeline_id, event_id, ordinal)
);
-- DEATH: one row, role 1 (target), the deceased; no actor for a natural death (ruling 11). EXTINCTION: no row.
-- An event's ordinals are 0..n-1 with no gap: the canonical text form writes roles as a list whose position is the
-- ordinal (specs/stream/canonical_v0.md §3), so a gap could not round-trip. The self-test checks it.
-- schema.sql's concrete_role codes (initiator, target, partner, bystander, instrument) and abstract_role are reconciled
-- with [event_roles] in P1.9 (SPR-011 Blocker 6); channel A v0 uses the Registry's codes.

CREATE TABLE IF NOT EXISTS causal_edge (          -- child rows of EVENT: its causes, all with smaller event_id
    timeline_id UINTEGER NOT NULL,
    cause_id    UBIGINT  NOT NULL,
    effect_id   UBIGINT  NOT NULL,
    kind        UTINYINT NOT NULL,                 -- schema.sql codes; P1.0 writes SPR-006's untyped causes as 1 triggered (ruling 11)
    PRIMARY KEY (timeline_id, effect_id, cause_id)
);

CREATE TABLE IF NOT EXISTS life_course_entry (    -- kind 3 LIFE_ENTRY: a person enters the world (05.10 §4 fidelity promise)
    timeline_id UINTEGER  NOT NULL,
    seq         UBIGINT   NOT NULL,
    tick        BIGINT    NOT NULL,                -- = the life_course row's entry_tick (0 for founders)
    kind        USMALLINT NOT NULL CHECK (kind = 3),
    person_id   UBIGINT   NOT NULL,
    origin      UTINYINT  NOT NULL,                -- 0 born, 1 founder, 2 immigrant
    sex         UTINYINT  NOT NULL,                -- 0 male, 1 female (life_course.sex = person.sex at entry)
    mother_id   UBIGINT   NOT NULL,                -- 0 = none (founders, immigrants without one)
    father_id   UBIGINT   NOT NULL,                -- 0 = none or unknown
    birth_tick  BIGINT    NOT NULL,                -- negative for founders born before the epoch
    PRIMARY KEY (timeline_id, seq)
);

CREATE TABLE IF NOT EXISTS life_course_exit (     -- kind 4 LIFE_EXIT: a person leaves the world
    timeline_id UINTEGER  NOT NULL,
    seq         UBIGINT   NOT NULL,
    tick        BIGINT    NOT NULL,                -- = the life_course row's exit_tick
    kind        USMALLINT NOT NULL CHECK (kind = 4),
    person_id   UBIGINT   NOT NULL,
    exit_kind   UTINYINT  NOT NULL,                -- 0 death, 1 emigration off-map (schema.sql; M2's column adds 0 = open)
    death_cause UTINYINT  NOT NULL,                -- M2 enum: 0 none (emigration), 1 natural, 2 murder, 3 battle, 4 execution, 5 accident
    event_id    UBIGINT,                           -- the DEATH event, when one was admitted (DATA-5; every death at rungs 0-2)
    PRIMARY KEY (timeline_id, seq)
);

CREATE TABLE IF NOT EXISTS step_digest (          -- kind 5 STEP_DIGEST: once per Month pulse, after Bookkeeping (ruling 13)
    timeline_id      UINTEGER  NOT NULL,
    seq              UBIGINT   NOT NULL,
    tick             BIGINT    NOT NULL,           -- the Month pulse instant
    kind             USMALLINT NOT NULL CHECK (kind = 5),
    living           UBIGINT   NOT NULL,           -- ledger rows after the compaction of this instant
    births           UINTEGER  NOT NULL,           -- LIFE_ENTRY records with origin born since the previous digest (0 in P1.0)
    deaths           UINTEGER  NOT NULL,           -- LIFE_EXIT records with exit_kind death since the previous digest
    state_hash       UBIGINT   NOT NULL,           -- plane hashes after Bookkeeping (05.1 §6 item 6; XXH3-64)
    observation_hash UBIGINT   NOT NULL,
    narrative_hash   UBIGINT   NOT NULL,
    PRIMARY KEY (timeline_id, seq)
);
-- Region-level digests (prices, era state, Ψ; 05.10 §4) join when their owners exist; they are new columns or a child
-- table in a later format, never floats.

CREATE TABLE IF NOT EXISTS keyframe_marker (      -- kind 6 KEYFRAME: a pointer to a full snapshot (every 10 sim-years)
    timeline_id      UINTEGER  NOT NULL,
    seq              UBIGINT   NOT NULL,
    tick             BIGINT    NOT NULL,
    kind             USMALLINT NOT NULL CHECK (kind = 6),
    keyframe_format  VARCHAR   NOT NULL,           -- 'rehoboam/rh-snapshot/keyframe/v1'
    keyframe_path    VARCHAR   NOT NULL,           -- relative to the run directory
    state_hash       UBIGINT   NOT NULL,           -- the hash the keyframe's header carries
    journal_position UBIGINT   NOT NULL,           -- accepted inputs at the keyframe (05.1 §6 item 6)
    PRIMARY KEY (timeline_id, seq)
);

-- ============================================================================================ views
-- Every record's header, for ordering and gap checks (R3, R4).
CREATE OR REPLACE VIEW record_header AS
    SELECT timeline_id, seq, tick, kind, 0 AS class, journal_seq AS k1, 0::UBIGINT AS k2 FROM input
    UNION ALL SELECT timeline_id, seq, tick, kind, 1, event_id, 0 FROM event
    UNION ALL SELECT timeline_id, seq, tick, kind, 2, person_id, 0 FROM life_course_entry
    UNION ALL SELECT timeline_id, seq, tick, kind, 2, person_id, 1 FROM life_course_exit
    UNION ALL SELECT timeline_id, seq, tick, kind, 3, 0, 0 FROM step_digest
    UNION ALL SELECT timeline_id, seq, tick, kind, 4, 0, 0 FROM keyframe_marker;

-- R4 holds iff this view is empty: a record whose seq order disagrees with the order key, or a gap in seq.
CREATE OR REPLACE VIEW order_violation AS
    WITH h AS (
        SELECT *, row_number() OVER (PARTITION BY timeline_id ORDER BY tick, class, k1, k2) - 1 AS by_key,
                  row_number() OVER (PARTITION BY timeline_id ORDER BY seq) - 1 AS by_seq
        FROM record_header
    )
    SELECT * FROM h WHERE by_key <> by_seq OR seq <> by_seq;

-- The chronicle's life_course (schema.sql) as the join of entries and exits (SPR-011 ruling 13).
CREATE OR REPLACE VIEW life_course_v AS
    SELECT e.timeline_id, e.person_id, e.origin, e.sex,
           NULLIF(e.mother_id, 0) AS mother_id, NULLIF(e.father_id, 0) AS father_id, e.birth_tick,
           CASE WHEN x.exit_kind = 0 THEN x.tick END AS death_tick,
           CASE WHEN x.exit_kind = 0 THEN x.death_cause::USMALLINT END AS death_cause,
           x.exit_kind
    FROM life_course_entry e
    LEFT JOIN life_course_exit x ON x.timeline_id = e.timeline_id AND x.person_id = e.person_id;
