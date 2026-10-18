//! The host seam over the engine (SPR-017 T-07; Q9): a [`Host`] built on a system's
//! [`SystemView`] and [`StepCtx`], and the conversion of an effects program's actions into queued
//! [`Command`]s.
//!
//! - [`SystemViewHost`] reads every column by name through the view, so the view enforces the
//!   declaring system's `AccessSpec` at run time: a read the system did not declare fails with
//!   [`SimError::UndeclaredRead`] naming the column. The evaluator's [`Reads`] has no error
//!   channel, so the host keeps the first such error, answers `Missing`, and the caller takes the
//!   error with [`SystemViewHost::take_error`] after the evaluation (never a silent zero).
//! - An entity's row is found through its id column (`<kind>.id`, or the key column a table is
//!   read by, [`SystemViewHost::with_key`]: the life-course log by `life_course.person_id`): by
//!   binary search when the column is strictly ascending (the ledger), else through an index built
//!   on first use. A table with no id column is a log: its row index is the id. The id column is
//!   read through the view like any other, so the declaring system declares it (M7 0.3.4's
//!   `[[reads]]` lacks `person.id`: an identity read SPR-021 adds, reported at T-07).
//! - The inverse lookups ([`Indexes::rows_where`]) and `children` use indexes built on first use;
//!   they agree with the scan defaults (the T-07 tests compare them on random worlds).
//! - The neutral of an absent owner is the checker's (compiled into the read, §4.5); this host
//!   never answers [`ReadOutcome::Neutral`]. SPR-014's keyed lookup takes that path when it merges.
//! - [`StepDraws`] draws from the step's keyed stream ([`StepCtx::rng`]) under the purpose the
//!   program names, `t` being the step's instant. A draw is keyed by a person (`PersonId`) or a
//!   region (`RegionId`), the identifier types an engine draw takes; another kind has no draw yet.
//! - [`to_command`] builds a [`Command`] from an action, with the verb's draft as its cause (§6).
//!
//! REF: `specs/registry/effect_language_v0.md` §4.5, §4.6, §5.1, §6, §12 Q9; `05.1` §2 rules 1, 2.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use rh_core::access::AccessSpec;
use rh_core::command::Command;
use rh_core::fixed::{Q8_8, Q16_16};
use rh_core::system::{SimError, SpecPurpose, StepCtx};
use rh_core::world::SystemView;
use rh_events::{DraftRef, EventRef};
use rh_ids::{PersonId, RegionId};

use super::env::{ColumnId, Env, KindId, Ty};
use super::eval::{Action, Arg};
use super::host::{Draws, Host, Indexes, ReadOutcome, Reads};
use super::value::{EventArg, Value};

/// How the rows of one entity kind are found by id.
#[derive(Clone, Debug)]
enum RowIndex {
    /// The id column is strictly ascending: binary search over it.
    Sorted(Vec<u64>),
    /// Any other id column: id to row.
    Map(BTreeMap<u64, usize>),
    /// No id column: a log of this many rows, the row index being the id.
    Log(usize),
}

impl RowIndex {
    fn row(&self, id: u64) -> Option<usize> {
        match self {
            Self::Sorted(ids) => ids.binary_search(&id).ok(),
            Self::Map(m) => m.get(&id).copied(),
            Self::Log(n) => usize::try_from(id).ok().filter(|&r| r < *n),
        }
    }

    fn ids(&self) -> Vec<u64> {
        match self {
            Self::Sorted(ids) => ids.clone(),
            Self::Map(m) => m.keys().copied().collect(),
            Self::Log(n) => (0..*n).filter_map(|r| u64::try_from(r).ok()).collect(),
        }
    }
}

/// A [`Host`] over one system's step: its view, its draws, its parameters and the calendar.
pub struct SystemViewHost<'h, 'v, D> {
    env: &'h Env,
    view: &'h SystemView<'v>,
    draws: D,
    params: BTreeMap<String, Value>,
    days_per_year: i64,
    keys: BTreeMap<KindId, String>,
    rows: RefCell<BTreeMap<KindId, RowIndex>>,
    inverse: RefCell<BTreeMap<ColumnId, BTreeMap<u64, Vec<u64>>>>,
    children: RefCell<Option<BTreeMap<u64, Vec<u64>>>>,
    error: RefCell<Option<SimError>>,
}

