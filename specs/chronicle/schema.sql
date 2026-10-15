-- Rehoboam Chronicle schema v0 (DuckDB dialect; the system of record is Parquet with the same columns).
-- Spec: docs/05_architecture/05.2 §4, ADR-0007. Replaces the draft's chronicle_events(context_json, causes_json).
-- Conventions: timeline-scoped composite keys; event_id = (day_index << 24) | seq_in_day (ordered, deterministic).

CREATE TABLE IF NOT EXISTS event (
    timeline_id   UINTEGER  NOT NULL,
    event_id      UBIGINT   NOT NULL,
    tick          UBIGINT   NOT NULL,          -- SimTime, minutes since world epoch
    verb_id       USMALLINT NOT NULL,          -- FK registry verbs
    act_classes   UINTEGER  NOT NULL,          -- bitset over Registry act classes
    place_id      UINTEGER,
    magnitude     INTEGER   DEFAULT 0,
    tellability   UTINYINT  NOT NULL,          -- Registry prior (0..255), NOT the retroactive significance
    secrecy       UTINYINT  NOT NULL,          -- 0 public, 1 witnessed, 2 faction, 3 secret
    origin        UTINYINT  NOT NULL,          -- 0 sim, 1 player, 2 llm_choice, 3 director
    rule_id       USMALLINT,
    rule_version  USMALLINT,
    PRIMARY KEY (timeline_id, event_id)
);

CREATE TABLE IF NOT EXISTS event_role (
    timeline_id    UINTEGER NOT NULL,
    event_id       UBIGINT  NOT NULL,
    entity_id      UBIGINT  NOT NULL,
    concrete_role  UTINYINT NOT NULL,          -- 0 initiator, 1 target, 2 partner, 3 bystander, 4 instrument
    abstract_role  UTINYINT NOT NULL           -- Registry roles: authority, successor, kin, ally, rival, ...
);

