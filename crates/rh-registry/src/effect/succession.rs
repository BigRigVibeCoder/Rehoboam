//! The succession protocol (§7.2): a law's `candidates`, `eligible` and `order`, compiled together
//! and evaluated into the title's line.
//!
//! 1. `C` = the `candidates` relation for the holder and the title;
//! 2. `E` = the candidates for which `eligible(x)` holds, in canonical order;
//! 3. the line is `E` sorted by `(order(x), person_id)` ascending: the person id is the final
//!    tie-break, which orders twins (§3.4).
//!
//! The three programs share one checker, so their draw sites are unique across them (§4.6); each
//! evaluation of `eligible` or `order` for one candidate has its own draw ledger (N6), which makes
//! a repeated evaluation of the law in one step idempotent.
//!
//! REF: `specs/registry/effect_language_v0.md` §3.4, §4.6, §7.1, §7.2, §8.

use super::check::Checker;
use super::env::{Env, Ty};
use super::host::Host;
use super::value::{KeyAtom, Value, key_form};
use super::{
    Body, Compiled, Diagnostics, DrawLedger, ErrorCode, EvalError, EvalOptions, Form, Mutant,
    Output, Rejection, Span,
};

/// A law's three programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LawText<'a> {
    /// `candidates`.
    pub candidates: &'a str,
    /// `eligible`.
    pub eligible: &'a str,
    /// `order`.
    pub order: &'a str,
}

/// Which of a law's programs a rejection is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LawProgram {
    /// `candidates`.
    Candidates,
    /// `eligible`.
    Eligible,
    /// `order`.
    Order,
}

impl LawProgram {
    /// The program's Registry field name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidates => "candidates",
            Self::Eligible => "eligible",
            Self::Order => "order",
        }
    }
}

/// A compiled succession law.
#[derive(Clone, Debug)]
pub struct CompiledLaw {
    candidates: Compiled,
    eligible: Compiled,
    order: Compiled,
}

/// The scope §7.1 gives a law: `holder`, `title` and `now`.
///
/// # Errors
/// An environment without the `person` or `title` kind.
pub fn law_scope(env: &Env) -> Result<Vec<(String, Ty)>, Rejection> {
    let k = |name: &str| {
        env.kind(name).ok_or_else(|| {
            Rejection::new(
                ErrorCode::UnknownName,
                Span::default(),
                format!("entity kind {name}"),
            )
        })
    };
    Ok(vec![
        ("holder".into(), Ty::Ent(k("person")?)),
        ("title".into(), Ty::Ent(k("title")?)),
        ("now".into(), Ty::Int),
    ])
}

impl CompiledLaw {
    /// Compiles a law for `system` (M7's, for the Registry's laws) with §7.1's scope.
    ///
    /// # Errors
    /// The first fault, with the program it is in.
    pub fn compile(
        env: &Env,
        system: &str,
        law: LawText<'_>,
    ) -> Result<Self, (LawProgram, Rejection)> {
        let scope = law_scope(env).map_err(|r| (LawProgram::Candidates, r))?;
        Self::compile_in(env, system, law, &scope)
    }

