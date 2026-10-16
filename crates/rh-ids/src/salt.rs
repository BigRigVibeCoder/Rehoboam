//! Branch salts and salt lineage (SPR-002 T-04).
//!
//! `branch_salt(t) = H(the lineage path of the forks covering t)` (`05.1` §6.9; ADR-0006 v1.1
//! item 3). A timeline's lineage is the ordered list of the forks that made it, each a
//! [`ForkStep`] `(at, timeline)`; the forks *covering* `t` are those with `at <= t`. The canonical
//! timeline's path is empty.
//!
//! # Layout, version 1
//!
//! `branch_salt(t) = h64_words(tag("rehoboam/rh-ids/salt/v1"), [at₁, timeline₁, at₂, timeline₂, …])`
//! over the covering steps in order, `at` as two's-complement `u64` and `timeline` widened from
//! `u32` (the `SplitMix64` hash of [`crate::rng`]). [`CANONICAL_SALT`] is the empty path's salt.
//!
//! # Uniqueness
//!
//! Each extending fork appends a step naming the child's [`TimelineId`], which the Timeline
//! Manager allocates uniquely, so two different timelines never have the same path. Distinct paths
//! give distinct salts up to a 64-bit hash collision (2⁻⁶⁴ per pair; the V-DET-16 property test
//! checks random fork trees).
//!
//! # Fork policies (`05.9` IF-1 `ForkPolicy`)
//!
//! - **What-if** ([`ForkPolicy::WHAT_IF`]): keeps the parent's lineage, so every draw, and so every
//!   hash, matches the parent until an override input changes state (a clean counterfactual).
//! - **Rollout** and **reroll fate** ([`ForkPolicy::ROLLOUT`], [`ForkPolicy::REROLL_FATE`]): extend
//!   the lineage and re-draw pre-sampled draws whose purpose is flagged `resample_on_fork`
//!   ([`SaltLineage::presampled_salt`]); other pre-sampled draws are shared with the parent.
//!
//! REF: `05.1` §6.9; ADR-0006 v1.1 item 3; ADR-0015 item 2; ADR-0020 item 1; `05.9` IF-1; V-DET-16.

use core::fmt;

use crate::hash::BranchSalt;
use crate::ids::TimelineId;
use crate::rng::{Purpose, h64_words, tag};

/// Domain of branch salts.
pub const TAG_SALT: u64 = tag("rehoboam/rh-ids/salt/v1");

/// The canonical timeline's salt: the hash of the empty path.
pub const CANONICAL_SALT: BranchSalt = BranchSalt::from_raw(h64_words(TAG_SALT, &[]));

/// Words hashed per fork step.
const WORDS_PER_STEP: usize = 2;

/// One fork in a lineage: the tick it forked at and the timeline it created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForkStep {
    /// The fork tick: draws at `t >= at` use the extended salt.
    pub at: i64,
    /// The timeline the fork created (unique, allocated by the Timeline Manager).
    pub timeline: TimelineId,
    /// Whether this fork re-draws pre-sampled `resample_on_fork` purposes.
    pub resample: bool,
}

/// How a fork treats randomness (`05.9` IF-1 `ForkPolicy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForkPolicy {
    /// Keep the parent's salt (what-if A/B with common random numbers).
    pub keep_parent_salt: bool,
    /// Re-draw pre-sampled draws whose purpose is flagged `resample_on_fork`.
    pub resample_on_fork: bool,
}

impl ForkPolicy {
    /// A what-if fork: the parent's salt, nothing re-drawn.
    pub const WHAT_IF: Self = Self {
        keep_parent_salt: true,
        resample_on_fork: false,
    };
    /// A Foresight rollout: a new salt, `resample_on_fork` purposes re-drawn (ADR-0015 item 2).
    pub const ROLLOUT: Self = Self {
        keep_parent_salt: false,
        resample_on_fork: true,
    };
    /// A what-if with "reroll fate": a new salt, `resample_on_fork` purposes re-drawn.
    pub const REROLL_FATE: Self = Self {
        keep_parent_salt: false,
        resample_on_fork: true,
    };
}

