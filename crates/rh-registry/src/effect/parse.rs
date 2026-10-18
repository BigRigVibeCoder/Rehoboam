//! The recursive-descent parser of §2's EBNF, with §2's disambiguation rules.
//!
//! The grammar's expression nesting (`expr` inside parentheses, arguments, branches, aggregates)
//! is bounded at [`EXPRESSION_DEPTH_MAX`] (`E_BOUND`, §4.2). Two further bounds keep every walk of
//! the tree finite on a small stack, whatever the input: a chain of unary `not` and `-` is bounded
//! at [`UNARY_CHAIN_MAX`], and a node's height at [`TREE_HEIGHT_MAX`] (a left-associative chain of
//! 10⁵ `+` builds a tree that deep). Both are `E_BOUND`; the reference has neither and exceeds
//! Python's recursion limit instead (a crash), so no program the reference evaluates is affected
//! below roughly 900 levels.
//!
//! REF: `specs/registry/effect_language_v0.md` §2, §4.2, §10.

use super::ast::{Action, AggOp, BinOp, Effect, ForEach, Kind, Node, Relation};
use super::lex::{TokKind, Token, lex};
use super::{ErrorCode, Rejection, Span};

/// §4.2: expression nesting above 64 is `E_BOUND`.
pub const EXPRESSION_DEPTH_MAX: usize = 64;
/// The longest chain of unary `not` or `-` (a stack bound; see the module docs).
pub const UNARY_CHAIN_MAX: usize = 128;
/// The tallest syntax tree (a stack bound; see the module docs).
pub const TREE_HEIGHT_MAX: u32 = 192;

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
    unary: usize,
}

type PResult<T> = Result<T, Rejection>;

impl Parser {
    pub(crate) fn new(text: &str) -> PResult<Self> {
        Ok(Self {
            tokens: lex(text)?,
            pos: 0,
            depth: 0,
            unary: 0,
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

    fn node(kind: Kind, span: Span) -> PResult<Node> {
        let node = Node::new(kind, span);
        if node.height > TREE_HEIGHT_MAX {
            return Err(Rejection::new(
                ErrorCode::Bound,
                span,
                format!("syntax tree taller than {TREE_HEIGHT_MAX}"),
            ));
        }
        Ok(node)
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
        let mut node = self.and_expr()?;
        while self.accept("or") {
            let b = self.and_expr()?;
            let span = Self::join(node.span, b.span);
            node = Self::node(
                Kind::Bin {
                    op: BinOp::Or,
                    a: Box::new(node),
                    b: Box::new(b),
                },
                span,
            )?;
        }
        self.depth -= 1;
        Ok(node)
    }

    fn and_expr(&mut self) -> PResult<Node> {
        let mut node = self.not_expr()?;
        while self.accept("and") {
            let b = self.not_expr()?;
            let span = Self::join(node.span, b.span);
            node = Self::node(
                Kind::Bin {
                    op: BinOp::And,
                    a: Box::new(node),
                    b: Box::new(b),
                },
                span,
            )?;
        }
        Ok(node)
    }

    fn enter_unary(&mut self) -> PResult<()> {
        self.unary += 1;
        if self.unary > UNARY_CHAIN_MAX {
            let span = self.peek(0).span;
            return Err(Rejection::new(
                ErrorCode::Bound,
                span,
                format!("unary chain longer than {UNARY_CHAIN_MAX}"),
            ));
        }
        Ok(())
    }

    fn not_expr(&mut self) -> PResult<Node> {
        let start = self.peek(0).span;
        if self.accept("not") {
            self.enter_unary()?;
            let e = self.not_expr()?;
            self.unary -= 1;
            let span = Self::join(start, e.span);
            return Self::node(Kind::Not(Box::new(e)), span);
        }
        self.cmp_expr()
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
        let mut node = self.mul_expr()?;
        while self.peek(0).kind == TokKind::Op
            && (self.peek(0).text == "+" || self.peek(0).text == "-")
        {
            let op = if self.take().text == "+" {
                BinOp::Add
            } else {
                BinOp::Sub
            };
            let b = self.mul_expr()?;
            let span = Self::join(node.span, b.span);
            node = Self::node(
                Kind::Bin {
                    op,
                    a: Box::new(node),
                    b: Box::new(b),
                },
                span,
            )?;
        }
        Ok(node)
    }

    fn mul_expr(&mut self) -> PResult<Node> {
        let mut node = self.unary()?;
        while self.at("*") || self.at("/") || self.at("div") || self.at("mod") {
            let text = self.take().text;
            let op = BinOp::from_text(&text).unwrap_or(BinOp::Mul);
            let b = self.unary()?;
            let span = Self::join(node.span, b.span);
            node = Self::node(
                Kind::Bin {
                    op,
                    a: Box::new(node),
                    b: Box::new(b),
                },
                span,
            )?;
        }
        Ok(node)
    }

    fn unary(&mut self) -> PResult<Node> {
        let start = self.peek(0).span;
        if self.accept("-") {
            self.enter_unary()?;
            let e = self.unary()?;
            self.unary -= 1;
            let span = Self::join(start, e.span);
            return Self::node(Kind::Neg(Box::new(e)), span);
        }
        self.primary()
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
