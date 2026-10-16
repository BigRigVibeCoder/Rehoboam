//! A YAML loader that resolves scalars the way `PyYAML`'s `safe_load` does (YAML 1.1).
//!
//! The reference validator reads manifests with `PyYAML`, whose implicit typing differs from YAML
//! 1.2: `yes`/`on` are booleans, `1.0e9` (no exponent sign) is a *string*, `010` is octal 8, and
//! `2026-10-16` is a timestamp. Pydantic then coerces those values. To give the same verdict and
//! error count, this loader reproduces `PyYAML`'s resolver and constructor on top of the
//! `saphyr-parser` event stream; `manifest::coerce` reproduces Pydantic's coercion.
//!
//! Supported: one document, block and flow collections, anchors and aliases, merge keys (`<<`),
//! the core explicit tags (`!!str`, `!!int`, `!!float`, `!!bool`, `!!null`, `!!map`, `!!seq`),
//! and Python's dict semantics for repeated keys (the last value wins, at the first position;
//! `1`, `1.0` and `true` are the same key). Anything `PyYAML`'s safe loader refuses is a load error.
//!
//! Integers beyond 127 bits are kept as [`Value::BigInt`] (sign and text): every manifest integer
//! field is bounded to 64 bits (DEF-2 ruling), so only the sign matters to validation.
//!
//! Where `PyYAML` is stricter than the YAML specification, this loader follows `PyYAML` (GAP-1
//! ruling): a tab outside quoted scalars, block scalars and comments, a `?` inside a plain scalar
//! in a flow collection, and an empty implicit key (`: value`) are load errors. A leading UTF-8
//! byte order mark is skipped, as `PyYAML` does (DEF-1).

use std::collections::BTreeMap;
use std::fmt;

use saphyr_parser::{Event, Parser, ScalarStyle, Tag};

/// A YAML value after `PyYAML`-style resolution and construction.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, `~` or an empty plain scalar.
    Null,
    /// A YAML 1.1 boolean (`true`, `yes`, `on`, ... and their negations).
    Bool(bool),
    /// An integer (decimal, `0b`, `0x`, leading-zero octal or sexagesimal).
    Int(i128),
    /// An integer too large for `i128`: its sign and its text. Python holds it exactly.
    BigInt {
        /// Whether it is negative.
        negative: bool,
        /// The scalar as written.
        text: String,
    },
    /// A float.
    Float(f64),
    /// A string.
    Str(String),
    /// A YAML timestamp (`2026-10-16`); Python builds a date or datetime from it, which no
    /// manifest field accepts. The original text is kept for messages.
    Timestamp(String),
    /// A sequence.
    Seq(Vec<Value>),
    /// A mapping, in first-appearance key order, with Python dict semantics for repeated keys.
    Map(Vec<(Value, Value)>),
}

impl Value {
    /// The Python type name of this value, used in messages (`'str'`, `'int'`, ...).
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "NoneType",
            Value::Bool(_) => "bool",
            Value::Int(_) | Value::BigInt { .. } => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Timestamp(_) => "date",
            Value::Seq(_) => "list",
            Value::Map(_) => "dict",
        }
    }

    /// Python equality between two dict keys: numbers compare by value across bool, int and
    /// float (`1 == 1.0 == True`); everything else compares structurally.
    #[allow(
        clippy::float_cmp,
        reason = "Python dict keys compare with ==, exactly"
    )]
    fn same_key(&self, other: &Value) -> bool {
        match (self.as_key_number(), other.as_key_number()) {
            (Some(a), Some(b)) => a == b,
            (None, None) => self == other,
            _ => false,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Python compares int and float keys as floats"
    )]
    fn as_key_number(&self) -> Option<f64> {
        match self {
            Value::Bool(b) => Some(f64::from(u8::from(*b))),
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    /// Python's `repr`-like rendering, used in error locations and messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("None"),
            Value::Bool(true) => f.write_str("True"),
            Value::Bool(false) => f.write_str("False"),
            Value::Int(i) => write!(f, "{i}"),
            Value::BigInt { text, .. } => f.write_str(text),
            Value::Float(x) => f.write_str(&crate::pyfmt::float_repr(*x)),
            Value::Str(s) => f.write_str(s),
            Value::Timestamp(s) => write!(f, "date({s})"),
            Value::Seq(_) => f.write_str("[...]"),
            Value::Map(_) => f.write_str("{...}"),
        }
    }
}

