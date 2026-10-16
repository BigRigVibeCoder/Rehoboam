//! Structural validation of a manifest: YAML value to typed [`Manifest`], reporting every error
//! the way Pydantic v2 does for `manifest_v0.py`'s models.
//!
//! The parity contract (V-WC-04) is the verdict and the error count, so this module reproduces
//! Pydantic's rules where they decide either:
//! - all errors are collected (not fail-fast), in field-declaration order, then extra fields;
//! - lax coercion: `int` takes bools, integral floats and numeric strings; `float` takes bools,
//!   ints and numeric strings; `bool` takes 0/1 and the usual words; `str` takes only strings;
//! - a discriminated union reports one error for a missing or unknown tag;
//! - a model's `@model_validator(mode="after")` runs only when the model itself has no error.
//!
//! Messages follow Pydantic's wording but are not guaranteed identical.

use super::types::{
    ActivityResolution, Agent, Artifact, Calendar, CastMember, Catalyst, Children, ClassShares,
    Climate, ClimateShock, CovertOutbreak, CreatureParams, CreaturePeople, Culture,
    DynasticTension, DynasticTensionInitial, Dynasty, Ecology, Faith, Focus, FocusResolution,
    Geography, Horizon, HorizonLabel, Household, HumanPeople, Manifest, Milestone, Narrative,
    OffMapActor, OrderedMap, OverlayEffect, People, Polity, Presentation, Trajectory, Wager,
    WagerStage, WorldMeta,
};
use crate::yaml::Value;
use std::fmt;

/// One structural error, as Pydantic would report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    /// Location: field names, list indexes and union tags, outermost first.
    pub loc: Vec<String>,
    /// Pydantic's error type (`missing`, `extra_forbidden`, `int_parsing`, ...).
    pub kind: &'static str,
    /// Human-readable message.
    pub message: String,
}

impl SchemaError {
    /// The location as Pydantic prints it: parts joined by `.`, a part that itself contains a
    /// `.` in backticks (`` systems.`M1.regions`.e0 ``). Empty for the root.
    #[must_use]
    pub fn loc_text(&self) -> String {
        let parts: Vec<String> = self
            .loc
            .iter()
            .map(|p| {
                if p.contains('.') {
                    format!("`{p}`")
                } else {
                    p.clone()
                }
            })
            .collect();
        parts.join(".")
    }
}

impl fmt::Display for SchemaError {
    /// Pydantic's layout: the location line (absent for the root), then the message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.loc.is_empty() {
            writeln!(f, "{}", self.loc_text())?;
        }
        write!(f, "  {} [type={}]", self.message, self.kind)
    }
}

/// The slug pattern of `manifest_v0.py`.
pub const SLUG_PATTERN: &str = "^[a-z][a-z0-9_]{1,63}$";
/// The registry-version pattern of `manifest_v0.py`.
pub const SEMVER_PATTERN: &str = r"^[0-9]+\.[0-9]+\.[0-9]+$";

type Loc = Vec<String>;

fn at<T: ToString + ?Sized>(loc: &[String], part: &T) -> Loc {
    let mut l = loc.to_vec();
    l.push(part.to_string());
    l
}

#[derive(Default)]
struct Cx {
    errors: Vec<SchemaError>,
}

impl Cx {
    fn err(&mut self, loc: &[String], kind: &'static str, message: impl Into<String>) {
        self.errors.push(SchemaError {
            loc: loc.to_vec(),
            kind,
            message: message.into(),
        });
    }
}

/// Validates a parsed YAML value against the manifest models.
///
/// # Errors
/// Every structural error, in Pydantic's order.
pub fn manifest(value: &Value) -> Result<Manifest, Vec<SchemaError>> {
    let mut cx = Cx::default();
    let m = root(&mut cx, value);
    match m {
        Some(m) if cx.errors.is_empty() => Ok(m),
        _ => Err(cx.errors),
    }
}

// ----------------------------------------------------------------------------- scalars

/// Python's underscore rule for numeric strings: single underscores between digits only.
fn without_underscores(s: &str) -> Option<String> {
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b'_' {
            let prev = i.checked_sub(1).and_then(|j| b.get(j));
            let next = b.get(i + 1);
            if !(prev.is_some_and(u8::is_ascii_digit) && next.is_some_and(u8::is_ascii_digit)) {
                return None;
            }
        }
    }
    Some(s.replace('_', ""))
}

/// A YAML key as a Pydantic location part: the key's Python `str` (`True` is `1`, a date is
/// `datetime.date(2026, 1, 1)`).
fn loc_item(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Null => "None".to_owned(),
        Value::Bool(b) => u8::from(*b).to_string(),
        Value::Timestamp(t) => match t
            .split('-')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(d) if d.len() == 3 => format!("datetime.date({}, {}, {})", d[0], d[1], d[2]),
            _ => t.clone(),
        },
        other => other.to_string(),
    }
}

/// An integer as Pydantic sees it: exact, or larger than this loader holds (only its sign
/// matters: every integer field is bounded within 64 bits).
#[derive(Clone, Copy)]
enum Parsed {
    Exact(i128),
    Big { negative: bool },
}

fn parse_int_str(s: &str) -> Option<Parsed> {
    let t = without_underscores(s.trim())?;
    let (neg, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t.as_str()),
    };
    let (digits, frac) = body.split_once('.').unwrap_or((body, ""));
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if body.contains('.') && (frac.is_empty() || !frac.bytes().all(|c| c == b'0')) {
        return None;
    }
    Some(match digits.parse::<i128>() {
        Ok(v) => Parsed::Exact(if neg { -v } else { v }),
        Err(_) => Parsed::Big { negative: neg },
    })
}

