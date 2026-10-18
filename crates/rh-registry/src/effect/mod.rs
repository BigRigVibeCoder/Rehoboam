//! The Registry effect language v0: a typed, total expression language for succession laws, verb
//! effects and vow predicates (`specs/registry/effect_language_v0.md` 0.5.0, D37).
//!
//! A program is Registry data, compiled once against an [`Env`] (§7.4) and evaluated by its host
//! over one snapshot through the [`host`] traits. It reads only its system's declared columns,
//! iterates only over the bounded prelude relations (§5.3), draws only from the keyed RNG under a
//! declared purpose (§4.6), and changes nothing: an effect program yields typed actions the host
//! queues to their owners (§6). Every well-typed program terminates with a value; the evaluator
//! has two run-time failures only, both hard invariants: a `lineage` above its bound after pruning
//! ([`EvalError::Truncated`]) and a draw key repeated within one ledger ([`EvalError::RngReuse`]).
//!
//! - [`env`]: the environment; [`value`]: values, the §4.3 arithmetic and the golden rendering;
//! - [`compile`] and [`compile_with`]: the front end (lexer, parser, checker, §2–§4, §10);
//! - [`Compiled`]: one program, evaluated per binding; [`VerbStep`]: a step's verb evaluations;
//! - [`succession`]: the protocol of §7.2 over a law's three programs;
//! - [`host`]: the seam to the engine (Q9); [`seam`]: the host over a system's `SystemView` and
//!   `StepCtx`, and an action's `Command` (T-07).
//!
//! Plane A, float-free, no I/O, `BTreeMap`/`BTreeSet`/`Vec` only. The reference implementation is
//! `specs/registry/effect_ref.py`; the golden cases (`effect_language_golden_v0.toml`) are the
//! contract, reproduced by `crates/rh-manifest/tests/effect_golden.rs` in debug and release.
//!
//! REF: `specs/registry/effect_language_v0.md` §0–§13; `CODEX/05_PROJECT/SPR-017_EffectLanguage.md`.

mod ast;
mod check;
pub mod env;
mod eval;
pub mod host;
mod lex;
mod parse;
mod prelude;
pub mod seam;
pub mod succession;
pub mod value;

use std::fmt;

pub use check::{BOUND_PRODUCT_MAX, CheckOptions, LINEAGE_BOUND, NESTING_MAX};
pub use env::{ColumnId, CommandId, EnumId, Env, EnvError, KindId, Ty};
pub use eval::{Action, Arg, DrawKey, DrawLedger};
pub use host::{Draws, Host, Indexes, ReadOutcome, Reads};
pub use parse::EXPRESSION_DEPTH_MAX;
pub use value::{EventArg, Value};

use ast::{Effect, Node};
use check::Checker;
use eval::Ev;
use parse::Parser;

/// The language version this evaluator implements (§11): a Registry or golden file of another
/// version is refused.
pub const LANGUAGE_VERSION: &str = "0.5.1";

/// A byte range of the program text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// The first byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

/// The compile-time error classes of §10 (a closed set).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCode {
    /// `E_SYNTAX`.
    Syntax,
    /// `E_FLOAT_LITERAL`.
    FloatLiteral,
    /// `E_LITERAL_RANGE`.
    LiteralRange,
    /// `E_UNKNOWN_NAME`.
    UnknownName,
    /// `E_TYPE`.
    Type,
    /// `E_ARITY`.
    Arity,
    /// `E_CONST`.
    Const,
    /// `E_DIV`.
    Div,
    /// `E_BOUND`.
    Bound,
    /// `E_PLANE`.
    Plane,
    /// `E_UNDECLARED_READ`.
    UndeclaredRead,
    /// `E_NO_NEUTRAL`.
    NoNeutral,
    /// `E_UNDECLARED_COMMAND`.
    UndeclaredCommand,
    /// `E_COMMAND_OWNER`.
    CommandOwner,
    /// `E_RNG_PURPOSE`.
    RngPurpose,
    /// `E_RNG_REUSE`.
    RngReuse,
}

