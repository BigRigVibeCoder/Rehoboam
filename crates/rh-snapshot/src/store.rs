//! The keyframe store's policy: which keyframes a run keeps (SPR-007 T-02; Architect ruling 4 on
//! the T-02 plan; `05.2` §6: "permanent every 10 sim-years, plus a yearly LRU near the cursor").
//!
//! [`KeyframeStore`] decides; the host writes, places and deletes the files. A run takes a yearly
//! keyframe; a year that is a multiple of the permanent cadence is kept for good, and every other
//! year enters a least-recently-used cache of fixed capacity, which evicts the least recently
//! admitted or loaded year. Loading a keyframe ([`KeyframeStore::touch`]) makes it the most recent,
//! so the years near the cursor stay. Nothing here reads the clock: recency is the order of calls.
//!
//! REF: `docs/05_architecture/05.2_world_model_and_data.md` §6; `docs/09` V-DET-05;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02.

use std::collections::BTreeSet;

/// What [`KeyframeStore::admit`] did with a year's keyframe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Kept for good: the year is on the permanent cadence.
    Permanent,
    /// Kept in the yearly cache; the host deletes the evicted year's keyframe, if any.
    Cached {
        /// The year the cache dropped to make room.
        evicted: Option<i64>,
    },
}

/// The keep-and-evict policy for one timeline's keyframes, by sim-year.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyframeStore {
    permanent_every: i64,
    capacity: usize,
    permanent: BTreeSet<i64>,
    /// The cached years, least recently used first.
    cache: Vec<i64>,
}

impl KeyframeStore {
    /// The permanent cadence of `05.2` §6, in sim-years.
    pub const PERMANENT_EVERY_YEARS: u32 = 10;
    /// The yearly cache's capacity when the host sets none (`05.2` names no number; P1 measures
    /// keyframe sizes, `08`).
    pub const DEFAULT_CAPACITY: usize = 8;

    /// A store keeping every `permanent_every_years`-th year (at least 1) and up to `capacity`
    /// other years.
    #[must_use]
    pub fn new(permanent_every_years: u32, capacity: usize) -> Self {
        Self {
            permanent_every: i64::from(permanent_every_years.max(1)),
            capacity,
            permanent: BTreeSet::new(),
            cache: Vec::new(),
        }
    }

    /// Whether `year` is on the permanent cadence (year 0 is).
    #[must_use]
    pub fn is_permanent_year(&self, year: i64) -> bool {
        year.rem_euclid(self.permanent_every) == 0
    }

    /// Takes the keyframe of `year`. A year already held is only touched.
    pub fn admit(&mut self, year: i64) -> Admission {
        if self.is_permanent_year(year) {
            self.permanent.insert(year);
            return Admission::Permanent;
        }
        if self.touch(year) {
            return Admission::Cached { evicted: None };
        }
        if self.capacity == 0 {
            return Admission::Cached {
                evicted: Some(year),
            };
        }
        let evicted = (self.cache.len() == self.capacity).then(|| self.cache.remove(0));
        self.cache.push(year);
        Admission::Cached { evicted }
    }

    /// Marks a cached year as just used (a load). Returns whether the year is cached.
    pub fn touch(&mut self, year: i64) -> bool {
        match self.cache.iter().position(|&y| y == year) {
            Some(k) => {
                let y = self.cache.remove(k);
                self.cache.push(y);
                true
            }
            None => false,
        }
    }

    /// Whether the keyframe of `year` is held, permanent or cached.
    #[must_use]
    pub fn holds(&self, year: i64) -> bool {
        self.permanent.contains(&year) || self.cache.contains(&year)
    }

    /// The latest held year at or before `year`: where a seek to `year` starts replaying.
    #[must_use]
    pub fn nearest_at_or_before(&self, year: i64) -> Option<i64> {
        let permanent = self.permanent.range(..=year).next_back().copied();
        let cached = self.cache.iter().copied().filter(|&y| y <= year).max();
        permanent.max(cached)
    }

    /// The permanent years, ascending.
    pub fn permanent(&self) -> impl Iterator<Item = i64> + '_ {
        self.permanent.iter().copied()
    }

    /// The cached years, least recently used first.
    #[must_use]
    pub fn cached(&self) -> &[i64] {
        &self.cache
    }
}

impl Default for KeyframeStore {
    fn default() -> Self {
        Self::new(Self::PERMANENT_EVERY_YEARS, Self::DEFAULT_CAPACITY)
    }
}
