//! The checker: types (§3, §4.1), names (§4.7), constants and guarded division (§4.4), planes,
//! declared reads, neutrals, commands and owners (§4.5, §6), the static draw-site lint (§4.6) and
//! the iteration bounds (§4.8).
//!
//! A node is checked, which gives it a possibly polymorphic type, then resolved to a concrete type
//! by its context (a literal adopts the type the other operand, the parameter or the program's
//! result demands, §4.1). The walk is innermost first and left to right, and stops at the first
//! fault (§10), so a program with one fault is rejected with that fault's code.
//!
//! REF: `specs/registry/effect_language_v0.md` §3, §4, §5.4, §6, §7.1, §10.

use std::collections::{BTreeMap, BTreeSet};

use super::ast::{Action, AggOp, BinOp, Dotted, Effect, Kind, Node, Relation, canon, mentions};
use super::env::{Env, KindId, SystemDecl, Ty};
use super::value::{ArithOp, arith_int, literal_bits, sat_i64};
use super::{ErrorCode, Rejection, Span};

/// The aggregate nesting bound (§4.8).
pub const NESTING_MAX: usize = 2;
/// The bound-product limit (§4.8).
pub const BOUND_PRODUCT_MAX: u64 = 65_536;
/// `kin`, `related`, `up` and `common` heights (§5.3).
pub const KIN_DEGREES_MAX: i128 = 6;
/// Generations for `ancestors`, `descendants`, `descends` and the lines (§5.3).
pub const DESCENT_MAX: usize = 6;
/// `lineage`'s height (§5.3).
pub const LINEAGE_HEIGHT_MAX: i128 = 3;
/// `holders_of`'s window (§5.3).
pub const TENURES_MAX: i128 = 16;
/// `lineage`'s bound, after pruning (§5.2, D12).
pub const LINEAGE_BOUND: usize = 4096;
/// `rh_core::time::MINUTES_PER_DAY` (minor 6).
pub const MINUTES_PER_DAY: i64 = rh_core::time::MINUTES_PER_DAY;

const GENEALOGY: [&str; 3] = [
    "life_course.person_id",
    "life_course.mother",
    "life_course.father",
];
const AGNATIC: [&str; 2] = ["life_course.person_id", "life_course.father"];
const LINEAGE_READS: [&str; 5] = [
    "life_course.person_id",
    "life_course.mother",
    "life_course.father",
    "life_course.birth_tick",
    "person.alive",
];

/// An argument of a relation: an entity of a kind, or a degree with its upper bound.
#[derive(Clone, Copy)]
enum ArgSpec {
    Ent(&'static str),
    Degree(i128),
}

/// A relation's signature (§5.3): its arguments, the kind it yields, the columns it reads and its
/// bound.
pub(crate) struct RelSpec {
    args: &'static [ArgSpec],
    pub yields: &'static str,
    reads: &'static [&'static str],
    pub bound: usize,
}

pub(crate) fn rel_spec(rel: Relation) -> RelSpec {
    use ArgSpec::{Degree, Ent};
    let (args, yields, reads, bound): (&'static [ArgSpec], &str, &'static [&'static str], usize) =
        match rel {
            Relation::Children => (&[Ent("person")], "person", &GENEALOGY, 64),
            Relation::Kin => (
                &[Ent("person"), Degree(KIN_DEGREES_MAX)],
                "person",
                &GENEALOGY,
                256,
            ),
            Relation::Ancestors => (&[Ent("person"), Degree(6)], "person", &GENEALOGY, 126),
            Relation::Descendants => (&[Ent("person"), Degree(6)], "person", &GENEALOGY, 256),
            Relation::Lineage => (
                &[Ent("person"), Degree(LINEAGE_HEIGHT_MAX)],
                "person",
                &LINEAGE_READS,
                LINEAGE_BOUND,
            ),
            Relation::Line => (
                &[Ent("person"), Ent("person")],
                "person",
                &GENEALOGY,
                DESCENT_MAX,
            ),
            Relation::LineAgnatic => (
                &[Ent("person"), Ent("person")],
                "person",
                &AGNATIC,
                DESCENT_MAX,
            ),
            Relation::Titles => (&[Ent("person")], "title", &["title.holder"], 16),
            Relation::Vassals => (&[Ent("title")], "title", &["title.liege"], 64),
            Relation::Offices => (&[Ent("person")], "office", &["office.holder"], 16),
            Relation::Court => (
                &[Ent("title")],
                "person",
                &[
                    "title.liege",
                    "title.holder",
                    "office.title",
                    "office.holder",
                ],
                128,
            ),
            Relation::ClaimsOn => (&[Ent("title")], "claim", &["claim.title"], 64),
            Relation::ClaimsOf => (&[Ent("person")], "claim", &["claim.claimant"], 16),
            Relation::VowsOf => (&[Ent("person")], "vow", &["vow.vower"], 8),
            Relation::HoldersOf => (
                &[Ent("title"), Degree(TENURES_MAX)],
                "person",
                &["tenure.title", "tenure.holder"],
                16,
            ),
        };
    RelSpec {
        args,
        yields,
        reads,
        bound,
    }
}

/// The per-program options of a compilation: what §7.1's kind implies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckOptions {
    /// Whether key tuples and lists are allowed (an `order` program).
    pub allow_key: bool,
    /// An effects program: its draws must key on `actor` or `target` (r4-2).
    pub verb_program: bool,
    /// A verb whose event carries a drawn outcome: the program draws nothing (minor 2).
    pub outcome_payload: bool,
}

/// The checker's state over one program, or over the programs of one law.
pub(crate) struct Checker<'e> {
    pub env: &'e Env,
    system_name: String,
    scope: BTreeMap<String, (Ty, usize)>,
    pub next_slot: usize,
    pub opts: CheckOptions,
    depth: usize,
    bound_product: u64,
    pub sites: BTreeSet<String>,
    loop_vars: Vec<String>,
}