impl ErrorCode {
    /// Every code, in §10's order.
    pub const ALL: [Self; 16] = [
        Self::Syntax,
        Self::FloatLiteral,
        Self::LiteralRange,
        Self::UnknownName,
        Self::Type,
        Self::Arity,
        Self::Const,
        Self::Div,
        Self::Bound,
        Self::Plane,
        Self::UndeclaredRead,
        Self::NoNeutral,
        Self::UndeclaredCommand,
        Self::CommandOwner,
        Self::RngPurpose,
        Self::RngReuse,
    ];

    /// The code's name, as §10 and the golden file spell it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "E_SYNTAX",
            Self::FloatLiteral => "E_FLOAT_LITERAL",
            Self::LiteralRange => "E_LITERAL_RANGE",
            Self::UnknownName => "E_UNKNOWN_NAME",
            Self::Type => "E_TYPE",
            Self::Arity => "E_ARITY",
            Self::Const => "E_CONST",
            Self::Div => "E_DIV",
            Self::Bound => "E_BOUND",
            Self::Plane => "E_PLANE",
            Self::UndeclaredRead => "E_UNDECLARED_READ",
            Self::NoNeutral => "E_NO_NEUTRAL",
            Self::UndeclaredCommand => "E_UNDECLARED_COMMAND",
            Self::CommandOwner => "E_COMMAND_OWNER",
            Self::RngPurpose => "E_RNG_PURPOSE",
            Self::RngReuse => "E_RNG_REUSE",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A program rejected at compile time: its code, where, and a message (not contractual).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    /// The §10 code.
    pub code: ErrorCode,
    /// Where the checker met the fault.
    pub span: Span,
    /// A human-readable detail.
    pub message: String,
}

impl Rejection {
    pub(crate) fn new(code: ErrorCode, span: Span, message: String) -> Self {
        Self {
            code,
            span,
            message,
        }
    }
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}..{}: {}",
            self.code, self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for Rejection {}

/// A run-time failure. The first two are the language's hard invariants (§4.8); the others mean
/// the host broke its side of the seam (a parameter or a draw it does not have).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// `E_TRUNCATED`: a candidates relation above its bound after pruning (§5.2).
    Truncated {
        /// The relation.
        relation: String,
        /// Its size after pruning.
        members: usize,
    },
    /// `E_RNG_REUSE`: a key drawn twice within one ledger (§4.6).
    RngReuse {
        /// The purpose.
        purpose: String,
        /// The entity's kind.
        kind: String,
        /// The entity's id.
        id: u64,
        /// The index.
        k: i64,
    },
    /// The host has no draw for a key.
    DrawUnavailable {
        /// The purpose.
        purpose: String,
        /// The entity's kind.
        kind: String,
        /// The entity's id.
        id: u64,
        /// The index.
        k: i64,
    },
    /// The host has no value for a parameter.
    MissingParam(String),
    /// A broken internal invariant of the evaluator (a bug, never a program's fault).
    Internal(String),
}

