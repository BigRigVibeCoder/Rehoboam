//! The evaluator: innermost first and left to right (§4.2); `and` and `or` evaluate both sides,
//! `if` only its taken branch. Reads go through the host and count `missing_reads` and
//! `neutral_reads` (§4.5); draws go through the host's keyed stream under the run-time ledger
//! (§4.6); effects evaluate to ordered, typed actions (§6).
//!
//! REF: `specs/registry/effect_language_v0.md` §4.2–§4.6, §5.4, §6.

use std::collections::BTreeSet;

use rh_core::fixed::{Q16_16, draw_hits};

use super::ast::{AggOp, BinOp, Dotted, Effect, Kind, Node};
use super::check::{MINUTES_PER_DAY, arith_op};
use super::env::{CommandId, Env, KindId, Ty};
use super::host::{Host, ReadOutcome};
use super::prelude::Prelude;
use super::value::{
    Arith, ArithOp, EventArg, Value, arith, fixed_of_int, floor_fx, is_zero, literal_value, negate,
    numeric_key, round_fx, sat_i64,
};
use super::{Diagnostics, EvalError, EvalOptions, Mutant};

/// One draw key of the run-time ledger: exactly `H`'s varying inputs, `(purpose, entity kind,
/// entity id, k as an i64)` (§4.6, GAP-C): an entity `k` and an `int` `k` of one value collide.
pub type DrawKey = (String, KindId, u64, i64);

/// The run-time draw ledger (§4.6, N6, r4-2): a key drawn twice within it is the invariant
/// `E_RNG_REUSE`. A law's evaluation starts a fresh one per program and binding; a system's verb
/// evaluations in one step share one ([`super::VerbStep`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DrawLedger {
    drawn: BTreeSet<DrawKey>,
}

impl DrawLedger {
    /// An empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of keys drawn.
    #[must_use]
    pub fn len(&self) -> usize {
        self.drawn.len()
    }

    /// Whether nothing was drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.drawn.is_empty()
    }
}

/// One typed argument of an emitted action (§6). A program never encodes an event id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arg {
    /// An `int`.
    Int(i64),
    /// A fixed-point value's raw bits (`q8_8` or `q16_16`).
    Bits(i64),
    /// A `bool`.
    Bool(bool),
    /// An enum's ordinal.
    Enum(u32),
    /// An entity's id (`none` is 0).
    Entity(u64),
    /// An event-typed field: the verb's own event, or none.
    Event(EventArg),
}

impl Arg {
    /// The `Command::args` encoding of §6, or `None` for an event-typed field (the host maps it
    /// to an `EventRef`; the merge resolves its id).
    #[must_use]
    pub fn encode(self) -> Option<i64> {
        Some(match self {
            Self::Int(v) | Self::Bits(v) => v,
            Self::Bool(b) => i64::from(b),
            Self::Enum(i) => i64::from(i),
            Self::Entity(id) => id.cast_signed(),
            Self::Event(_) => return None,
        })
    }
}

/// An emitted action: a command, its queue-key person and its typed arguments in payload order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Action {
    /// The command.
    pub command: CommandId,
    /// The queue key (`Command::person`).
    pub person: u64,
    /// The parameters in the command's declared order.
    pub args: Vec<Arg>,
}

pub(crate) struct Ev<'a> {
    pub env: &'a Env,
    pub host: &'a dyn Host,
    pub system: &'a str,
    pub vars: Vec<Option<Value>>,
    pub diags: &'a mut Diagnostics,
    pub ledger: &'a mut DrawLedger,
    pub opts: EvalOptions,
}

type EResult<T> = Result<T, EvalError>;

fn internal<T>(what: &str) -> EResult<T> {
    Err(EvalError::Internal(what.to_owned()))
}