type CResult<T> = Result<T, Rejection>;

fn err<T>(code: ErrorCode, span: Span, message: impl Into<String>) -> CResult<T> {
    Err(Rejection::new(code, span, message.into()))
}

impl<'e> Checker<'e> {
    pub(crate) fn new(env: &'e Env, system: &str, opts: CheckOptions) -> Self {
        Self {
            env,
            system_name: system.to_owned(),
            scope: BTreeMap::new(),
            next_slot: 0,
            opts,
            depth: 0,
            bound_product: 1,
            sites: BTreeSet::new(),
            loop_vars: Vec::new(),
        }
    }

    fn system(&self, span: Span) -> CResult<&'e SystemDecl> {
        match self.env.systems.get(&self.system_name) {
            Some(s) => Ok(s),
            None => err(
                ErrorCode::UnknownName,
                span,
                format!("system {}", self.system_name),
            ),
        }
    }

    fn kind(&self, name: &str, span: Span) -> CResult<KindId> {
        match self.env.kind(name) {
            Some(k) => Ok(k),
            None => err(ErrorCode::UnknownName, span, format!("entity kind {name}")),
        }
    }

    /// Binds a variable; shadowing is `E_SYNTAX`. A loop variable is one a draw must key on.
    pub(crate) fn bind(&mut self, name: &str, ty: Ty, is_loop: bool, span: Span) -> CResult<usize> {
        if self.scope.contains_key(name) || super::lex::KEYWORDS.contains(&name) {
            return err(
                ErrorCode::Syntax,
                span,
                format!("variable {name} shadows a name"),
            );
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        self.scope.insert(name.to_owned(), (ty, slot));
        if is_loop {
            self.loop_vars.push(name.to_owned());
        }
        Ok(slot)
    }

    pub(crate) fn unbind(&mut self, name: &str) {
        self.scope.remove(name);
        if self.loop_vars.last().is_some_and(|v| v == name) {
            self.loop_vars.pop();
        }
    }

    fn check_read(&self, name: &str, span: Span) -> CResult<super::env::ColumnId> {
        let Some(id) = self.env.column_id(name) else {
            return err(ErrorCode::UnknownName, span, format!("column {name}"));
        };
        let Some(column) = self.env.column(id) else {
            return err(ErrorCode::UnknownName, span, format!("column {name}"));
        };
        if !column.plane_a {
            return err(ErrorCode::Plane, span, name);
        }
        if column.owner != self.system_name {
            let system = self.system(span)?;
            let Some(decl) = system.reads.get(name) else {
                return err(
                    ErrorCode::UndeclaredRead,
                    span,
                    format!("{} reads {name}", self.system_name),
                );
            };
            if decl.neutral.is_none() && !decl.identity {
                return err(
                    ErrorCode::NoNeutral,
                    span,
                    format!("{} reads {name}: no neutral", self.system_name),
                );
            }
        }
        Ok(id)
    }

    fn neutral_for(&self, name: &str, id: super::env::ColumnId) -> Option<super::value::Value> {
        let column = self.env.column(id)?;
        if self.env.registered.contains(&column.owner) || column.owner == self.system_name {
            return None;
        }
        self.env
            .systems
            .get(&self.system_name)?
            .reads
            .get(name)?
            .neutral
            .clone()
    }

    fn note_draw(&mut self, site: String, keyed: [&Node; 2], span: Span) -> CResult<()> {
        if self.sites.contains(&site) {
            return err(
                ErrorCode::RngReuse,
                span,
                format!("draw site {site} repeated"),
            );
        }
        self.sites.insert(site);
        for name in &self.loop_vars {
            if !keyed.iter().any(|n| mentions(n, name)) {
                return err(
                    ErrorCode::RngReuse,
                    span,
                    format!("draw inside a loop over {name} not keyed on it"),
                );
            }
        }
        Ok(())
    }

    fn enter(&mut self, bound: usize, span: Span) -> CResult<()> {
        self.depth += 1;
        self.bound_product = self.bound_product.saturating_mul(bound as u64);
        if self.depth > NESTING_MAX || self.bound_product > BOUND_PRODUCT_MAX {
            return err(
                ErrorCode::Bound,
                span,
                format!("depth {}, product {}", self.depth, self.bound_product),
            );
        }
        Ok(())
    }

    fn leave(&mut self, bound: usize) {
        self.depth -= 1;
        self.bound_product /= (bound as u64).max(1);
    }

    // ---------------------------------------------------------------- types
    pub(crate) fn unify(&self, ta: &Ty, tb: &Ty, span: Span) -> CResult<Ty> {
        if ta == tb {
            return Ok(ta.clone());
        }
        let mismatch = || {
            err(
                ErrorCode::Type,
                span,
                format!("{} with {}", self.env.type_name(ta), self.env.type_name(tb)),
            )
        };
        if ta.is_literal() && tb.is_literal() {
            if matches!(
                (ta, tb),
                (Ty::LitInt, Ty::LitDec) | (Ty::LitDec, Ty::LitInt)
            ) {
                return Ok(Ty::LitDec);
            }
            return mismatch();
        }
        let (lit, con) = if ta.is_literal() { (ta, tb) } else { (tb, ta) };
        let ok = match lit {
            Ty::LitInt => con.is_numeric(),
            Ty::LitDec => con.is_fixed(),
            Ty::LitNone => matches!(con, Ty::Ent(_) | Ty::Event),
            _ => false,
        };
        if ok { Ok(con.clone()) } else { mismatch() }
    }

    #[allow(
        clippy::unused_self,
        reason = "INVARIANT: kept a method beside unify, its pair in the reference"
    )]
    pub(crate) fn concrete(&self, ty: &Ty, span: Span) -> CResult<Ty> {
        match ty {
            Ty::LitInt => Ok(Ty::Int),
            Ty::LitDec => Ok(Ty::Q16),
            Ty::LitNone => err(ErrorCode::Type, span, "none without an entity kind"),
            other => Ok(other.clone()),
        }
    }

    fn require_numeric(&self, ty: &Ty, span: Span) -> CResult<()> {
        if ty.is_numeric() || matches!(ty, Ty::LitInt | Ty::LitDec) {
            Ok(())
        } else {
            err(
                ErrorCode::Type,
                span,
                format!("{} is not numeric", self.env.type_name(ty)),
            )
        }
    }

    pub(crate) fn no_event(ty: Ty, span: Span) -> CResult<Ty> {
        if ty == Ty::Event {
            return err(
                ErrorCode::Type,
                span,
                "an event has no value outside a payload field",
            );
        }
        Ok(ty)
    }

    /// Resolves a node to its default concrete type after checking.
    pub(crate) fn finish(&mut self, node: &mut Node) -> CResult<Ty> {
        let ty = self.concrete(&node.ty, node.span)?;
        self.resolve(node, ty)?;
        Ok(node.ty.clone())
    }

    /// Gives a node (and its literal-carrying children) the concrete type `ty`.
    pub(crate) fn resolve(&mut self, node: &mut Node, ty: Ty) -> CResult<()> {
        let mut kids: [Option<&mut Node>; 2] = [None, None];
        match &mut node.kind {
            Kind::Num { text, bits } => match literal_bits(&ty, text) {
                Some(b) => *bits = b,
                None => return Err(self.literal_range(text, &ty, node.span)),
            },
            Kind::Call { name, args } if is_poly(name) => {
                return self.resolve_all(args, ty, &mut node.ty);
            }
            Kind::Agg {
                op: AggOp::Sum | AggOp::Min | AggOp::Max,
                body,
                orelse,
                ..
            } => {
                kids = [body.as_deref_mut(), orelse.as_deref_mut()];
            }
            Kind::If { a, b, .. } => kids = [Some(a), Some(b)],
            Kind::Neg(_, e) => kids = [Some(e), None],
            Kind::Chain { ops, items } if ops.first().is_some_and(|o| o.is_arith()) => {
                return self.resolve_all(items, ty, &mut node.ty);
            }
            _ => {}
        }
        for kid in kids.into_iter().flatten() {
            self.resolve(kid, ty.clone())?;
        }
        node.ty = ty;
        Ok(())
    }

    #[inline(never)]
    fn resolve_all(&mut self, args: &mut [Node], ty: Ty, own: &mut Ty) -> CResult<()> {
        for arg in args.iter_mut() {
            self.resolve(arg, ty.clone())?;
        }
        *own = ty;
        Ok(())
    }

    #[inline(never)]
    fn literal_range(&self, text: &str, ty: &Ty, span: Span) -> Rejection {
        Rejection::new(
            ErrorCode::LiteralRange,
            span,
            format!("{text} as {}", self.env.type_name(ty)),
        )
    }

    /// Types a node and returns its (possibly polymorphic) type. Each kind has its own small
    /// function, so a level of the tree costs a small stack frame.
    pub(crate) fn check(&mut self, node: &mut Node) -> CResult<Ty> {
        let ty = match &node.kind {
            Kind::Num { text, .. } => Ok(if text.contains('.') {
                Ty::LitDec
            } else {
                Ty::LitInt
            }),
            Kind::Bool(..) => Ok(Ty::Bool),
            Kind::None => Ok(Ty::LitNone),
            Kind::Var { .. } => self.of_var(node),
            Kind::Dotted { .. } => self.of_dotted(node),
            Kind::Read { .. } => self.of_read(node),
            Kind::Rel { .. } => self.of_rel(node),
            Kind::Call { .. } => self.check_call(node),
            Kind::Agg { .. } => self.check_agg(node),
            Kind::Tuple(..) => self.of_tuple(node),
            Kind::ListKey { .. } => self.of_list_key(node),
            Kind::If { .. } => self.of_if(node),
            Kind::Neg(..) => self.of_neg(node),
            Kind::Chain { .. } => self.of_chain(node),
            Kind::Not(..) => self.of_not(node),
            Kind::Bin { .. } => self.of_bin(node),
        }?;
        node.ty = ty.clone();
        Ok(ty)
    }

    #[inline(never)]
    fn of_var(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::Var { name, slot } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            match self.scope.get(name.as_str()) {
                Some((ty, s)) => {
                    *slot = *s;
                    ty.clone()
                }
                None => return err(ErrorCode::UnknownName, span, name.clone()),
            }
        })
    }

    #[inline(never)]
    fn of_dotted(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::Dotted { head, tail, res } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            let (ty, r) = self.check_dotted(head, tail, span)?;
            *res = r;
            ty
        })
    }

    #[inline(never)]
    fn of_read(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::Read {
            entity,
            column,
            arg,
            col,
            neutral,
        } = &mut node.kind
        else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            let name = format!("{entity}.{column}");
            let id = self.check_read(&name, span)?;
            *col = Some(id);
            *neutral = self.neutral_for(&name, id);
            let def = self.env.column(id).map(|c| (c.key, c.ty.clone()));
            let Some((key, cty)) = def else {
                return err(ErrorCode::UnknownName, span, name);
            };
            let at = self.check(arg)?;
            let want = self.unify(&at, &Ty::Ent(key), arg.span)?;
            self.resolve(arg, want)?;
            cty
        })
    }

    #[inline(never)]
    fn of_rel(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::Rel { rel, name, args } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            let spec = rel_spec(*rel);
            if args.len() != spec.args.len() {
                return err(ErrorCode::Arity, span, name.clone());
            }
            for (arg, want) in args.iter_mut().zip(spec.args) {
                match want {
                    ArgSpec::Degree(upper) => self.check_degree(arg, *upper)?,
                    ArgSpec::Ent(kind) => {
                        let k = self.kind(kind, arg.span)?;
                        let at = self.check(arg)?;
                        let t = self.unify(&at, &Ty::Ent(k), arg.span)?;
                        self.resolve(arg, t)?;
                    }
                }
            }
            for column in spec.reads {
                self.check_read(column, span)?;
            }
            Ty::Rel(self.kind(spec.yields, span)?)
        })
    }

    #[inline(never)]
    fn of_tuple(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::Tuple(items) = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            if !self.opts.allow_key {
                return err(ErrorCode::Type, span, "a tuple outside an order program");
            }
            let mut parts = Vec::with_capacity(items.len());
            for item in items.iter_mut() {
                self.check(item)?;
                let t = self.finish(item)?;
                parts.push(Self::no_event(t, item.span)?);
            }
            Ty::Tuple(parts)
        })
    }

    #[inline(never)]
    fn of_list_key(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::ListKey {
            body,
            var,
            slot,
            rel,
        } = &mut node.kind
        else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            if !self.opts.allow_key {
                return err(ErrorCode::Type, span, "a list key outside an order program");
            }
            let rel_ty = self.check(rel)?;
            let bound = rel_bound(rel);
            self.enter(bound, span)?;
            *slot = self.bind(var, ent_of(&rel_ty), true, span)?;
            self.check(body)?;
            let element = self
                .finish(body)
                .and_then(|t| Self::no_event(t, body.span))?;
            self.unbind(var);
            self.leave(bound);
            Ty::List(Box::new(element))
        })
    }

    #[inline(never)]
    fn of_if(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let _ = span;
        let Kind::If { cond, a, b } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        Ok({
            let ct = self.check(cond)?;
            let ct = self.unify(&ct, &Ty::Bool, cond.span)?;
            self.resolve(cond, ct)?;
            let ta = self.check(a)?;
            let tb = self.check(b)?;
            let ty = self.unify(&ta, &tb, span)?;
            if !ty.is_literal() {
                self.resolve(a, ty.clone())?;
                self.resolve(b, ty.clone())?;
            }
            ty
        })
    }

    #[inline(never)]
    fn of_neg(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Neg(_, e) = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        let ty = self.check(e)?;
        self.require_numeric(&ty, span)?;
        Ok(ty)
    }

    #[inline(never)]
    fn of_not(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Not(_, e) = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        let t = self.check(e)?;
        let t = self.unify(&t, &Ty::Bool, e.span)?;
        self.resolve(e, t)?;
        Ok(Ty::Bool)
    }

    #[inline(never)]
    fn of_bin(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Bin { op, a, b } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        self.bin_cmp(*op, a, b, span)
    }

    /// A chain, checked as the reference checks its left-deep tree: operand by operand, the
    /// running type unified with each, resolved as soon as it is concrete (§4.1).
    #[inline(never)]
    fn of_chain(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Chain { ops, items } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "node");
        };
        if ops
            .first()
            .is_some_and(|o| matches!(o, BinOp::And | BinOp::Or))
        {
            for item in items.iter_mut() {
                let t = self.check(item)?;
                let t = self.unify(&t, &Ty::Bool, item.span)?;
                self.resolve(item, t)?;
            }
            return Ok(Ty::Bool);
        }
        let mut ty = self.check(&mut items[0])?;
        let first = items[0].span;
        let mut resolved = false;
        for i in 1..items.len() {
            let op = ops[i - 1];
            let level = Span {
                start: first.start,
                end: items[i].span.end,
            };
            let t = self.check(&mut items[i])?;
            ty = self.unify(&ty, &t, level)?;
            self.require_numeric(&ty, level)?;
            if matches!(op, BinOp::IntDiv | BinOp::Mod) && !matches!(ty, Ty::Int | Ty::LitInt) {
                return Err(self.bad_comparison(op, &ty, level));
            }
            if !ty.is_literal() {
                // Once concrete the running type never changes, so the operands before this one
                // are resolved once, when it first becomes concrete.
                let from = if resolved { i } else { 0 };
                for item in &mut items[from..=i] {
                    self.resolve(item, ty.clone())?;
                }
                resolved = true;
            }
            if op.is_division() {
                let c = self.concrete(&ty, level)?;
                self.check_divisor(&items[i], &c)?;
            }
        }
        Ok(ty)
    }

    #[inline(never)]
    fn bin_cmp(&mut self, op: BinOp, a: &mut Node, b: &mut Node, span: Span) -> CResult<Ty> {
        let ta = self.check(a)?;
        let tb = self.check(b)?;
        let ty = self.unify(&ta, &tb, span)?;
        let ty = self.concrete(&ty, span)?;
        if ty.is_key()
            || matches!(ty, Ty::Rel(_) | Ty::Event)
            || (op.is_order() && !ty.is_numeric())
        {
            return Err(self.bad_comparison(op, &ty, span));
        }
        self.resolve(a, ty.clone())?;
        self.resolve(b, ty)?;
        Ok(Ty::Bool)
    }

    #[inline(never)]
    fn bad_comparison(&self, op: BinOp, ty: &Ty, span: Span) -> Rejection {
        Rejection::new(
            ErrorCode::Type,
            span,
            format!("{op:?} on {}", self.env.type_name(ty)),
        )
    }

    fn check_dotted(&self, head: &str, tail: &str, span: Span) -> CResult<(Ty, Dotted)> {
        if head == "params" {
            let system = self.system(span)?;
            return match system.params.get(tail) {
                Some(p) => Ok((p.ty(), Dotted::Param)),
                None => err(ErrorCode::UnknownName, span, format!("params.{tail}")),
            };
        }
        if head == "calendar" {
            return match tail {
                "days_per_year" => Ok((Ty::Int, Dotted::DaysPerYear)),
                "minutes_per_day" => Ok((Ty::Int, Dotted::MinutesPerDay)),
                _ => err(ErrorCode::UnknownName, span, format!("calendar.{tail}")),
            };
        }
        if let Some(e) = self.env.enum_id(head)
            && let Some(i) = self
                .env
                .enum_def(e)
                .and_then(|d| d.values.iter().position(|v| v == tail))
        {
            let i = u32::try_from(i).unwrap_or(u32::MAX);
            return Ok((Ty::Enum(e), Dotted::Enum(super::value::Value::Enum(e, i))));
        }
        err(ErrorCode::UnknownName, span, format!("{head}.{tail}"))
    }

    /// A divisor must be a constant whose bits, in the division's type, cannot be zero (§4.4).
    fn check_divisor(&self, b: &Node, ty: &Ty) -> CResult<()> {
        // The reference strips one minus only: `x / --2` is E_DIV.
        let d = if let Kind::Neg(1, e) = &b.kind { e } else { b };
        match &d.kind {
            Kind::Num { text, .. } => {
                let Some(bits) = literal_bits(ty, text) else {
                    return err(
                        ErrorCode::LiteralRange,
                        d.span,
                        format!("{text} as {}", self.env.type_name(ty)),
                    );
                };
                if bits != 0 {
                    return Ok(());
                }
            }
            Kind::Dotted { head, tail, .. } if head == "calendar" => {
                let _ = tail;
                return Ok(());
            }
            Kind::Dotted { head, tail, .. } if head == "params" => {
                let p = self.system(d.span)?.params.get(tail.as_str());
                if p.is_some_and(|p| p.lo > 0 || p.hi < 0) {
                    return Ok(());
                }
            }
            _ => {}
        }
        err(
            ErrorCode::Div,
            b.span,
            "divisor is not a constant with non-zero bits",
        )
    }

    /// A degree: an `Int` literal, or an `int` parameter whose bounds lie in `1..upper`.
    fn check_degree(&mut self, arg: &mut Node, upper: i128) -> CResult<()> {
        let span = arg.span;
        let (lo, hi) = match &arg.kind {
            Kind::Num { text, .. } if !text.contains('.') => {
                let v = text.bytes().try_fold(0_i128, |acc, d| {
                    acc.checked_mul(10)?.checked_add(i128::from(d - b'0'))
                });
                let v = v.unwrap_or(i128::MAX);
                (v, v)
            }
            Kind::Dotted { head, .. } if head == "params" => {
                if self.check(arg)? != Ty::Int {
                    return err(
                        ErrorCode::Type,
                        span,
                        "the parameter is not an int parameter",
                    );
                }
                let Kind::Dotted { tail, .. } = &arg.kind else {
                    return err(ErrorCode::Const, span, "degree");
                };
                let p = self.system(span)?.params.get(tail.as_str()).copied();
                let Some(p) = p else {
                    return err(ErrorCode::UnknownName, span, format!("params.{tail}"));
                };
                (p.lo, p.hi)
            }
            _ => {
                return err(
                    ErrorCode::Const,
                    span,
                    "a degree must be an Int literal or an int parameter",
                );
            }
        };
        if !(1 <= lo && lo <= hi && hi <= upper) {
            return err(
                ErrorCode::Bound,
                span,
                format!("degree in [{lo}, {hi}] outside 1..{upper}"),
            );
        }
        self.resolve(arg, Ty::Int)
    }

    fn arity(n: usize, args: &[Node], name: &str, span: Span) -> CResult<()> {
        if args.len() == n {
            Ok(())
        } else {
            err(ErrorCode::Arity, span, format!("{name} takes {n}"))
        }
    }

    fn check_call(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Call { name, args } = &mut node.kind else {
            return err(ErrorCode::Syntax, span, "call");
        };
        let name = name.clone();
        match name.as_str() {
            "ordinal" => self.call_ordinal(&name, args, span),
            "exists" => self.call_exists(&name, args, span),
            "floor" | "round" => self.call_floor(&name, args, span),
            "draw" | "chance" => self.check_draw(&name, args, span),
            "related" => self.call_related(&name, args, span),
            "widen" | "narrow" | "q8" | "q16" => self.call_widen(&name, args, span),
            "abs" | "min" | "max" | "clamp" | "safe_div" => self.call_abs(&name, args, span),
            "up" | "common" | "up_agnatic" | "common_agnatic" | "descends" => {
                self.call_up(&name, args, span)
            }
            _ => err(ErrorCode::UnknownName, span, format!("call {name}")),
        }
    }

    #[inline(never)]
    fn call_ordinal(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            Self::arity(1, args, name, span)?;
            if !matches!(self.check(&mut args[0])?, Ty::Enum(_)) {
                return err(ErrorCode::Type, span, "ordinal of a non-enum");
            }
            Ok(Ty::Int)
        }
    }

    #[inline(never)]
    fn call_exists(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            Self::arity(1, args, name, span)?;
            let t = self.check(&mut args[0])?;
            let t = self.concrete(&t, args[0].span)?;
            if !matches!(t, Ty::Ent(_)) {
                return err(ErrorCode::Type, span, "exists of a non-entity");
            }
            self.resolve(&mut args[0], t)?;
            Ok(Ty::Bool)
        }
    }

    #[inline(never)]
    fn call_floor(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            Self::arity(1, args, name, span)?;
            let mut t = self.check(&mut args[0])?;
            if t == Ty::LitDec {
                t = Ty::Q16;
            }
            if !t.is_fixed() {
                return err(
                    ErrorCode::Type,
                    span,
                    format!("{name} of {}", self.env.type_name(&t)),
                );
            }
            self.resolve(&mut args[0], t)?;
            Ok(Ty::Int)
        }
    }

    #[inline(never)]
    fn call_related(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            Self::arity(3, args, name, span)?;
            let person = self.kind("person", span)?;
            for arg in args.iter_mut().take(2) {
                let t = self.check(arg)?;
                let t = self.unify(&t, &Ty::Ent(person), arg.span)?;
                self.resolve(arg, t)?;
            }
            self.check_degree(&mut args[2], KIN_DEGREES_MAX)?;
            for column in GENEALOGY {
                self.check_read(column, span)?;
            }
            Ok(Ty::Bool)
        }
    }

    #[inline(never)]
    fn call_widen(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            let (want, result) = match name {
                "widen" => (Ty::Q8, Ty::Q16),
                "narrow" => (Ty::Q16, Ty::Q8),
                "q8" => (Ty::Int, Ty::Q8),
                _ => (Ty::Int, Ty::Q16),
            };
            Self::arity(1, args, name, span)?;
            let t = self.check(&mut args[0])?;
            let t = self.unify(&t, &want, args[0].span)?;
            self.resolve(&mut args[0], t)?;
            Ok(result)
        }
    }

    #[inline(never)]
    fn call_abs(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            let n = match name {
                "abs" => 1,
                "min" | "max" => 2,
                _ => 3,
            };
            Self::arity(n, args, name, span)?;
            let (first, rest) = args.split_at_mut(1);
            let mut ty = self.check(&mut first[0])?;
            for arg in rest {
                let t = self.check(arg)?;
                ty = self.unify(&ty, &t, arg.span)?;
            }
            self.require_numeric(&ty, span)?;
            if !ty.is_literal() {
                for arg in args.iter_mut() {
                    self.resolve(arg, ty.clone())?;
                }
            }
            Ok(ty)
        }
    }

    #[inline(never)]
    fn call_up(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        {
            Self::arity(2, args, name, span)?;
            let person = self.kind("person", span)?;
            for arg in args.iter_mut() {
                let t = self.check(arg)?;
                let t = self.unify(&t, &Ty::Ent(person), arg.span)?;
                self.resolve(arg, t)?;
            }
            let reads: &[&str] = if name.ends_with("_agnatic") {
                &AGNATIC
            } else {
                &GENEALOGY
            };
            for column in reads {
                self.check_read(column, span)?;
            }
            Ok(match name {
                "up" | "up_agnatic" => Ty::Int,
                "descends" => Ty::Bool,
                _ => Ty::Ent(person),
            })
        }
    }

    fn check_draw(&mut self, name: &str, args: &mut [Node], span: Span) -> CResult<Ty> {
        Self::arity(if name == "draw" { 3 } else { 4 }, args, name, span)?;
        let purpose = match &args[0].kind {
            Kind::Var { name, .. } => name.clone(),
            _ => {
                return err(
                    ErrorCode::Const,
                    args[0].span,
                    "purpose must be an identifier",
                );
            }
        };
        if !self.system(span)?.rng.contains(&purpose) {
            return err(ErrorCode::RngPurpose, args[0].span, purpose);
        }
        if self.opts.outcome_payload {
            return err(
                ErrorCode::RngReuse,
                span,
                "a verb with an outcome payload draws nothing",
            );
        }
        if self.opts.verb_program
            && !args[1..3]
                .iter()
                .any(|n| mentions(n, "actor") || mentions(n, "target"))
        {
            return err(
                ErrorCode::RngReuse,
                span,
                "a verb's draw must key on its actor or target",
            );
        }
        let t = self.check(&mut args[1])?;
        let t = self.concrete(&t, args[1].span)?;
        self.resolve(&mut args[1], t)?;
        if !matches!(args[1].ty, Ty::Ent(_)) {
            return err(ErrorCode::Type, args[1].span, "draw entity");
        }
        let t = self.check(&mut args[2])?;
        let t = self.concrete(&t, args[2].span)?;
        self.resolve(&mut args[2], t)?;
        if !matches!(args[2].ty, Ty::Int | Ty::Ent(_)) {
            return err(
                ErrorCode::Type,
                args[2].span,
                "draw index must be an int or an entity",
            );
        }
        if name == "chance" {
            let t = self.check(&mut args[3])?;
            let t = self.unify(&t, &Ty::Q16, args[3].span)?;
            self.resolve(&mut args[3], t)?;
        }
        let site = format!(
            "{purpose}/{}/{}",
            self.site_key(&args[1]),
            self.site_key(&args[2])
        );
        self.note_draw(site, [&args[1], &args[2]], span)?;
        Ok(if name == "draw" { Ty::Int } else { Ty::Bool })
    }

    fn site_key(&self, node: &Node) -> String {
        if let Some(v) = fold_int(node) {
            format!("int:{v}")
        } else {
            let mut text = format!("{}:", self.env.type_name(&node.ty));
            canon(node, &mut text);
            text
        }
    }

    fn check_agg(&mut self, node: &mut Node) -> CResult<Ty> {
        let span = node.span;
        let Kind::Agg {
            op,
            var,
            slot,
            rel,
            cond,
            body,
            orelse,
        } = &mut node.kind
        else {
            return err(ErrorCode::Syntax, span, "aggregate");
        };
        let rel_ty = self.check(rel)?;
        let bound = rel_bound(rel);
        self.enter(bound, span)?;
        *slot = self.bind(var, ent_of(&rel_ty), true, span)?;
        if let Some(c) = cond.as_deref_mut() {
            let t = self.check(c)?;
            let t = self.unify(&t, &Ty::Bool, c.span)?;
            self.resolve(c, t)?;
        }
        let ty = match op {
            AggOp::Count | AggOp::Any | AggOp::All => {
                if orelse.is_some() || (*op == AggOp::Count && body.is_some()) {
                    return err(ErrorCode::Syntax, span, "takes no body or else");
                }
                if let Some(b) = body.as_deref_mut() {
                    if cond.is_some() {
                        return err(ErrorCode::Syntax, span, "both where and a body");
                    }
                    let t = self.check(b)?;
                    let t = self.unify(&t, &Ty::Bool, b.span)?;
                    self.resolve(b, t)?;
                }
                if *op == AggOp::Count {
                    Ty::Int
                } else {
                    Ty::Bool
                }
            }
            AggOp::Sum | AggOp::Min | AggOp::Max => {
                let needs_else = *op != AggOp::Sum;
                let Some(b) = body.as_deref_mut() else {
                    return err(ErrorCode::Syntax, span, "a body is required");
                };
                if needs_else != orelse.is_some() {
                    return err(
                        ErrorCode::Syntax,
                        span,
                        "else is required for min and max only",
                    );
                }
                let mut ty = self.check(b)?;
                if let Some(o) = orelse.as_deref_mut() {
                    let t = self.check(o)?;
                    ty = self.unify(&ty, &t, o.span)?;
                }
                self.require_numeric(&ty, span)?;
                if !ty.is_literal() {
                    self.resolve(b, ty.clone())?;
                    if let Some(o) = orelse.as_deref_mut() {
                        self.resolve(o, ty.clone())?;
                    }
                }
                ty
            }
        };
        self.unbind(var);
        self.leave(bound);
        Ok(ty)
    }

    // ---------------------------------------------------------------- effects
    pub(crate) fn check_effect(&mut self, effect: &mut Effect) -> CResult<()> {
        if let Some(when) = effect.when.as_mut() {
            let t = self.check(when)?;
            let t = self.unify(&t, &Ty::Bool, when.span)?;
            self.resolve(when, t)?;
        }
        let Some(each) = effect.each.as_mut() else {
            return self.check_action(&mut effect.action);
        };
        let rel_ty = self.check(&mut each.rel)?;
        let bound = rel_bound(&each.rel);
        let span = each.rel.span;
        self.enter(bound, span)?;
        each.slot = self.bind(&each.var, ent_of(&rel_ty), true, span)?;
        if let Some(c) = each.cond.as_mut() {
            let t = self.check(c)?;
            let t = self.unify(&t, &Ty::Bool, c.span)?;
            self.resolve(c, t)?;
        }
        self.check_action(&mut effect.action)?;
        self.unbind(&each.var);
        self.leave(bound);
        Ok(())
    }

    fn check_action(&mut self, action: &mut Action) -> CResult<()> {
        let span = action.span;
        let Some(id) = self.env.command_id(&action.name) else {
            return err(
                ErrorCode::UnknownName,
                span,
                format!("command {}", action.name),
            );
        };
        let Some(command) = self.env.command(id) else {
            return err(
                ErrorCode::UnknownName,
                span,
                format!("command {}", action.name),
            );
        };
        let Some(target) = self.system(span)?.sends.get(&action.name) else {
            return err(
                ErrorCode::UndeclaredCommand,
                span,
                format!("{} sends {}", self.system_name, action.name),
            );
        };
        if *target != command.owner {
            return err(
                ErrorCode::CommandOwner,
                span,
                format!("{} to {target}, owner {}", action.name, command.owner),
            );
        }
        action.command = Some(id);
        action.dropped = !self.env.registered.contains(&command.owner);
        let person = self.kind("person", span)?;
        let mut wanted: BTreeMap<&str, Ty> = BTreeMap::new();
        wanted.insert("person", Ty::Ent(person));
        for (name, ty) in &command.params {
            wanted.insert(name.as_str(), ty.clone());
        }
        let mut given: Vec<&str> = action.args.iter().map(|(n, _)| n.as_str()).collect();
        given.sort_unstable();
        if given != wanted.keys().copied().collect::<Vec<_>>() {
            return err(
                ErrorCode::Arity,
                span,
                format!("{} arguments {given:?}", action.name),
            );
        }
        for (name, node) in &mut action.args {
            let want = wanted.get(name.as_str()).cloned().unwrap_or(Ty::Int);
            let t = self.check(node)?;
            let t = self.unify(&t, &want, node.span)?;
            self.resolve(node, t)?;
        }
        let by_name: BTreeMap<&str, usize> = action
            .args
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.as_str(), i))
            .collect();
        action.person_index = by_name.get("person").copied().unwrap_or(0);
        action.order = command
            .params
            .iter()
            .map(|(n, _)| by_name.get(n.as_str()).copied().unwrap_or(0))
            .collect();
        Ok(())
    }
}