/// A manifest file that YAML (as `PyYAML` reads it) cannot load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    /// What went wrong, with a position when the parser gives one.
    pub message: String,
}

impl fmt::Display for YamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for YamlError {}

fn yerr(message: impl Into<String>) -> YamlError {
    YamlError {
        message: message.into(),
    }
}

/// Parses one YAML document as `PyYAML`'s `safe_load` would. An empty stream is `Null`.
///
/// # Errors
/// A syntax error, more than one document, an unknown tag, an unhashable or recursive key, or a
/// value `PyYAML`'s constructor rejects.
pub fn load(text: &str) -> Result<Value, YamlError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut builder = Builder::default();
    let mut grammar = PyYamlGrammar::new(text);
    let mut documents = 0usize;
    for item in Parser::new_from_str(text) {
        let (event, span) = item.map_err(|e| yerr(format!("YAML syntax error: {e}")))?;
        let at = || format!("line {}, column {}", span.start.line(), span.start.col());
        grammar
            .event(
                &event,
                span.start.index(),
                span.end.index(),
                builder.key_position(),
            )
            .map_err(|m| yerr(format!("{m} ({})", at())))?;
        match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    return Err(yerr(format!(
                        "expected a single document in the stream, but found another document ({})",
                        at()
                    )));
                }
            }
            Event::Scalar(text, style, anchor, tag) => {
                let value = scalar(&text, style, tag.as_deref())
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
                builder
                    .value(value, anchor)
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
            }
            Event::SequenceStart(anchor, tag) => {
                collection_tag(tag.as_deref(), "seq")
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
                builder.open(Frame::Seq(Vec::new(), anchor));
            }
            Event::MappingStart(anchor, tag) => {
                collection_tag(tag.as_deref(), "map")
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
                builder.open(Frame::Map(Vec::new(), None, anchor));
            }
            Event::SequenceEnd | Event::MappingEnd => {
                builder
                    .close()
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
            }
            Event::Alias(id) => {
                let value = builder.anchors.get(&id).cloned().ok_or_else(|| {
                    yerr(format!("found undefined or recursive alias ({})", at()))
                })?;
                builder
                    .value(value, 0)
                    .map_err(|m| yerr(format!("{m} ({})", at())))?;
            }
            Event::StreamStart | Event::StreamEnd | Event::DocumentEnd | Event::Nothing => {}
        }
    }
    grammar.tabs().map_err(yerr)?;
    Ok(builder.root.unwrap_or(Value::Null))
}

/// The three places where `PyYAML` refuses what the YAML specification allows (GAP-1).
struct PyYamlGrammar<'t> {
    text: &'t str,
    /// For each open collection: whether it is a flow collection (`[` or `{`).
    flow: Vec<bool>,
    /// Byte ranges where a tab is allowed: quoted and block scalars.
    tab_ok: Vec<(usize, usize)>,
    /// Byte ranges of every scalar, so that a `#` inside one does not start a comment.
    scalars: Vec<(usize, usize)>,
}

impl<'t> PyYamlGrammar<'t> {
    fn new(text: &'t str) -> Self {
        PyYamlGrammar {
            text,
            flow: Vec::new(),
            tab_ok: Vec::new(),
            scalars: Vec::new(),
        }
    }

