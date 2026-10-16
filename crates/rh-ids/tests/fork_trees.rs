//! V-DET-16 (branch salts, DET-2): a property test over random fork trees (SPR-002 T-04).
//!
//! `docs/09` V-DET-16: "No two rollouts share a salt; `resample_on_fork` purposes re-draw after a
//! fork and others are shared; a what-if fork without overrides matches its parent's hashes."
//!
//! Each tree grows from the canonical timeline by forking a random existing timeline at a random
//! tick with a random policy (what-if, rollout, reroll fate); child timeline ids are allocated
//! uniquely, as the Timeline Manager does. The trees are drawn from a keyed stream, so the test is
//! itself deterministic; a failure names its tree seed.

#![allow(
    clippy::expect_used,
    reason = "test helpers: a refused fork must fail the test loudly"
)]

use std::collections::BTreeMap;

use rh_ids::rng::h64_words;
use rh_ids::{
    BranchSalt, CANONICAL_SALT, ForkPolicy, LightStream, Purpose, PurposeId, RngKey, SaltLineage,
    SystemId, TimelineId, h,
};

const TREES: u64 = 300;
const MAX_FORKS: u64 = 40;
const HORIZON: u64 = 1_000;
const ENTITIES: u64 = 4;
const VITAL: Purpose = Purpose::new("death_time", true);
const SHARED: Purpose = Purpose::new("activity_block", false);
const WORLD_SEED: u64 = 0x5EED;

struct Node {
    parent: usize,
    at: i64,
    policy: ForkPolicy,
    lineage: SaltLineage,
}

/// A deterministic random fork tree. Node 0 is the canonical timeline.
fn tree(seed: u64) -> Vec<Node> {
    let pick = LightStream::from_parts(
        seed,
        CANONICAL_SALT,
        SystemId::from_raw(0),
        PurposeId::of("test_fork_tree"),
        0,
        0,
    );
    let mut k = 0_u64;
    let mut next = |n: u64| {
        k += 1;
        pick.draw(k) % n
    };
    let mut nodes = vec![Node {
        parent: 0,
        at: 0,
        policy: ForkPolicy::WHAT_IF,
        lineage: SaltLineage::canonical(),
    }];
    let forks = 1 + next(MAX_FORKS);
    for child in 1..=forks {
        let parent = usize::try_from(next(nodes.len() as u64)).expect("index");
        let at = i64::try_from(next(HORIZON)).expect("tick");
        let policy = [
            ForkPolicy::WHAT_IF,
            ForkPolicy::ROLLOUT,
            ForkPolicy::REROLL_FATE,
        ][usize::try_from(next(3)).expect("index")];
        let id = TimelineId::from_raw(u32::try_from(child).expect("id"));
        let lineage = nodes[parent]
            .lineage
            .fork(at, id, policy)
            .expect("fork accepted");
        nodes.push(Node {
            parent,
            at,
            policy,
            lineage,
        });
    }
    nodes
}

fn draw(salt: BranchSalt, purpose: &Purpose, entity: u64, t: i64) -> u64 {
    let key = RngKey {
        world_seed: WORLD_SEED,
        branch_salt: salt,
        system: SystemId::from_raw(5),
        purpose: purpose.id(),
        entity,
        t,
    };
    h(&key, 0)
}

/// A stand-in for a plane hash: every draw of every entity and purpose at the sampled ticks.
fn draw_hash(lineage: &SaltLineage) -> u64 {
    let mut words = Vec::new();
    for t in (0..i64::try_from(HORIZON).expect("tick")).step_by(37) {
        for e in 0..ENTITIES {
            for p in [&VITAL, &SHARED] {
                words.push(draw(lineage.branch_salt(t), p, e, t));
            }
        }
    }
    h64_words(0, &words)
}

