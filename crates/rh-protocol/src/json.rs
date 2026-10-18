//! Canonical JSON (`wire_v1.md` §3.1) and the control payload's parser.
//!
//! **Writing.** A control payload is `serde_json`'s compact output of a [`serde_json::Value`]
//! whose maps are sorted (the workspace builds `serde_json` without `preserve_order`): keys in
//! byte order, `,` and `:` as separators, integers only, strings of printable ASCII with `"` and
//! `\` escaped. [`canonical`] refuses a value outside the wire (a float, a non-ASCII string, a
//! nesting deeper than 4).
//!
//! **Reading.** [`parse`] reads the JSON grammar (any spelling: whitespace, another key order)
//! and refuses what `serde_json` alone would let through, with the reference's wording
//! (`specs/stream/selftest_wire_v1.py` `parse_json`), in the reference's order:
//!
//! - while parsing, in text order: a syntax error (`parse_error: JSONDecodeError`), a payload
//!   that is not UTF-8 (`parse_error: UnicodeDecodeError`), a nesting deeper than
//!   [`PARSE_DEPTH_LIMIT`] (`parse_error: RecursionError`), a float (`a float (1.5); the wire
//!   carries integers only`), and, at each object's end, a repeated key;
//! - then, depth first in text order: an integer outside `i64..u64`, a string outside printable
//!   ASCII (keys included: §3.1's "a non-ASCII string anywhere"), a container deeper than
//!   [`MAX_DEPTH`].
//!
//! The parser keeps its own recursion bounded, so 100 000 nested arrays is a refusal, not a stack
//! overflow.

use serde_json::{Map, Number, Value};

use crate::wire::{WireError, WireErrorKind};

/// The deepest a control payload may nest (the root is level 1).
pub const MAX_DEPTH: usize = 4;
/// The nesting at which the parser stops, as the reference's parser does at its recursion limit.
/// Between [`MAX_DEPTH`] and this, the payload parses and is refused by the depth check.
pub const PARSE_DEPTH_LIMIT: usize = 64;

/// A parsed JSON value before the wire's checks: integers of any size, objects in text order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum J {
    Null,
    Bool(bool),
    /// An integer; `None` when it does not fit even an `i128` (outside the wire in any case).
    Int(Option<i128>, String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

fn parse_error(what: &str) -> WireError {
    WireError::new(WireErrorKind::Parse, format!("parse_error: {what}"))
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, lit: &[u8]) -> Result<(), WireError> {
        if self.s.get(self.i..self.i + lit.len()) == Some(lit) {
            self.i += lit.len();
            Ok(())
        } else {
            Err(parse_error("JSONDecodeError"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<J, WireError> {
        self.ws();
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(J::Str),
            Some(b't') => self.eat(b"true").map(|()| J::Bool(true)),
            Some(b'f') => self.eat(b"false").map(|()| J::Bool(false)),
            Some(b'n') => self.eat(b"null").map(|()| J::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(parse_error("JSONDecodeError")),
        }
    }

    fn enter(depth: usize) -> Result<(), WireError> {
        if depth > PARSE_DEPTH_LIMIT {
            return Err(parse_error("RecursionError"));
        }
        Ok(())
    }

    fn object(&mut self, depth: usize) -> Result<J, WireError> {
        Self::enter(depth)?;
        self.i += 1;
        let mut pairs: Vec<(String, J)> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(J::Obj(pairs));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(parse_error("JSONDecodeError"));
            }
            let key = self.string()?;
            self.ws();
            self.eat(b":")?;
            let v = self.value(depth)?;
            pairs.push((key, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(parse_error("JSONDecodeError")),
            }
        }
        // The reference refuses a repeated key when the object closes (object_pairs_hook).
        for (n, (k, _)) in pairs.iter().enumerate() {
            if pairs[..n].iter().any(|(e, _)| e == k) {
                return Err(WireError::new(WireErrorKind::Request, "a repeated key"));
            }
        }
        Ok(J::Obj(pairs))
    }

    fn array(&mut self, depth: usize) -> Result<J, WireError> {
        Self::enter(depth)?;
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(J::Arr(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(parse_error("JSONDecodeError")),
            }
        }
        Ok(J::Arr(items))
    }

    fn hex4(&mut self) -> Result<u32, WireError> {
        let digits = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| parse_error("JSONDecodeError"))?;
        let text = std::str::from_utf8(digits).map_err(|_| parse_error("JSONDecodeError"))?;
        let v = u32::from_str_radix(text, 16).map_err(|_| parse_error("JSONDecodeError"))?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return Err(parse_error("JSONDecodeError"));
        }
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, WireError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&b) = self.s.get(self.i) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.i += 1;
            }
            // The payload was checked as UTF-8 before parsing, and the run ends at an ASCII byte.
            out.push_str(&String::from_utf8_lossy(&self.s[start..self.i]));
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let esc = self.peek().ok_or_else(|| parse_error("JSONDecodeError"))?;
                    self.i += 1;
                    let ch = match esc {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&hi)
                                && self.s.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                let save = self.i;
                                self.i += 2;
                                let lo = self.hex4()?;
                                if (0xdc00..0xe000).contains(&lo) {
                                    0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                                } else {
                                    self.i = save;
                                    hi
                                }
                            } else {
                                hi
                            };
                            // A lone surrogate is kept as U+FFFD: the string is refused anyway.
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return Err(parse_error("JSONDecodeError")),
                    };
                    out.push(ch);
                }
                // An unescaped control character, or the end of the payload.
                _ => return Err(parse_error("JSONDecodeError")),
            }
        }
    }

    fn number(&mut self) -> Result<J, WireError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while let Some(b'0'..=b'9') = self.peek() {
                    self.i += 1;
                }
            }
            _ => return Err(parse_error("JSONDecodeError")),
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            float = true;
            self.i += 1;
            let d = self.i;
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
            }
            if self.i == d {
                return Err(parse_error("JSONDecodeError"));
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            float = true;
            self.i += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.i += 1;
            }
            let d = self.i;
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
            }
            if self.i == d {
                return Err(parse_error("JSONDecodeError"));
            }
        }
        let text = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
        if float {
            return Err(WireError::new(
                WireErrorKind::Params,
                format!("a float ({text}); the wire carries integers only"),
            ));
        }
        Ok(J::Int(text.parse::<i128>().ok(), text))
    }
}