impl Ev<'_> {
    fn on(&self, m: Mutant) -> bool {
        cfg!(feature = "mutant-effect") && self.opts.mutant == m
    }

    fn arith_mode(&self) -> Arith {
        Arith {
            mul_half_up: self.on(Mutant::MulHalfUp),
        }
    }

    fn prelude(&self) -> Prelude<'_> {
        Prelude {
            env: self.env,
            host: self.host,
            mutant: self.opts.mutant,
        }
    }

    fn bool(&mut self, node: &Node) -> EResult<bool> {
        match self.eval(node)? {
            Value::Bool(b) => Ok(b),
            _ => internal("a boolean was expected"),
        }
    }

    fn ent(&mut self, node: &Node) -> EResult<(KindId, u64)> {
        match self.eval(node)? {
            Value::Ent(k, id) => Ok((k, id)),
            _ => internal("an entity was expected"),
        }
    }

    fn set(&mut self, slot: usize, v: Value) {
        if self.vars.len() <= slot {
            self.vars.resize(slot + 1, None);
        }
        self.vars[slot] = Some(v);
    }

    fn clear(&mut self, slot: usize) {
        if let Some(v) = self.vars.get_mut(slot) {
            *v = None;
        }
    }

    /// A relation's elements, with their kind.
    pub(crate) fn relation(&mut self, node: &Node) -> EResult<(KindId, Vec<u64>)> {
        let Kind::Rel { rel, args, .. } = &node.kind else {
            return internal("a relation was expected");
        };
        let mut ids = Vec::with_capacity(args.len());
        for arg in args {
            ids.push(match self.eval(arg)? {
                Value::Ent(_, id) => id,
                other => u64::try_from(super::value::scalar_int(&other)).unwrap_or(0),
            });
        }
        let prelude = Prelude {
            env: self.env,
            host: self.host,
            mutant: self.opts.mutant,
        };
        let found = prelude.relation(*rel, &ids, self.diags)?;
        let Ty::Rel(kind) = node.ty else {
            return internal("a relation type was expected");
        };
        Ok((kind, found))
    }

    /// Evaluates a node. Each kind has its own small function, so a level of the tree costs a
    /// small stack frame.
    pub(crate) fn eval(&mut self, node: &Node) -> EResult<Value> {
        match &node.kind {
            Kind::Num { bits, .. } => Ok(literal_value(&node.ty, *bits)),
            Kind::Bool(b) => Ok(Value::Bool(*b)),
            Kind::None => match node.ty {
                Ty::Event => Ok(Value::Event(EventArg::None)),
                Ty::Ent(k) => Ok(Value::Ent(k, 0)),
                _ => internal("none without a kind"),
            },
            Kind::Var { .. } => self.ev_var(node),
            Kind::Dotted { .. } => self.ev_dotted(node),
            Kind::Read { .. } => self.ev_read(node),
            Kind::Rel { .. } => self.ev_rel(node),
            Kind::Call { .. } => self.ev_call(node),
            Kind::Agg { .. } => self.ev_agg(node),
            Kind::Tuple(..) => self.ev_tuple(node),
            Kind::ListKey { .. } => self.ev_list_key(node),
            Kind::If { .. } => self.ev_if(node),
            Kind::Neg(..) => self.ev_neg(node),
            Kind::Not(..) => self.ev_not(node),
            Kind::Bin { .. } => self.ev_bin(node),
            Kind::Chain { .. } => self.ev_chain(node),
        }
    }

    #[inline(never)]
    fn ev_var(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Var { slot, name } = &node.kind else {
            return internal("node");
        };
        match self.vars.get(*slot).cloned().flatten() {
            Some(v) => Ok(v),
            None => Err(EvalError::Internal(format!("variable {name} is unbound"))),
        }
    }

    #[inline(never)]
    fn ev_dotted(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Dotted { tail, res, .. } = &node.kind else {
            return internal("node");
        };
        match res {
            Dotted::Param => self
                .host
                .param(self.system, tail)
                .ok_or_else(|| EvalError::MissingParam(tail.clone())),
            Dotted::DaysPerYear => Ok(Value::Int(self.host.days_per_year())),
            Dotted::MinutesPerDay => Ok(Value::Int(MINUTES_PER_DAY)),
            Dotted::Enum(v) => Ok(v.clone()),
            Dotted::Unresolved => internal("an unresolved name"),
        }
    }

    #[inline(never)]
    fn ev_read(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Read {
            arg, col, neutral, ..
        } = &node.kind
        else {
            return internal("node");
        };
        {
            let (_, id) = self.ent(arg)?;
            let Some(col) = *col else {
                return internal("an unresolved column");
            };
            let ty = self.env.column(col).map_or(Ty::Int, |c| c.ty.clone());
            if let Some(v) = neutral {
                self.diags.neutral_reads += 1;
                return Ok(if self.on(Mutant::NeutralIsZero) {
                    self.env.zero_of(&ty)
                } else {
                    v.clone()
                });
            }
            let outcome = if id == 0 {
                ReadOutcome::Missing
            } else {
                self.host.read(col, id)
            };
            Ok(match outcome {
                ReadOutcome::Row(v) => v,
                ReadOutcome::Missing => {
                    self.diags.missing_reads += 1;
                    match ty {
                        Ty::Enum(e) if self.on(Mutant::MissingEnumOrdinalZero) => Value::Enum(e, 0),
                        _ => self.env.zero_of(&ty),
                    }
                }
                ReadOutcome::Neutral(v) => {
                    self.diags.neutral_reads += 1;
                    if self.on(Mutant::NeutralIsZero) {
                        self.env.zero_of(&ty)
                    } else {
                        v
                    }
                }
            })
        }
    }

    #[inline(never)]
    fn ev_rel(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Rel { .. } = &node.kind else {
            return internal("node");
        };
        {
            let (kind, ids) = self.relation(node)?;
            Ok(Value::List(
                ids.into_iter().map(|id| Value::Ent(kind, id)).collect(),
            ))
        }
    }

    #[inline(never)]
    fn ev_call(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Call { name, args } = &node.kind else {
            return internal("node");
        };
        self.call(name, args)
    }

    #[inline(never)]
    fn ev_agg(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Agg {
            op,
            slot,
            rel,
            cond,
            body,
            orelse,
            ..
        } = &node.kind
        else {
            return internal("node");
        };
        {
            let (kind, elements) = self.relation(rel)?;
            let mut picked = Vec::new();
            for id in elements {
                self.set(*slot, Value::Ent(kind, id));
                let pass = match cond {
                    Some(c) => self.bool(c)?,
                    None => true,
                };
                if pass {
                    picked.push(match body {
                        Some(b) => self.eval(b)?,
                        None => Value::Bool(true),
                    });
                }
            }
            self.clear(*slot);
            self.fold(*op, &node.ty, picked, orelse.as_deref())
        }
    }

    #[inline(never)]
    fn ev_tuple(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Tuple(items) = &node.kind else {
            return internal("node");
        };
        {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(self.eval(item)?);
            }
            Ok(Value::Tuple(out))
        }
    }

    #[inline(never)]
    fn ev_list_key(&mut self, node: &Node) -> EResult<Value> {
        let Kind::ListKey {
            body, slot, rel, ..
        } = &node.kind
        else {
            return internal("node");
        };
        {
            let (kind, elements) = self.relation(rel)?;
            let mut out = Vec::with_capacity(elements.len());
            for id in elements {
                self.set(*slot, Value::Ent(kind, id));
                out.push(self.eval(body)?);
            }
            self.clear(*slot);
            Ok(Value::List(out))
        }
    }

    #[inline(never)]
    fn ev_if(&mut self, node: &Node) -> EResult<Value> {
        let Kind::If { cond, a, b } = &node.kind else {
            return internal("node");
        };
        {
            if self.bool(cond)? {
                self.eval(a)
            } else {
                self.eval(b)
            }
        }
    }

    #[inline(never)]
    fn ev_neg(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Neg(n, e) = &node.kind else {
            return internal("node");
        };
        let mut v = self.eval(e)?;
        for _ in 0..*n {
            v = negate(&v).ok_or_else(|| EvalError::Internal("negation of a non-number".into()))?;
        }
        Ok(v)
    }

    #[inline(never)]
    fn ev_not(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Not(n, e) = &node.kind else {
            return internal("node");
        };
        let v = self.bool(e)?;
        Ok(Value::Bool(v ^ (n % 2 == 1)))
    }

    #[inline(never)]
    fn ev_bin(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Bin { op, a, b } = &node.kind else {
            return internal("node");
        };
        let va = self.eval(a)?;
        let vb = self.eval(b)?;
        Ok(Value::Bool(match op {
            BinOp::Eq => va == vb,
            BinOp::Ne => va != vb,
            BinOp::Lt => numeric_key(&va) < numeric_key(&vb),
            BinOp::Le => numeric_key(&va) <= numeric_key(&vb),
            BinOp::Gt => numeric_key(&va) > numeric_key(&vb),
            _ => numeric_key(&va) >= numeric_key(&vb),
        }))
    }

    /// A chain folded left to right (§4.8). `and` and `or` evaluate every operand (§4.5); the
    /// short-circuit mutant stops at the first operand that decides.
    #[inline(never)]
    fn ev_chain(&mut self, node: &Node) -> EResult<Value> {
        let Kind::Chain { ops, items } = &node.kind else {
            return internal("node");
        };
        let logic = ops
            .first()
            .copied()
            .filter(|o| matches!(o, BinOp::And | BinOp::Or));
        if let Some(op) = logic {
            let mut acc = op == BinOp::And;
            for item in items {
                let v = self.bool(item)?;
                acc = if op == BinOp::And { acc && v } else { acc || v };
                if self.on(Mutant::ShortCircuit) && acc != (op == BinOp::And) {
                    break;
                }
            }
            return Ok(Value::Bool(acc));
        }
        let mut acc = self.eval(&items[0])?;
        for (op, item) in ops.iter().zip(&items[1..]) {
            let v = self.eval(item)?;
            acc = arith(arith_op(*op), &acc, &v, self.arith_mode())
                .ok_or_else(|| EvalError::Internal("a division by zero at run time".into()))?;
        }
        Ok(acc)
    }

    fn fold(
        &mut self,
        op: AggOp,
        ty: &Ty,
        picked: Vec<Value>,
        orelse: Option<&Node>,
    ) -> EResult<Value> {
        let truth = |v: &Value| matches!(v, Value::Bool(true));
        Ok(match op {
            AggOp::Count => Value::Int(i64::try_from(picked.len()).unwrap_or(i64::MAX)),
            AggOp::Any => Value::Bool(picked.iter().any(truth)),
            AggOp::All => Value::Bool(picked.iter().all(truth)),
            AggOp::Sum => {
                let mut total = self.env.zero_of(ty);
                for v in &picked {
                    total = arith(ArithOp::Add, &total, v, self.arith_mode())
                        .ok_or_else(|| EvalError::Internal("a sum of mixed types".into()))?;
                }
                total
            }
            AggOp::Min | AggOp::Max => {
                let mut best: Option<Value> = None;
                for v in picked {
                    let better = best.as_ref().is_none_or(|b| {
                        if op == AggOp::Min {
                            numeric_key(&v) < numeric_key(b)
                        } else {
                            numeric_key(&v) > numeric_key(b)
                        }
                    });
                    if better {
                        best = Some(v);
                    }
                }
                match best {
                    Some(v) => v,
                    None => match orelse {
                        Some(d) => self.eval(d)?,
                        None => return internal("min or max without else"),
                    },
                }
            }
        })
    }

    fn call(&mut self, name: &str, args: &[Node]) -> EResult<Value> {
        match name {
            "draw" | "chance" => self.draw(name, args),
            "related" => self.call_related(args),
            "up" | "common" | "up_agnatic" | "common_agnatic" | "descends" => {
                self.call_accessor(name, args)
            }
            "exists" => self.call_exists(args),
            _ => self.call_builtin(name, args),
        }
    }

    #[inline(never)]
    fn call_related(&mut self, args: &[Node]) -> EResult<Value> {
        let (_, a) = self.ent(&args[0])?;
        let (_, b) = self.ent(&args[1])?;
        let n = match self.eval(&args[2])? {
            Value::Int(n) => usize::try_from(n).unwrap_or(0),
            _ => return internal("a degree"),
        };
        Ok(Value::Bool(self.prelude().related(a, b, n)?))
    }

    #[inline(never)]
    fn call_accessor(&mut self, name: &str, args: &[Node]) -> EResult<Value> {
        let (_, x) = self.ent(&args[0])?;
        let (pk, p) = self.ent(&args[1])?;
        if name == "descends" {
            return Ok(Value::Bool(self.prelude().descends(x, p)?));
        }
        let (height, common) = self
            .prelude()
            .up_and_common(x, p, name.ends_with("_agnatic"))?;
        Ok(if name.starts_with("up") {
            Value::Int(height)
        } else {
            Value::Ent(pk, common)
        })
    }

    #[inline(never)]
    fn call_exists(&mut self, args: &[Node]) -> EResult<Value> {
        let (kind, id) = self.ent(&args[0])?;
        Ok(Value::Bool(id != 0 && self.host.exists(kind, id)))
    }

    #[inline(never)]
    fn call_builtin(&mut self, name: &str, args: &[Node]) -> EResult<Value> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(arg)?);
        }
        self.builtin(name, &values, args)
    }

    fn builtin(&self, name: &str, v: &[Value], args: &[Node]) -> EResult<Value> {
        let bad = || EvalError::Internal(format!("{name} of a mistyped value"));
        Ok(match name {
            "ordinal" => match v[0] {
                Value::Enum(_, i) => Value::Int(i64::from(i)),
                _ => return Err(bad()),
            },
            "widen" => match &v[0] {
                Value::Q8(q) => Value::Q16(q.widen()),
                _ => return Err(bad()),
            },
            "narrow" => match &v[0] {
                Value::Q16(q) => Value::Q8(q.narrow_saturating()),
                _ => return Err(bad()),
            },
            "q8" | "q16" => match v[0] {
                Value::Int(n) => fixed_of_int(if name == "q8" { &Ty::Q8 } else { &Ty::Q16 }, n),
                _ => return Err(bad()),
            },
            "floor" => Value::Int(floor_fx(&v[0]).ok_or_else(bad)?),
            "round" => Value::Int(round_fx(&v[0]).ok_or_else(bad)?),
            "abs" => {
                if numeric_key(&v[0]) >= 0 {
                    v[0].clone()
                } else {
                    negate(&v[0]).ok_or_else(bad)?
                }
            }
            "min" => {
                if numeric_key(&v[1]) < numeric_key(&v[0]) {
                    v[1].clone()
                } else {
                    v[0].clone()
                }
            }
            "max" => {
                if numeric_key(&v[1]) > numeric_key(&v[0]) {
                    v[1].clone()
                } else {
                    v[0].clone()
                }
            }
            "clamp" => {
                let lo = if numeric_key(&v[1]) > numeric_key(&v[0]) {
                    &v[1]
                } else {
                    &v[0]
                };
                if numeric_key(&v[2]) < numeric_key(lo) {
                    v[2].clone()
                } else {
                    lo.clone()
                }
            }
            "safe_div" => {
                if is_zero(&v[1]) {
                    v[2].clone()
                } else {
                    arith(ArithOp::Div, &v[0], &v[1], self.arith_mode()).ok_or_else(bad)?
                }
            }
            _ => {
                let _ = args;
                return Err(EvalError::Internal(format!("unknown builtin {name}")));
            }
        })
    }

    fn draw(&mut self, name: &str, args: &[Node]) -> EResult<Value> {
        let Kind::Var { name: purpose, .. } = &args[0].kind else {
            return internal("a draw purpose");
        };
        let (kind, id) = self.ent(&args[1])?;
        let k = match self.eval(&args[2])? {
            Value::Ent(_, id) => id.cast_signed(),
            Value::Int(n) => n,
            _ => return internal("a draw index"),
        };
        let key = (purpose.clone(), kind, id, k);
        if self.ledger.drawn.contains(&key) {
            return Err(EvalError::RngReuse {
                purpose: purpose.clone(),
                kind: self.env.kind_name(kind).to_owned(),
                id,
                k,
            });
        }
        self.ledger.drawn.insert(key);
        let Some(u) = self.host.draw(purpose, kind, id, k) else {
            return Err(EvalError::DrawUnavailable {
                purpose: purpose.clone(),
                kind: self.env.kind_name(kind).to_owned(),
                id,
                k,
            });
        };
        if name == "draw" {
            return Ok(Value::Int(sat_i64(i128::from(u >> 1))));
        }
        match self.eval(&args[3])? {
            Value::Q16(p) => Ok(Value::Bool(draw_hits(u, p))),
            _ => Ok(Value::Bool(draw_hits(u, Q16_16::ZERO))),
        }
    }

    /// Evaluates an effect program into its ordered actions.
    pub(crate) fn effects(&mut self, items: &[Effect]) -> EResult<Vec<Action>> {
        let mut out = Vec::new();
        for item in items {
            match &item.each {
                None => {
                    let go = match &item.when {
                        Some(w) => self.bool(w)?,
                        None => true,
                    };
                    if go {
                        out.extend(self.action(&item.action)?);
                    }
                }
                Some(each) => {
                    let (kind, elements) = self.relation(&each.rel)?;
                    for id in elements {
                        self.set(each.slot, Value::Ent(kind, id));
                        let pass = match &each.cond {
                            Some(c) => self.bool(c)?,
                            None => true,
                        };
                        if pass {
                            out.extend(self.action(&item.action)?);
                        }
                    }
                    self.clear(each.slot);
                }
            }
        }
        Ok(out)
    }

    fn action(&mut self, action: &super::ast::Action) -> EResult<Option<Action>> {
        let mut values = Vec::with_capacity(action.args.len());
        for (_, node) in &action.args {
            values.push(self.eval(node)?);
        }
        if action.dropped {
            self.diags.dropped_actions += 1;
            return Ok(None);
        }
        let Some(command) = action.command else {
            return internal("an unresolved command");
        };
        let person = match values.get(action.person_index) {
            Some(Value::Ent(_, id)) => *id,
            _ => return internal("the queue key"),
        };
        let mut args = Vec::with_capacity(action.order.len());
        for &i in &action.order {
            args.push(match values.get(i) {
                Some(Value::Int(n)) => Arg::Int(*n),
                Some(Value::Q8(q)) => Arg::Bits(i64::from(q.to_bits())),
                Some(Value::Q16(q)) => Arg::Bits(i64::from(q.to_bits())),
                Some(Value::Bool(b)) => Arg::Bool(*b),
                Some(Value::Enum(_, i)) => Arg::Enum(*i),
                Some(Value::Ent(_, id)) => Arg::Entity(*id),
                Some(Value::Event(e)) => Arg::Event(*e),
                _ => return internal("a command argument"),
            });
        }
        Ok(Some(Action {
            command,
            person,
            args,
        }))
    }
}
