-- World Stream v0, channel C: the sealed-segment schema of sift matches (DuckDB dialect; the files are Parquet with the
-- same columns). Written by `rh-cli sift <run-dir>` (rh-sifter v0, downstream of the engine; SPR-011 ruling 14), read by
-- `rh-cli tail` (annotations) and by the twin check (specs/sifting/twin_check.py, SPR-013 T-11).
-- Spec: SPR-011 rulings 13 and 14; SPR-013 T-11 (PR #37 Q10); 05.4 §2.3 (match semantics, MatchId), 05.10 §4, §5, §7;
-- 05.9 IF-0; 05.2 §1 (the MatchId space); ADR-0024 (plane C never feeds plane A); schema.sql sift_match (plane C).
-- Self-test: python -I specs/stream/selftest_channel_c.py (builds these tables, checks the rules below, writes and reads
-- a synthetic run's channel-C segments next to its channel-A segments, and compares them with a SQL twin of life_ends).
--
-- Status: v0, for P1.0 (SPR-013, T-11). A C2 contract: a change to a column, a code or the order is a new format version
-- (`segment.format`), announced like a Registry change. Channel C is plane C: re-sifting a timeline with another pattern
-- library rewrites channel C and never touches channel A or the state ladder (V-PLN-04).
--
-- ============================================================================================ rules
-- R1  Plane C only, one writer. Channel C holds what the sifter derives from sealed channel-A segments: matches in P1.0;
--     significance, suspense and visibility are later kinds (05.9 IF-0; reserved below). It lives in its own directory
--     tree and never interleaves with channel A (05.10 §5). The engine never reads it (ADR-0024).
-- R2  No float column in v0, including list elements (FLOAT, REAL, DOUBLE). Plane C may hold floats (schema.sql's
--     sift_match.score), but no v0 field needs one: a match is pattern, bindings, event ids and instants, all integers
--     or identifiers. `score` is significance (SIFT-5, P2.5) and is not a column here; when it comes it is a SIGNIFICANCE
--     record of a later format, fixed point if it can be, so that the channel-C digest stays bit-stable across machines.
--     The self-test refuses a float column in this file.
-- R3  Record header, as channel A's (channel_a_v0.sql R3): (timeline_id UINTEGER, seq UBIGINT, tick BIGINT,
--     kind USMALLINT). `seq` is the record's position in the timeline's channel C, contiguous from 0 across all its
--     segments, independent of channel A's seq. `tick` is the closing instant of the match: the tick of the channel-A
--     event that completed it. Child rows (sift_binding, sift_match_event) hang off their MATCH by (timeline_id,
--     match_id) and carry no seq.
-- R4  Order. The sifter reads channel A in seq order and completes matches as it goes; channel C is written in that order:
--       key = (closed_by_seq, pack, pattern_id, pattern_version, event_ids, bindings),
--     where closed_by_seq is the channel-A seq of the completing event, strings compare bytewise, event_ids (the match's
--     events in clause order, R6) compare lexicographically, and bindings compare as the list of (role, entity_kind,
--     entity_id) in bytewise role order. Two completed matches never share the whole key: a match is one pattern over
--     one event tuple and one binding (one event can complete several matches of one pattern when several entities fill
--     a role; match_id is content-addressed over the same key, R5). tick never decreases along seq, because channel A's
--     does. A DuckDB check is the order_violation view below.
-- R5  Identity.
--       match_id = XXH3-64(D_MATCH || str(pack) || str(pattern_id) || u16(pattern_version)
--                          || u32(n) || (u64(event_id)) x n                      -- the events in clause order
--                          || u32(m) || (str(role) || u8(entity_kind) || u64(entity_id)) x m)   -- bindings, bytewise role order
--     with D_MATCH = "rehoboam/sift/match/v0", str = u32 length || UTF-8 bytes, every integer little-endian, seed 0,
--     default secret (the engine's hash, rh_snapshot::hash::xxh3; the stdlib reference is bin/gen_hash_golden.py and
--     specs/stream/canonical_v0.py match_id). It is 05.4 §2.3 item 10's content-addressed MatchId with the pack (a pattern
--     id is unique within its pack only) and the bindings (one event tuple can carry several bindings) added; 05.4 is
--     amended to match. It is stable across re-sifts and equal to the twin's: the twin check recomputes it in Python
--     from the twin's (pack, pattern, version, event ids, bindings) and compares (V-SIFT-01: match ids, bindings,
--     event ids).
-- R6  Content. A MATCH record names its pattern (pack and id, the Registry's [pattern_packs] member and the pattern's
--     `id`), its version, the declared semantics it was matched under (selection, on_complete; 05.4 §2.3, SIFT-8), its
--     status, the opening and closing instants, the completing event and its channel-A seq, and, as child rows, the
--     bindings (role variable -> entity) and the matched events (clause bind name -> event id, in clause order, which
--     is kernel order: strictly increasing event ids). dedupe, the firing budget and the pattern's other declarations
--     are in the pattern dictionary (R7), not repeated per row.
-- R7  Segments and self-description. One directory per (timeline, channel, sim-year), as channel A's R5:
--       stream/t<timeline_id, 10 digits>/C/y<sim_year, 6 digits>/<table>.parquet
--     with sim_year = floor(tick / minutes_per_year) of the closing tick. A sealed segment holds one file per table below,
--     every one of them, empty or not: the record table, the two child tables, the dictionaries (record_kind, pattern)
--     and its `segment` row. A sim-year the sifter has processed with no match still seals a segment (record_count = 0),
--     so that `tail` can tell "no match" from "not sifted yet". Sealing is atomic: temporary file, fsync, rename;
--     segment.parquet is written last. The segment carries the pattern library it was sifted with (R8) and the channel
--     A it was derived from: the canonical digest of the channel-A segment of the same sim-year and the chain of all
--     channel-A digests up to it,
--       a_chain(y) = XXH3-128(D_CHAIN || a_chain(y - 1) || a_canonical_digest(y)),
--       a_chain(-1) = XXH3-128(D_CHAIN || u32(timeline_id)),   D_CHAIN = "rehoboam/stream/channel-c/v0/a-chain",
--     so that a channel-C segment pins every channel-A segment before it, not only its own year (as the journal's chain
--     does for its rows, rh-snapshot journal.rs). All 16-byte digests are XXH3's canonical form.
-- R8  Reproducibility (V-PLN-04, V-SIFT-01). Channel C is a function of (channel A, the pattern library, the sifter
--     build): sifting the same run twice gives byte-identical files and equal `canonical_digest`s. The digest is the
--     XXH3-128 of the records' canonical lines in seq order (specs/stream/canonical_v0.md; the MATCH line); an empty
--     segment digests the empty string. The pattern library is identified by its packs' file digests (`pattern.pack_digest`,
--     the XXH3-128 of each pack file's bytes, with the Registry pack id), not by a version number: v0 has no library
--     version separate from the files. `segment.dsl_version` names the grammar the packs were compiled under (05.4 §2.2).

