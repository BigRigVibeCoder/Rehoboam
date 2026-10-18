//! The syntax tree of a program, with the types and resolutions the checker fills in.
//!
//! REF: `specs/registry/effect_language_v0.md` §2, §5.3, §6.

use super::Span;
use super::env::{ColumnId, CommandId, Ty};
use super::value::Value;

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
}

impl BinOp {
    pub(crate) fn from_text(text: &str) -> Option<Self> {
        Some(match text {
            "or" => Self::Or,
            "and" => Self::And,
            "==" => Self::Eq,
            "!=" => Self::Ne,
            "<" => Self::Lt,
            "<=" => Self::Le,
            ">" => Self::Gt,
            ">=" => Self::Ge,
            "+" => Self::Add,
            "-" => Self::Sub,
            "*" => Self::Mul,
            "/" => Self::Div,
            "div" => Self::IntDiv,
            "mod" => Self::Mod,
            _ => return None,
        })
    }

    pub(crate) fn is_cmp(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge
        )
    }

    pub(crate) fn is_order(self) -> bool {
        matches!(self, Self::Lt | Self::Le | Self::Gt | Self::Ge)
    }

    pub(crate) fn is_arith(self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::IntDiv | Self::Mod
        )
    }

    /// The precedence level of a chainable operator (0 `or`, 1 `and`, 2 `+ -`, 3 `* / div mod`);
    /// `None` for a comparison.
    pub(crate) fn level(self) -> Option<u8> {
        match self {
            Self::Or => Some(0),
            Self::And => Some(1),
            Self::Add | Self::Sub => Some(2),
            Self::Mul | Self::Div | Self::IntDiv | Self::Mod => Some(3),
            _ => None,
        }
    }

    pub(crate) fn is_division(self) -> bool {
        matches!(self, Self::Div | Self::IntDiv | Self::Mod)
    }
}

/// An aggregate (§5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum AggOp {
    Count,
    Any,
    All,
    Sum,
    Min,
    Max,
}

impl AggOp {
    pub(crate) fn from_text(text: &str) -> Option<Self> {
        Some(match text {
            "count" => Self::Count,
            "any" => Self::Any,
            "all" => Self::All,
            "sum" => Self::Sum,
            "min" => Self::Min,
            "max" => Self::Max,
            _ => return None,
        })
    }
}

/// A prelude relation (§5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Relation {
    Children,
    Kin,
    Ancestors,
    Descendants,
    Lineage,
    Line,
    LineAgnatic,
    Titles,
    Vassals,
    Offices,
    Court,
    ClaimsOn,
    ClaimsOf,
    VowsOf,
    HoldersOf,
}

/// What a dotted name without a call resolved to (§4.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dotted {
    Unresolved,
    Param,
    DaysPerYear,
    MinutesPerDay,
    Enum(Value),
}

/// A node's form.
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Num {
        text: String,
        bits: i64,
    },
    Bool(bool),
    None,
    Var {
        name: String,
        slot: usize,
    },
    Dotted {
        head: String,
        tail: String,
        res: Dotted,
    },
    Read {
        entity: String,
        column: String,
        arg: Box<Node>,
        col: Option<ColumnId>,
        neutral: Option<Value>,
    },
    Rel {
        rel: Relation,
        name: String,
        args: Vec<Node>,
    },
    Call {
        name: String,
        args: Vec<Node>,
    },
    Agg {
        op: AggOp,
        var: String,
        slot: usize,
        rel: Box<Node>,
        cond: Option<Box<Node>>,
        body: Option<Box<Node>>,
        orelse: Option<Box<Node>>,
    },
    Tuple(Vec<Node>),
    ListKey {
        body: Box<Node>,
        var: String,
        slot: usize,
        rel: Box<Node>,
    },
    If {
        cond: Box<Node>,
        a: Box<Node>,
        b: Box<Node>,
    },
    /// A run of `count` prefix minuses (§4.8: prefix operators do not nest).
    Neg(u32, Box<Node>),
    /// A run of `count` prefix `not`s.
    Not(u32, Box<Node>),
    /// A left-associative chain of one precedence level's operators (`or`, `and`, `+`/`-`, or
    /// `*`/`/`/`div`/`mod`): `items[0] ops[0] items[1] ops[1] …`, folded left to right. One node,
    /// so a chain of any length costs no recursion (GAP-A).
    Chain {
        ops: Vec<BinOp>,
        items: Vec<Node>,
    },
    /// A comparison (non-associative).
    Bin {
        op: BinOp,
        a: Box<Node>,
        b: Box<Node>,
    },
}

