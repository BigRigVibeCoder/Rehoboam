//! The recursive-descent parser of §2's EBNF, with §2's disambiguation rules.
//!
//! The grammar's expression nesting (the `expr` production: parentheses, arguments, bodies,
//! conditions, branches) is bounded at [`EXPRESSION_DEPTH_MAX`] (`E_BOUND`, §4.2, §4.8). Operator
//! chains and prefix operators do not nest (GAP-A): a left-associative chain of one precedence
//! level is parsed by a loop into one [`Kind::Chain`] node, and a run of prefix `not` or `-` into
//! one counted node, so a chain of any length costs no recursion in the parser, the checker or
//! the evaluator. A chain whose first operand is itself a parenthesised chain of the same level
//! extends it, as the reference's left-deep tree does (`(a + b) + c` is `a + b + c`).
//!
//! REF: `specs/registry/effect_language_v0.md` §2, §4.2, §4.8, §10.

use super::ast::{Action, AggOp, BinOp, Effect, ForEach, Kind, Node, Relation};
use super::lex::{TokKind, Token, lex};
use super::{ErrorCode, Rejection, Span};

/// §4.2: expression nesting above 64 is `E_BOUND`.
pub const EXPRESSION_DEPTH_MAX: usize = 64;

/// Names that parse as calls rather than relations (the reference's `KNOWN_CALLS`).
const KNOWN_CALLS: [&str; 18] = [
    "widen",
    "narrow",
    "q8",
    "q16",
    "abs",
    "min",
    "max",
    "clamp",
    "safe_div",
    "up",
    "common",
    "up_agnatic",
    "common_agnatic",
    "descends",
    "ordinal",
    "exists",
    "floor",
    "round",
];
const MORE_CALLS: [&str; 3] = ["draw", "chance", "related"];

pub(crate) fn relation_named(name: &str) -> Option<Relation> {
    Some(match name {
        "children" => Relation::Children,
        "kin" => Relation::Kin,
        "ancestors" => Relation::Ancestors,
        "descendants" => Relation::Descendants,
        "lineage" => Relation::Lineage,
        "line" => Relation::Line,
        "line_agnatic" => Relation::LineAgnatic,
        "titles" => Relation::Titles,
        "vassals" => Relation::Vassals,
        "offices" => Relation::Offices,
        "court" => Relation::Court,
        "claims_on" => Relation::ClaimsOn,
        "claims_of" => Relation::ClaimsOf,
        "vows_of" => Relation::VowsOf,
        "holders_of" => Relation::HoldersOf,
        _ => return None,
    })
}

fn is_known_call(name: &str) -> bool {
    KNOWN_CALLS.contains(&name) || MORE_CALLS.contains(&name)
}

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

type PResult<T> = Result<T, Rejection>;

impl Parser {
    pub(crate) fn new(text: &str) -> PResult<Self> {
        Ok(Self {
            tokens: lex(text)?,
            pos: 0,
            depth: 0,
        })
    }

    fn peek(&self, offset: usize) -> &Token {
        let i = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[i]
    }

    fn take(&mut self) -> Token {
        let token = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if token.kind != TokKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn at(&self, text: &str) -> bool {
        let t = self.peek(0);
        matches!(t.kind, TokKind::Op | TokKind::Kw) && t.text == text
    }

    fn accept(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.take();
            true
        } else {
            false
        }
    }

    fn syntax<T>(&self, message: String) -> PResult<T> {
        Err(Rejection::new(
            ErrorCode::Syntax,
            self.peek(0).span,
            message,
        ))
    }

    fn expect(&mut self, text: &str) -> PResult<()> {
        if self.accept(text) {
            Ok(())
        } else {
            let found = self.peek(0).text.clone();
            self.syntax(format!("expected {text:?}, found {found:?}"))
        }
    }

    fn ident(&mut self) -> PResult<(String, Span)> {
        let t = self.take();
        if t.kind == TokKind::Ident {
            Ok((t.text, t.span))
        } else {
            Err(Rejection::new(
                ErrorCode::Syntax,
                t.span,
                format!("expected a name, found {:?}", t.text),
            ))
        }
    }