-- Anchored state: exactly the predicate values rules/sifter read AT EMISSION (fixes the draft's temporal bug).
CREATE TABLE IF NOT EXISTS event_snapshot (
    timeline_id UINTEGER  NOT NULL,
    event_id    UBIGINT   NOT NULL,
    entity_id   UBIGINT   NOT NULL,
    attr_id     USMALLINT NOT NULL,            -- e.g. fear, trust(B,A) encoded with partner in entity_id pair table
    partner_id  UBIGINT,                       -- for relational attributes (trust(B, A)): entity=B, partner=A
    value       INTEGER   NOT NULL             -- fixed-point (Q8.8 for psych, raw counts otherwise)
);

-- Typed causality (R02 §R2): contingency only from explicit unlocking; utility contributors are motivated_by.
CREATE TABLE IF NOT EXISTS causal_edge (
    timeline_id UINTEGER NOT NULL,
    cause_id    UBIGINT  NOT NULL,
    effect_id   UBIGINT  NOT NULL,
    kind        UTINYINT NOT NULL,             -- 0 contingent, 1 triggered, 2 relayed_knowledge, 3 motivated_by,
                                               -- 4 enabled, 5 probabilistic, 6 player, 7 director
    weight      SMALLINT DEFAULT 0,
    evidence    VARCHAR                         -- optional short provenance key
);

CREATE TABLE IF NOT EXISTS knowledge (
    timeline_id   UINTEGER NOT NULL,
    entity_id     UBIGINT  NOT NULL,
    event_id      UBIGINT  NOT NULL,
    acquired_tick UBIGINT  NOT NULL,
    via_event_id  UBIGINT,
    channel       UTINYINT NOT NULL,           -- witness, testimony, document, crowd, whisper, hive
    fidelity      UTINYINT NOT NULL,
    variant_id    UINTEGER DEFAULT 0,          -- distorted rumour variant
    salience      UTINYINT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection (
    timeline_id   UINTEGER NOT NULL,
    collection_id UBIGINT  NOT NULL,
    parent_id     UBIGINT,
    ctype         USMALLINT NOT NULL,          -- war, campaign, battle, plague, wave, famine, summary_fold
    members       UBIGINT[],
    counts        MAP(VARCHAR, BIGINT)         -- for compaction folds: preserved base rates
);

CREATE TABLE IF NOT EXISTS state_interval (       -- SCD2 for slowly changing facts (titles, factions, marriages)
    timeline_id UINTEGER  NOT NULL,
    entity_id   UBIGINT   NOT NULL,
    attr_id     USMALLINT NOT NULL,
    value       BIGINT    NOT NULL,
    valid_from  UBIGINT   NOT NULL,
    valid_to    UBIGINT                         -- NULL = current
);

CREATE TABLE IF NOT EXISTS sig_component (        -- retroactive significance (recomputed per era)
    timeline_id      UINTEGER  NOT NULL,
    event_id         UBIGINT   NOT NULL,
    component        UTINYINT  NOT NULL,       -- tellability, rarity, consequence, centrality, stakes, reversal, irony, suspense, kernel
    value            FLOAT     NOT NULL,
    computed_at_tick UBIGINT   NOT NULL
);

CREATE TABLE IF NOT EXISTS sift_match (
    timeline_id     UINTEGER  NOT NULL,
    match_id        UBIGINT   NOT NULL,
    pattern_id      VARCHAR   NOT NULL,
    pattern_version USMALLINT NOT NULL,
    bindings        MAP(VARCHAR, UBIGINT),
    event_ids       UBIGINT[],
    status          UTINYINT  NOT NULL,        -- 0 partial, 1 complete, 2 expired
    opened_tick     UBIGINT   NOT NULL,
    closed_tick     UBIGINT,
    score           FLOAT
);

CREATE TABLE IF NOT EXISTS facts_ledger (
    timeline_id     UINTEGER NOT NULL,
    subject_id      UBIGINT  NOT NULL,
    fact_type       USMALLINT NOT NULL,
    value           VARCHAR  NOT NULL,
    source_event_id UBIGINT,
    revealed_tick   UBIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS regional_aggregate (
    timeline_id UINTEGER  NOT NULL,
    region_id   USMALLINT NOT NULL,
    month       UINTEGER  NOT NULL,
    metric_id   USMALLINT NOT NULL,
    value       DOUBLE    NOT NULL
);

-- Journal / hashes / keyframes live in SQLite (WAL) in production; mirrored here for analytics.
CREATE TABLE IF NOT EXISTS input_journal (
    timeline_id  UINTEGER NOT NULL,
    tick         UBIGINT  NOT NULL,
    seq          UINTEGER NOT NULL,
    kind         VARCHAR  NOT NULL,
    producer     VARCHAR  NOT NULL,            -- player | llm:<job_id>:<model_id> | system
    payload      BLOB     NOT NULL,
    payload_hash BLOB     NOT NULL
);

CREATE TABLE IF NOT EXISTS state_hash (
    timeline_id UINTEGER NOT NULL,
    tick        UBIGINT  NOT NULL,
    xxh3_64     UBIGINT  NOT NULL
);

-- ---------------------------------------------------------------------------------------------
-- SQL TWIN EXAMPLE: pattern `trust_then_betrayal` (specs/sifting/patterns.example.toml)
--   e1: A aids/promises B with trust(B,A)@e1 >= 0.6 (Q8.8 => 154)
--   e2: A harms/betrays/conceals B, after e1, within 3650 days
--   unless: B aids A strictly between e1 and e2
-- Act-class bits used here (illustrative): aid=1<<1, promise=1<<2, harm=1<<0, betray=1<<3, conceal=1<<5
-- ---------------------------------------------------------------------------------------------
CREATE OR REPLACE MACRO has_act(bits, mask) AS (bits & mask) <> 0;

CREATE OR REPLACE VIEW twin_trust_then_betrayal AS
WITH initiator AS (
    SELECT r.timeline_id, r.event_id, r.entity_id AS a FROM event_role r WHERE r.concrete_role = 0
), target AS (
    SELECT r.timeline_id, r.event_id, r.entity_id AS b FROM event_role r WHERE r.concrete_role = 1
), ev AS (
    SELECT e.timeline_id, e.event_id, e.tick, e.act_classes, i.a, t.b
    FROM event e JOIN initiator i USING (timeline_id, event_id) JOIN target t USING (timeline_id, event_id)
), e1 AS (
    SELECT ev.* FROM ev
    JOIN event_snapshot s ON s.timeline_id = ev.timeline_id AND s.event_id = ev.event_id
                          AND s.entity_id = ev.b AND s.partner_id = ev.a AND s.attr_id = 1 /* trust */
    WHERE has_act(ev.act_classes, (1<<1) | (1<<2)) AND s.value >= 154
), e2 AS (
    SELECT ev.* FROM ev WHERE has_act(ev.act_classes, (1<<0) | (1<<3) | (1<<5))
)
SELECT e1.timeline_id, e1.a, e1.b, e1.event_id AS e1_id, e2.event_id AS e2_id
FROM e1 JOIN e2
  ON e2.timeline_id = e1.timeline_id AND e2.a = e1.a AND e2.b = e1.b
 AND e2.event_id > e1.event_id AND e2.tick <= e1.tick + 3650 * 1440
WHERE NOT EXISTS (
    SELECT 1 FROM ev r
    WHERE r.timeline_id = e1.timeline_id AND r.a = e1.b AND r.b = e1.a
      AND has_act(r.act_classes, 1<<1)
      AND r.event_id > e1.event_id AND r.event_id < e2.event_id
);

-- Causal ancestors (for hindsight pinning; R02 §3 "Labov pre-construction"): contingent + motivated_by edges.
-- Usage: SELECT * FROM causal_ancestors WHERE root = <event_id>;
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
CREATE OR REPLACE MACRO title_holder_at(tl, title_attr, t) AS TABLE
SELECT entity_id, value FROM state_interval
WHERE timeline_id = tl AND attr_id = title_attr AND valid_from <= t AND (valid_to IS NULL OR valid_to > t);