fn parse_float_str(s: &str) -> Option<f64> {
    let t = without_underscores(s.trim())?;
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

/// 2^63: Pydantic converts a float to an integer only inside `i64`.
const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;

#[allow(
    clippy::cast_possible_truncation,
    reason = "the float is finite, integral and inside i64, so the conversion is exact"
)]
fn int(cx: &mut Cx, v: &Value, loc: &[String], ge: Option<i128>, le: Option<i128>) -> Option<i128> {
    let parsed = match v {
        Value::Int(i) => Parsed::Exact(*i),
        Value::BigInt { negative, .. } => Parsed::Big {
            negative: *negative,
        },
        Value::Bool(b) => Parsed::Exact(i128::from(*b)),
        Value::Float(f) if !f.is_finite() => {
            cx.err(loc, "finite_number", "Input should be a finite number");
            return None;
        }
        Value::Float(f) if f.fract() != 0.0 => {
            cx.err(
                loc,
                "int_from_float",
                "Input should be a valid integer, got a number with a fractional part",
            );
            return None;
        }
        Value::Float(f) if *f >= TWO_POW_63 || *f < -TWO_POW_63 => {
            cx.err(
                loc,
                "int_parsing_size",
                "Unable to parse input string as an integer, exceeded maximum size",
            );
            return None;
        }
        Value::Float(f) => Parsed::Exact(i128::from(*f as i64)),
        Value::Str(s) => {
            if let Some(n) = parse_int_str(s) {
                n
            } else {
                cx.err(
                    loc,
                    "int_parsing",
                    "Input should be a valid integer, unable to parse string as an integer",
                );
                return None;
            }
        }
        _ => {
            cx.err(loc, "int_type", "Input should be a valid integer");
            return None;
        }
    };
    match parsed {
        Parsed::Exact(n) => bounded(cx, loc, n, ge, le),
        // Beyond i128: it fails the bound on its side, as Pydantic's big integer does.
        Parsed::Big { negative } => {
            let (bound, kind, word) = if negative {
                (
                    ge.unwrap_or(i128::from(i64::MIN)),
                    "greater_than_equal",
                    "greater",
                )
            } else {
                (
                    le.unwrap_or(i128::from(i64::MAX)),
                    "less_than_equal",
                    "less",
                )
            };
            cx.err(
                loc,
                kind,
                format!("Input should be {word} than or equal to {bound}"),
            );
            None
        }
    }
}

#[allow(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "NaN must fail a bound, as `not (lo <= v)` does in Pydantic"
)]
fn bounded<T: PartialOrd + fmt::Display + Copy>(
    cx: &mut Cx,
    loc: &[String],
    n: T,
    ge: Option<T>,
    le: Option<T>,
) -> Option<T> {
    if let Some(lo) = ge
        && !(n >= lo)
    {
        cx.err(
            loc,
            "greater_than_equal",
            format!("Input should be greater than or equal to {lo}"),
        );
        return None;
    }
    if let Some(hi) = le
        && !(n <= hi)
    {
        cx.err(
            loc,
            "less_than_equal",
            format!("Input should be less than or equal to {hi}"),
        );
        return None;
    }
    Some(n)
}

/// An `int` field whose bounds fit `i64`.
fn int64(cx: &mut Cx, v: &Value, loc: &[String], ge: Option<i64>, le: Option<i64>) -> Option<i64> {
    let n = int(cx, v, loc, ge.map(i128::from), le.map(i128::from))?;
    if let Ok(n) = i64::try_from(n) {
        Some(n)
    } else {
        // Unreachable: every integer field is bounded within i64 (SPR-004 DEF-2 ruling); kept
        // so that an unbounded field added later fails closed.
        cx.err(
            loc,
            "int_type",
            "Input should be a valid integer within 64 bits",
        );
        None
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Python converts int to float the same way"
)]
fn float(cx: &mut Cx, v: &Value, loc: &[String], ge: Option<f64>, le: Option<f64>) -> Option<f64> {
    let x = match v {
        Value::Float(f) => *f,
        Value::Int(i) => *i as f64,
        // Python's float(n) for a big integer: exact up to f64's range, an error beyond it.
        Value::BigInt { text, .. } => match parse_float_str(text) {
            Some(x) if x.is_finite() => x,
            _ => {
                cx.err(loc, "float_type", "Input should be a valid number");
                return None;
            }
        },
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Str(s) => {
            if let Some(x) = parse_float_str(s) {
                x
            } else {
                cx.err(
                    loc,
                    "float_parsing",
                    "Input should be a valid number, unable to parse string as a number",
                );
                return None;
            }
        }
        _ => {
            cx.err(loc, "float_type", "Input should be a valid number");
            return None;
        }
    };
    bounded(cx, loc, x, ge, le)
}

