//! M2's O(N + L) invariants against the oracle (SPR-038 T-03): the same results, names and texts
//! on random tables, valid ones and ones with one planted violation each, over three sizes and
//! eight seeds per kind of table.
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-03; `crates/rh-sys-demography/src/demography.rs`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test code: a failed comparison must fail loudly"
)]

use rh_core::world::WorldTables;
use rh_sys_demography::demography::{
    LC_BIRTH, LC_EXIT_KIND, LC_PERSON, PERSON_ALIVE, PERSON_BIRTH, PERSON_DEATH_TICK, PERSON_ID,
    invariants, invariants_oracle,
};

/// A fixed-seed `SplitMix64`.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n.max(1)).unwrap()).unwrap()
    }
}

/// The columns M2's invariants read, of a valid world: `n` living and `dead` dead persons, the log
/// in id order or shuffled.
struct World {
    ids: Vec<i64>,
    births: Vec<i64>,
    alive: Vec<i64>,
    death: Vec<i64>,
    lc_person: Vec<i64>,
    lc_kind: Vec<i64>,
    lc_birth: Vec<i64>,
}

impl World {
    fn valid(n: usize, dead: usize, shuffled: bool, rng: &mut SplitMix64) -> Self {
        let mut all: Vec<i64> = (0..n + dead)
            .map(|_| (rng.next() >> 2).cast_signed())
            .collect();
        all.sort_unstable();
        all.dedup();
        let birth = |id: i64| -(id % 50_000) * 1_440;
        let mut living: Vec<i64> = Vec::new();
        let mut log: Vec<(i64, i64, i64)> = Vec::new();
        for (k, &id) in all.iter().enumerate() {
            let is_dead = k % (1 + all.len() / dead.max(1)) == 0 && dead > 0;
            log.push((id, i64::from(is_dead), birth(id)));
            if !is_dead {
                living.push(id);
            }
        }
        if shuffled {
            for i in (1..log.len()).rev() {
                log.swap(i, rng.below(i + 1));
            }
        }
        Self {
            births: living.iter().map(|&i| birth(i)).collect(),
            alive: vec![1; living.len()],
            death: vec![i64::MAX; living.len()],
            ids: living,
            lc_person: log.iter().map(|e| e.0).collect(),
            lc_kind: log.iter().map(|e| e.1).collect(),
            lc_birth: log.iter().map(|e| e.2).collect(),
        }
    }

    fn tables(&self) -> WorldTables {
        let mut t = WorldTables::new();
        t.insert(PERSON_ID, self.ids.clone());
        t.insert(PERSON_BIRTH, self.births.clone());
        t.insert(PERSON_ALIVE, self.alive.clone());
        t.insert(PERSON_DEATH_TICK, self.death.clone());
        t.insert(LC_PERSON, self.lc_person.clone());
        t.insert(LC_EXIT_KIND, self.lc_kind.clone());
        t.insert(LC_BIRTH, self.lc_birth.clone());
        t
    }
}

/// One planted violation, by kind, on a valid world.
fn plant(w: &mut World, kind: usize, rng: &mut SplitMix64) -> &'static str {
    let n = w.ids.len();
    let l = w.lc_person.len();
    match kind {
        0 if n > 0 => {
            w.alive[rng.below(n)] = 0;
            "a dead row"
        }
        1 if n > 0 => {
            w.death[rng.below(n)] = 5;
            "a death tick"
        }
        2 if n > 0 => {
            w.births[rng.below(n)] += 1;
            "a birth that differs from the entry's"
        }
        3 if l > 0 => {
            let r = rng.below(l);
            w.lc_kind[r] = 1 - w.lc_kind[r].min(1);
            "an entry opened or closed"
        }
        4 if n > 1 => {
            w.ids.swap(0, n - 1);
            w.births.swap(0, n - 1);
            "an unsorted ledger"
        }
        5 if n > 0 && l > 0 => {
            // A closed entry for a living person.
            let id = w.ids[rng.below(n)];
            w.lc_person.push(id);
            w.lc_kind.push(1);
            w.lc_birth.push(0);
            "a closed entry for a living person"
        }
        6 if n > 0 => {
            w.ids.pop();
            w.births.pop();
            w.alive.pop();
            w.death.pop();
            "a ledger row removed"
        }
        _ => "nothing",
    }
}

#[test]
fn the_fast_invariants_equal_the_oracle_on_random_tables() {
    let mut rng = SplitMix64(0x0038_0302);
    let mut compared = 0;
    let mut violations = 0;
    for &(n, dead) in &[(1_usize, 0_usize), (60, 15), (600, 2_000)] {
        for _ in 0..8 {
            for shuffled in [false, true] {
                let base = World::valid(n, dead, shuffled, &mut rng);
                let fast = invariants(&base.tables());
                assert_eq!(
                    fast,
                    invariants_oracle(&base.tables()),
                    "n {n}, a valid world"
                );
                assert!(fast.iter().all(|r| r.holds), "n {n}: {fast:?}");
                compared += 1;
                for kind in 0..7 {
                    let mut w = World::valid(n, dead, shuffled, &mut rng);
                    let what = plant(&mut w, kind, &mut rng);
                    let t = w.tables();
                    let (fast, oracle) = (invariants(&t), invariants_oracle(&t));
                    assert_eq!(fast, oracle, "n {n}, {what}");
                    violations += usize::from(oracle.iter().any(|r| !r.holds));
                    compared += 1;
                }
            }
        }
    }
    assert!(violations > 200, "planted violations seen: {violations}");
    println!(
        "SPR-038 T-03: M2's invariants equal the oracle on {compared} tables, {violations} violated"
    );
}
