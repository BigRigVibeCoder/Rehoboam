//! The v0 prelude (§5.3) as code (D8): kinship over the life-course log, `holders_of` over the
//! tenure log, M7's court relations and S7's `vows_of`, each with its bound and canonical order.
//!
//! Genealogy is read from the log, keyed by person (M7), so a line reaches through the dead. Every
//! set is a `BTreeSet` or a sorted `Vec`; nothing iterates a hash order.
//!
//! REF: `specs/registry/effect_language_v0.md` §5.1–§5.3; D4, D5, D8, D12, D13, D14; B2, B3, N2.

use std::collections::{BTreeSet, VecDeque};

use super::ast::Relation;
use super::check::{DESCENT_MAX, LINEAGE_BOUND, rel_spec};
use super::env::{ColumnId, Env};
use super::host::{Host, ReadOutcome};
use super::value::{KeyAtom, Value, scalar_int};
use super::{Diagnostics, EvalError, Mutant};

const KIN_MAX: usize = 6;

/// The prelude over one host, with the evaluation's mutant (T-05) and diagnostics.
pub(crate) struct Prelude<'a> {
    pub env: &'a Env,
    pub host: &'a dyn Host,
    pub mutant: Mutant,
}

fn col(c: Option<ColumnId>) -> Result<ColumnId, EvalError> {
    c.ok_or_else(|| EvalError::Internal("a prelude column is missing from the environment".into()))
}