fn unit(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<f64> {
    float(cx, v, loc, Some(0.0), Some(1.0))
}

#[allow(
    clippy::float_cmp,
    reason = "Pydantic accepts exactly 0.0 and 1.0 as booleans"
)]
fn boolean(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<bool> {
    let parsing = |cx: &mut Cx| {
        cx.err(
            loc,
            "bool_parsing",
            "Input should be a valid boolean, unable to interpret input",
        );
    };
    match v {
        Value::Bool(b) => Some(*b),
        Value::Int(0) => Some(false),
        Value::Int(1) => Some(true),
        // Pydantic reads integers beyond 64 bits as another type altogether.
        Value::Int(i) if i64::try_from(*i).is_err() => {
            cx.err(loc, "bool_type", "Input should be a valid boolean");
            None
        }
        Value::Int(_) => {
            parsing(cx);
            None
        }
        Value::Float(f) if *f == 0.0 => Some(false),
        Value::Float(f) if *f == 1.0 => Some(true),
        Value::Float(f) if f.is_finite() && f.fract() == 0.0 => {
            parsing(cx);
            None
        }
        Value::Str(s) => match s.to_ascii_lowercase().as_str() {
            "0" | "off" | "f" | "false" | "n" | "no" => Some(false),
            "1" | "on" | "t" | "true" | "y" | "yes" => Some(true),
            _ => {
                parsing(cx);
                None
            }
        },
        _ => {
            cx.err(loc, "bool_type", "Input should be a valid boolean");
            None
        }
    }
}

fn string(
    cx: &mut Cx,
    v: &Value,
    loc: &[String],
    min: Option<usize>,
    max: Option<usize>,
) -> Option<String> {
    let Value::Str(s) = v else {
        cx.err(loc, "string_type", "Input should be a valid string");
        return None;
    };
    let n = s.chars().count();
    if let Some(lo) = min
        && n < lo
    {
        cx.err(
            loc,
            "string_too_short",
            format!("String should have at least {lo} character{}", plural(lo)),
        );
        return None;
    }
    if let Some(hi) = max
        && n > hi
    {
        cx.err(
            loc,
            "string_too_long",
            format!("String should have at most {hi} character{}", plural(hi)),
        );
        return None;
    }
    Some(s.clone())
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn text(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<String> {
    string(cx, v, loc, None, None)
}

/// `^[a-z][a-z0-9_]{1,63}$`.
#[must_use]
pub fn is_slug(s: &str) -> bool {
    let b = s.as_bytes();
    (2..=64).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b[1..]
            .iter()
            .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn slug(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<String> {
    let s = text(cx, v, loc)?;
    if is_slug(&s) {
        Some(s)
    } else {
        cx.err(
            loc,
            "string_pattern_mismatch",
            format!("String should match pattern '{SLUG_PATTERN}'"),
        );
        None
    }
}

fn expected_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {last}", init.join(", ")),
    }
}

fn lit_str(cx: &mut Cx, v: &Value, loc: &[String], allowed: &[&str]) -> Option<String> {
    if let Value::Str(s) = v
        && allowed.contains(&s.as_str())
    {
        return Some(s.clone());
    }
    let quoted: Vec<String> = allowed.iter().map(|a| format!("'{a}'")).collect();
    cx.err(
        loc,
        "literal_error",
        format!("Input should be {}", expected_list(&quoted)),
    );
    None
}

#[allow(clippy::cast_precision_loss, reason = "literal ints are small")]
fn lit_int(cx: &mut Cx, v: &Value, loc: &[String], allowed: &[i64]) -> Option<i64> {
    let hit = allowed.iter().copied().find(|&a| match v {
        Value::Int(i) => *i == i128::from(a),
        Value::Bool(b) => i64::from(*b) == a,
        Value::Float(f) => f.fract() == 0.0 && (*f - a as f64).abs() < 0.5,
        _ => false,
    });
    if hit.is_none() {
        let items: Vec<String> = allowed.iter().map(ToString::to_string).collect();
        cx.err(
            loc,
            "literal_error",
            format!("Input should be {}", expected_list(&items)),
        );
    }
    hit
}

// ----------------------------------------------------------------------------- containers

fn list<T>(
    cx: &mut Cx,
    v: &Value,
    loc: &[String],
    min: usize,
    mut item: impl FnMut(&mut Cx, &Value, &[String]) -> Option<T>,
) -> Option<Vec<T>> {
    let Value::Seq(items) = v else {
        cx.err(loc, "list_type", "Input should be a valid list");
        return None;
    };
    let start = cx.errors.len();
    let out: Vec<Option<T>> = items
        .iter()
        .enumerate()
        .map(|(i, x)| item(cx, x, &at(loc, &i)))
        .collect();
    if items.len() < min {
        cx.err(
            loc,
            "too_short",
            format!(
                "List should have at least {min} item{} after validation, not {}",
                plural(min),
                items.len()
            ),
        );
    }
    if cx.errors.len() > start {
        return None;
    }
    out.into_iter().collect()
}

/// What a dict's keys must be.
#[derive(Clone, Copy)]
enum Key {
    Str,
    Slug,
}

fn dict<T>(
    cx: &mut Cx,
    v: &Value,
    loc: &[String],
    key: Key,
    mut item: impl FnMut(&mut Cx, &Value, &[String]) -> Option<T>,
) -> Option<OrderedMap<T>> {
    let Value::Map(pairs) = v else {
        cx.err(loc, "dict_type", "Input should be a valid dictionary");
        return None;
    };
    let start = cx.errors.len();
    let mut out = Vec::with_capacity(pairs.len());
    for (k, x) in pairs {
        let key_loc = loc_item(k);
        let kloc = at(&at(loc, &key_loc), "[key]");
        let k = match key {
            Key::Str => text(cx, k, &kloc),
            Key::Slug => slug(cx, k, &kloc),
        };
        let x = item(cx, x, &at(loc, &key_loc));
        if let (Some(k), Some(x)) = (k, x) {
            out.push((k, x));
        }
    }
    (cx.errors.len() == start).then_some(OrderedMap(out))
}

/// A model's input mapping. Fields are taken in declaration order; [`Obj::finish`] reports the
/// keys no field took as `extra_forbidden`.
struct Obj<'v> {
    pairs: &'v [(Value, Value)],
    loc: Loc,
    taken: Vec<&'static str>,
    start: usize,
}

impl<'v> Obj<'v> {
    /// Opens a model; `name` is the model's class name, for the message of a non-mapping input.
    fn new(cx: &mut Cx, v: &'v Value, loc: &[String], name: &str) -> Option<Self> {
        let Value::Map(pairs) = v else {
            cx.err(
                loc,
                "model_type",
                format!("Input should be a valid dictionary or instance of {name}"),
            );
            return None;
        };
        Some(Obj {
            pairs,
            loc: loc.to_vec(),
            taken: Vec::new(),
            start: cx.errors.len(),
        })
    }

    fn lookup(&mut self, name: &'static str) -> Option<&'v Value> {
        self.taken.push(name);
        self.pairs
            .iter()
            .find(|(k, _)| matches!(k, Value::Str(s) if s == name))
            .map(|(_, v)| v)
    }

    fn req<T>(
        &mut self,
        cx: &mut Cx,
        name: &'static str,
        f: impl FnOnce(&mut Cx, &Value, &[String]) -> Option<T>,
    ) -> Option<T> {
        let loc = at(&self.loc, name);
        if let Some(v) = self.lookup(name) {
            f(cx, v, &loc)
        } else {
            cx.err(&loc, "missing", "Field required");
            None
        }
    }

    fn opt<T>(
        &mut self,
        cx: &mut Cx,
        name: &'static str,
        default: impl FnOnce() -> T,
        f: impl FnOnce(&mut Cx, &Value, &[String]) -> Option<T>,
    ) -> Option<T> {
        let loc = at(&self.loc, name);
        match self.lookup(name) {
            Some(v) => f(cx, v, &loc),
            None => Some(default()),
        }
    }

    /// An optional, nullable field (`X | None = None`): `Some(None)` is a valid null.
    #[allow(
        clippy::option_option,
        reason = "outer: valid or not; inner: the field's own Option"
    )]
    fn nullable<T>(
        &mut self,
        cx: &mut Cx,
        name: &'static str,
        f: impl FnOnce(&mut Cx, &Value, &[String]) -> Option<T>,
    ) -> Option<Option<T>> {
        let loc = at(&self.loc, name);
        match self.lookup(name) {
            None | Some(Value::Null) => Some(None),
            Some(v) => f(cx, v, &loc).map(Some),
        }
    }

    /// Reports extra keys; true when the model (fields and extras) had no error.
    fn finish(self, cx: &mut Cx) -> bool {
        for (k, _) in self.pairs {
            match k {
                Value::Str(s) if self.taken.contains(&s.as_str()) => {}
                Value::Str(_) => cx.err(
                    &at(&self.loc, &loc_item(k)),
                    "extra_forbidden",
                    "Extra inputs are not permitted",
                ),
                _ => cx.err(
                    &at(&self.loc, &loc_item(k)),
                    "invalid_key",
                    "Keys should be strings",
                ),
            }
        }
        cx.errors.len() == self.start
    }
}

/// Runs a model's after-validator when the model itself is clean.
fn after<T>(
    cx: &mut Cx,
    loc: &[String],
    model: Option<T>,
    check: impl FnOnce(&T) -> Option<String>,
) -> Option<T> {
    let m = model?;
    if let Some(msg) = check(&m) {
        cx.err(loc, "value_error", format!("Value error, {msg}"));
        return None;
    }
    Some(m)
}

/// A discriminated union: finds the tag, then validates the matching model under `loc.tag`.
fn tagged<T>(
    cx: &mut Cx,
    v: &Value,
    loc: &[String],
    field: &str,
    variants: &[&str],
    f: impl FnOnce(&mut Cx, &str, &[String]) -> Option<T>,
) -> Option<T> {
    let Value::Map(pairs) = v else {
        cx.err(
            loc,
            "model_attributes_type",
            "Input should be a valid dictionary or object to extract fields from",
        );
        return None;
    };
    let Some((_, tag)) = pairs
        .iter()
        .find(|(k, _)| matches!(k, Value::Str(s) if s == field))
    else {
        cx.err(
            loc,
            "union_tag_not_found",
            format!("Unable to extract tag using discriminator '{field}'"),
        );
        return None;
    };
    match tag {
        Value::Str(t) if variants.contains(&t.as_str()) => f(cx, t, &at(loc, t)),
        other => {
            let expected: Vec<String> = variants.iter().map(|x| format!("'{x}'")).collect();
            cx.err(
                loc,
                "union_tag_invalid",
                format!(
                    "Input tag '{other}' found using '{field}' does not match any of the expected tags: {}",
                    expected.join(", ")
                ),
            );
            None
        }
    }
}

// ----------------------------------------------------------------------------- models

fn root(cx: &mut Cx, v: &Value) -> Option<Manifest> {
    let mut o = Obj::new(cx, v, &[], "Manifest")?;
    let manifest_version = o.req(cx, "manifest_version", |cx, v, l| lit_int(cx, v, l, &[0]));
    let registry_version = o.req(cx, "registry_version", |cx, v, l| {
        let s = text(cx, v, l)?;
        if crate::registry::is_semver(&s) {
            Some(s)
        } else {
            cx.err(
                l,
                "string_pattern_mismatch",
                format!("String should match pattern '{SEMVER_PATTERN}'"),
            );
            None
        }
    });
    let world = o.req(cx, "world", world);
    let geography = o.req(cx, "geography", geography);
    let peoples = o.req(cx, "peoples", |cx, v, l| list(cx, v, l, 1, people));
    let polities = o.opt(cx, "polities", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, polity)
    });
    let cast = o.opt(cx, "cast", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, cast_member)
    });
    let households = o.opt(cx, "households", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, household)
    });
    let off_map_actors = o.opt(cx, "off_map_actors", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, off_map_actor)
    });
    let agents = o.opt(cx, "agents", Vec::new, |cx, v, l| list(cx, v, l, 0, agent));
    let systems = o.req(cx, "systems", |cx, v, l| {
        dict(cx, v, l, Key::Str, |cx, v, l| {
            dict(cx, v, l, Key::Str, |cx, v, l| float(cx, v, l, None, None))
        })
    });
    let institutions = o.opt(cx, "institutions", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, slug)
    });
    let technologies = o.opt(cx, "technologies", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, slug)
    });
    let catalysts = o.opt(cx, "catalysts", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, catalyst)
    });
    let overlays = o.opt(cx, "overlays", Vec::new, |cx, v, l| list(cx, v, l, 0, slug));
    let narrative = o.req(cx, "narrative", narrative);
    let presentation = o.req(cx, "presentation", presentation);
    if !o.finish(cx) {
        return None;
    }
    Some(Manifest {
        manifest_version: manifest_version?,
        registry_version: registry_version?,
        world: world?,
        geography: geography?,
        peoples: peoples?,
        polities: polities?,
        cast: cast?,
        households: households?,
        off_map_actors: off_map_actors?,
        agents: agents?,
        systems: systems?,
        institutions: institutions?,
        technologies: technologies?,
        catalysts: catalysts?,
        overlays: overlays?,
        narrative: narrative?,
        presentation: presentation?,
    })
}