-- ============================================================================================ dictionaries
CREATE TABLE IF NOT EXISTS record_kind (          -- channel C's kind codes (R3); its own code space, constant in format v0
    kind  USMALLINT NOT NULL PRIMARY KEY,
    name  VARCHAR   NOT NULL UNIQUE,
    class UTINYINT  NOT NULL                       -- reserved for an R4 class rank when a second kind exists; 0 in v0
);
INSERT OR IGNORE INTO record_kind VALUES
    (1, 'MATCH', 0);
-- Reserved for later formats (05.9 IF-0): SIGNIFICANCE, SUSPENSE, VISIBILITY. 0 is never a kind.

CREATE TABLE IF NOT EXISTS pattern (              -- the pattern library in force at sealing (R7, R8): one row per pattern
    pack            VARCHAR   NOT NULL,            -- Registry [pattern_packs] id, e.g. 'solitary_life'
    pattern_id      VARCHAR   NOT NULL,            -- the pattern's `id`, e.g. 'life_ends'
    pattern_version USMALLINT NOT NULL,            -- the pattern's `version`
    selection       UTINYINT  NOT NULL,            -- 0 latest, 1 earliest, 2 all (05.4 §2.3; declared, never implicit)
    on_complete     UTINYINT  NOT NULL,            -- 0 clear_binding, 1 consume, 2 keep
    clauses         UTINYINT  NOT NULL,            -- kernel clauses; 1 in v0 (single-clause patterns, SPR-011 ruling 14)
    roles           VARCHAR[] NOT NULL,            -- the pattern's role variables, bytewise order
    dedupe_key      VARCHAR[] NOT NULL,            -- dedupe.key (role variables), empty when the pattern declares no dedupe
    dedupe_days     UINTEGER  NOT NULL,            -- dedupe.bucket_days, 0 when the pattern declares no dedupe
    budget_num      UBIGINT   NOT NULL,            -- firing_budget.per_century_per_100k as the exact rational num/den
    budget_den      UBIGINT   NOT NULL CHECK (budget_den > 0),   -- (budgets such as 0.5 and 0.2 are not integers; SIFT-8)
    pack_digest     BLOB      NOT NULL,            -- XXH3-128 of the pack file's bytes (16 bytes, canonical form)
    PRIMARY KEY (pack, pattern_id, pattern_version)
);