    fn event(
        &mut self,
        event: &Event<'_>,
        start: usize,
        end: usize,
        key_position: bool,
    ) -> Result<(), String> {
        let bytes = self.text.as_bytes();
        match event {
            Event::SequenceStart(..) | Event::MappingStart(..) => {
                self.flow
                    .push(matches!(bytes.get(start), Some(b'[' | b'{')));
            }
            Event::SequenceEnd | Event::MappingEnd => {
                self.flow.pop();
            }
            Event::Scalar(value, style, ..) => {
                let in_flow = self.flow.last().copied().unwrap_or(false);
                match style {
                    ScalarStyle::Plain => {
                        let end = end.max(start);
                        self.scalars.push((start, end));
                        if in_flow && value.chars().skip(1).any(|c| c == '?') {
                            return Err(format!(
                                "found '?' inside the plain scalar '{value}' in a flow collection (PyYAML refuses it)"
                            ));
                        }
                        if key_position
                            && value.is_empty()
                            && start == end
                            && self.implicit_empty_key(start)
                        {
                            return Err("found an empty key written ': value' (PyYAML refuses it)"
                                .to_owned());
                        }
                    }
                    ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted => {
                        let close = self.closing_quote(start, *style == ScalarStyle::SingleQuoted);
                        self.tab_ok.push((start, close));
                        self.scalars.push((start, close));
                    }
                    _ => {
                        self.tab_ok.push((start, end));
                        self.scalars.push((start, end));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// An empty implicit key sits at a `:` that no `?` indicator precedes.
    fn implicit_empty_key(&self, at: usize) -> bool {
        let bytes = self.text.as_bytes();
        if bytes.get(at) != Some(&b':') {
            return false;
        }
        let before = self.text[..at].trim_end_matches([' ', '\t', '\r', '\n']);
        !before.ends_with('?')
    }

    /// The byte index just past the closing quote of a quoted scalar starting at `start`.
    fn closing_quote(&self, start: usize, single: bool) -> usize {
        let bytes = self.text.as_bytes();
        let quote = if single { b'\'' } else { b'"' };
        let mut i = start + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if !single => i += 2,
                c if c == quote && single && bytes.get(i + 1) == Some(&quote) => i += 2,
                c if c == quote => return i + 1,
                _ => i += 1,
            }
        }
        bytes.len()
    }

    /// A tab is allowed only inside quoted and block scalars and in comments.
    fn tabs(&self) -> Result<(), String> {
        let inside =
            |ranges: &[(usize, usize)], i: usize| ranges.iter().any(|&(a, b)| a <= i && i < b);
        let mut line_start = 0;
        for line in self.text.split_inclusive('\n') {
            let mut comment = false;
            for (off, c) in line.char_indices() {
                let i = line_start + off;
                if c == '#' && !comment && !inside(&self.scalars, i) {
                    let prev = line[..off].chars().last();
                    comment = prev.is_none_or(|p| p == ' ' || p == '\t');
                }
                if c == '\t' && !comment && !inside(&self.tab_ok, i) {
                    let line_no = self.text[..i].matches('\n').count() + 1;
                    return Err(format!(
                        "found a tab outside a quoted or block scalar or a comment (line {line_no}; PyYAML refuses it)"
                    ));
                }
            }
            line_start += line.len();
        }
        Ok(())
    }
}

/// A collection under construction. A mapping holds its pairs, a pending key, and its merge
/// sources (`<<`), which are applied when it closes.
enum Frame {
    Seq(Vec<Value>, usize),
    Map(Vec<(Value, Value)>, Option<PendingKey>, usize),
}

enum PendingKey {
    Key(Value),
    Merge,
}

/// The pairs of one mapping.
type Pairs = Vec<(Value, Value)>;

#[derive(Default)]
struct Builder {
    /// Open collections, each with the merge sources (`<<`) collected for it.
    stack: Vec<(Frame, Vec<Pairs>)>,
    anchors: BTreeMap<usize, Value>,
    root: Option<Value>,
}

impl Builder {
    /// Whether the next value is a mapping key.
    fn key_position(&self) -> bool {
        matches!(self.stack.last(), Some((Frame::Map(_, None, _), _)))
    }

    fn open(&mut self, frame: Frame) {
        self.stack.push((frame, Vec::new()));
    }

    fn close(&mut self) -> Result<(), String> {
        let Some((frame, merges)) = self.stack.pop() else {
            return Err("unbalanced collection end".to_owned());
        };
        let (value, anchor) = match frame {
            Frame::Seq(items, anchor) => (Value::Seq(items), anchor),
            Frame::Map(pairs, _, anchor) => {
                // PyYAML's flatten_mapping: merged pairs come first, in the order collected,
                // and later pairs (finally the explicit keys) override earlier ones.
                let mut all: Vec<(Value, Value)> = merges.into_iter().flatten().collect();
                all.extend(pairs);
                let mut out: Vec<(Value, Value)> = Vec::with_capacity(all.len());
                for (k, v) in all {
                    insert(&mut out, k, v);
                }
                (Value::Map(out), anchor)
            }
        };
        self.value(value, anchor)
    }

    fn value(&mut self, value: Value, anchor: usize) -> Result<(), String> {
        if anchor != 0 {
            self.anchors.insert(anchor, value.clone());
        }
        let key_position = matches!(self.stack.last(), Some((Frame::Map(_, None, _), _)));
        if is_merge_marker(&value) && !key_position {
            return Err(
                "could not determine a constructor for the tag 'tag:yaml.org,2002:merge'"
                    .to_owned(),
            );
        }
        let Some((frame, merges)) = self.stack.last_mut() else {
            self.root = Some(value);
            return Ok(());
        };
        match frame {
            Frame::Seq(items, _) => items.push(value),
            Frame::Map(pairs, pending, _) => match pending.take() {
                None => {
                    if matches!(value, Value::Seq(_) | Value::Map(_)) {
                        return Err("found unhashable key".to_owned());
                    }
                    *pending = Some(if is_merge_marker(&value) {
                        PendingKey::Merge
                    } else {
                        PendingKey::Key(value)
                    });
                }
                Some(PendingKey::Key(k)) => pairs.push((k, value)),
                Some(PendingKey::Merge) => match value {
                    Value::Map(m) => merges.push(m),
                    Value::Seq(items) => {
                        let mut sources = Vec::new();
                        for item in items {
                            let Value::Map(m) = item else {
                                return Err("expected a mapping for merging".to_owned());
                            };
                            sources.push(m);
                        }
                        // PyYAML reverses a merge sequence so that earlier sources win.
                        sources.reverse();
                        merges.extend(sources);
                    }
                    _ => {
                        return Err("expected a mapping or list of mappings for merging".to_owned());
                    }
                },
            },
        }
        Ok(())
    }
}

/// The merge key resolves only from a plain `<<`; `scalar` marks it with this private text.
const MERGE_KEY: &str = "\u{0}<<merge>>";

fn is_merge_marker(v: &Value) -> bool {
    matches!(v, Value::Str(s) if s == MERGE_KEY)
}

/// Python dict assignment: an equal key keeps its position and takes the new value.
fn insert(pairs: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    if let Some(slot) = pairs.iter_mut().find(|(k, _)| k.same_key(&key)) {
        slot.1 = value;
    } else {
        pairs.push((key, value));
    }
}

fn core_tag_name(tag: &Tag) -> Option<&str> {
    if tag.handle == "tag:yaml.org,2002:" || tag.handle == "!!" {
        Some(tag.suffix.as_str())
    } else {
        None
    }
}

fn collection_tag(tag: Option<&Tag>, expected: &str) -> Result<(), String> {
    match tag {
        None => Ok(()),
        Some(t) if core_tag_name(t) == Some(expected) => Ok(()),
        Some(t) => Err(format!(
            "could not determine a constructor for the tag '{t}'"
        )),
    }
}

fn scalar(text: &str, style: ScalarStyle, tag: Option<&Tag>) -> Result<Value, String> {
    if let Some(t) = tag {
        // A non-specific `!` tag forces a string, as in PyYAML.
        if t.handle == "!" && t.suffix.is_empty() {
            return Ok(Value::Str(text.to_string()));
        }
        let Some(name) = core_tag_name(t) else {
            return Err(format!(
                "could not determine a constructor for the tag '{t}'"
            ));
        };
        return match name {
            "str" => Ok(Value::Str(text.to_string())),
            "null" => Ok(Value::Null),
            "bool" => bool_value(text)
                .map(Value::Bool)
                .ok_or_else(|| format!("invalid boolean '{text}'")),
            "int" => int_value(text),
            "float" => float_value(text).map(Value::Float),
            _ => Err(format!(
                "could not determine a constructor for the tag '{t}'"
            )),
        };
    }
    if style != ScalarStyle::Plain {
        return Ok(Value::Str(text.to_string()));
    }
    resolve_plain(text)
}

/// `PyYAML`'s implicit resolvers, in their registration order: bool, float, int, merge, null,
/// timestamp, value.
fn resolve_plain(s: &str) -> Result<Value, String> {
    if let Some(b) = bool_value(s) {
        return Ok(Value::Bool(b));
    }
    if is_float(s) {
        return float_value(s).map(Value::Float);
    }
    if is_int(s) {
        return int_value(s);
    }
    if s == "<<" {
        return Ok(Value::Str(MERGE_KEY.to_owned()));
    }
    if matches!(s, "" | "~" | "null" | "Null" | "NULL") {
        return Ok(Value::Null);
    }
    if is_timestamp(s) {
        return Ok(Value::Timestamp(s.to_owned()));
    }
    if s == "=" {
        return Err(
            "could not determine a constructor for the tag 'tag:yaml.org,2002:value'".to_owned(),
        );
    }
    Ok(Value::Str(s.to_owned()))
}

fn bool_value(s: &str) -> Option<bool> {
    match s {
        "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" | "on" | "On" | "ON" => Some(true),
        "no" | "No" | "NO" | "false" | "False" | "FALSE" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    }
}

/// Splits an optional leading sign.
fn unsign(s: &str) -> (bool, &str) {
    match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    }
}

fn all(s: &str, f: impl Fn(u8) -> bool) -> bool {
    s.bytes().all(f)
}

fn digit_us(c: u8) -> bool {
    c.is_ascii_digit() || c == b'_'
}

/// `[eE][-+][0-9]+` (`PyYAML` requires the exponent sign).
fn exponent(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3
        && matches!(b[0], b'e' | b'E')
        && matches!(b[1], b'-' | b'+')
        && all(&s[2..], |c| c.is_ascii_digit())
}

/// `[0-9][0-9_]*\.[0-9_]*` followed by an optional exponent, with `lead_digit` saying whether the
/// part before the point must start with a digit (true) or be empty (false).
fn decimal_with_point(s: &str, lead_digit: bool) -> bool {
    let Some(dot) = s.find('.') else { return false };
    let (int_part, rest) = (&s[..dot], &s[dot + 1..]);
    let int_ok = if lead_digit {
        int_part.as_bytes().first().is_some_and(u8::is_ascii_digit) && all(int_part, digit_us)
    } else {
        int_part.is_empty()
    };
    if !int_ok {
        return false;
    }
    let frac_end = rest.find(['e', 'E']).unwrap_or(rest.len());
    let (frac, exp) = (&rest[..frac_end], &rest[frac_end..]);
    if !lead_digit && !frac.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    all(frac, digit_us) && (exp.is_empty() || exponent(exp))
}

/// `[0-9][0-9_]*(:[0-5]?[0-9])+` (base 60), with `first` saying whether the first group may
/// start with 0.
fn sexagesimal(s: &str, first_may_be_zero: bool) -> bool {
    let mut parts = s.split(':');
    let Some(head) = parts.next() else {
        return false;
    };
    let head_ok = match head.as_bytes().first() {
        Some(b'0') => first_may_be_zero && all(head, digit_us),
        Some(c) if c.is_ascii_digit() => all(head, digit_us),
        _ => false,
    };
    let mut groups = 0;
    for p in parts {
        groups += 1;
        let ok = match p.as_bytes() {
            [d] => d.is_ascii_digit(),
            [a, b] => (b'0'..=b'5').contains(a) && b.is_ascii_digit(),
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    head_ok && groups > 0
}

fn is_float(s: &str) -> bool {
    let (_, body) = unsign(s);
    let signed = body.len() != s.len();
    if decimal_with_point(body, true) {
        return true;
    }
    if !signed && decimal_with_point(s, false) {
        return true;
    }
    if let Some((head, frac)) = body.rsplit_once('.')
        && sexagesimal(head, true)
        && all(frac, digit_us)
    {
        return true;
    }
    if matches!(body, ".inf" | ".Inf" | ".INF") {
        return true;
    }
    !signed && matches!(s, ".nan" | ".NaN" | ".NAN")
}

fn is_int(s: &str) -> bool {
    let (_, body) = unsign(s);
    if let Some(bin) = body.strip_prefix("0b") {
        return !bin.is_empty() && all(bin, |c| matches!(c, b'0' | b'1' | b'_'));
    }
    if let Some(hex) = body.strip_prefix("0x") {
        return !hex.is_empty() && all(hex, |c| c.is_ascii_hexdigit() || c == b'_');
    }
    if body == "0" {
        return true;
    }
    if let Some(oct) = body.strip_prefix('0') {
        return !oct.is_empty() && all(oct, |c| (b'0'..=b'7').contains(&c) || c == b'_');
    }
    if body
        .as_bytes()
        .first()
        .is_some_and(|c| (b'1'..=b'9').contains(c))
    {
        return all(body, digit_us) || sexagesimal(body, false);
    }
    false
}

#[allow(
    clippy::many_single_char_names,
    reason = "the names follow the timestamp's fields"
)]
fn is_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    let d = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    // Date only: YYYY-MM-DD.
    if b.len() == 10
        && (0..4).all(d)
        && b[4] == b'-'
        && d(5)
        && d(6)
        && b[7] == b'-'
        && d(8)
        && d(9)
    {
        return true;
    }
    // Date and time: YYYY-M[M]-D[D](T|t|whitespace)H[H]:MM:SS[.frac][tz]. Checked loosely: the
    // date part, a separator, and a time with two colons.
    let Some(sep) = s.find(['T', 't', ' ', '\t']) else {
        return false;
    };
    let (date, time) = (
        &s[..sep],
        s[sep..].trim_start_matches(['T', 't', ' ', '\t']),
    );
    let mut dp = date.split('-');
    let (Some(y), Some(m), Some(dd), None) = (dp.next(), dp.next(), dp.next(), dp.next()) else {
        return false;
    };
    let nd = |p: &str, lo: usize, hi: usize| {
        (lo..=hi).contains(&p.len()) && all(p, |c| c.is_ascii_digit())
    };
    if !(nd(y, 4, 4) && nd(m, 1, 2) && nd(dd, 1, 2)) {
        return false;
    }
    let mut tp = time.splitn(3, ':');
    let (Some(h), Some(mi), Some(rest)) = (tp.next(), tp.next(), tp.next()) else {
        return false;
    };
    nd(h, 1, 2)
        && nd(mi, 2, 2)
        && rest.len() >= 2
        && rest.as_bytes()[..2].iter().all(u8::is_ascii_digit)
}

/// `PyYAML`'s `construct_yaml_int`; magnitudes beyond `i128` become [`Value::BigInt`].
fn int_value(text: &str) -> Result<Value, String> {
    let cleaned: String = text.chars().filter(|&c| c != '_').collect();
    let (neg, body) = unsign(&cleaned);
    let big = || Value::BigInt {
        negative: neg,
        text: text.to_owned(),
    };
    let parsed: Option<i128> = if body == "0" {
        Some(0)
    } else if let Some(bin) = body.strip_prefix("0b") {
        i128::from_str_radix(bin, 2).ok()
    } else if let Some(hex) = body.strip_prefix("0x") {
        i128::from_str_radix(hex, 16).ok()
    } else if body.starts_with('0') {
        i128::from_str_radix(body, 8).ok()
    } else if body.contains(':') {
        let mut v: Option<i128> = Some(0);
        for part in body.split(':') {
            let p: i128 = part
                .parse()
                .map_err(|_| format!("invalid integer '{text}'"))?;
            v = v
                .and_then(|v| v.checked_mul(60))
                .and_then(|v| v.checked_add(p));
        }
        v
    } else {
        body.parse().ok()
    };
    Ok(match parsed {
        Some(m) => Value::Int(if neg { -m } else { m }),
        None => big(),
    })
}

/// `PyYAML`'s `construct_yaml_float`.
fn float_value(text: &str) -> Result<f64, String> {
    let cleaned: String = text
        .chars()
        .filter(|&c| c != '_')
        .collect::<String>()
        .to_lowercase();
    let (neg, body) = unsign(&cleaned);
    let sign = if neg { -1.0 } else { 1.0 };
    if body == ".inf" {
        return Ok(sign * f64::INFINITY);
    }
    if body == ".nan" {
        return Ok(f64::NAN);
    }
    if body.contains(':') {
        let mut v = 0.0;
        for part in body.split(':') {
            let p: f64 = part
                .parse()
                .map_err(|_| format!("invalid float '{text}'"))?;
            v = v * 60.0 + p;
        }
        return Ok(sign * v);
    }
    body.parse::<f64>()
        .map(|v| sign * v)
        .map_err(|_| format!("invalid float '{text}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> Value {
        match load(&format!("k: {text}")) {
            Ok(Value::Map(mut m)) => m.remove(0).1,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn yaml_1_1_scalars_resolve_like_pyyaml() {
        assert_eq!(one("yes"), Value::Bool(true));
        assert_eq!(one("Off"), Value::Bool(false));
        assert_eq!(one("1.0e9"), Value::Str("1.0e9".to_owned()));
        assert_eq!(one("1.0e+9"), Value::Float(1.0e9));
        assert_eq!(one("1e9"), Value::Str("1e9".to_owned()));
        assert_eq!(one("010"), Value::Int(8));
        assert_eq!(one("0x1F"), Value::Int(31));
        assert_eq!(one("1_000"), Value::Int(1000));
        assert_eq!(one("1:30"), Value::Int(90));
        assert_eq!(one(".5"), Value::Float(0.5));
        assert_eq!(one("-.inf"), Value::Float(f64::NEG_INFINITY));
        assert_eq!(one("~"), Value::Null);
        assert_eq!(one(""), Value::Null);
        assert_eq!(one("'yes'"), Value::Str("yes".to_owned()));
        assert_eq!(one("2026-10-16"), Value::Timestamp("2026-10-16".to_owned()));
        assert_eq!(one("!!str 5"), Value::Str("5".to_owned()));
        assert_eq!(one("09"), Value::Str("09".to_owned()));
    }

    #[test]
    fn repeated_keys_follow_python_dicts() {
        let v = load("a: 1\nb: 2\na: 3\n1: x\n1.0: y\n").ok();
        let want = Value::Map(vec![
            (Value::Str("a".into()), Value::Int(3)),
            (Value::Str("b".into()), Value::Int(2)),
            (Value::Int(1), Value::Str("y".into())),
        ]);
        assert_eq!(v, Some(want));
    }

    #[test]
    fn anchors_aliases_and_merges() {
        let v = load("base: &b {x: 1, y: 2}\nd:\n  <<: *b\n  y: 3\n").ok();
        let Some(Value::Map(m)) = v else {
            panic!("not a map")
        };
        let want = Value::Map(vec![
            (Value::Str("x".into()), Value::Int(1)),
            (Value::Str("y".into()), Value::Int(3)),
        ]);
        assert_eq!(m[1].1, want);
    }

    /// GAP-1 and DEF-1: accept and refuse exactly as `PyYAML` 6 does (observed 2026-10-16).
    #[test]
    fn pyyaml_strictness_is_reproduced() {
        let refused = [
            "a:\tx\n",
            "a:\t{b: 1}\n",
            "a: {b:\t1}\n",
            "a: [1,\t2]\n",
            "a: x\ty\n",
            "a: x\t\n",
            "a: x\t# c\n",
            "a:\n\tb: 1\n",
            "a: 1\n\t\nb: 2\n",
            "\t# c\na: 1\n",
            "a:\n  -\tx\n",
            "a: \tx\n",
            "a: x\n  \ty\n",
            "a: 1\t\n",
            "a: 'x'\t# c\n",
            "a: {b: c?d}\n",
            "a: [c?d]\n",
            ": v\n",
            "a:\n  : v\n",
            "a: {: v}\n",
        ];
        let accepted = [
            "a: x # c\tc\n",
            "a: 'x\ty'\n",
            "a: \"x\ty\"\n",
            "a: |\n  x\ty\n",
            "a: >\n  x\ty\n",
            "a: 'x\n  \ty'\n",
            "a: c?d\n",
            "a: ?x\n",
            "a: {b: c:d}\n",
            "a: [c:d]\n",
            "a: c:d\n",
            "a: [http://x]\n",
            "? \n: v\n",
            "~: v\n",
            "'': v\n",
            "a: {? b: c}\n",
            "a: b#c\n",
            "a: [b c, d]\n",
            "a: 'it''s'\t# fine? no\n",
        ];
        for t in refused {
            assert!(load(t).is_err(), "accepted {t:?}");
        }
        for t in &accepted[..accepted.len() - 1] {
            assert!(load(t).is_ok(), "refused {t:?}: {:?}", load(t));
        }
        assert!(
            load(accepted[accepted.len() - 1]).is_err(),
            "tab after a quoted scalar"
        );
        assert_eq!(load("\u{feff}a: 1\n").ok(), load("a: 1\n").ok());
    }

    #[test]
    fn integers_beyond_i128_are_kept_with_their_sign() {
        let v = one("-100000000000000000000000000000000000000000");
        assert!(matches!(v, Value::BigInt { negative: true, .. }), "{v:?}");
        assert_eq!(
            one("170141183460469231731687303715884105727"),
            Value::Int(i128::MAX)
        );
    }

    #[test]
    fn refusals() {
        assert!(load("a: 1\n---\nb: 2\n").is_err());
        assert!(load("? [a]\n: 1\n").is_err());
        assert!(load("a: !custom x\n").is_err());
        assert_eq!(load("").ok(), Some(Value::Null));
    }
}
