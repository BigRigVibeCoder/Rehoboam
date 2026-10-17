-- The SQL twin of pack solitary_life (specs/sifting/patterns.solitary_life.toml), DuckDB dialect (SIFT-3; SPR-013 T-11).
-- Pattern life_ends v1: one clause, verb DEATH, the target bound as A; selection latest, on_complete clear_binding (one clause:
-- no partial match is ever open, so every DEATH completes one match per target entity).
-- Input: views event, event_role and verb over a run's sealed channel-A segments (specs/stream/channel_a_v0.sql), which
-- specs/sifting/twin_check.py creates. Output: one row per match, in channel C's R4 order (closing seq, then the binding),
-- with the columns twin_check.py turns into MatchIds (specs/stream/canonical_v0.md §6) and compares with rh-sifter's
-- channel C (V-SIFT-01: match ids, bindings, event ids).
SELECT 'solitary_life'  AS pack,
       'life_ends'      AS pattern_id,
       1                AS pattern_version,
       e.seq            AS closed_by_seq,
       e.tick           AS tick,
       e.event_id       AS event_id,
       'A'              AS role,
       r.entity_kind    AS entity_kind,
       r.entity_id      AS entity_id
FROM event e
JOIN (SELECT DISTINCT verb_id, name FROM verb) v ON v.verb_id = e.verb_id
JOIN event_role r ON r.timeline_id = e.timeline_id AND r.event_id = e.event_id
WHERE v.name = 'DEATH' AND r.role = 1          -- [event_roles] index 1: target (SPR-011 ruling 11)
ORDER BY e.seq, r.entity_kind, r.entity_id;