/// An expression node.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub kind: Kind,
    pub span: Span,
    /// The node's type, filled in by the checker.
    pub ty: Ty,
}

impl Node {
    pub(crate) fn new(kind: Kind, span: Span) -> Self {
        Self {
            kind,
            span,
            ty: Ty::Int,
        }
    }
}

/// The direct children of a node, in syntax order.
pub(crate) fn children(kind: &Kind) -> Vec<&Node> {
    match kind {
        Kind::Num { .. } | Kind::Bool(_) | Kind::None | Kind::Var { .. } | Kind::Dotted { .. } => {
            Vec::new()
        }
        Kind::Read { arg, .. } => vec![arg],
        Kind::Rel { args, .. } | Kind::Call { args, .. } | Kind::Tuple(args) => {
            args.iter().collect()
        }
        Kind::Agg {
            rel,
            cond,
            body,
            orelse,
            ..
        } => {
            let mut out: Vec<&Node> = vec![rel];
            out.extend(cond.as_deref());
            out.extend(body.as_deref());
            out.extend(orelse.as_deref());
            out
        }
        Kind::ListKey { body, rel, .. } => vec![body, rel],
        Kind::If { cond, a, b } => vec![cond, a, b],
        Kind::Neg(_, e) | Kind::Not(_, e) => vec![e],
        Kind::Chain { items, .. } => items.iter().collect(),
        Kind::Bin { a, b, .. } => vec![a, b],
    }
}

/// Whether a variable occurs in a node (the loop-variable rule of the draw lint, §4.6).
pub(crate) fn mentions(node: &Node, name: &str) -> bool {
    if let Kind::Var { name: n, .. } = &node.kind {
        return n == name;
    }
    children(&node.kind).into_iter().any(|c| mentions(c, name))
}

/// A canonical text of a node's syntax, without its types: two draw sites are the same site when
/// their canonical texts are equal (§4.6).
pub(crate) fn canon(node: &Node, out: &mut String) {
    use std::fmt::Write as _;
    let _ = match &node.kind {
        Kind::Num { text, .. } => write!(out, "Num({text})"),
        Kind::Bool(b) => write!(out, "Bool({b})"),
        Kind::None => write!(out, "None"),
        Kind::Var { name, .. } => write!(out, "Var({name})"),
        Kind::Dotted { head, tail, .. } => write!(out, "Dotted({head}.{tail})"),
        Kind::Read { entity, column, .. } => write!(out, "Read({entity}.{column}"),
        Kind::Rel { name, .. } => write!(out, "Rel({name}"),
        Kind::Call { name, .. } => write!(out, "Call({name}"),
        Kind::Agg { op, var, .. } => write!(out, "Agg({op:?},{var}"),
        Kind::Tuple(_) => write!(out, "Tuple("),
        Kind::ListKey { var, .. } => write!(out, "ListKey({var}"),
        Kind::If { .. } => write!(out, "If("),
        Kind::Neg(n, _) => write!(out, "Neg({n}"),
        Kind::Not(n, _) => write!(out, "Not({n}"),
        Kind::Chain { ops, .. } => write!(out, "Chain({ops:?}"),
        Kind::Bin { op, .. } => write!(out, "Bin({op:?}"),
    };
    let kids = children(&node.kind);
    let leaf = matches!(
        node.kind,
        Kind::Num { .. } | Kind::Bool(_) | Kind::None | Kind::Var { .. } | Kind::Dotted { .. }
    );
    if !leaf {
        if let Kind::Agg {
            cond, body, orelse, ..
        } = &node.kind
        {
            // Optional parts are marked, so `where c` and `: c` are different sites.
            let _ = write!(
                out,
                ",{}{}{}",
                u8::from(cond.is_some()),
                u8::from(body.is_some()),
                u8::from(orelse.is_some())
            );
        }
        for kid in kids {
            out.push(',');
            canon(kid, out);
        }
        out.push(')');
    }
}

/// An action `Cmd(name: expr, …)` (§6).
#[derive(Clone, Debug)]
pub(crate) struct Action {
    pub name: String,
    pub span: Span,
    pub args: Vec<(String, Node)>,
    pub command: Option<CommandId>,
    pub person_index: usize,
    pub order: Vec<usize>,
    pub dropped: bool,
}

/// A bounded `for` over a relation.
#[derive(Clone, Debug)]
pub(crate) struct ForEach {
    pub var: String,
    pub slot: usize,
    pub rel: Node,
    pub cond: Option<Node>,
}

/// One effect: a conditional action or a bounded `for`.
#[derive(Clone, Debug)]
pub(crate) struct Effect {
    pub action: Action,
    pub when: Option<Node>,
    pub each: Option<ForEach>,
}