/// Why a fork was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkError {
    /// `keep_parent_salt` with `resample_on_fork`: a re-draw under the same salt reproduces the
    /// same values, so the request cannot mean what it says.
    ResampleWithParentSalt,
    /// The child timeline already appears in the lineage; the path would not be unique.
    TimelineInLineage(TimelineId),
}

impl fmt::Display for ForkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResampleWithParentSalt => {
                f.write_str("fork policy keeps the parent salt but asks to re-draw: a no-op")
            }
            Self::TimelineInLineage(id) => {
                write!(
                    f,
                    "timeline {id} is already in the lineage; salts would not be unique"
                )
            }
        }
    }
}

// DECISION: a hand-written error rather than `thiserror` for two variants, keeping this C3
// machinery crate's dependency graph to the RNG building blocks.
impl std::error::Error for ForkError {}

/// The fork lineage of one timeline (`05.9` IF-1 `SaltLineage`). Empty = canonical.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaltLineage {
    steps: Vec<ForkStep>,
}

impl SaltLineage {
    /// The canonical timeline's lineage (empty path).
    #[must_use]
    pub const fn canonical() -> Self {
        Self { steps: Vec::new() }
    }

    /// The fork steps, oldest first.
    #[must_use]
    pub fn steps(&self) -> &[ForkStep] {
        &self.steps
    }

    /// `branch_salt(t)`: the hash of the steps with `at <= t`.
    #[must_use]
    pub fn branch_salt(&self, t: i64) -> BranchSalt {
        // INVARIANT: steps are sorted by `at` (non-decreasing): `fork` keeps a prefix with
        // `at <= fork tick` and appends the fork tick. So the covering steps are a prefix.
        let covering = self.steps.iter().take_while(|s| s.at <= t);
        let mut words = Vec::with_capacity(self.steps.len() * WORDS_PER_STEP);
        for s in covering {
            words.push(s.at.cast_unsigned());
            words.push(u64::from(s.timeline.to_raw()));
        }
        BranchSalt::from_raw(h64_words(TAG_SALT, &words))
    }

    /// The lineage of a new timeline `child` forked from this one at tick `at`.
    ///
    /// With `keep_parent_salt` the child's lineage is this one, unchanged, so its salts match the
    /// parent's at every tick. Otherwise the steps after `at` are dropped (forking before this
    /// timeline's own fork point shares only the ancestor's history) and the step `(at, child)`
    /// is appended.
    ///
    /// # Errors
    ///
    /// [`ForkError::ResampleWithParentSalt`] for a contradictory policy;
    /// [`ForkError::TimelineInLineage`] if `child` already names a step of this lineage.
    pub fn fork(
        &self,
        at: i64,
        child: TimelineId,
        policy: ForkPolicy,
    ) -> Result<SaltLineage, ForkError> {
        if policy.keep_parent_salt && policy.resample_on_fork {
            return Err(ForkError::ResampleWithParentSalt);
        }
        if self.steps.iter().any(|s| s.timeline == child) {
            return Err(ForkError::TimelineInLineage(child));
        }
        if policy.keep_parent_salt {
            return Ok(self.clone());
        }
        let mut steps: Vec<ForkStep> = self
            .steps
            .iter()
            .copied()
            .take_while(|s| s.at <= at)
            .collect();
        steps.push(ForkStep {
            at,
            timeline: child,
            resample: policy.resample_on_fork,
        });
        Ok(Self { steps })
    }