impl EvalError {
    /// The invariant's code (`E_TRUNCATED`, `E_RNG_REUSE`), if this is one.
    #[must_use]
    pub fn invariant(&self) -> Option<&'static str> {
        match self {
            Self::Truncated { .. } => Some("E_TRUNCATED"),
            Self::RngReuse { .. } => Some("E_RNG_REUSE"),
            _ => None,
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { relation, members } => {
                write!(
                    f,
                    "E_TRUNCATED: {relation} has {members} members after pruning, above its bound"
                )
            }
            Self::RngReuse {
                purpose,
                kind,
                id,
                k,
            } => {
                write!(f, "E_RNG_REUSE: ({purpose}, {kind}:{id}, {k}) drawn twice")
            }
            Self::DrawUnavailable {
                purpose,
                kind,
                id,
                k,
            } => {
                write!(f, "the host has no draw for ({purpose}, {kind}:{id}, {k})")
            }
            Self::MissingParam(name) => write!(f, "the host has no value for params.{name}"),
            Self::Internal(what) => write!(f, "evaluator invariant broken: {what}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// The evaluator's diagnostics (§4.5). None of them enters a value or a hash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    /// Reads through `none` or of an id with no row.
    pub missing_reads: u64,
    /// Reads of an absent owner's column, which took the coupling's neutral (NEW-4).
    pub neutral_reads: u64,
    /// Relations cut at their bound (every relation but `lineage`).
    pub truncated_relations: u64,
    /// Actions to an unregistered owner, not emitted (NEW-4).
    pub dropped_actions: u64,
}

/// A planted fault for the mutation tests (T-05). Only a build with the `mutant-effect` feature
/// honours a mutant; every other build evaluates [`Mutant::None`] whatever is passed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mutant {
    /// The language as specified.
    #[default]
    None,
    /// The life-course log read by row id (REV-SPR-017 M7); a host-side mutant.
    LogByRowId,
    /// `and` and `or` short-circuit (§4.5, minor 3).
    ShortCircuit,
    /// `lineage` stops at height 2 (B2).
    LineageHeightTwo,
    /// The lineage bound applied before pruning (N2).
    BoundBeforePrune,
    /// A silent cut above the lineage bound (§5.2).
    SilentCut,
    /// An absent owner's read gives the type's zero or default (M5).
    NeutralIsZero,
    /// A missing enum read gives ordinal 0 (round 1 minor).
    MissingEnumOrdinalZero,
    /// Fixed-point multiply rounds half up (D1).
    MulHalfUp,
    /// `common` and `line` go mother-first (D4).
    MotherFirst,
    /// The law's draw ledger kept per step, not per evaluation (N6); a host-side mutant.
    LedgerPerStep,
    /// The final tie-break by descending `person_id` (§7.2, NEW-7).
    TieBreakDescending,
    /// A list key puts the prefix last (§3.4).
    PrefixLast,
}

impl Mutant {
    /// Every planted mutant (not `None`).
    pub const ALL: [Self; 12] = [
        Self::LogByRowId,
        Self::ShortCircuit,
        Self::LineageHeightTwo,
        Self::BoundBeforePrune,
        Self::SilentCut,
        Self::NeutralIsZero,
        Self::MissingEnumOrdinalZero,
        Self::MulHalfUp,
        Self::MotherFirst,
        Self::LedgerPerStep,
        Self::TieBreakDescending,
        Self::PrefixLast,
    ];

    /// Whether this build honours mutants (the `mutant-effect` feature).
    #[must_use]
    pub const fn enabled() -> bool {
        cfg!(feature = "mutant-effect")
    }
}

/// Options of an evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvalOptions {
    /// A planted mutant (test builds only).
    pub mutant: Mutant,
}

/// The program kinds of §7.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramKind {
    /// A law's `candidates`: a relation of persons.
    Candidates,
    /// A law's `eligible`: `bool`, per candidate `x`.
    Eligible,
    /// A law's `order`: a key, per candidate `x`.
    Order,
    /// A verb's `effects`.
    Effects,
    /// A vow kind's `fulfilled`: `bool`.
    Fulfilled,
    /// A vow kind's `conflict`: `q8_8`.
    Conflict,
    /// An overlay's `derive` (reserved, Q10): `q8_8`.
    Derive,
}

/// The syntactic form of a program (§2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// An expression, optionally of a required result type.
    Expr,
    /// A relation (`candidates`).
    Candidates,
    /// An effect list.
    Effects,
}