impl<'h, 'v, D: Draws> SystemViewHost<'h, 'v, D> {
    /// A host over `view` for programs compiled against `env`, drawing through `draws`, with the
    /// declaring system's parameter values (`int` or `q16_16`) and the manifest's year.
    #[must_use]
    pub fn new(
        env: &'h Env,
        view: &'h SystemView<'v>,
        draws: D,
        params: BTreeMap<String, Value>,
        days_per_year: i64,
    ) -> Self {
        Self {
            env,
            view,
            draws,
            params,
            days_per_year,
            keys: BTreeMap::new(),
            rows: RefCell::new(BTreeMap::new()),
            inverse: RefCell::new(BTreeMap::new()),
            children: RefCell::new(None),
            error: RefCell::new(None),
        }
    }

    /// Reads the rows of `kind` by `column` (`entity.column`) instead of `<kind>.id`: the
    /// life-course log by `life_course.person_id` (`[effect_language.keys]`, §4.5).
    #[must_use]
    pub fn with_key(mut self, kind: &str, column: &str) -> Self {
        if let Some(k) = self.env.kind(kind) {
            self.keys.insert(k, column.to_owned());
        }
        self
    }

    /// The first view error an evaluation met (an undeclared read, a missing column), which the
    /// caller must return instead of the evaluation's result.
    pub fn take_error(&self) -> Option<SimError> {
        self.error.borrow_mut().take()
    }

    fn fail(&self, e: SimError) {
        let mut slot = self.error.borrow_mut();
        if slot.is_none() {
            *slot = Some(e);
        }
    }