    /// Compiles a law with an explicit base scope (`x` is added for `eligible` and `order`).
    ///
    /// # Errors
    /// The first fault, with the program it is in.
    pub fn compile_in(
        env: &Env,
        system: &str,
        law: LawText<'_>,
        scope: &[(String, Ty)],
    ) -> Result<Self, (LawProgram, Rejection)> {
        let mut cx = Checker::new(env, system, super::CheckOptions::default());
        let c = |p: LawProgram| move |r: Rejection| (p, r);
        for (name, ty) in scope {
            cx.bind(name, ty.clone(), false, Span::default())
                .map_err(c(LawProgram::Candidates))?;
        }
        let (cand, cand_ty) = super::compile_body(Form::Candidates, law.candidates, &mut cx, None)
            .map_err(c(LawProgram::Candidates))?;
        let sites_c: Vec<String> = cx.sites.iter().cloned().collect();
        let person = env.kind("person").map_or(Ty::Int, Ty::Ent);
        let x_slot = cx
            .bind("x", person.clone(), true, Span::default())
            .map_err(c(LawProgram::Eligible))?;
        let (elig, elig_ty) =
            super::compile_body(Form::Expr, law.eligible, &mut cx, Some(&Ty::Bool))
                .map_err(c(LawProgram::Eligible))?;
        cx.opts.allow_key = true;
        let (order, order_ty) = super::compile_body(Form::Expr, law.order, &mut cx, None)
            .map_err(c(LawProgram::Order))?;
        cx.unbind("x");
        let slots = cx.next_slot;
        let sites: Vec<String> = cx.sites.iter().cloned().collect();
        let mut with_x = scope.to_vec();
        with_x.push(("x".into(), person));
        let mut slot_of: Vec<usize> = (0..scope.len()).collect();
        let make = |body: Body,
                    ty: Ty,
                    scope: Vec<(String, Ty)>,
                    slot_of: Vec<usize>,
                    sites: Vec<String>| Compiled {
            body,
            scope,
            slot_of,
            slots,
            ty,
            sites,
            system: system.to_owned(),
        };
        let candidates = make(cand, cand_ty, scope.to_vec(), slot_of.clone(), sites_c);
        slot_of.push(x_slot);
        Ok(Self {
            candidates,
            eligible: make(
                elig,
                elig_ty,
                with_x.clone(),
                slot_of.clone(),
                sites.clone(),
            ),
            order: make(order, order_ty, with_x, slot_of, sites),
        })
    }

    /// Every static draw site of the law's programs.
    #[must_use]
    pub fn draw_sites(&self) -> &[String] {
        self.order.draw_sites()
    }

    /// The law's line for the bound scope (`holder`, `title`, `now`, in [`law_scope`]'s order or
    /// the scope given at compilation): §7.2's three steps.
    ///
    /// # Errors
    /// `E_TRUNCATED` from `lineage`, `E_RNG_REUSE` within one evaluation, or a host fault.
    pub fn line(
        &self,
        env: &Env,
        host: &dyn Host,
        bindings: &[Value],
        diags: &mut Diagnostics,
        opts: EvalOptions,
    ) -> Result<Vec<u64>, EvalError> {
        self.line_in(env, host, bindings, diags, opts, None)
    }

    /// As [`CompiledLaw::line`], drawing under a caller's ledger when one is given: only the T-05
    /// mutant "the ledger kept per step" passes one; the language gives each evaluation its own.
    ///
    /// # Errors
    /// As [`CompiledLaw::line`].
    pub fn line_in(
        &self,
        env: &Env,
        host: &dyn Host,
        bindings: &[Value],
        diags: &mut Diagnostics,
        opts: EvalOptions,
        mut step: Option<&mut DrawLedger>,
    ) -> Result<Vec<u64>, EvalError> {
        let Output::Relation(candidates) =
            self.candidates.eval(env, host, bindings, diags, opts)?
        else {
            return Err(EvalError::Internal("candidates is a relation".into()));
        };
        let person = env
            .kind("person")
            .ok_or_else(|| EvalError::Internal("no person kind".into()))?;
        let mut with_x = bindings.to_vec();
        with_x.push(Value::Ent(person, 0));
        let x_at = with_x.len() - 1;
        let mut keyed: Vec<(Vec<KeyAtom>, u64)> = Vec::new();
        let prefix_last = cfg!(feature = "mutant-effect") && opts.mutant == Mutant::PrefixLast;
        for x in candidates {
            with_x[x_at] = Value::Ent(person, x);
            let mut fresh = DrawLedger::new();
            let ledger = match step.as_deref_mut() {
                Some(l) => l,
                None => &mut fresh,
            };
            let eligible = self
                .eligible
                .eval_in(env, host, &with_x, diags, ledger, opts)?;
            if eligible != Output::Value(Value::Bool(true)) {
                continue;
            }
            let mut fresh = DrawLedger::new();
            let ledger = match step.as_deref_mut() {
                Some(l) => l,
                None => &mut fresh,
            };
            let Output::Value(key) = self
                .order
                .eval_in(env, host, &with_x, diags, ledger, opts)?
            else {
                return Err(EvalError::Internal("order is an expression".into()));
            };
            let mut form = Vec::new();
            key_form(&key, prefix_last, &mut form);
            keyed.push((form, x));
        }
        if cfg!(feature = "mutant-effect") && opts.mutant == Mutant::TieBreakDescending {
            keyed.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        } else {
            keyed.sort();
        }
        Ok(keyed.into_iter().map(|(_, x)| x).collect())
    }
}