/// A compilation's inputs beyond the text: the bound variables, the expected result type and the
/// kind's options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileOptions {
    /// The bound variables, in binding order; the evaluation passes their values in this order.
    pub scope: Vec<(String, Ty)>,
    /// The variables that are loop variables for the draw lint (a law's `x`).
    pub loop_vars: Vec<String>,
    /// The required result type of an expression program, if the kind declares one.
    pub expected: Option<Ty>,
    /// Key, verb and payload options.
    pub check: CheckOptions,
}

#[derive(Clone, Debug)]
enum Body {
    Expr(Node),
    Candidates(Node),
    Effects(Vec<Effect>),
}

/// A compiled program.
#[derive(Clone, Debug)]
pub struct Compiled {
    body: Body,
    scope: Vec<(String, Ty)>,
    slot_of: Vec<usize>,
    slots: usize,
    ty: Ty,
    sites: Vec<String>,
    system: String,
}

/// What an evaluation yields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    /// An expression's value (a key for `order`).
    Value(Value),
    /// A `candidates` relation's elements.
    Relation(Vec<u64>),
    /// An effect program's actions, in emission order.
    Effects(Vec<Action>),
}

/// The bound variables and options §7.1 gives a kind (a host adds a verb's payload fields to an
/// `Effects` program's scope before [`compile_with`]).
///
/// # Errors
/// An environment without the `person`, `title` or `artifact` kind.
pub fn kind_options(kind: ProgramKind, env: &Env) -> Result<CompileOptions, Rejection> {
    let k = |name: &str| {
        env.kind(name).ok_or_else(|| {
            Rejection::new(
                ErrorCode::UnknownName,
                Span::default(),
                format!("entity kind {name}"),
            )
        })
    };
    let person = Ty::Ent(k("person")?);
    let mut o = CompileOptions::default();
    match kind {
        ProgramKind::Candidates | ProgramKind::Eligible | ProgramKind::Order => {
            o.scope = vec![
                ("holder".into(), person.clone()),
                ("title".into(), Ty::Ent(k("title")?)),
                ("now".into(), Ty::Int),
            ];
            if kind != ProgramKind::Candidates {
                o.scope.push(("x".into(), person));
                o.loop_vars.push("x".into());
            }
            o.expected = (kind == ProgramKind::Eligible).then_some(Ty::Bool);
            o.check.allow_key = kind == ProgramKind::Order;
        }
        ProgramKind::Effects => {
            o.scope = vec![
                ("actor".into(), person.clone()),
                ("target".into(), person),
                ("instrument".into(), Ty::Ent(k("artifact")?)),
                ("event".into(), Ty::Event),
                ("now".into(), Ty::Int),
            ];
            o.check.verb_program = true;
        }
        ProgramKind::Fulfilled | ProgramKind::Conflict => {
            o.scope = vec![
                ("vower".into(), person.clone()),
                ("target".into(), person),
                ("now".into(), Ty::Int),
            ];
            o.expected = Some(if kind == ProgramKind::Fulfilled {
                Ty::Bool
            } else {
                Ty::Q8
            });
        }
        ProgramKind::Derive => {
            o.scope = vec![("self".into(), person), ("now".into(), Ty::Int)];
            o.expected = Some(Ty::Q8);
        }
    }
    Ok(o)
}

/// Compiles a program of a §7.1 kind for `system`, with the kind's bound variables. A vow
/// predicate's `target` is bound as a person here; a host whose kind targets another entity kind
/// compiles with [`compile_with`]. A verb with a payload passes its fields through
/// [`compile_with`].
///
/// # Errors
/// The first fault met, with its §10 code.
pub fn compile(
    kind: ProgramKind,
    text: &str,
    env: &Env,
    system: &str,
) -> Result<Compiled, Rejection> {
    let opts = kind_options(kind, env)?;
    let form = match kind {
        ProgramKind::Candidates => Form::Candidates,
        ProgramKind::Effects => Form::Effects,
        _ => Form::Expr,
    };
    compile_with(form, text, env, system, &opts)
}