    /// A column through the view, or `None` after recording the view's error.
    fn column(&self, name: &str) -> Option<&'h [i64]> {
        match self.view.read(name) {
            Ok(rows) => Some(rows),
            Err(e) => {
                self.fail(e);
                None
            }
        }
    }

    /// The id column of `kind`: the declared key, else `<kind>.id` if the environment has it.
    fn id_column(&self, kind: KindId) -> Option<String> {
        if let Some(k) = self.keys.get(&kind) {
            return Some(k.clone());
        }
        let name = format!("{}.id", self.env.kind_name(kind));
        self.env.column_id(&name).map(|_| name)
    }

    /// The row index of `kind`, built on first use.
    fn with_rows<T>(&self, kind: KindId, f: impl FnOnce(&RowIndex) -> T) -> Option<T> {
        if let Some(index) = self.rows.borrow().get(&kind) {
            return Some(f(index));
        }
        let index = if let Some(name) = self.id_column(kind) {
            Self::index_of(self.column(&name)?)
        } else {
            RowIndex::Log(self.log_len(kind)?)
        };
        let out = f(&index);
        self.rows.borrow_mut().insert(kind, index);
        Some(out)
    }

    /// The index of an id column: binary search when strictly ascending, else a map.
    fn index_of(column: &[i64]) -> RowIndex {
        let ids: Vec<u64> = column.iter().map(|&v| v.cast_unsigned()).collect();
        if ids.windows(2).all(|w| w[0] < w[1]) {
            return RowIndex::Sorted(ids);
        }
        let mut m = BTreeMap::new();
        for (row, id) in ids.into_iter().enumerate() {
            m.entry(id).or_insert(row);
        }
        RowIndex::Map(m)
    }

    /// The row count of a log: that of the first of its columns the system can read; the first
    /// view error when it can read none.
    fn log_len(&self, kind: KindId) -> Option<usize> {
        let mut first_error = None;
        for c in self.env.columns().iter().filter(|c| c.entity == kind) {
            match self.view.read(&c.name) {
                Ok(rows) => return Some(rows.len()),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        if let Some(e) = first_error {
            self.fail(e);
        }
        None
    }

    /// A cell as a value of the column's type.
    fn decode(ty: &Ty, cell: i64) -> Option<Value> {
        Some(match ty {
            Ty::Bool => Value::Bool(cell != 0),
            Ty::Int => Value::Int(cell),
            Ty::Q8 => Value::Q8(Q8_8::from_bits(i16::try_from(cell).ok()?)),
            Ty::Q16 => Value::Q16(Q16_16::from_bits(i32::try_from(cell).ok()?)),
            Ty::Enum(e) => Value::Enum(*e, u32::try_from(cell).ok()?),
            Ty::Ent(k) => Value::Ent(*k, cell.cast_unsigned()),
            _ => return None,
        })
    }

    /// The rows of `kind` whose reference column `column` holds `target`, indexed on first use.
    fn inverse(&self, kind: KindId, column: ColumnId, target: u64) -> Vec<u64> {
        if let Some(m) = self.inverse.borrow().get(&column) {
            return m.get(&target).cloned().unwrap_or_default();
        }
        let mut m: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
        for id in self.ids(kind) {
            if let ReadOutcome::Row(Value::Ent(_, t)) = self.read(column, id) {
                m.entry(t).or_default().insert(id);
            }
        }
        let m: BTreeMap<u64, Vec<u64>> = m
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        let out = m.get(&target).cloned().unwrap_or_default();
        self.inverse.borrow_mut().insert(column, m);
        out
    }
}

impl<D: Draws> Reads for SystemViewHost<'_, '_, D> {
    fn read(&self, column: ColumnId, id: u64) -> ReadOutcome {
        let Some(def) = self.env.column(column) else {
            return ReadOutcome::Missing;
        };
        // A table read by a key (the life-course log by person) is indexed under that key.
        let kind = def.entity;
        let Some(Some(row)) = self.with_rows(kind, |r| r.row(id)) else {
            return ReadOutcome::Missing;
        };
        match self
            .column(&def.name)
            .and_then(|rows| rows.get(row).copied())
            .and_then(|cell| Self::decode(&def.ty, cell))
        {
            Some(v) => ReadOutcome::Row(v),
            None => ReadOutcome::Missing,
        }
    }

    fn exists(&self, kind: KindId, id: u64) -> bool {
        self.with_rows(kind, |r| r.row(id).is_some())
            .unwrap_or(false)
    }

    fn ids(&self, kind: KindId) -> Vec<u64> {
        self.with_rows(kind, RowIndex::ids).unwrap_or_default()
    }
}

impl<D: Draws> Indexes for SystemViewHost<'_, '_, D> {
    fn children(&self, person: u64) -> Vec<u64> {
        if let Some(m) = self.children.borrow().as_ref() {
            return m.get(&person).cloned().unwrap_or_default();
        }
        let mut m: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
        let (Some(lc), Some(mother), Some(father)) = (
            self.env.kind("life_course"),
            self.env.prelude().lc_mother,
            self.env.prelude().lc_father,
        ) else {
            return Vec::new();
        };
        for child in self.ids(lc) {
            for col in [father, mother] {
                if let ReadOutcome::Row(Value::Ent(_, parent)) = self.read(col, child)
                    && parent != 0
                {
                    m.entry(parent).or_default().insert(child);
                }
            }
        }
        let m: BTreeMap<u64, Vec<u64>> = m
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        let out = m.get(&person).cloned().unwrap_or_default();
        *self.children.borrow_mut() = Some(m);
        out
    }

    fn rows_where(&self, kind: KindId, column: ColumnId, target: u64) -> Vec<u64> {
        self.inverse(kind, column, target)
    }
}

impl<D: Draws> Draws for SystemViewHost<'_, '_, D> {
    fn draw(&self, purpose: &str, kind: KindId, id: u64, k: i64) -> Option<u64> {
        self.draws.draw(purpose, kind, id, k)
    }
}

impl<D: Draws> Host for SystemViewHost<'_, '_, D> {
    fn param(&self, _system: &str, name: &str) -> Option<Value> {
        self.params.get(name).cloned()
    }

    fn days_per_year(&self) -> i64 {
        self.days_per_year
    }
}

/// The step's keyed draws under the system's [`SpecPurpose`]s: `H(…, purpose, entity, t, k)` with
/// `t` the step's instant and `k` the program's index as its `u64` bits.
pub struct StepDraws<'c, 's, P> {
    ctx: &'c StepCtx<'s>,
    env: &'c Env,
    purposes: std::marker::PhantomData<P>,
    error: RefCell<Option<SimError>>,
}

impl<'c, 's, P: SpecPurpose> StepDraws<'c, 's, P> {
    /// Draws from `ctx` for programs compiled against `env`.
    #[must_use]
    pub fn new(ctx: &'c StepCtx<'s>, env: &'c Env) -> Self {
        Self {
            ctx,
            env,
            purposes: std::marker::PhantomData,
            error: RefCell::new(None),
        }
    }

    /// The first error a draw met (an undeclared purpose, a stream reuse), which the caller must
    /// return instead of the evaluation's result.
    pub fn take_error(&self) -> Option<SimError> {
        self.error.borrow_mut().take()
    }
}

impl<P: SpecPurpose> Draws for StepDraws<'_, '_, P> {
    fn draw(&self, purpose: &str, kind: KindId, id: u64, k: i64) -> Option<u64> {
        let p = P::ALL.iter().find(|p| p.name() == purpose)?;
        let k = k.cast_unsigned();
        let drawn = match self.env.kind_name(kind) {
            "person" => self.ctx.rng(p, PersonId::from_raw(id), k),
            "region" => self
                .ctx
                .rng(p, RegionId::from_raw(u16::try_from(id).ok()?), k),
            _ => return None,
        };
        match drawn {
            Ok(u) => Some(u),
            Err(e) => {
                let mut slot = self.error.borrow_mut();
                if slot.is_none() {
                    *slot = Some(e);
                }
                None
            }
        }
    }
}

/// Why an action is not a [`Command`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToCommandError {
    /// The system does not declare the command in its `AccessSpec::sends`.
    Undeclared(String),
    /// An event-typed argument, which needs SPR-014's `EventRef` (until then not convertible).
    EventArgument {
        /// The command.
        command: String,
        /// The event-typed parameter.
        parameter: String,
    },
    /// The action names a command the environment lacks.
    Unknown,
}