CREATE TABLE IF NOT EXISTS segment (              -- one row per sealed segment; written last (R7)
    timeline_id      UINTEGER NOT NULL,
    channel          VARCHAR  NOT NULL CHECK (channel = 'C'),
    sim_year         INTEGER  NOT NULL CHECK (sim_year >= 0),
    format           VARCHAR  NOT NULL CHECK (format = 'rehoboam/stream/channel-c/v0'),
    first_seq        UBIGINT  NOT NULL,            -- the first record's seq; for an empty segment, the next seq to be used
    last_seq         UBIGINT,                      -- first_seq + record_count - 1; NULL when record_count = 0
    record_count     UBIGINT  NOT NULL,
    first_tick       BIGINT,                       -- NULL when record_count = 0
    last_tick        BIGINT,
    sifter_build     VARCHAR  NOT NULL,            -- rh-sifter's build id (the plane-C half of the provenance) ...
    dsl_version      USMALLINT NOT NULL,           -- ... the pattern DSL grammar version (0 = v0, 05.4 §2.2) ...
    packs            VARCHAR[] NOT NULL,           -- ... and the packs sifted, in the manifest's order (each in `pattern`,
                                                   --     whose pack_digest values identify the library, R8)
    engine_build     VARCHAR  NOT NULL,            -- the history tuple of the channel A it was derived from (05.1 §6) ...
    registry_version VARCHAR  NOT NULL,
    manifest_hash    BLOB     NOT NULL,
    world_seed       UBIGINT  NOT NULL,
    a_last_seq       UBIGINT  NOT NULL,            -- ... the last channel-A seq read when this segment sealed ...
    a_canonical_digest BLOB   NOT NULL,            -- ... the channel-A segment (same timeline, sim-year) it closes with ...
    a_chain          BLOB     NOT NULL,            -- ... and the chain of every channel-A digest up to it (R7)
    canonical_digest BLOB     NOT NULL,            -- R8: XXH3-128 of the MATCH canonical lines in seq order (16 bytes)
    PRIMARY KEY (timeline_id, channel, sim_year),
    CHECK ((record_count = 0 AND last_seq IS NULL AND first_tick IS NULL AND last_tick IS NULL)
        OR (record_count > 0 AND last_seq = first_seq + record_count - 1 AND first_tick IS NOT NULL AND last_tick IS NOT NULL))
);

-- ============================================================================================ records
CREATE TABLE IF NOT EXISTS sift_match (           -- kind 1 MATCH: one completed pattern instance (schema.sql sift_match, plane C)
    timeline_id        UINTEGER  NOT NULL,
    seq                UBIGINT   NOT NULL,
    tick               BIGINT    NOT NULL,         -- the closing instant: the completing event's tick
    kind               USMALLINT NOT NULL CHECK (kind = 1),
    match_id           UBIGINT   NOT NULL,         -- R5; the MatchId space, never an EventId (05.2 §1)
    pack               VARCHAR   NOT NULL,         -- pattern.pack
    pattern_id         VARCHAR   NOT NULL,         -- pattern.pattern_id
    pattern_version    USMALLINT NOT NULL,         -- pattern.pattern_version
    selection          UTINYINT  NOT NULL CHECK (selection IN (0, 1, 2)),       -- the semantics matched under (R6)
    on_complete        UTINYINT  NOT NULL CHECK (on_complete IN (0, 1, 2)),
    status             UTINYINT  NOT NULL CHECK (status = 1),   -- 1 complete; 0 partial, 2 expired, 3 killed are records
                                                   -- of a later format (the live sifter, P2.1), never written in v0
    opened_tick        BIGINT    NOT NULL,         -- the first bound event's tick (= tick for a single-clause pattern)
    closed_by_event_id UBIGINT   NOT NULL,         -- the completing event (schema.sql closed_by_event_id)
    closed_by_seq      UBIGINT   NOT NULL,         -- its channel-A seq: the R4 order key and the twin check's join key
    PRIMARY KEY (timeline_id, seq),
    UNIQUE (timeline_id, match_id)
);