/// "No two rollouts share a salt": distinct lineage paths (every prefix of every timeline) map to
/// distinct salts, and equal paths to equal salts.
#[test]
fn distinct_paths_have_distinct_salts() {
    for seed in 0..TREES {
        let nodes = tree(seed);
        let mut by_salt: BTreeMap<u64, Vec<(i64, u32)>> = BTreeMap::new();
        for node in &nodes {
            let steps = node.lineage.steps();
            for len in 0..=steps.len() {
                let path: Vec<(i64, u32)> = steps[..len]
                    .iter()
                    .map(|s| (s.at, s.timeline.to_raw()))
                    .collect();
                let t = if len == 0 {
                    i64::MIN
                } else {
                    steps[len - 1].at
                };
                // The salt at t covers exactly this prefix only if no later step shares its tick.
                if steps.get(len).is_some_and(|s| s.at == t) {
                    continue;
                }
                let salt = node.lineage.branch_salt(t).to_raw();
                let seen = by_salt.entry(salt).or_insert_with(|| path.clone());
                assert_eq!(
                    *seen, path,
                    "tree {seed}: two different paths share salt {salt:#x}"
                );
            }
        }
        // Every extending fork's own salt is new: at its fork tick it differs from the salt of
        // every timeline whose covering path differs (descendants share it over shared history).
        for (i, a) in nodes.iter().enumerate().skip(1) {
            if a.policy.keep_parent_salt {
                continue;
            }
            for (j, b) in nodes.iter().enumerate() {
                if covering(&b.lineage, a.at) != covering(&a.lineage, a.at) {
                    assert_ne!(
                        a.lineage.branch_salt(a.at),
                        b.lineage.branch_salt(a.at),
                        "tree {seed}: rollouts {i} and {j} share a salt at {}",
                        a.at
                    );
                }
            }
        }
    }
}

/// The steps of `lineage` covering tick `t`.
fn covering(lineage: &SaltLineage, t: i64) -> Vec<(i64, u32)> {
    lineage
        .steps()
        .iter()
        .take_while(|s| s.at <= t)
        .map(|s| (s.at, s.timeline.to_raw()))
        .collect()
}

/// A what-if fork without overrides keeps every salt, so every draw, of its parent.
#[test]
fn what_if_forks_match_their_parent() {
    for seed in 0..TREES {
        let nodes = tree(seed);
        for node in nodes.iter().skip(1).filter(|n| n.policy.keep_parent_salt) {
            let parent = &nodes[node.parent];
            assert_eq!(node.lineage, parent.lineage, "tree {seed}");
            assert_eq!(
                draw_hash(&node.lineage),
                draw_hash(&parent.lineage),
                "tree {seed}"
            );
        }
    }
}

/// History before a fork is shared; `resample_on_fork` purposes re-draw pending pre-fork draws
/// after an extending fork, and other purposes keep the parent's value.
#[test]
fn presampled_draws_follow_resample_on_fork() {
    for seed in 0..TREES {
        let nodes = tree(seed);
        for node in nodes.iter().skip(1).filter(|n| !n.policy.keep_parent_salt) {
            let parent = &nodes[node.parent];
            for sampled_at in [0, node.at / 2, node.at - 1]
                .into_iter()
                .filter(|&s| s < node.at)
            {
                assert_eq!(
                    node.lineage.branch_salt(sampled_at),
                    parent.lineage.branch_salt(sampled_at),
                    "tree {seed}: history before the fork at {} is not shared",
                    node.at
                );
                for e in 0..ENTITIES {
                    let now = node.at;
                    let child_salt = |p: &Purpose| node.lineage.presampled_salt(p, sampled_at, now);
                    let parent_salt =
                        |p: &Purpose| parent.lineage.presampled_salt(p, sampled_at, now);
                    assert_eq!(
                        draw(child_salt(&SHARED), &SHARED, e, sampled_at),
                        draw(parent_salt(&SHARED), &SHARED, e, sampled_at),
                        "tree {seed}: a non-resample purpose was re-drawn"
                    );
                    assert_ne!(
                        draw(child_salt(&VITAL), &VITAL, e, sampled_at),
                        draw(parent_salt(&VITAL), &VITAL, e, sampled_at),
                        "tree {seed}: a resample_on_fork purpose was not re-drawn"
                    );
                }
            }
        }
    }
}