impl Prelude<'_> {
    fn on(&self, m: Mutant) -> bool {
        cfg!(feature = "mutant-effect") && self.mutant == m
    }

    fn ent(&self, column: ColumnId, id: u64) -> u64 {
        match self.host.read(column, id) {
            ReadOutcome::Row(Value::Ent(_, v)) | ReadOutcome::Neutral(Value::Ent(_, v)) => v,
            _ => 0,
        }
    }

    /// The parent ids from the log, father first (mother first under the D4 mutant, for the
    /// cognatic walks only); fathers only if `agnatic`.
    fn parents(&self, id: u64, agnatic: bool, cognatic_order: bool) -> Result<Vec<u64>, EvalError> {
        let p = self.env.prelude();
        let father = self.ent(col(p.lc_father)?, id);
        let mut out = Vec::with_capacity(2);
        if father != 0 {
            out.push(father);
        }
        if !agnatic {
            let mother = self.ent(col(p.lc_mother)?, id);
            if mother != 0 {
                if cognatic_order && self.on(Mutant::MotherFirst) {
                    out.insert(0, mother);
                } else {
                    out.push(mother);
                }
            }
        }
        Ok(out)
    }

    fn alive(&self, id: u64) -> Result<bool, EvalError> {
        let column = col(self.env.prelude().person_alive)?;
        Ok(matches!(
            self.host.read(column, id),
            ReadOutcome::Row(Value::Bool(true)) | ReadOutcome::Neutral(Value::Bool(true))
        ))
    }

    fn birth_tick(&self, id: u64) -> Result<i128, EvalError> {
        let column = col(self.env.prelude().lc_birth_tick)?;
        Ok(match self.host.read(column, id) {
            ReadOutcome::Row(v) | ReadOutcome::Neutral(v) => scalar_int(&v),
            ReadOutcome::Missing => 0,
        })
    }

    fn ancestors_or_self(
        &self,
        id: u64,
        depth: usize,
        agnatic: bool,
    ) -> Result<BTreeSet<u64>, EvalError> {
        let mut seen = BTreeSet::from([id]);
        let mut frontier = vec![id];
        for _ in 0..depth {
            let mut next = Vec::new();
            for &y in &frontier {
                for p in self.parents(y, agnatic, false)? {
                    if !seen.contains(&p) {
                        next.push(p);
                    }
                }
            }
            seen.extend(next.iter().copied());
            frontier = next;
        }
        Ok(seen)
    }

    /// `ancestors` or `descendants` within `depth` generations: by generation, then id.
    fn by_generation(&self, id: u64, depth: usize, down: bool) -> Result<Vec<u64>, EvalError> {
        let mut seen = BTreeSet::from([id]);
        let mut frontier = vec![id];
        let mut out = Vec::new();
        for _ in 0..depth {
            let mut next = BTreeSet::new();
            for &y in &frontier {
                let step = if down {
                    self.host.children(y)
                } else {
                    self.parents(y, false, false)?
                };
                for z in step {
                    if seen.insert(z) {
                        next.insert(z);
                    }
                }
            }
            frontier = next.into_iter().collect();
            out.extend(frontier.iter().copied());
        }
        Ok(out)
    }

    /// `descends(x, a)`: `x` is `a` or a descendant of `a` within six generations.
    pub(crate) fn descends(&self, x: u64, a: u64) -> Result<bool, EvalError> {
        Ok(x != 0 && a != 0 && self.ancestors_or_self(x, DESCENT_MAX, false)?.contains(&a))
    }

    /// The shortest, father-first parent chain from `x` up to `a`, listed downward.
    fn line_between(&self, a: u64, x: u64, agnatic: bool) -> Result<Vec<u64>, EvalError> {
        if a == x || a == 0 || x == 0 {
            return Ok(Vec::new());
        }
        let mut queue: VecDeque<Vec<u64>> = VecDeque::from([vec![x]]);
        while let Some(path) = queue.pop_front() {
            if path.len() > DESCENT_MAX {
                return Ok(Vec::new());
            }
            let last = path.last().copied().unwrap_or(0);
            for parent in self.parents(last, agnatic, true)? {
                if parent == a {
                    return Ok(path.into_iter().rev().collect());
                }
                if !path.contains(&parent) {
                    let mut longer = path.clone();
                    longer.push(parent);
                    queue.push_back(longer);
                }
            }
        }
        Ok(Vec::new())
    }

    /// `up(x, p)` and `common(x, p)`: the least height and the father-first ancestor found.
    pub(crate) fn up_and_common(
        &self,
        x: u64,
        p: u64,
        agnatic: bool,
    ) -> Result<(i64, u64), EvalError> {
        let x_anc = if x == 0 {
            BTreeSet::new()
        } else {
            self.ancestors_or_self(x, DESCENT_MAX, agnatic)?
        };
        let mut level = if p == 0 { Vec::new() } else { vec![p] };
        for height in 0..=KIN_MAX {
            if let Some(&c) = level.iter().find(|c| x_anc.contains(c)) {
                return Ok((i64::try_from(height).unwrap_or(i64::MAX), c));
            }
            let mut next: Vec<u64> = Vec::new();
            for &y in &level {
                for q in self.parents(y, agnatic, true)? {
                    if !next.contains(&q) {
                        next.push(q);
                    }
                }
            }
            level = next;
        }
        Ok((7, 0))
    }

    fn line_key(&self, a: u64, x: u64) -> Result<Vec<KeyAtom>, EvalError> {
        let mut out = Vec::new();
        for y in self.line_between(a, x, false)? {
            out.push(KeyAtom::Scalar(self.birth_tick(y)?));
            out.push(KeyAtom::Scalar(i128::from(y)));
        }
        Ok(out)
    }

    fn lineage(&self, p: u64, height: usize) -> Result<Vec<u64>, EvalError> {
        let height = if self.on(Mutant::LineageHeightTwo) {
            height.min(2)
        } else {
            height
        };
        let mut members = BTreeSet::new();
        let mut level = vec![p];
        for _ in 0..=height {
            for &a in &level {
                members.extend(self.by_generation(a, DESCENT_MAX, true)?);
            }
            let mut next = Vec::new();
            for &y in &level {
                next.extend(self.parents(y, false, false)?);
            }
            level = next;
        }
        members.remove(&p);
        if self.on(Mutant::BoundBeforePrune) && members.len() > LINEAGE_BOUND {
            return Err(EvalError::Truncated {
                relation: "lineage".into(),
                members: members.len(),
            });
        }
        let kept = self.prune_dead(&members)?;
        if kept.len() > LINEAGE_BOUND && !self.on(Mutant::SilentCut) {
            return Err(EvalError::Truncated {
                relation: "lineage".into(),
                members: kept.len(),
            });
        }
        let mut keyed = Vec::with_capacity(kept.len());
        for x in kept {
            let (up, common) = self.up_and_common(x, p, false)?;
            keyed.push(((up, self.line_key(common, x)?), x));
        }
        keyed.sort();
        Ok(keyed.into_iter().map(|(_, x)| x).collect())
    }

    /// Drops the dead members with no living descendant among the members (N2).
    fn prune_dead(&self, members: &BTreeSet<u64>) -> Result<Vec<u64>, EvalError> {
        let mut kept = Vec::new();
        for &x in members {
            let mut keep = self.alive(x)?;
            if !keep {
                for z in self.by_generation(x, DESCENT_MAX, true)? {
                    if members.contains(&z) && self.alive(z)? {
                        keep = true;
                        break;
                    }
                }
            }
            if keep {
                kept.push(x);
            }
        }
        Ok(kept)
    }

    fn holders_of(&self, title: u64, n: usize) -> Result<Vec<u64>, EvalError> {
        let p = self.env.prelude();
        let tenure = self
            .env
            .kind("tenure")
            .ok_or_else(|| EvalError::Internal("no tenure kind".into()))?;
        let rows = self.host.tenure_rows(tenure, col(p.tenure_title)?, title);
        let holder = col(p.tenure_holder)?;
        let tenures: Vec<u64> = rows.iter().map(|&r| self.ent(holder, r)).collect();
        let mut out = Vec::new();
        for &h in tenures.iter().rev().take(n) {
            if h != 0 && !out.contains(&h) {
                out.push(h);
            }
        }
        Ok(out)
    }

    fn kin_within(&self, id: u64, degrees: usize) -> Result<Vec<u64>, EvalError> {
        let mut seen = BTreeSet::from([id]);
        let mut frontier = vec![id];
        for _ in 0..degrees {
            let mut next = Vec::new();
            for &y in &frontier {
                let mut step = self.parents(y, false, false)?;
                step.extend(self.host.children(y));
                for z in step {
                    if seen.insert(z) {
                        next.push(z);
                    }
                }
            }
            frontier = next;
        }
        seen.remove(&id);
        Ok(seen.into_iter().collect())
    }

    /// `related(a, b, n)`.
    pub(crate) fn related(&self, a: u64, b: u64, n: usize) -> Result<bool, EvalError> {
        Ok(a != 0 && b != 0 && self.kin_within(a, n)?.contains(&b))
    }

    fn rows_where(
        &self,
        kind: &str,
        column: Option<ColumnId>,
        target: u64,
    ) -> Result<Vec<u64>, EvalError> {
        let kind = self
            .env
            .kind(kind)
            .ok_or_else(|| EvalError::Internal(format!("no {kind} kind")))?;
        Ok(self.host.rows_where(kind, col(column)?, target))
    }

    fn court_of(&self, title: u64) -> Result<Vec<u64>, EvalError> {
        let p = self.env.prelude();
        let mut holders = BTreeSet::new();
        for v in self.rows_where("title", p.title_liege, title)? {
            holders.insert(self.ent(col(p.title_holder)?, v));
        }
        for o in self.rows_where("office", p.office_title, title)? {
            holders.insert(self.ent(col(p.office_holder)?, o));
        }
        holders.remove(&0);
        Ok(holders.into_iter().collect())
    }

    /// A relation's elements in canonical order, at most its bound (`lineage` never truncates
    /// silently: above its bound after pruning it is `E_TRUNCATED`).
    pub(crate) fn relation(
        &self,
        rel: Relation,
        args: &[u64],
        diags: &mut Diagnostics,
    ) -> Result<Vec<u64>, EvalError> {
        let a0 = args.first().copied().unwrap_or(0);
        let a1 = args.get(1).copied().unwrap_or(0);
        let n1 = usize::try_from(a1).unwrap_or(usize::MAX);
        let p = self.env.prelude();
        let path = matches!(
            rel,
            Relation::Line | Relation::LineAgnatic | Relation::Lineage
        );
        let mut found = if a0 == 0 && !path {
            Vec::new()
        } else {
            match rel {
                Relation::Children => self.host.children(a0),
                Relation::Kin => self.kin_within(a0, n1)?,
                Relation::Ancestors => self.by_generation(a0, n1, false)?,
                Relation::Descendants => self.by_generation(a0, n1, true)?,
                Relation::Lineage => self.lineage(a0, n1)?,
                Relation::Line => self.line_between(a0, a1, false)?,
                Relation::LineAgnatic => self.line_between(a0, a1, true)?,
                Relation::Titles => self.rows_where("title", p.title_holder, a0)?,
                Relation::Vassals => self.rows_where("title", p.title_liege, a0)?,
                Relation::Offices => self.rows_where("office", p.office_holder, a0)?,
                Relation::Court => self.court_of(a0)?,
                Relation::ClaimsOn => self.rows_where("claim", p.claim_title, a0)?,
                Relation::ClaimsOf => self.rows_where("claim", p.claim_claimant, a0)?,
                Relation::VowsOf => self.rows_where("vow", p.vow_vower, a0)?,
                Relation::HoldersOf => self.holders_of(a0, n1)?,
            }
        };
        let bound = rel_spec(rel).bound;
        if found.len() > bound {
            diags.truncated_relations += 1;
            found.truncate(bound);
        }
        Ok(found)
    }
}