fn calendar(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Calendar> {
    let mut o = Obj::new(cx, v, loc, "Calendar")?;
    let d = Calendar::default();
    let days_per_year = o.opt(
        cx,
        "days_per_year",
        || d.days_per_year,
        |cx, v, l| lit_int(cx, v, l, &[360, 365]),
    );
    let months = o.opt(
        cx,
        "months",
        || d.months,
        |cx, v, l| lit_int(cx, v, l, &[12]),
    );
    let seasons = o.opt(
        cx,
        "seasons",
        || d.seasons,
        |cx, v, l| lit_int(cx, v, l, &[2, 4]),
    );
    let epoch_label = o.opt(cx, "epoch_label", || d.epoch_label.clone(), text);
    o.finish(cx).then_some(())?;
    Some(Calendar {
        days_per_year: days_per_year?,
        months: months?,
        seasons: seasons?,
        epoch_label: epoch_label?,
    })
}

fn horizon(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Horizon> {
    let mut o = Obj::new(cx, v, loc, "Horizon")?;
    let years = o.req(cx, "years", |cx, v, l| int64(cx, v, l, Some(1), Some(5000)));
    let label = o.opt(
        cx,
        "label",
        || HorizonLabel::Historical,
        |cx, v, l| {
            lit_str(cx, v, l, &["historical", "speculative"]).map(|s| {
                if s == "speculative" {
                    HorizonLabel::Speculative
                } else {
                    HorizonLabel::Historical
                }
            })
        },
    );
    o.finish(cx).then_some(())?;
    Some(Horizon {
        years: years?,
        label: label?,
    })
}

fn milestone(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Milestone> {
    let mut o = Obj::new(cx, v, loc, "Milestone")?;
    let id = o.req(cx, "id", slug);
    let by_year = o.req(cx, "by_year", |cx, v, l| {
        int64(cx, v, l, Some(0), Some(5000))
    });
    o.finish(cx).then_some(())?;
    Some(Milestone {
        id: id?,
        by_year: by_year?,
    })
}

fn trajectory(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Trajectory> {
    let mut o = Obj::new(cx, v, loc, "Trajectory")?;
    let technologies = o.opt(cx, "technologies", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, milestone)
    });
    let institutions = o.opt(cx, "institutions", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, milestone)
    });
    o.finish(cx).then_some(())?;
    Some(Trajectory {
        technologies: technologies?,
        institutions: institutions?,
    })
}