/// Compiles a program of a form with explicit bound variables and options.
///
/// # Errors
/// The first fault met, with its §10 code.
pub fn compile_with(
    form: Form,
    text: &str,
    env: &Env,
    system: &str,
    opts: &CompileOptions,
) -> Result<Compiled, Rejection> {
    let mut cx = Checker::new(env, system, opts.check.clone());
    for (name, ty) in &opts.scope {
        cx.bind(
            name,
            ty.clone(),
            opts.loop_vars.contains(name),
            Span::default(),
        )?;
    }
    let (body, ty) = compile_body(form, text, &mut cx, opts.expected.as_ref())?;
    Ok(Compiled {
        body,
        scope: opts.scope.clone(),
        slot_of: (0..opts.scope.len()).collect(),
        slots: cx.next_slot,
        ty,
        sites: cx.sites.iter().cloned().collect(),
        system: system.to_owned(),
    })
}

fn compile_body(
    form: Form,
    text: &str,
    cx: &mut Checker<'_>,
    expected: Option<&Ty>,
) -> Result<(Body, Ty), Rejection> {
    let mut parser = Parser::new(text)?;
    match form {
        Form::Expr => {
            let mut node = parser.program()?;
            let ty = cx.check(&mut node)?;
            let ty = Checker::no_rel(Checker::no_event(ty, node.span)?, node.span)?;
            match expected {
                Some(want) => {
                    let t = cx.unify(&ty, want, node.span)?;
                    cx.resolve(&mut node, t)?;
                }
                None => {
                    cx.finish(&mut node)?;
                }
            }
            let ty = node.ty.clone();
            Ok((Body::Expr(node), ty))
        }
        Form::Candidates => {
            let mut node = parser.candidates()?;
            let ty = cx.check(&mut node)?;
            let person = cx.env.kind("person");
            if person.is_none_or(|p| ty != Ty::Rel(p)) {
                return Err(Rejection::new(
                    ErrorCode::Type,
                    node.span,
                    "candidates must yield persons".into(),
                ));
            }
            Ok((Body::Candidates(node), ty))
        }
        Form::Effects => {
            let mut items = parser.effects()?;
            for item in &mut items {
                cx.check_effect(item)?;
            }
            Ok((Body::Effects(items), Ty::Bool))
        }
    }
}

impl Compiled {
    /// The program's result type (a key type for `order`; `Bool` stands in for effects).
    #[must_use]
    pub fn ty(&self) -> &Ty {
        &self.ty
    }

    /// The bound variables, in the order an evaluation takes their values.
    #[must_use]
    pub fn scope(&self) -> &[(String, Ty)] {
        &self.scope
    }

    /// The program's static draw sites `(purpose/entity/index)`, for the loader's cross-program
    /// lint over the programs that share a step (§4.6).
    #[must_use]
    pub fn draw_sites(&self) -> &[String] {
        &self.sites
    }

    /// The declaring system.
    #[must_use]
    pub fn system(&self) -> &str {
        &self.system
    }

    /// Every action argument of an effects program that is a constant enum literal, as
    /// `(command, parameter, value)`: what a Registry lint needs to refuse a constant the owner
    /// would reject at its queue (Q13: `death_cause.none` sent to `ScheduleDeath`).
    #[must_use]
    pub fn constant_enum_args(&self) -> Vec<(CommandId, String, Value)> {
        let Body::Effects(items) = &self.body else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in items {
            let Some(command) = item.action.command else {
                continue;
            };
            for (name, node) in &item.action.args {
                if let ast::Kind::Dotted {
                    res: ast::Dotted::Enum(v),
                    ..
                } = &node.kind
                {
                    out.push((command, name.clone(), v.clone()));
                }
            }
        }
        out
    }

