//! `rh-sifter` v0 on the canonical golden segment and on planted packs (SPR-013 T-11).
//!
//! - The golden channel-A segment of `specs/stream/golden/canonical_v0.txt`, sifted with
//!   `life_ends`, gives the golden channel-C file's two `life_ends` matches: their `MatchId`s,
//!   closing seqs, bindings and events (the `match_id` vectors `rh-sifter` must reproduce,
//!   `canonical_v0.md` §7).
//! - A `DEATH` with an actor and a target completes one match binding the target (only a witness
//!   may repeat in a cast, `envelope_v0.md` §1, so a role binds one entity); an actor constraint
//!   binds the actor; two deaths complete two matches in seq order; an event of another verb
//!   completes none.
//! - v0 refuses what it cannot honour (a second clause, `where`, `dedupe`, `unless`, an unknown
//!   role, no budget) instead of ignoring it.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use rh_sifter::pattern::compile_pack;
use rh_sifter::sift::sift_segment;
use rh_stream::canonical::parse;
use rh_stream::record::{Body, Record};
use rh_stream::segment::{Dictionaries, Provenance, SegmentA, SegmentRow};

const GOLDEN: &str = include_str!("../../../specs/stream/golden/canonical_v0.txt");
const PACK: &str = include_str!("../../../specs/sifting/patterns.solitary_life.toml");

fn segment(records: Vec<Record>) -> SegmentA {
    SegmentA {
        row: SegmentRow {
            timeline_id: 0,
            sim_year: 0,
            first_seq: 0,
            last_seq: 0,
            record_count: 0,
            first_tick: 0,
            last_tick: 0,
            provenance: Provenance {
                engine_build: String::new(),
                manifest_hash: Vec::new(),
                world_seed: 0,
                built_systems: Vec::new(),
                not_built: Vec::new(),
            },
            registry_version: String::new(),
            canonical_digest: [0; 16],
        },
        dictionaries: Dictionaries {
            registry_version: String::new(),
            verbs: Vec::new(),
            act_classes: (0..34)
                .map(|i| {
                    if i == 15 {
                        "death".to_owned()
                    } else {
                        format!("c{i}")
                    }
                })
                .collect(),
            event_roles: Vec::new(),
            motive_edges: Vec::new(),
            attributes: Vec::new(),
        },
        records,
    }
}

fn golden_block(head: &str) -> Vec<&'static str> {
    let lines: Vec<&str> = GOLDEN.lines().collect();
    let i = lines.iter().position(|l| l.starts_with(head)).unwrap();
    let n: usize = lines[i].rsplit(' ').next().unwrap().parse().unwrap();
    lines[i + 1..=i + n].to_vec()
}

#[test]
fn the_golden_segment_sifts_to_the_golden_life_ends_matches() {
    let patterns = compile_pack("solitary_life", PACK).unwrap();
    assert_eq!(patterns.len(), 1);
    let a: Vec<Record> = golden_block("segment 0 A 0")
        .iter()
        .map(|l| parse(l).unwrap())
        .collect();
    let got = sift_segment(&segment(a), &patterns, 0);
    let want: Vec<Record> = golden_block("segment 0 C 0")
        .iter()
        .map(|l| parse(l).unwrap())
        .filter(|r| matches!(&r.body, Body::Match(m) if m.pattern_id == "life_ends"))
        .collect();
    assert_eq!(got.len(), 2);
    for (g, w) in got.iter().zip(&want) {
        let (Body::Match(g), Body::Match(w)) = (&g.body, &w.body) else {
            panic!()
        };
        assert_eq!(
            (
                g.match_id,
                g.closed_by_seq,
                g.closed_by_event_id,
                &g.bindings,
                &g.events
            ),
            (
                w.match_id,
                w.closed_by_seq,
                w.closed_by_event_id,
                &w.bindings,
                &w.events
            )
        );
    }
    println!(
        "T-11: the golden segment sifts to the golden life_ends matches ce0cbf3420904cae, ffacbd9f4e37aed5"
    );
}

#[test]
fn each_death_completes_one_match_binding_its_target() {
    let patterns = compile_pack("solitary_life", PACK).unwrap();
    let murder = "EVENT 0 5 100 7 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/77/0,1/12/1] [] []";
    let natural = "EVENT 0 6 100 8 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/900/1] [] []";
    let other = "EVENT 0 7 100 9 4 7 ISSUE_DECREE 16 ~ 0 96 0 0 [] [1/900/0] [] []";
    let records = || {
        [murder, natural, other]
            .iter()
            .map(|l| parse(l).unwrap())
            .collect::<Vec<_>>()
    };
    let got = sift_segment(&segment(records()), &patterns, 4);
    let ids: Vec<(u64, u64)> = got
        .iter()
        .map(|r| match &r.body {
            Body::Match(m) => (r.header.seq, m.bindings[0].entity_id),
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        ids,
        vec![(4, 12), (5, 900)],
        "one match per death, binding its target, in seq order; the actor and the decree bind none"
    );
    let actor = PACK.replace("target = \"A\"", "actor = \"A\"");
    let p = compile_pack("solitary_life", &actor).unwrap();
    let got = sift_segment(&segment(records()), &p, 0);
    assert_eq!(got.len(), 1, "only the murder has an actor");
}

#[test]
fn what_v0_cannot_honour_is_refused() {
    let base = PACK.to_owned();
    let cases = [
        (
            "a second clause",
            format!("{base}\n  [[pattern.clause]]\n  bind = \"e2\"\n  verb = \"DEATH\"\n"),
        ),
        (
            "where",
            base.replace(
                "target = \"A\"",
                "target = \"A\"\n  where = [\"age(A)@e1 >= 60\"]",
            ),
        ),
        (
            "dedupe",
            base.replace(
                "on_complete",
                "dedupe = { key = [\"A\"], bucket_days = 30 }\non_complete",
            ),
        ),
        (
            "unless",
            format!("{base}\n  [[pattern.unless]]\n  bind = \"u1\"\n  verb = \"DEATH\"\n"),
        ),
        (
            "unknown role",
            base.replace("target = \"A\"", "mourner = \"A\""),
        ),
        (
            "no budget",
            base.replace("firing_budget = { per_century_per_100k = 333333 }", ""),
        ),
        (
            "an unbound variable",
            base.replace("target = \"A\"", "target = \"B\""),
        ),
        (
            "a role domain",
            base.replace("roles = { A = \"any\" }", "roles = { A = \"authority\" }"),
        ),
        (
            "a pattern-level within_days",
            base.replace("on_complete", "within_days = 365\non_complete"),
        ),
        (
            "a pattern-level require_edge",
            base.replace(
                "on_complete",
                "require_edge = { into = \"e1\", kind = \"motivated_by\" }\non_complete",
            ),
        ),
        (
            "a declared role no clause binds",
            base.replace(
                "roles = { A = \"any\" }",
                "roles = { A = \"any\", B = \"any\" }",
            ),
        ),
    ];
    for (what, text) in cases {
        assert_ne!(text, base, "{what}: the plant did not apply");
        let err = compile_pack("solitary_life", &text).expect_err(what);
        let message = err.to_string();
        let named = match what {
            "a role domain" => "role domain `A",
            "a pattern-level within_days" => "`within_days` is not supported",
            "a pattern-level require_edge" => "`require_edge` is not supported",
            "a declared role no clause binds" => "role `B` is declared but no clause binds it",
            _ => "",
        };
        assert!(
            message.contains(named),
            "{what}: refused for another reason: {message}"
        );
    }
}