    /// The salt of a draw of `purpose` sampled at `sampled_at`, as seen by this timeline when it
    /// re-evaluates the draw at `now` (a pending timer at a fork tick, or when it fires).
    ///
    /// If `purpose` is flagged `resample_on_fork` and a re-drawing fork lies in
    /// `(sampled_at, now]`, the draw is re-drawn under the salt of the latest such fork;
    /// otherwise it keeps `branch_salt(sampled_at)` and is shared with the parent. The other key
    /// fields (including `t = sampled_at`) do not change.
    #[must_use]
    pub fn presampled_salt(&self, purpose: &Purpose, sampled_at: i64, now: i64) -> BranchSalt {
        if purpose.resample_on_fork() {
            let redraw = self
                .steps
                .iter()
                .rev()
                .find(|s| s.resample && s.at > sampled_at && s.at <= now)
                .map(|s| s.at);
            if let Some(at) = redraw {
                return self.branch_salt(at);
            }
        }
        self.branch_salt(sampled_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VITAL: Purpose = Purpose::new("death_time", true);
    const SHARED: Purpose = Purpose::new("activity_block", false);

    fn tl(n: u32) -> TimelineId {
        TimelineId::from_raw(n)
    }

    #[test]
    fn canonical_salt_is_the_empty_path() {
        let c = SaltLineage::canonical();
        assert_eq!(c.branch_salt(i64::MIN), CANONICAL_SALT);
        assert_eq!(c.branch_salt(i64::MAX), CANONICAL_SALT);
    }

    #[test]
    fn salt_changes_at_the_fork_tick() {
        let child = SaltLineage::canonical()
            .fork(100, tl(1), ForkPolicy::ROLLOUT)
            .unwrap();
        assert_eq!(child.branch_salt(99), CANONICAL_SALT);
        assert_ne!(child.branch_salt(100), CANONICAL_SALT);
        assert_eq!(child.branch_salt(100), child.branch_salt(i64::MAX));
    }

    #[test]
    fn siblings_differ_and_what_if_keeps_the_parent() {
        let c = SaltLineage::canonical();
        let a = c.fork(100, tl(1), ForkPolicy::ROLLOUT).unwrap();
        let b = c.fork(100, tl(2), ForkPolicy::ROLLOUT).unwrap();
        assert_ne!(a.branch_salt(100), b.branch_salt(100));
        let w = a.fork(50, tl(3), ForkPolicy::WHAT_IF).unwrap();
        assert_eq!(w, a);
    }

    #[test]
    fn forking_before_the_own_fork_drops_later_steps() {
        let a = SaltLineage::canonical()
            .fork(100, tl(1), ForkPolicy::ROLLOUT)
            .unwrap();
        let b = a.fork(50, tl(2), ForkPolicy::ROLLOUT).unwrap();
        let direct = SaltLineage::canonical()
            .fork(50, tl(2), ForkPolicy::ROLLOUT)
            .unwrap();
        assert_eq!(b, direct);
    }

    #[test]
    fn contradictory_or_duplicate_forks_are_refused() {
        let a = SaltLineage::canonical()
            .fork(1, tl(1), ForkPolicy::ROLLOUT)
            .unwrap();
        let bad = ForkPolicy {
            keep_parent_salt: true,
            resample_on_fork: true,
        };
        assert_eq!(
            a.fork(2, tl(2), bad),
            Err(ForkError::ResampleWithParentSalt)
        );
        assert_eq!(
            a.fork(2, tl(1), ForkPolicy::ROLLOUT),
            Err(ForkError::TimelineInLineage(tl(1)))
        );
        assert!(
            ForkError::TimelineInLineage(tl(1))
                .to_string()
                .contains("0x1")
        );
    }

    #[test]
    fn presampled_draws_redraw_only_for_flagged_purposes() {
        let c = SaltLineage::canonical();
        let r = c.fork(100, tl(1), ForkPolicy::ROLLOUT).unwrap();
        // Sampled at 40, pending at the fork at 100.
        assert_eq!(r.presampled_salt(&SHARED, 40, 100), c.branch_salt(40));
        assert_eq!(r.presampled_salt(&VITAL, 40, 100), r.branch_salt(100));
        assert_ne!(
            r.presampled_salt(&VITAL, 40, 100),
            c.presampled_salt(&VITAL, 40, 100)
        );
        // Before the fork, or sampled at or after it: nothing to re-draw.
        assert_eq!(r.presampled_salt(&VITAL, 40, 99), c.branch_salt(40));
        assert_eq!(r.presampled_salt(&VITAL, 100, 200), r.branch_salt(100));
        // A what-if re-draws nothing.
        let w = c.fork(100, tl(2), ForkPolicy::WHAT_IF).unwrap();
        assert_eq!(
            w.presampled_salt(&VITAL, 40, 100),
            c.presampled_salt(&VITAL, 40, 100)
        );
    }
}
