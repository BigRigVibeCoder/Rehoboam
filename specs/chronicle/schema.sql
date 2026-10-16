-- Rehoboam Chronicle schema v0.3 (DuckDB dialect; the system of record is Parquet with the same columns).
-- Spec: docs/05_architecture/05.2 §4 and §6, 05.4 §2.3, ADR-0007, ADR-0024 (planes), ADR-0025, ADR-0026.
-- Conventions:
--   * Timeline-scoped composite keys.
--   * event_id: one counter per timeline, from 0, assigned at the scheduler's level merge (rh-events; 05.1 §2 rule 4).
--     EventId order IS emission order (tick, phase, seq); no back-dating. The id carries no date: day = floor(tick / 1440).
--   * tick = SimTime: signed BIGINT minutes since the world epoch. Negative values are allowed (founders born before year 0).
--   * Each table is tagged with its plane: A authoritative, B observation, C narrative. Plane A never reads B or C.

-- ============================================================================================ plane A
CREATE TABLE IF NOT EXISTS event (                -- plane A
    timeline_id   UINTEGER  NOT NULL,
    event_id      UBIGINT   NOT NULL,
    tick          BIGINT    NOT NULL,          -- SimTime (i64 minutes); always the emission tick
    verb_id       USMALLINT NOT NULL,          -- FK Registry verbs
    act_classes   UINTEGER  NOT NULL,          -- bitset over Registry act classes
    place_id      UINTEGER,
    magnitude     INTEGER   DEFAULT 0,
    tellability   UTINYINT  NOT NULL,          -- Registry prior (0..255), NOT the retroactive significance (plane C)
    secrecy       UTINYINT  NOT NULL,          -- 0 public, 1 witnessed, 2 faction, 3 secret
    origin        UTINYINT  NOT NULL,          -- 0 sim, 1 player, 2 reserved (was llm_choice; retired by ADR-0034), 3 oracle, 4 director (opt-in storyteller modes only)
    rule_id       USMALLINT,
    rule_version  USMALLINT,
    PRIMARY KEY (timeline_id, event_id)
);

CREATE TABLE IF NOT EXISTS event_role (           -- plane A
    timeline_id    UINTEGER NOT NULL,
    event_id       UBIGINT  NOT NULL,
    entity_id      UBIGINT  NOT NULL,          -- PersonId = H64(mother_id, parity) etc. (ADR-0026)
    concrete_role  UTINYINT NOT NULL,          -- 0 initiator, 1 target, 2 partner, 3 bystander, 4 instrument
    abstract_role  UTINYINT NOT NULL           -- Registry roles: authority, successor, kin, ally, rival, ...
);