fn is_poly(name: &str) -> bool {
    matches!(name, "abs" | "min" | "max" | "clamp" | "safe_div")
}

fn rel_bound(rel: &Node) -> usize {
    match &rel.kind {
        Kind::Rel { rel, .. } => rel_spec(*rel).bound,
        _ => 1,
    }
}

fn ent_of(rel_ty: &Ty) -> Ty {
    match rel_ty {
        Ty::Rel(k) => Ty::Ent(*k),
        other => other.clone(),
    }
}

/// The value of an `int` constant expression (literals, the day length), or `None` (§4.6: the
/// draw-site lint folds constants; parameters do not fold).
pub(crate) fn fold_int(node: &Node) -> Option<i64> {
    match &node.kind {
        Kind::Num { text, .. } if !text.contains('.') => {
            let v = text.bytes().try_fold(0_i128, |acc, d| {
                acc.checked_mul(10)?.checked_add(i128::from(d - b'0'))
            })?;
            Some(sat_i64(v))
        }
        Kind::Dotted { head, tail, .. } if head == "calendar" && tail == "minutes_per_day" => {
            Some(MINUTES_PER_DAY)
        }
        Kind::Neg(n, e) => {
            let mut v = fold_int(e)?;
            for _ in 0..*n {
                v = v.saturating_neg();
            }
            Some(v)
        }
        Kind::Chain { ops, items } if ops.first().is_some_and(|o| o.is_arith()) => {
            let mut acc = fold_int(&items[0])?;
            for (op, item) in ops.iter().zip(&items[1..]) {
                let b = fold_int(item)?;
                acc = arith_int(arith_op(*op), acc, b)?;
            }
            Some(acc)
        }
        _ => None,
    }
}

/// The arithmetic operator of a chainable arithmetic `BinOp`.
pub(crate) fn arith_op(op: BinOp) -> ArithOp {
    match op {
        BinOp::Add => ArithOp::Add,
        BinOp::Sub => ArithOp::Sub,
        BinOp::Mul => ArithOp::Mul,
        BinOp::Div => ArithOp::Div,
        BinOp::IntDiv => ArithOp::IntDiv,
        _ => ArithOp::Mod,
    }
}
