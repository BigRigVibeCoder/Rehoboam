//! `rh-sifter` v0 on the canonical golden segment and on planted packs (SPR-013 T-11).
//!
//! - The golden channel-A segment of `specs/stream/golden/canonical_v0.txt`, sifted with
//!   `life_ends`, gives the golden channel-C file's two `life_ends` matches: their `MatchId`s,
//!   closing seqs, bindings and events (the `match_id` vectors `rh-sifter` must reproduce,
//!   `canonical_v0.md` §7).
//! - A `DEATH` with two targets completes two matches, in bytewise binding order; an actor
//!   constraint binds the actor; an event of another verb completes none.
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
fn several_targets_complete_several_matches_in_binding_order() {
    let patterns = compile_pack("solitary_life", PACK).unwrap();
    let line = "EVENT 0 5 100 7 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/900/1,1/12/1,1/77/0] []";
    let other = "EVENT 0 6 100 8 4 7 ISSUE_DECREE 16 ~ 0 96 0 0 [] [1/900/1] []";
    let got = sift_segment(
        &segment(vec![parse(line).unwrap(), parse(other).unwrap()]),
        &patterns,
        4,
    );
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
        "two targets, ordered by binding; the actor is not a target"
    );
    let actor = PACK.replace("target = \"A\"", "actor = \"A\"");
    let p = compile_pack("solitary_life", &actor).unwrap();
    let got = sift_segment(&segment(vec![parse(line).unwrap()]), &p, 0);
    assert_eq!(got.len(), 1);
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
    ];
    for (what, text) in cases {
        assert!(
            compile_pack("solitary_life", &text).is_err(),
            "{what} accepted"
        );
    }
}
