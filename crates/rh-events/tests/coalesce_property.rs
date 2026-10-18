//! SPR-020 T-02: coalescing over random contributor sets equals a reference written here from
//! `envelope_v0.md` §5's text (as amended by REV-SPR-029 R20), not from the code.
//!
//! The reference, read off §5 sentence by sentence:
//! 1. "The key is `(effect_id, cause_id)`": group the contributors by cause.
//! 2. "The kind is the first, in this precedence, among the contributors' kinds: `motivated_by`,
//!    contingent, triggered, enabled, probabilistic, `relayed_knowledge`."
//! 3. "The weight is the saturating `Q8_8` sum of the contributors' weights when the kept kind is
//!    `motivated_by`, and 0 otherwise. The exact sum is computed in `i64` and saturated once."
//! 4. "The motive takes the sign of the summed weight ... positive: the motivated contributor with
//!    the largest positive weight; negative: the contributor with the most negative weight; 0:
//!    NULL. A tie goes to the lower `[motive_edges]` index, and an unnamed contributor counts as the
//!    highest index."
//! 5. "The evidence is the largest among the contributors of the kept kind."
//! 6. "After coalescing, edges are ordered by `cause_id` ascending."
//!
//! Sizes and seeds vary (three contributor-set sizes, eight seeds each, many trials per seed).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "a test: the generator narrows on purpose and a failure must be loud"
)]

use rh_events::{Edge, EdgeKind, coalesce};
use rh_ids::EventId;
use rh_ids::rng::{h64_words, tag};

const MOTIVES: [&str; 9] = [
    "RETALIATION_FOR",
    "JEALOUSY_OF",
    "VOW_OF",
    "AMBITION_FOR",
    "FEAR_OF",
    "LOYALTY_TO",
    "GRIEF_FOR",
    "GREED_FOR",
    "CORRUPTION_BY",
];

/// §5's precedence, by kind code: `motivated_by` 3, `contingent` 0, `triggered` 1, `enabled` 4,
/// `probabilistic` 5, `relayed_knowledge` 2.
const PRECEDENCE_CODES: [u8; 6] = [3, 0, 1, 4, 5, 2];

fn motive_index(m: &str) -> Option<usize> {
    MOTIVES.iter().position(|x| *x == m)
}

/// The reference: one coalesced `(cause, kind code, motive, weight, evidence)` per cause.
fn reference(edges: &[Edge]) -> Vec<(u64, u8, Option<&'static str>, i16, i16)> {
    let mut causes: Vec<u64> = edges.iter().map(|e| e.cause.to_raw()).collect();
    causes.sort_unstable();
    causes.dedup();
    let mut out = Vec::new();
    for cause in causes {
        let group: Vec<&Edge> = edges.iter().filter(|e| e.cause.to_raw() == cause).collect();
        let kind = *PRECEDENCE_CODES
            .iter()
            .find(|code| group.iter().any(|e| e.kind.code() == **code))
            .expect("a writable kind");
        let mut evidence = i16::MIN;
        for e in &group {
            if e.kind.code() == kind && e.evidence > evidence {
                evidence = e.evidence;
            }
        }
        if kind != 3 {
            out.push((cause, kind, None, 0, evidence));
            continue;
        }
        let mut exact: i64 = 0;
        for e in &group {
            exact += i64::from(e.weight);
        }
        let weight = exact.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
        // The candidates in the sum's direction, as (strength, index, motive).
        let index = |e: &Edge| match e.motive {
            None => MOTIVES.len(),
            Some(m) => motive_index(m).unwrap_or(MOTIVES.len()),
        };
        let mut best: Option<(i64, usize, Option<&'static str>)> = None;
        for e in group.iter().filter(|e| e.kind.code() == 3) {
            let strength = i64::from(e.weight) * exact.signum();
            if exact == 0 || strength <= 0 {
                continue;
            }
            let cand = (strength, index(e), e.motive);
            best = match best {
                None => Some(cand),
                Some(b) if cand.0 > b.0 || (cand.0 == b.0 && cand.1 < b.1) => Some(cand),
                keep => keep,
            };
        }
        out.push((cause, kind, best.and_then(|b| b.2), weight, evidence));
    }
    out
}

struct Gen {
    seed: u64,
    k: u64,
}

impl Gen {
    fn next(&mut self) -> u64 {
        self.k += 1;
        h64_words(tag("rehoboam/test/coalesce"), &[self.seed, self.k])
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn weight(&mut self) -> i16 {
        // Boundaries often: the extremes, 0, ±1, and ties.
        match self.below(6) {
            0 => i16::MAX,
            1 => i16::MIN,
            2 => 0,
            3 => [1_i16, -1, 100, -100][self.below(4) as usize],
            _ => self.next() as u16 as i16,
        }
    }
}

fn contributor(g: &mut Gen, causes: u64) -> Edge {
    let cause = EventId::from_raw(g.below(causes));
    let code = [3_u8, 3, 3, 0, 1, 4, 5, 2][g.below(8) as usize];
    let kind = EdgeKind::from_code(code).expect("a kind");
    if kind == EdgeKind::MotivatedBy {
        let motive = match g.below(10) {
            9 => None,
            i => Some(MOTIVES[i as usize % MOTIVES.len()]),
        };
        let evidence = g.below(257) as i16;
        return Edge {
            cause,
            kind,
            motive,
            weight: g.weight(),
            evidence,
        };
    }
    let evidence = if kind.is_deterministic() {
        256
    } else {
        g.below(257) as i16
    };
    Edge {
        cause,
        kind,
        motive: None,
        weight: 0,
        evidence,
    }
}

#[test]
fn coalescing_equals_the_reference_from_the_text() {
    let mut checked = 0_u64;
    for (size, causes) in [(2_u64, 1_u64), (6, 3), (24, 5)] {
        for seed in 1..=8_u64 {
            let mut g = Gen {
                seed: seed * 1000 + size,
                k: 0,
            };
            for _ in 0..400 {
                let n = 1 + g.below(size);
                let edges: Vec<Edge> = (0..n).map(|_| contributor(&mut g, causes)).collect();
                let got =
                    coalesce("X", edges.clone(), motive_index).unwrap_or_else(|e| panic!("{e}"));
                let got: Vec<_> = got
                    .iter()
                    .map(|e| {
                        (
                            e.cause.to_raw(),
                            e.kind.code(),
                            e.motive,
                            e.weight,
                            e.evidence,
                        )
                    })
                    .collect();
                assert_eq!(got, reference(&edges), "size {size} seed {seed}: {edges:?}");
                // Independent of the contributors' order.
                let mut rev = edges.clone();
                rev.reverse();
                let again: Vec<_> = coalesce("X", rev, motive_index)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .iter()
                    .map(|e| {
                        (
                            e.cause.to_raw(),
                            e.kind.code(),
                            e.motive,
                            e.weight,
                            e.evidence,
                        )
                    })
                    .collect();
                assert_eq!(again, got, "order dependence: {edges:?}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 3 * 8 * 400);
}