CREATE TABLE IF NOT EXISTS sift_binding (         -- child rows of MATCH: role variable -> entity (schema.sql bindings MAP)
    timeline_id UINTEGER NOT NULL,
    match_id    UBIGINT  NOT NULL,
    role        VARCHAR  NOT NULL,                 -- the pattern's role variable, e.g. 'A'
    entity_kind UTINYINT NOT NULL,                 -- rh-ids EntityKind: 0 world, 1 person, 2 household, 3 settlement, 4 region
    entity_id   UBIGINT  NOT NULL,
    PRIMARY KEY (timeline_id, match_id, role)
);

CREATE TABLE IF NOT EXISTS sift_match_event (     -- child rows of MATCH: clause -> the channel-A event it bound
    timeline_id UINTEGER NOT NULL,
    match_id    UBIGINT  NOT NULL,
    clause      VARCHAR  NOT NULL,                 -- the clause's `bind`, e.g. 'e1'; clause order = kernel order (R6)
    ordinal     UTINYINT NOT NULL,                 -- the clause's position, from 0
    event_id    UBIGINT  NOT NULL,                 -- channel A event.event_id
    PRIMARY KEY (timeline_id, match_id, ordinal),
    UNIQUE (timeline_id, match_id, clause)
);
-- life_ends (patterns.solitary_life.toml): one binding row (A -> the deceased, entity_kind 1) and one event row
-- (e1 -> the DEATH event); opened_tick = tick; closed_by_event_id = that event.

-- ============================================================================================ views
-- The event ids of each match in clause order, as one list (for the R4 key and the twin check).
CREATE OR REPLACE VIEW match_events_v AS
    SELECT timeline_id, match_id, list(event_id ORDER BY ordinal) AS event_ids
    FROM sift_match_event GROUP BY timeline_id, match_id;

-- The bindings of each match as one list of (role, entity_kind, entity_id) in bytewise role order (the R4 key).
CREATE OR REPLACE VIEW match_bindings_v AS
    SELECT timeline_id, match_id,
           list(struct_pack(role := role, entity_kind := entity_kind, entity_id := entity_id)
                ORDER BY encode(role)) AS bindings
    FROM sift_binding GROUP BY timeline_id, match_id;

-- R4 holds iff this view is empty: a record whose seq order disagrees with the order key, or a gap in seq.
CREATE OR REPLACE VIEW order_violation AS
    WITH h AS (
        SELECT m.timeline_id, m.seq, m.tick,
               row_number() OVER (PARTITION BY m.timeline_id
                                  ORDER BY m.closed_by_seq, encode(m.pack), encode(m.pattern_id), m.pattern_version,
                                           e.event_ids, b.bindings) - 1 AS by_key,
               row_number() OVER (PARTITION BY m.timeline_id ORDER BY m.seq) - 1 AS by_seq
        FROM sift_match m
        LEFT JOIN match_events_v e ON e.timeline_id = m.timeline_id AND e.match_id = m.match_id
        LEFT JOIN match_bindings_v b ON b.timeline_id = m.timeline_id AND b.match_id = m.match_id
    )
    SELECT * FROM h WHERE by_key <> by_seq OR seq <> by_seq;

-- schema.sql's sift_match (plane C) as a view: its columns in order, except `score` (R2: significance is not in v0).
-- pattern_lib_version is NULL: v0 identifies the library by its pack digests (R8), not by a number; the column stays
-- so that the view has schema.sql's shape until P1.9 reconciles schema.sql's sift_match with this format.
-- bindings is the MAP schema.sql declares.
CREATE OR REPLACE VIEW sift_match_v AS
    SELECT m.timeline_id, m.match_id, m.pattern_id, m.pattern_version,
           NULL::USMALLINT AS pattern_lib_version,
           (SELECT map(list(role ORDER BY role), list(entity_id ORDER BY role))
              FROM sift_binding b WHERE b.timeline_id = m.timeline_id AND b.match_id = m.match_id) AS bindings,
           e.event_ids, m.status, m.opened_tick, m.tick AS closed_tick, m.closed_by_event_id
    FROM sift_match m
    LEFT JOIN match_events_v e ON e.timeline_id = m.timeline_id AND e.match_id = m.match_id;