-- Anchored state: exactly the attribute values that rules and patterns read AT EMISSION (fixes the draft's temporal bug).
-- Attributes are typed in the Registry `[attributes]` section; the pattern form is attr(args)@eN.
CREATE TABLE IF NOT EXISTS event_snapshot (       -- plane A
    timeline_id UINTEGER  NOT NULL,
    event_id    UBIGINT   NOT NULL,
    entity_id   UBIGINT   NOT NULL,            -- first attribute argument, e.g. B in trust(B, A)
    attr_id     USMALLINT NOT NULL,            -- Registry attribute
    partner_id  UBIGINT,                       -- second argument for relational attributes, e.g. A in trust(B, A); NULL otherwise
    value       INTEGER   NOT NULL             -- q8_8 fixed point (value/256), bool 0/1, enum index, or raw int
);

-- Typed causality (R02 §R2): contingency only from explicit unlocking; utility contributors are motivated_by.
CREATE TABLE IF NOT EXISTS causal_edge (          -- plane A
    timeline_id UINTEGER NOT NULL,
    cause_id    UBIGINT  NOT NULL,
    effect_id   UBIGINT  NOT NULL,
    kind        UTINYINT NOT NULL,             -- 0 contingent, 1 triggered, 2 relayed_knowledge, 3 motivated_by,
                                               -- 4 enabled, 5 probabilistic, 6 player, 7 oracle, 8 director (05.4 §4)
    weight      SMALLINT DEFAULT 0,
    evidence    VARCHAR                         -- optional short provenance key
);

-- Per-actor knowledge: witnessed, faction-internal and secret events, and testimony hops between named actors only.
CREATE TABLE IF NOT EXISTS knowledge (            -- plane A
    timeline_id   UINTEGER NOT NULL,
    entity_id     UBIGINT  NOT NULL,
    event_id      UBIGINT  NOT NULL,
    acquired_tick BIGINT   NOT NULL,
    via_event_id  UBIGINT,
    channel       UTINYINT NOT NULL,           -- witness, testimony, document, crowd, whisper, hive, vision
    fidelity      UTINYINT NOT NULL,
    variant_id    UINTEGER DEFAULT 0,          -- distorted rumour variant
    salience      UTINYINT NOT NULL
);

-- Public / crowd / document knowledge as arrival times per region (05.2 §4.1, RT-M7).
-- knows(a, e, t) = knowledge row (a, e) OR (public(e) AND arrival(region(a, t), e) <= t)
CREATE TABLE IF NOT EXISTS diffusion_front (      -- plane A
    timeline_id  UINTEGER  NOT NULL,
    event_id     UBIGINT   NOT NULL,
    region_id    USMALLINT NOT NULL,
    arrival_tick BIGINT    NOT NULL,
    channel      UTINYINT  NOT NULL,
    fidelity     UTINYINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS collection (           -- plane A: war -> campaign -> battle; plague -> wave
    timeline_id   UINTEGER NOT NULL,
    collection_id UBIGINT  NOT NULL,
    parent_id     UBIGINT,
    ctype         USMALLINT NOT NULL,          -- war, campaign, battle, plague, wave, famine
    members       UBIGINT[]
);

-- Mass events (a famine killing 40,000): one event row plus counts, instead of 40,000 rows.
CREATE TABLE IF NOT EXISTS aggregate_event (      -- plane A
    timeline_id UINTEGER  NOT NULL,
    event_id    UBIGINT   NOT NULL,            -- FK event (verb with aggregate = true, e.g. FAMINE_DECLARED)
    region_id   USMALLINT NOT NULL,
    age_band    UTINYINT  NOT NULL,            -- 5-year bands
    sex         UTINYINT  NOT NULL,
    class_id    UTINYINT  NOT NULL,            -- commoner, burgher, elite
    count       UINTEGER  NOT NULL
);

CREATE TABLE IF NOT EXISTS state_interval (       -- plane A: SCD2 for slowly changing facts (titles, factions, marriages)
    timeline_id UINTEGER  NOT NULL,
    entity_id   UBIGINT   NOT NULL,
    attr_id     USMALLINT NOT NULL,
    value       BIGINT    NOT NULL,
    valid_from  BIGINT    NOT NULL,
    valid_to    BIGINT                          -- NULL = current
);

-- Everyone ever born (ADR-0026). The live ledger holds only the living; this log is append-only.
CREATE TABLE IF NOT EXISTS life_course (          -- plane A
    timeline_id UINTEGER  NOT NULL,
    person_id   UBIGINT   NOT NULL,            -- H64(mother_id, parity) | H64(seed,"founder",k) | H64(seed,"immigrant",origin,t,k)
    origin      UTINYINT  NOT NULL,            -- 0 born, 1 founder, 2 immigrant
    mother_id   UBIGINT,
    father_id   UBIGINT,                       -- social father as recorded by the sim
    birth_tick  BIGINT    NOT NULL,            -- may be negative for founders
    death_tick  BIGINT,                        -- NULL = alive (or emigrated, see exit_kind)
    death_cause USMALLINT,
    exit_kind   UTINYINT,                      -- NULL alive, 0 death, 1 emigration off-map
    PRIMARY KEY (timeline_id, person_id)
);

CREATE TABLE IF NOT EXISTS life_interval (        -- plane A: residence, occupation, faction, marriage spells
    timeline_id UINTEGER  NOT NULL,
    person_id   UBIGINT   NOT NULL,
    kind        UTINYINT  NOT NULL,            -- 0 residence (region), 1 occupation, 2 faction, 3 marriage (partner id)
    value       BIGINT    NOT NULL,
    valid_from  BIGINT    NOT NULL,
    valid_to    BIGINT
);

CREATE TABLE IF NOT EXISTS regional_aggregate (   -- plane A telemetry for commoners
    timeline_id UINTEGER  NOT NULL,
    region_id   USMALLINT NOT NULL,
    month       INTEGER   NOT NULL,            -- signed month index since epoch
    metric_id   USMALLINT NOT NULL,
    value       DOUBLE    NOT NULL
);

-- ============================================================================================ plane B
CREATE TABLE IF NOT EXISTS facts_ledger (         -- plane B: facts shown to you; never read by plane A
    timeline_id     UINTEGER  NOT NULL,
    subject_id      UBIGINT   NOT NULL,
    fact_type       USMALLINT NOT NULL,
    value           VARCHAR   NOT NULL,
    source_event_id UBIGINT,
    revealed_tick   BIGINT    NOT NULL
);

-- ============================================================================================ plane C
CREATE TABLE IF NOT EXISTS sig_component (        -- plane C: retroactive significance (recomputed per era)
    timeline_id         UINTEGER  NOT NULL,
    event_id            UBIGINT   NOT NULL,
    component           UTINYINT  NOT NULL,    -- tellability, rarity, consequence, centrality, stakes, reversal, irony, suspense, kernel
    value               FLOAT     NOT NULL,
    computed_at_tick    BIGINT    NOT NULL,
    pattern_lib_version USMALLINT NOT NULL
);

CREATE TABLE IF NOT EXISTS sift_match (           -- plane C: separate MatchId space, never interleaved with EventIds
    timeline_id         UINTEGER  NOT NULL,
    match_id            UBIGINT   NOT NULL,    -- H64(pattern_id, pattern_version, event_ids): content-addressed, stable across re-sifts and equal to the twin's
    pattern_id          VARCHAR   NOT NULL,
    pattern_version     USMALLINT NOT NULL,
    pattern_lib_version USMALLINT NOT NULL,
    bindings            MAP(VARCHAR, UBIGINT),
    event_ids           UBIGINT[],
    status              UTINYINT  NOT NULL,    -- 0 partial, 1 complete, 2 expired (window), 3 killed (unless or death of a non-tolerant role)
    opened_tick         BIGINT    NOT NULL,
    closed_tick         BIGINT,
    closed_by_event_id  UBIGINT,               -- the completing or killing event (B3: redemption arcs need the kill)
    score               FLOAT
);

-- Retention without physical compaction: low-salience events are hidden, never moved or rewritten (05.2 §6).
CREATE TABLE IF NOT EXISTS visibility (           -- plane C
    timeline_id UINTEGER NOT NULL,
    event_id    UBIGINT  NOT NULL,
    tier        UTINYINT NOT NULL              -- 0 hidden, 1 era digest only, 2 browsable, 3 headline
);

CREATE TABLE IF NOT EXISTS base_rate (            -- plane C: rarity base rates per (pattern, era)
    timeline_id         UINTEGER  NOT NULL,
    pattern_id          VARCHAR   NOT NULL,
    era                 INTEGER   NOT NULL,    -- simulated century index
    matches             UBIGINT   NOT NULL,
    pattern_lib_version USMALLINT NOT NULL
);

-- ============================================================================================ journal, hashes, keyframes
-- These live in SQLite (WAL) in production; mirrored here for analytics.
CREATE TABLE IF NOT EXISTS input_journal (
    timeline_id   UINTEGER NOT NULL,
    tick          BIGINT   NOT NULL,
    seq           UINTEGER NOT NULL,
    input_class   UTINYINT NOT NULL,           -- 0 push (the only class; pull inputs retired by ADR-0034)
    kind          VARCHAR  NOT NULL,
    scope_kind    UTINYINT NOT NULL,           -- 0 none, 1 person, 2 household, 3 settlement, 4 region
    scope_id      UBIGINT,
    pull_key      BLOB,                        -- reserved, always NULL (ADR-0034)
    producer      VARCHAR  NOT NULL,           -- player | llm:<job_id> | system
    model_id      VARCHAR,                     -- LLM-produced inputs only
    weights_hash  BLOB,                        -- local models
    provider_ver  VARCHAR,                     -- API models
    prompt_hash   BLOB,
    payload       BLOB     NOT NULL,
    payload_hash  BLOB     NOT NULL
);

CREATE TABLE IF NOT EXISTS plane_hash (
    timeline_id UINTEGER NOT NULL,
    tick        BIGINT   NOT NULL,
    plane       UTINYINT NOT NULL,             -- 0 state (A), 1 observation (B), 2 narrative (C)
    xxh3_64     UBIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS keyframe (
    timeline_id    UINTEGER NOT NULL,
    tick           BIGINT   NOT NULL,
    path           VARCHAR  NOT NULL,
    state_hash     UBIGINT  NOT NULL,
    bytes          UBIGINT  NOT NULL,
    schema_version USMALLINT NOT NULL,
    engine_build   VARCHAR  NOT NULL,          -- content-addressed build that wrote it ("frozen past, migrated future")
    is_migration   BOOLEAN  NOT NULL DEFAULT false,
    permanent      BOOLEAN  NOT NULL           -- every 10 sim-years; others are LRU near the cursor
);

-- ---------------------------------------------------------------------------------------------
-- SQL TWINS: pattern `trust_then_betrayal` (specs/sifting/patterns.example.toml)
--   e1: A aids/promises B with trust(B, A)@e1 >= 0.6 (q8_8 => 154)
--   e2: A harms/betrays/conceals B, after e1, within 3650 days
--   unless: B aids A strictly between e1 and e2
-- Act-class bits used here (illustrative): harm=1<<0, aid=1<<1, promise=1<<2, betray=1<<3, conceal=1<<5
--
-- Three views:
--   twin_trust_then_betrayal_all    selection = "all", on_complete = "keep" (every (e1, e2) pair; floods on clustered data)
--   twin_trust_then_betrayal        selection = "latest", on_complete = "clear_binding" (the DSL default, 05.4 §2.3)
--   twin_trust_then_betrayal_dedup  the default plus dedupe = { key = ["A", "B"], bucket_days = 365 }
--
-- Incremental semantics the twins reproduce (per event, in emission order; per binding (A, B)):
--   1. completion: if the event is an e2 for (A, B), it pairs with the LATEST open partial; a match needs that
--      partial to lie within the window. After a match, every open partial for (A, B) is discarded.
--   2. unless: an event that is an `unless` for (A, B) discards every open partial for (A, B).
--   3. opening: if the event is an e1 for (A, B), it opens a new partial.
-- Consequence used by the twin: for an e2, the eligible e1s are those at or after the binding's latest earlier
-- "reset" (any earlier e2 or unless event for the binding). An earlier e2 that did not match is a valid reset too,
-- because it found only expired partials, and partials only get older.
-- dedupe filters emitted matches only (at most one per binding per bucket); it does not change partial state.
-- ---------------------------------------------------------------------------------------------
CREATE OR REPLACE MACRO has_act(bits, mask) AS (bits & mask) <> 0;

CREATE OR REPLACE VIEW ttb_ev AS
WITH initiator AS (
    SELECT r.timeline_id, r.event_id, r.entity_id AS a FROM event_role r WHERE r.concrete_role = 0
), target AS (
    SELECT r.timeline_id, r.event_id, r.entity_id AS b FROM event_role r WHERE r.concrete_role = 1
)
SELECT e.timeline_id, e.event_id, e.tick, e.act_classes, i.a, t.b
FROM event e JOIN initiator i USING (timeline_id, event_id) JOIN target t USING (timeline_id, event_id);

CREATE OR REPLACE VIEW ttb_e1 AS
SELECT ev.* FROM ttb_ev ev
JOIN event_snapshot s ON s.timeline_id = ev.timeline_id AND s.event_id = ev.event_id
                      AND s.entity_id = ev.b AND s.partner_id = ev.a AND s.attr_id = 1 /* trust */
WHERE has_act(ev.act_classes, (1<<1) | (1<<2)) AND s.value >= 154;

CREATE OR REPLACE VIEW ttb_e2 AS
SELECT ev.* FROM ttb_ev ev WHERE has_act(ev.act_classes, (1<<0) | (1<<3) | (1<<5));

CREATE OR REPLACE VIEW ttb_unless AS              -- keyed to the binding it cancels: (A, B) = (target, initiator)
SELECT ev.timeline_id, ev.event_id, ev.tick, ev.b AS a, ev.a AS b FROM ttb_ev ev WHERE has_act(ev.act_classes, 1<<1);

CREATE OR REPLACE VIEW twin_trust_then_betrayal_all AS
SELECT e1.timeline_id, e1.a, e1.b, e1.event_id AS e1_id, e2.event_id AS e2_id
FROM ttb_e1 e1 JOIN ttb_e2 e2
  ON e2.timeline_id = e1.timeline_id AND e2.a = e1.a AND e2.b = e1.b
 AND e2.event_id > e1.event_id AND e2.tick <= e1.tick + 3650 * 1440
WHERE NOT EXISTS (
    SELECT 1 FROM ttb_unless u
    WHERE u.timeline_id = e1.timeline_id AND u.a = e1.a AND u.b = e1.b
      AND u.event_id > e1.event_id AND u.event_id < e2.event_id
);

CREATE OR REPLACE VIEW twin_trust_then_betrayal AS
WITH resets AS (
    SELECT timeline_id, a, b, event_id, tick, true  AS is_e2 FROM ttb_e2
    UNION ALL
    SELECT timeline_id, a, b, event_id, tick, false AS is_e2 FROM ttb_unless
), e2_window AS (                                 -- each e2 with the binding's latest earlier reset
    SELECT timeline_id, a, b, event_id AS e2_id, tick AS e2_tick,
           LAG(event_id) OVER (PARTITION BY timeline_id, a, b ORDER BY event_id) AS reset_id
    FROM resets
    QUALIFY is_e2
), latest AS (                                    -- selection = "latest": the newest e1 since the reset
    SELECT w.timeline_id, w.a, w.b, w.e2_id, w.e2_tick, max(e1.event_id) AS e1_id
    FROM e2_window w JOIN ttb_e1 e1
      ON e1.timeline_id = w.timeline_id AND e1.a = w.a AND e1.b = w.b
     AND e1.event_id < w.e2_id AND (w.reset_id IS NULL OR e1.event_id >= w.reset_id)
    GROUP BY ALL
)
SELECT l.timeline_id, l.a, l.b, l.e1_id, l.e2_id
FROM latest l JOIN ttb_e1 e1
  ON e1.timeline_id = l.timeline_id AND e1.event_id = l.e1_id AND e1.a = l.a AND e1.b = l.b
WHERE l.e2_tick <= e1.tick + 3650 * 1440;

CREATE OR REPLACE VIEW twin_trust_then_betrayal_dedup AS
SELECT m.timeline_id, m.a, m.b, m.e1_id, m.e2_id
FROM twin_trust_then_betrayal m JOIN event e ON e.timeline_id = m.timeline_id AND e.event_id = m.e2_id
QUALIFY row_number() OVER (PARTITION BY m.timeline_id, m.a, m.b, CAST(floor(e.tick / (365 * 1440.0)) AS BIGINT)
                           ORDER BY m.e2_id) = 1;

-- Causal ancestors (for hindsight retention and the causal inspector; R02 §3 "Labov pre-construction"): contingent + motivated_by edges.
-- Usage: SELECT * FROM causal_ancestors(<timeline_id>, <event_id>, <max_depth>);
CREATE OR REPLACE MACRO causal_ancestors(tl, root_id, max_depth) AS TABLE
WITH RECURSIVE walk(event_id, depth) AS (
    SELECT root_id::UBIGINT, 0
    UNION
    SELECT c.cause_id, w.depth + 1
    FROM walk w JOIN causal_edge c ON c.effect_id = w.event_id AND c.timeline_id = tl
    WHERE c.kind IN (0, 3) AND w.depth < max_depth
)
SELECT event_id, min(depth) AS depth FROM walk GROUP BY event_id;

-- As-of read of a slowly changing fact (e.g. "who held the title at tick t").
-- Usage: SELECT * FROM title_holder_at(<timeline_id>, <title_attr_id>, <tick>);
CREATE OR REPLACE MACRO title_holder_at(tl, title_attr, t) AS TABLE
SELECT entity_id, value FROM state_interval
WHERE timeline_id = tl AND attr_id = title_attr AND valid_from <= t AND (valid_to IS NULL OR valid_to > t);