    fn frame(&self, bindings: &[Value]) -> Result<Vec<Option<Value>>, EvalError> {
        if bindings.len() != self.scope.len() {
            return Err(EvalError::Internal(format!(
                "{} bindings for a scope of {}",
                bindings.len(),
                self.scope.len()
            )));
        }
        let mut vars: Vec<Option<Value>> = vec![None; self.slots.max(bindings.len())];
        for (&slot, value) in self.slot_of.iter().zip(bindings) {
            if let Some(v) = vars.get_mut(slot) {
                *v = Some(value.clone());
            }
        }
        Ok(vars)
    }

    /// One evaluation of one program for one binding, with a fresh draw ledger (a law's or a vow
    /// predicate's scope, N6). Diagnostics accumulate into `diags`.
    ///
    /// # Errors
    /// The run-time invariants, or a host that lacks a parameter or a draw.
    pub fn eval(
        &self,
        env: &Env,
        host: &dyn Host,
        bindings: &[Value],
        diags: &mut Diagnostics,
        opts: EvalOptions,
    ) -> Result<Output, EvalError> {
        let mut ledger = DrawLedger::new();
        self.eval_in(env, host, bindings, diags, &mut ledger, opts)
    }

    /// An evaluation that draws under a caller's ledger ([`VerbStep`]; the T-05 per-step mutant).
    ///
    /// # Errors
    /// As [`Compiled::eval`].
    pub fn eval_in(
        &self,
        env: &Env,
        host: &dyn Host,
        bindings: &[Value],
        diags: &mut Diagnostics,
        ledger: &mut DrawLedger,
        opts: EvalOptions,
    ) -> Result<Output, EvalError> {
        let vars = self.frame(bindings)?;
        let mut ev = Ev {
            env,
            host,
            system: &self.system,
            vars,
            diags,
            ledger,
            opts,
        };
        match &self.body {
            Body::Expr(node) => ev.eval(node).map(Output::Value),
            Body::Candidates(node) => ev.relation(node).map(|(_, ids)| Output::Relation(ids)),
            Body::Effects(items) => ev.effects(items).map(Output::Effects),
        }
    }
}

/// The verb evaluations of one system in one step (r4-2): verb programs are never re-evaluated,
/// so one draw ledger spans them all, and two events whose draws share a key are the invariant
/// `E_RNG_REUSE`. The host owns one per step.
#[derive(Clone, Debug, Default)]
pub struct VerbStep {
    ledger: DrawLedger,
    /// The step's diagnostics.
    pub diags: Diagnostics,
}

impl VerbStep {
    /// A new step.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates a verb's effects for one event of the step.
    ///
    /// # Errors
    /// As [`Compiled::eval`].
    pub fn eval(
        &mut self,
        program: &Compiled,
        env: &Env,
        host: &dyn Host,
        bindings: &[Value],
        opts: EvalOptions,
    ) -> Result<Vec<Action>, EvalError> {
        match program.eval_in(env, host, bindings, &mut self.diags, &mut self.ledger, opts)? {
            Output::Effects(actions) => Ok(actions),
            _ => Err(EvalError::Internal(
                "a verb program is an effects program".into(),
            )),
        }
    }
}

/// Renders actions in §6's queue form, as the golden file writes them:
/// `effects:[Name(person=<id>, args=[<i64>, …]); …]`, with `event` for the verb's own event.
#[must_use]
pub fn render_actions(env: &Env, actions: &[Action]) -> String {
    let items: Vec<String> = actions
        .iter()
        .map(|a| {
            let name = env.command(a.command).map_or("?", |c| c.name.as_str());
            let args: Vec<String> = a
                .args
                .iter()
                .map(|arg| match arg {
                    Arg::Event(EventArg::Own) => "event".to_owned(),
                    other => other.encode().unwrap_or(0).to_string(),
                })
                .collect();
            format!("{name}(person={}, args=[{}])", a.person, args.join(", "))
        })
        .collect();
    format!("effects:[{}]", items.join("; "))
}