fn printable(s: &str) -> bool {
    s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

/// The reference's `_check_json`: integers within `i64..u64`, printable ASCII strings (and keys),
/// containers at most [`MAX_DEPTH`] deep.
fn check(v: &J, at: &str, depth: usize) -> Result<(), WireError> {
    let bad = |m: String| Err(WireError::new(WireErrorKind::Request, m));
    match v {
        J::Null | J::Bool(_) => Ok(()),
        J::Int(n, text) => match n {
            Some(n) if *n >= i128::from(i64::MIN) && *n <= i128::from(u64::MAX) => Ok(()),
            _ => bad(format!("{at}: {text} is outside i64..u64")),
        },
        J::Str(s) if printable(s) => Ok(()),
        J::Str(s) => bad(format!("{at}: a string outside printable ASCII ({s:?})")),
        J::Arr(_) | J::Obj(_) if depth > MAX_DEPTH => {
            bad(format!("{at}: nesting depth exceeds {MAX_DEPTH}"))
        }
        J::Arr(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(i, x)| check(x, &format!("{at}[{i}]"), depth + 1)),
        J::Obj(pairs) => pairs.iter().try_for_each(|(k, x)| {
            if !printable(k) {
                return bad(format!("{at}: a key outside printable ASCII ({k:?})"));
            }
            check(x, &format!("{at}.{k}"), depth + 1)
        }),
    }
}

fn to_value(v: J) -> Value {
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(b),
        J::Int(n, _) => {
            let n = n.unwrap_or_default();
            u64::try_from(n).map_or_else(
                |_| Value::Number(Number::from(i64::try_from(n).unwrap_or_default())),
                |u| Value::Number(Number::from(u)),
            )
        }
        J::Str(s) => Value::String(s),
        J::Arr(items) => Value::Array(items.into_iter().map(to_value).collect()),
        J::Obj(pairs) => {
            let mut m = Map::new();
            for (k, x) in pairs {
                m.insert(k, to_value(x));
            }
            Value::Object(m)
        }
    }
}

/// Parses a control payload: JSON with no float, no repeated key, integers within `i64..u64`,
/// printable ASCII strings and keys, nesting at most [`MAX_DEPTH`] (module docs).
///
/// # Errors
/// [`WireError`] of kind `Parse`, `Request` or `Params`, worded as the reference words it.
pub fn parse(payload: &[u8]) -> Result<Value, WireError> {
    if std::str::from_utf8(payload).is_err() {
        return Err(parse_error("UnicodeDecodeError"));
    }
    let mut p = Parser { s: payload, i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i != payload.len() {
        return Err(parse_error("JSONDecodeError"));
    }
    check(&v, "payload", 1)?;
    Ok(to_value(v))
}

fn check_value(v: &Value, at: &str, depth: usize) -> Result<(), WireError> {
    let bad = |m: String| Err(WireError::new(WireErrorKind::Request, m));
    match v {
        Value::Null | Value::Bool(_) => Ok(()),
        Value::Number(n) if n.is_i64() || n.is_u64() => Ok(()),
        Value::Number(n) => Err(WireError::new(
            WireErrorKind::Params,
            format!("{at}: a float ({n}); the wire carries integers only"),
        )),
        Value::String(s) if printable(s) => Ok(()),
        Value::String(s) => bad(format!("{at}: a string outside printable ASCII ({s:?})")),
        Value::Array(_) | Value::Object(_) if depth > MAX_DEPTH => {
            bad(format!("{at}: nesting depth exceeds {MAX_DEPTH}"))
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(i, x)| check_value(x, &format!("{at}[{i}]"), depth + 1)),
        Value::Object(m) => m.iter().try_for_each(|(k, x)| {
            if !printable(k) {
                return bad(format!("{at}: a key outside printable ASCII ({k:?})"));
            }
            check_value(x, &format!("{at}.{k}"), depth + 1)
        }),
    }
}

/// The canonical JSON bytes of `value` (§3.1).
///
/// # Errors
/// [`WireError`] if the value is outside the wire (a float, a non-ASCII string, too deep).
pub fn canonical(value: &Value) -> Result<Vec<u8>, WireError> {
    check_value(value, "value", 1)?;
    serde_json::to_vec(value).map_err(|e| WireError::new(WireErrorKind::Request, e.to_string()))
}