fn focus(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Focus> {
    let mut o = Obj::new(cx, v, loc, "Focus")?;
    let settlement = o.req(cx, "settlement", slug);
    let activity_resolution = o.req(cx, "activity_resolution", |cx, v, l| {
        lit_str(cx, v, l, &["blocks", "minutes"]).map(|s| {
            if s == "minutes" {
                FocusResolution::Minutes
            } else {
                FocusResolution::Blocks
            }
        })
    });
    o.finish(cx).then_some(())?;
    Some(Focus {
        settlement: settlement?,
        activity_resolution: activity_resolution?,
    })
}

fn world(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<WorldMeta> {
    let mut o = Obj::new(cx, v, loc, "WorldMeta")?;
    let id = o.req(cx, "id", slug);
    let display_name = o.req(cx, "display_name", |cx, v, l| {
        string(cx, v, l, Some(1), Some(120))
    });
    let template = o.req(cx, "template", slug);
    let seed = o.req(cx, "seed", |cx, v, l| {
        int64(cx, v, l, Some(0), Some(i64::MAX))
    });
    let calendar = o.opt(cx, "calendar", Calendar::default, calendar);
    let horizon = o.req(cx, "horizon", horizon);
    let trajectory = o.nullable(cx, "trajectory", trajectory);
    let fidelity_profile = o.opt(cx, "fidelity_profile", OrderedMap::default, |cx, v, l| {
        dict(cx, v, l, Key::Str, |cx, v, l| {
            lit_str(cx, v, l, &["F0", "F1", "F2", "F3"])
        })
    });
    let activity_resolution = o.nullable(cx, "activity_resolution", |cx, v, l| {
        lit_str(cx, v, l, &["life_course", "blocks", "minutes"]).map(|s| match s.as_str() {
            "life_course" => ActivityResolution::LifeCourse,
            "blocks" => ActivityResolution::Blocks,
            _ => ActivityResolution::Minutes,
        })
    });
    let focus = o.opt(cx, "focus", Vec::new, |cx, v, l| list(cx, v, l, 0, focus));
    let clean = o.finish(cx);
    let w = clean.then_some(()).and_then(|()| {
        Some(WorldMeta {
            id: id?,
            display_name: display_name?,
            template: template?,
            seed: seed?,
            calendar: calendar?,
            horizon: horizon?,
            trajectory: trajectory?,
            fidelity_profile: fidelity_profile?,
            activity_resolution: activity_resolution?,
            focus: focus?,
        })
    });
    after(cx, loc, w, |w| {
        let dated = w
            .trajectory
            .as_ref()
            .is_some_and(|t| !(t.technologies.is_empty() && t.institutions.is_empty()));
        (w.horizon.label == HorizonLabel::Speculative && !dated).then(|| {
            "horizon.label = 'speculative' requires world.trajectory with at least one dated technology or institution"
                .to_owned()
        })
    })
}

fn climate(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Climate> {
    let mut o = Obj::new(cx, v, loc, "Climate")?;
    let zone = o.req(cx, "zone", slug);
    let f = |o: &mut Obj<'_>, cx: &mut Cx, name: &'static str, d: f64| {
        o.opt(cx, name, || d, |cx, v, l| float(cx, v, l, None, None))
    };
    let harvest_sigma = f(&mut o, cx, "harvest_sigma", 0.2);
    let autocorr = f(&mut o, cx, "autocorr", 0.1);
    let spatial_share = f(&mut o, cx, "spatial_share", 0.6);
    let volcanic_rate = f(&mut o, cx, "volcanic_rate", 0.004);
    o.finish(cx).then_some(())?;
    Some(Climate {
        zone: zone?,
        harvest_sigma: harvest_sigma?,
        autocorr: autocorr?,
        spatial_share: spatial_share?,
        volcanic_rate: volcanic_rate?,
    })
}

fn ecology(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Ecology> {
    let mut o = Obj::new(cx, v, loc, "Ecology")?;
    let livestock = o.opt(
        cx,
        "livestock_kg_per_capita",
        || 150.0,
        |cx, v, l| float(cx, v, l, Some(0.0), Some(2000.0)),
    );
    let prey = o.opt(
        cx,
        "wild_prey_tonnes",
        || 0.0,
        |cx, v, l| float(cx, v, l, Some(0.0), None),
    );
    o.finish(cx).then_some(())?;
    Some(Ecology {
        livestock_kg_per_capita: livestock?,
        wild_prey_tonnes: prey?,
    })
}

fn geography(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Geography> {
    let mut o = Obj::new(cx, v, loc, "Geography")?;
    let raster_resolution = o.opt(
        cx,
        "raster_resolution",
        || 512,
        |cx, v, l| lit_int(cx, v, l, &[64, 128, 256, 512, 1024, 2048, 4096]),
    );
    let region_count = o.opt(
        cx,
        "region_count",
        || 600,
        |cx, v, l| int64(cx, v, l, Some(1), Some(5000)),
    );
    let heightmap_template = o.req(cx, "heightmap_template", slug);
    let climate = o.req(cx, "climate", climate);
    let ecology = o.opt(cx, "ecology", Ecology::default, ecology);
    o.finish(cx).then_some(())?;
    Some(Geography {
        raster_resolution: raster_resolution?,
        region_count: region_count?,
        heightmap_template: heightmap_template?,
        climate: climate?,
        ecology: ecology?,
    })
}

fn culture(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Culture> {
    let mut o = Obj::new(cx, v, loc, "Culture")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let naming_seed = o.req(cx, "naming_seed", |cx, v, l| {
        string(cx, v, l, None, Some(200))
    });
    let value_bias = o.opt(cx, "value_bias", OrderedMap::default, |cx, v, l| {
        dict(cx, v, l, Key::Str, |cx, v, l| {
            float(cx, v, l, Some(-1.0), Some(1.0))
        })
    });
    o.finish(cx).then_some(())?;
    Some(Culture {
        id: id?,
        name: name?,
        naming_seed: naming_seed?,
        value_bias: value_bias?,
    })
}

fn faith(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Faith> {
    let mut o = Obj::new(cx, v, loc, "Faith")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let doctrine_tags = o.opt(cx, "doctrine_tags", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, slug)
    });
    o.finish(cx).then_some(())?;
    Some(Faith {
        id: id?,
        name: name?,
        doctrine_tags: doctrine_tags?,
    })
}

fn class_shares(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<ClassShares> {
    let mut o = Obj::new(cx, v, loc, "ClassShares")?;
    let commoner = o.req(cx, "commoner", unit);
    let burgher = o.req(cx, "burgher", unit);
    let elite = o.req(cx, "elite", unit);
    let clean = o.finish(cx);
    let shares = clean.then_some(()).and_then(|()| {
        Some(ClassShares {
            commoner: commoner?,
            burgher: burgher?,
            elite: elite?,
        })
    });
    after(cx, loc, shares, |c| {
        let s = c.commoner + c.burgher + c.elite;
        ((s - 1.0).abs() > 1e-6).then(|| format!("class shares must sum to 1.0 (got {s:.4})"))
    })
}

fn people(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<People> {
    tagged(
        cx,
        v,
        loc,
        "archetype",
        &["human", "apex_predator"],
        |cx, tag, loc| {
            if tag == "human" {
                human(cx, v, loc).map(People::Human)
            } else {
                creature(cx, v, loc).map(People::Creature)
            }
        },
    )
}

fn human(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<HumanPeople> {
    let mut o = Obj::new(cx, v, loc, "HumanPeople")?;
    let _ = o.req(cx, "archetype", |cx, v, l| lit_str(cx, v, l, &["human"]));
    let population = o.req(cx, "population", |cx, v, l| {
        int64(cx, v, l, Some(1), Some(10_000_000))
    });
    let cultures = o.req(cx, "cultures", |cx, v, l| list(cx, v, l, 1, culture));
    let faiths = o.req(cx, "faiths", |cx, v, l| list(cx, v, l, 1, faith));
    let class_shares = o.req(cx, "class_shares", class_shares);
    o.finish(cx).then_some(())?;
    Some(HumanPeople {
        population: population?,
        cultures: cultures?,
        faiths: faiths?,
        class_shares: class_shares?,
    })
}

fn creature_params(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<CreatureParams> {
    let mut o = Obj::new(cx, v, loc, "CreatureParams")?;
    let mass_kg = o.req(cx, "mass_kg", |cx, v, l| float(cx, v, l, None, None));
    let metabolism_multiplier = o.opt(
        cx,
        "metabolism_multiplier",
        || 1.0,
        |cx, v, l| float(cx, v, l, None, None),
    );
    let flying = o.opt(cx, "flying", || false, boolean);
    let bondable = o.opt(cx, "bondable", || false, boolean);
    o.finish(cx).then_some(())?;
    Some(CreatureParams {
        mass_kg: mass_kg?,
        metabolism_multiplier: metabolism_multiplier?,
        flying: flying?,
        bondable: bondable?,
    })
}

fn creature(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<CreaturePeople> {
    let mut o = Obj::new(cx, v, loc, "CreaturePeople")?;
    let _ = o.req(cx, "archetype", |cx, v, l| {
        lit_str(cx, v, l, &["apex_predator"])
    });
    let variant = o.req(cx, "variant", slug);
    let population = o.req(cx, "population", |cx, v, l| {
        int64(cx, v, l, Some(1), Some(100_000))
    });
    let params = o.req(cx, "params", creature_params);
    o.finish(cx).then_some(())?;
    Some(CreaturePeople {
        variant: variant?,
        population: population?,
        params: params?,
    })
}

fn dynasty(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Dynasty> {
    let mut o = Obj::new(cx, v, loc, "Dynasty")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let generations = o.opt(
        cx,
        "generations",
        || 3,
        |cx, v, l| int64(cx, v, l, Some(1), Some(6)),
    );
    o.finish(cx).then_some(())?;
    Some(Dynasty {
        id: id?,
        name: name?,
        generations: generations?,
    })
}

fn polity(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Polity> {
    let mut o = Obj::new(cx, v, loc, "Polity")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let government = o.req(cx, "government", slug);
    let succession_law = o.req(cx, "succession_law", slug);
    let titles = o.req(cx, "titles", |cx, v, l| dict(cx, v, l, Key::Slug, text));
    let dynasties = o.opt(cx, "dynasties", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, dynasty)
    });
    o.finish(cx).then_some(())?;
    Some(Polity {
        id: id?,
        name: name?,
        government: government?,
        succession_law: succession_law?,
        titles: titles?,
        dynasties: dynasties?,
    })
}

fn tension_initial(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<DynasticTensionInitial> {
    let mut o = Obj::new(cx, v, loc, "DynasticTensionInitial")?;
    let opinion = |cx: &mut Cx, v: &Value, l: &[String]| int64(cx, v, l, Some(-100), Some(100));
    let sa = o.opt(cx, "opinion_successor_of_authority", || 0, opinion);
    let as_ = o.opt(cx, "opinion_authority_of_successor", || 0, opinion);
    let fear = o.opt(cx, "fear_authority", || 0.0, unit);
    let heir = o.opt(
        cx,
        "heir_age_years",
        || 20,
        |cx, v, l| int64(cx, v, l, Some(0), Some(80)),
    );
    o.finish(cx).then_some(())?;
    Some(DynasticTensionInitial {
        opinion_successor_of_authority: sa?,
        opinion_authority_of_successor: as_?,
        fear_authority: fear?,
        heir_age_years: heir?,
    })
}

fn overlay_effect(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<OverlayEffect> {
    let mut o = Obj::new(cx, v, loc, "OverlayEffect")?;
    let overlay = o.req(cx, "overlay", slug);
    let holder_delta = o.req(cx, "holder_delta", |cx, v, l| {
        float(cx, v, l, Some(-1.0), Some(1.0))
    });
    o.finish(cx).then_some(())?;
    Some(OverlayEffect {
        overlay: overlay?,
        holder_delta: holder_delta?,
    })
}

fn wager_stage(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<WagerStage> {
    let mut o = Obj::new(cx, v, loc, "WagerStage")?;
    let allow = o.req(cx, "allow", |cx, v, l| list(cx, v, l, 1, text));
    let spare_life = o.opt(cx, "spare_life", || true, boolean);
    let unlock_when = o.nullable(cx, "unlock_when", |cx, v, l| {
        string(cx, v, l, None, Some(200))
    });
    o.finish(cx).then_some(())?;
    Some(WagerStage {
        allow: allow?,
        spare_life: spare_life?,
        unlock_when: unlock_when?,
    })
}

const CATALYST_KINDS: [&str; 5] = [
    "dynastic_tension",
    "artifact",
    "covert_outbreak",
    "climate_shock",
    "wager",
];

fn catalyst(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    tagged(
        cx,
        v,
        loc,
        "kind",
        &CATALYST_KINDS,
        |cx, tag, loc| match tag {
            "dynastic_tension" => dynastic_tension(cx, v, loc),
            "artifact" => artifact(cx, v, loc),
            "covert_outbreak" => covert_outbreak(cx, v, loc),
            "climate_shock" => climate_shock(cx, v, loc),
            _ => wager(cx, v, loc),
        },
    )
}

fn kind(o: &mut Obj<'_>, cx: &mut Cx, k: &'static str) {
    let _ = o.req(cx, "kind", |cx, v, l| lit_str(cx, v, l, &[k]));
}

fn dynastic_tension(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    let mut o = Obj::new(cx, v, loc, "DynasticTension")?;
    kind(&mut o, cx, "dynastic_tension");
    let polity = o.req(cx, "polity", slug);
    let initial = o.req(cx, "initial", tension_initial);
    o.finish(cx).then_some(())?;
    Some(Catalyst::DynasticTension(DynasticTension {
        polity: polity?,
        initial: initial?,
    }))
}

fn artifact(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    let mut o = Obj::new(cx, v, loc, "Artifact")?;
    kind(&mut o, cx, "artifact");
    let name = o.req(cx, "name", text);
    let polity = o.req(cx, "polity", slug);
    let holder_role = o.req(cx, "holder_role", slug);
    let effects = o.req(cx, "effects", |cx, v, l| list(cx, v, l, 1, overlay_effect));
    o.finish(cx).then_some(())?;
    Some(Catalyst::Artifact(Artifact {
        name: name?,
        polity: polity?,
        holder_role: holder_role?,
        effects: effects?,
    }))
}

fn covert_outbreak(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    let mut o = Obj::new(cx, v, loc, "CovertOutbreak")?;
    kind(&mut o, cx, "covert_outbreak");
    let seed_infected = o.req(cx, "seed_infected", |cx, v, l| {
        int64(cx, v, l, Some(1), Some(10_000))
    });
    let hint = o.opt(
        cx,
        "seed_region_hint",
        || "random".to_owned(),
        |cx, v, l| lit_str(cx, v, l, &["capital", "port", "periphery", "random"]),
    );
    let channel = o.opt(
        cx,
        "channel",
        || "hive".to_owned(),
        |cx, v, l| lit_str(cx, v, l, &["hive"]),
    );
    let start_day = o.opt(
        cx,
        "start_day",
        || 0,
        |cx, v, l| int64(cx, v, l, Some(0), Some(i64::MAX)),
    );
    o.finish(cx).then_some(())?;
    Some(Catalyst::CovertOutbreak(CovertOutbreak {
        seed_infected: seed_infected?,
        seed_region_hint: hint?,
        channel: channel?,
        start_day: start_day?,
    }))
}

fn climate_shock(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    let mut o = Obj::new(cx, v, loc, "ClimateShock")?;
    kind(&mut o, cx, "climate_shock");
    let year = o.req(cx, "year", |cx, v, l| {
        int64(cx, v, l, Some(0), Some(i64::MAX))
    });
    let severity_sigma = o.req(cx, "severity_sigma", |cx, v, l| {
        float(cx, v, l, Some(0.5), Some(4.0))
    });
    let duration_years = o.opt(
        cx,
        "duration_years",
        || 1,
        |cx, v, l| int64(cx, v, l, Some(1), Some(10)),
    );
    o.finish(cx).then_some(())?;
    Some(Catalyst::ClimateShock(ClimateShock {
        year: year?,
        severity_sigma: severity_sigma?,
        duration_years: duration_years?,
    }))
}

fn wager(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Catalyst> {
    let mut o = Obj::new(cx, v, loc, "Wager")?;
    kind(&mut o, cx, "wager");
    let agent = o.req(cx, "agent", slug);
    let protagonist = o.req(cx, "protagonist", slug);
    let stages = o.req(cx, "stages", |cx, v, l| list(cx, v, l, 1, wager_stage));
    o.finish(cx).then_some(())?;
    Some(Catalyst::Wager(Wager {
        agent: agent?,
        protagonist: protagonist?,
        stages: stages?,
    }))
}

fn cast_member(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<CastMember> {
    let mut o = Obj::new(cx, v, loc, "CastMember")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let sex = o.req(cx, "sex", |cx, v, l| lit_str(cx, v, l, &["male", "female"]));
    let age_years = o.req(cx, "age_years", |cx, v, l| {
        int64(cx, v, l, Some(0), Some(120))
    });
    let household = o.nullable(cx, "household", slug);
    let flags = o.opt(cx, "flags", Vec::new, |cx, v, l| {
        list(cx, v, l, 0, |cx, v, l| {
            lit_str(cx, v, l, &["protagonist", "deep"])
        })
    });
    let initial = o.opt(cx, "initial", OrderedMap::default, |cx, v, l| {
        dict(cx, v, l, Key::Slug, |cx, v, l| {
            float(cx, v, l, Some(-1.0), Some(1.0))
        })
    });
    o.finish(cx).then_some(())?;
    Some(CastMember {
        id: id?,
        name: name?,
        sex: sex?,
        age_years: age_years?,
        household: household?,
        flags: flags?,
        initial: initial?,
    })
}

fn children(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Children> {
    let mut o = Obj::new(cx, v, loc, "Children")?;
    let n = |cx: &mut Cx, v: &Value, l: &[String]| int64(cx, v, l, Some(0), Some(40));
    let sons = o.opt(cx, "sons", || 0, n);
    let daughters = o.opt(cx, "daughters", || 0, n);
    o.finish(cx).then_some(())?;
    Some(Children {
        sons: sons?,
        daughters: daughters?,
    })
}

fn household(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Household> {
    let mut o = Obj::new(cx, v, loc, "Household")?;
    let id = o.req(cx, "id", slug);
    let head = o.req(cx, "head", slug);
    let children = o.opt(cx, "children", Children::default, children);
    let servants = o.opt(
        cx,
        "servants",
        || 0,
        |cx, v, l| int64(cx, v, l, Some(0), Some(100_000)),
    );
    let holdings = o.opt(cx, "holdings", OrderedMap::default, |cx, v, l| {
        dict(cx, v, l, Key::Slug, |cx, v, l| {
            int64(cx, v, l, Some(0), Some(i64::MAX))
        })
    });
    o.finish(cx).then_some(())?;
    Some(Household {
        id: id?,
        head: head?,
        children: children?,
        servants: servants?,
        holdings: holdings?,
    })
}

fn off_map_actor(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<OffMapActor> {
    let mut o = Obj::new(cx, v, loc, "OffMapActor")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let kind = o.req(cx, "kind", slug);
    let strength = o.opt(cx, "strength", || 0.5, unit);
    o.finish(cx).then_some(())?;
    Some(OffMapActor {
        id: id?,
        name: name?,
        kind: kind?,
        strength: strength?,
    })
}

fn agent(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Agent> {
    let mut o = Obj::new(cx, v, loc, "Agent")?;
    let id = o.req(cx, "id", slug);
    let name = o.req(cx, "name", text);
    let archetype = o.req(cx, "archetype", slug);
    let verbs = o.req(cx, "verbs", |cx, v, l| list(cx, v, l, 1, text));
    let policy = o.opt(
        cx,
        "policy",
        || "deterministic".to_owned(),
        |cx, v, l| lit_str(cx, v, l, &["deterministic", "player"]),
    );
    o.finish(cx).then_some(())?;
    Some(Agent {
        id: id?,
        name: name?,
        archetype: archetype?,
        verbs: verbs?,
        policy: policy?,
    })
}

fn narrative(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Narrative> {
    let mut o = Obj::new(cx, v, loc, "Narrative")?;
    let pattern_packs = o.req(cx, "pattern_packs", |cx, v, l| list(cx, v, l, 1, slug));
    let default_style_card = o.req(cx, "default_style_card", slug);
    o.finish(cx).then_some(())?;
    Some(Narrative {
        pattern_packs: pattern_packs?,
        default_style_card: default_style_card?,
    })
}

fn presentation(cx: &mut Cx, v: &Value, loc: &[String]) -> Option<Presentation> {
    let mut o = Obj::new(cx, v, loc, "Presentation")?;
    let era_art = o.req(cx, "era_art", slug);
    let palette = o.req(cx, "palette", slug);
    o.finish(cx).then_some(())?;
    Some(Presentation {
        era_art: era_art?,
        palette: palette?,
    })
}