impl std::fmt::Display for ToCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Undeclared(c) => write!(f, "command {c} is not one the system sends"),
            Self::EventArgument { command, parameter } => write!(
                f,
                "{command}.{parameter} is event-typed: an action with an event argument becomes a \
                 Command only once SPR-014's EventRef has merged"
            ),
            Self::Unknown => f.write_str("the action names no command of the environment"),
        }
    }
}

impl std::error::Error for ToCommandError {}

/// The [`Command`] of `action` (§6): its kind is the name `access` declares the send under, its
/// person the queue key, its arguments the payload encoded in order (`Arg::encode`), and its cause
/// the verb's draft.
///
/// # Errors
/// [`ToCommandError`]: an undeclared send, an event-typed argument (not yet convertible), or an
/// action of another environment.
pub fn to_command(
    env: &Env,
    access: &AccessSpec,
    action: &Action,
    verb_draft: Option<DraftRef>,
) -> Result<Command, ToCommandError> {
    let def = env.command(action.command).ok_or(ToCommandError::Unknown)?;
    let send = access
        .send_of(&def.name)
        .ok_or_else(|| ToCommandError::Undeclared(def.name.clone()))?;
    let mut args = Vec::with_capacity(action.args.len());
    for (arg, (name, _)) in action.args.iter().zip(&def.params) {
        match (*arg, arg.encode()) {
            (Arg::Event(EventArg::Own | EventArg::None), _) | (_, None) => {
                return Err(ToCommandError::EventArgument {
                    command: def.name.clone(),
                    parameter: name.clone(),
                });
            }
            (_, Some(v)) => args.push(rh_core::command::Arg::Int(v)),
        }
    }
    Ok(Command {
        kind: send.command,
        person: PersonId::from_raw(action.person),
        args,
        cause: verb_draft.map_or(EventRef::None, EventRef::Draft),
    })
}