    fn end(&self) -> PResult<()> {
        if self.peek(0).kind == TokKind::Eof {
            Ok(())
        } else {
            let found = self.peek(0).text.clone();
            self.syntax(format!("unexpected {found:?}"))
        }
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "INVARIANT: every constructor of the parser returns a PResult"
    )]
    fn node(kind: Kind, span: Span) -> PResult<Node> {
        Ok(Node::new(kind, span))
    }

    fn join(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    // ------------------------------------------------------------------ programs
    pub(crate) fn program(&mut self) -> PResult<Node> {
        let node = self.expr()?;
        self.end()?;
        Ok(node)
    }

    pub(crate) fn candidates(&mut self) -> PResult<Node> {
        let node = self.primary()?;
        self.end()?;
        if !matches!(node.kind, Kind::Rel { .. }) {
            return Err(Rejection::new(
                ErrorCode::Type,
                node.span,
                "candidates must be a relation".into(),
            ));
        }
        Ok(node)
    }

    pub(crate) fn effects(&mut self) -> PResult<Vec<Effect>> {
        let mut items = Vec::new();
        if self.peek(0).kind != TokKind::Eof {
            items.push(self.effect()?);
            while self.accept(";") {
                items.push(self.effect()?);
            }
        }
        self.end()?;
        Ok(items)
    }

    fn effect(&mut self) -> PResult<Effect> {
        if self.accept("when") {
            let cond = self.expr()?;
            self.expect(":")?;
            return Ok(Effect {
                action: self.action()?,
                when: Some(cond),
                each: None,
            });
        }
        if self.accept("for") {
            let (var, _) = self.ident()?;
            self.expect("in")?;
            let rel = self.relation()?;
            let cond = if self.accept("where") {
                Some(self.expr()?)
            } else {
                None
            };
            self.expect(":")?;
            let action = self.action()?;
            return Ok(Effect {
                action,
                when: None,
                each: Some(ForEach {
                    var,
                    slot: 0,
                    rel,
                    cond,
                }),
            });
        }
        Ok(Effect {
            action: self.action()?,
            when: None,
            each: None,
        })
    }

    fn action(&mut self) -> PResult<Action> {
        let (name, span) = self.ident()?;
        self.expect("(")?;
        let mut args = Vec::new();
        if !self.at(")") {
            args.push(self.named_arg()?);
            while self.accept(",") {
                args.push(self.named_arg()?);
            }
        }
        self.expect(")")?;
        Ok(Action {
            name,
            span,
            args,
            command: None,
            person_index: 0,
            order: Vec::new(),
            dropped: false,
        })
    }

    fn named_arg(&mut self) -> PResult<(String, Node)> {
        let (name, _) = self.ident()?;
        self.expect(":")?;
        Ok((name, self.expr()?))
    }

    fn relation(&mut self) -> PResult<Node> {
        let node = self.primary()?;
        if let Kind::Call { name, .. } = &node.kind
            && !is_known_call(name)
        {
            return Err(Rejection::new(
                ErrorCode::UnknownName,
                node.span,
                format!("relation {name}"),
            ));
        }
        if !matches!(node.kind, Kind::Rel { .. }) {
            return Err(Rejection::new(
                ErrorCode::Syntax,
                node.span,
                "expected a relation".into(),
            ));
        }
        Ok(node)
    }

    // ------------------------------------------------------------------ expressions
    fn expr(&mut self) -> PResult<Node> {
        self.depth += 1;
        if self.depth > EXPRESSION_DEPTH_MAX {
            let span = self.peek(0).span;
            return Err(Rejection::new(
                ErrorCode::Bound,
                span,
                format!("expression nesting above {EXPRESSION_DEPTH_MAX}"),
            ));
        }
        let first = self.and_expr()?;
        let node = self.chain(first, 0, Self::and_expr)?;
        self.depth -= 1;
        Ok(node)
    }

    /// The operator at the next token, if it is one of precedence `level`.
    fn chain_op(&self, level: u8) -> Option<BinOp> {
        let t = self.peek(0);
        if !matches!(t.kind, TokKind::Op | TokKind::Kw) {
            return None;
        }
        BinOp::from_text(&t.text).filter(|op| op.level() == Some(level))
    }

    /// A left-associative chain of `level` after its first operand, as one node.
    fn chain(
        &mut self,
        first: Node,
        level: u8,
        operand: fn(&mut Self) -> PResult<Node>,
    ) -> PResult<Node> {
        if self.chain_op(level).is_none() {
            return Ok(first);
        }
        let start = first.span;
        let (mut ops, mut items) = match first.kind {
            Kind::Chain { ops, items } if ops.first().and_then(|o| o.level()) == Some(level) => {
                (ops, items)
            }
            kind => (
                Vec::new(),
                vec![Node {
                    kind,
                    span: first.span,
                    ty: first.ty,
                }],
            ),
        };
        while let Some(op) = self.chain_op(level) {
            self.take();
            ops.push(op);
            items.push(operand(self)?);
        }
        let end = items.last().map_or(start, |n| n.span);
        Self::node(Kind::Chain { ops, items }, Self::join(start, end))
    }

    fn and_expr(&mut self) -> PResult<Node> {
        let first = self.not_expr()?;
        self.chain(first, 1, Self::not_expr)
    }

    fn not_expr(&mut self) -> PResult<Node> {
        let start = self.peek(0).span;
        let mut count: u32 = 0;
        while self.accept("not") {
            count = count.saturating_add(1);
        }
        let e = self.cmp_expr()?;
        if count == 0 {
            return Ok(e);
        }
        let span = Self::join(start, e.span);
        match e.kind {
            Kind::Not(n, inner) => Self::node(Kind::Not(count.saturating_add(n), inner), span),
            kind => Self::node(
                Kind::Not(
                    count,
                    Box::new(Node {
                        kind,
                        span: e.span,
                        ty: e.ty,
                    }),
                ),
                span,
            ),
        }
    }

    fn cmp_expr(&mut self) -> PResult<Node> {
        let node = self.add_expr()?;
        let t = self.peek(0);
        if t.kind == TokKind::Op
            && let Some(op) = BinOp::from_text(&t.text).filter(|op| op.is_cmp())
        {
            self.take();
            let b = self.add_expr()?;
            let span = Self::join(node.span, b.span);
            let node = Self::node(
                Kind::Bin {
                    op,
                    a: Box::new(node),
                    b: Box::new(b),
                },
                span,
            )?;
            let after = self.peek(0);
            if after.kind == TokKind::Op && BinOp::from_text(&after.text).is_some_and(BinOp::is_cmp)
            {
                return self.syntax("chained comparison".into());
            }
            return Ok(node);
        }
        Ok(node)
    }

    fn add_expr(&mut self) -> PResult<Node> {
        let first = self.mul_expr()?;
        self.chain(first, 2, Self::mul_expr)
    }

    fn mul_expr(&mut self) -> PResult<Node> {
        let first = self.unary()?;
        self.chain(first, 3, Self::unary)
    }

    fn unary(&mut self) -> PResult<Node> {
        let start = self.peek(0).span;
        let mut count: u32 = 0;
        while self.accept("-") {
            count = count.saturating_add(1);
        }
        let e = self.primary()?;
        if count == 0 {
            return Ok(e);
        }
        let span = Self::join(start, e.span);
        match e.kind {
            Kind::Neg(n, inner) => Self::node(Kind::Neg(count.saturating_add(n), inner), span),
            kind => Self::node(
                Kind::Neg(
                    count,
                    Box::new(Node {
                        kind,
                        span: e.span,
                        ty: e.ty,
                    }),
                ),
                span,
            ),
        }
    }

    fn primary(&mut self) -> PResult<Node> {
        let t = self.peek(0).clone();
        match t.kind {
            TokKind::Num => {
                self.take();
                Self::node(
                    Kind::Num {
                        text: t.text,
                        bits: 0,
                    },
                    t.span,
                )
            }
            TokKind::Kw => self.keyword_primary(),
            TokKind::Ident => self.name_primary(),
            _ if self.accept("(") => self.paren(t.span),
            _ if self.accept("[") => self.list_key(t.span),
            _ => self.syntax(format!("unexpected {:?}", t.text)),
        }
    }

    fn keyword_primary(&mut self) -> PResult<Node> {
        let t = self.take();
        match t.text.as_str() {
            "true" | "false" => Self::node(Kind::Bool(t.text == "true"), t.span),
            "none" => Self::node(Kind::None, t.span),
            "if" => {
                let cond = self.expr()?;
                self.expect("then")?;
                let a = self.expr()?;
                self.expect("else")?;
                let b = self.expr()?;
                let span = Self::join(t.span, b.span);
                Self::node(
                    Kind::If {
                        cond: Box::new(cond),
                        a: Box::new(a),
                        b: Box::new(b),
                    },
                    span,
                )
            }
            _ => Err(Rejection::new(
                ErrorCode::Syntax,
                t.span,
                format!("unexpected {:?}", t.text),
            )),
        }
    }

    fn name_primary(&mut self) -> PResult<Node> {
        let (name, span) = self.ident()?;
        if self.accept(".") {
            let t = self.take();
            if !matches!(t.kind, TokKind::Ident | TokKind::Kw) {
                return Err(Rejection::new(
                    ErrorCode::Syntax,
                    t.span,
                    format!("expected a name after '.', found {:?}", t.text),
                ));
            }
            if self.accept("(") {
                let arg = self.expr()?;
                let close = self.peek(0).span;
                self.expect(")")?;
                let kind = Kind::Read {
                    entity: name,
                    column: t.text,
                    arg: Box::new(arg),
                    col: None,
                    neutral: None,
                };
                return Self::node(kind, Self::join(span, close));
            }
            return Self::node(
                Kind::Dotted {
                    head: name,
                    tail: t.text,
                    res: super::ast::Dotted::Unresolved,
                },
                Self::join(span, t.span),
            );
        }
        if !self.at("(") {
            return Self::node(Kind::Var { name, slot: 0 }, span);
        }
        if let Some(op) = AggOp::from_text(&name)
            && self.peek(1).kind == TokKind::Ident
            && self.peek(2).text == "in"
        {
            return self.aggregate(op, span);
        }
        self.call(name, span)
    }

    fn call(&mut self, name: String, span: Span) -> PResult<Node> {
        self.expect("(")?;
        let mut args = Vec::new();
        if !self.at(")") {
            args.push(self.expr()?);
            while self.accept(",") {
                args.push(self.expr()?);
            }
        }
        let close = self.peek(0).span;
        self.expect(")")?;
        let span = Self::join(span, close);
        match relation_named(&name) {
            Some(rel) => Self::node(Kind::Rel { rel, name, args }, span),
            None => Self::node(Kind::Call { name, args }, span),
        }
    }

    fn aggregate(&mut self, op: AggOp, span: Span) -> PResult<Node> {
        self.expect("(")?;
        let (var, _) = self.ident()?;
        self.expect("in")?;
        let rel = self.relation()?;
        let cond = if self.accept("where") {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        let body = if self.accept(":") {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        let orelse = if self.accept("else") {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        let close = self.peek(0).span;
        self.expect(")")?;
        let kind = Kind::Agg {
            op,
            var,
            slot: 0,
            rel: Box::new(rel),
            cond,
            body,
            orelse,
        };
        Self::node(kind, Self::join(span, close))
    }

    fn paren(&mut self, open: Span) -> PResult<Node> {
        let first = self.expr()?;
        if !self.accept(",") {
            self.expect(")")?;
            return Ok(first);
        }
        let mut items = vec![first, self.expr()?];
        while self.accept(",") {
            items.push(self.expr()?);
        }
        let close = self.peek(0).span;
        self.expect(")")?;
        Self::node(Kind::Tuple(items), Self::join(open, close))
    }

    fn list_key(&mut self, open: Span) -> PResult<Node> {
        let body = self.expr()?;
        self.expect("for")?;
        let (var, _) = self.ident()?;
        self.expect("in")?;
        let rel = self.relation()?;
        let close = self.peek(0).span;
        self.expect("]")?;
        let kind = Kind::ListKey {
            body: Box::new(body),
            var,
            slot: 0,
            rel: Box::new(rel),
        };
        Self::node(kind, Self::join(open, close))
    }
}
