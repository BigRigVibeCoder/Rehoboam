//! Values and the arithmetic of §4.3: the `rh-core::fixed` rules, plus the integer rules the
//! language adds (half-to-even `int` division, Euclidean `div` and `mod`, `abs`, `floor`, `round`).
//!
//! Every result is computed exactly (in `i128`), rounded once half to even, then saturated. The
//! fixed-point operations are `rh_core::fixed`'s; the integer ones live here so `rh-core::fixed`
//! stays untouched (SPR-017 compliance table). There is no float anywhere.
//!
//! REF: `specs/registry/effect_language_v0.md` §3.1, §3.3, §3.4, §4.3, §6; SPR-010.

use core::cmp::Ordering;

use rh_core::fixed::{Q8_8, Q16_16};

use super::env::{EnumId, Env, KindId, Ty};

/// An event-typed value (§3.1, N3): the verb's own event or `none`. It never becomes an integer
/// inside the evaluator; the host maps it to an `EventRef` and the merge encodes `None` as −1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventArg {
    /// The verb's own event (its draft, rewritten at the merge).
    Own,
    /// No inciting event.
    None,
}

/// A value of the language.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Value {
    /// `bool`.
    Bool(bool),
    /// `int`.
    Int(i64),
    /// `q8_8`.
    Q8(Q8_8),
    /// `q16_16`.
    Q16(Q16_16),
    /// An enum value: its enum and ordinal.
    Enum(EnumId, u32),
    /// An entity: its kind and id (0 is `none`).
    Ent(KindId, u64),
    /// An event-typed value.
    Event(EventArg),
    /// A key tuple.
    Tuple(Vec<Value>),
    /// A list key.
    List(Vec<Value>),
}

/// The arithmetic of an evaluation: the one rounding rule, or a planted mutant of it (T-05).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Arith {
    /// The `mutant-effect` "fixed-point multiply rounds half up" (D1).
    pub mul_half_up: bool,
}

/// A binary arithmetic operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
}

/// `i64` saturation of an exact value.
pub(crate) fn sat_i64(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

fn sat_i16(v: i128) -> i16 {
    i16::try_from(v).unwrap_or(if v < 0 { i16::MIN } else { i16::MAX })
}

fn sat_i32(v: i128) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// `n / d` rounded half to even (`d != 0`), exact in `i128`.
pub(crate) fn div_half_even(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let floor = n.div_euclid(d);
    let twice = 2 * n.rem_euclid(d);
    if twice > d || (twice == d && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// `v / 2^shift` rounded half up: the planted D1 mutant only.
fn shr_half_up(v: i128, shift: u32) -> i128 {
    (v + (1_i128 << (shift - 1))) >> shift
}

/// The integer operators of §4.3. `Div`, `IntDiv` and `Mod` need `b != 0` (the checker's
/// guard); a zero divisor gives `None`.
pub(crate) fn arith_int(op: ArithOp, a: i64, b: i64) -> Option<i64> {
    let (a, b) = (i128::from(a), i128::from(b));
    Some(match op {
        ArithOp::Add => sat_i64(a + b),
        ArithOp::Sub => sat_i64(a - b),
        ArithOp::Mul => sat_i64(a * b),
        ArithOp::Div => {
            if b == 0 {
                return None;
            }
            sat_i64(div_half_even(a, b))
        }
        ArithOp::IntDiv | ArithOp::Mod => {
            if b == 0 {
                return None;
            }
            let remainder = a.rem_euclid(b.abs());
            if op == ArithOp::Mod {
                sat_i64(remainder)
            } else {
                sat_i64((a - remainder) / b)
            }
        }
    })
}

macro_rules! fixed_ops {
    ($fn:ident, $t:ty, $frac:expr, $sat:ident) => {
        fn $fn(op: ArithOp, a: $t, b: $t, arith: Arith) -> Option<$t> {
            Some(match op {
                ArithOp::Add => a.saturating_add(b),
                ArithOp::Sub => a.saturating_sub(b),
                ArithOp::Mul if arith.mul_half_up => {
                    let exact = i128::from(a.to_bits()) * i128::from(b.to_bits());
                    <$t>::from_bits($sat(shr_half_up(exact, $frac)))
                }
                ArithOp::Mul => a.saturating_mul(b),
                ArithOp::Div => a.checked_div(b)?,
                ArithOp::IntDiv | ArithOp::Mod => return None,
            })
        }
    };
}

fixed_ops!(arith_q8, Q8_8, 8, sat_i16);
fixed_ops!(arith_q16, Q16_16, 16, sat_i32);

/// A binary arithmetic operator on two values of one numeric type; `None` for a zero divisor or
/// mismatched operands (both excluded by the checker).
pub(crate) fn arith(op: ArithOp, a: &Value, b: &Value, mode: Arith) -> Option<Value> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => arith_int(op, *a, *b).map(Value::Int),
        (Value::Q8(a), Value::Q8(b)) => arith_q8(op, *a, *b, mode).map(Value::Q8),
        (Value::Q16(a), Value::Q16(b)) => arith_q16(op, *a, *b, mode).map(Value::Q16),
        _ => None,
    }
}

/// Saturating negation (`-MIN` is `MAX`).
pub(crate) fn negate(a: &Value) -> Option<Value> {
    Some(match a {
        Value::Int(v) => Value::Int(v.saturating_neg()),
        Value::Q8(v) => Value::Q8(v.saturating_neg()),
        Value::Q16(v) => Value::Q16(v.saturating_neg()),
        _ => return None,
    })
}

/// The integer that orders a numeric value: its bits.
pub(crate) fn numeric_key(a: &Value) -> i64 {
    match a {
        Value::Int(v) => *v,
        Value::Q8(v) => i64::from(v.to_bits()),
        Value::Q16(v) => i64::from(v.to_bits()),
        _ => 0,
    }
}

/// Whether a numeric value is zero.
pub(crate) fn is_zero(a: &Value) -> bool {
    numeric_key(a) == 0
}

/// A fixed-point value from an integer, saturating (`q8`, `q16`).
pub(crate) fn fixed_of_int(ty: &Ty, n: i64) -> Value {
    let n = i128::from(n);
    match ty {
        Ty::Q8 => Value::Q8(Q8_8::from_bits(sat_i16(n << 8))),
        _ => Value::Q16(Q16_16::from_bits(sat_i32(n << 16))),
    }
}

/// `floor`: toward negative infinity.
pub(crate) fn floor_fx(a: &Value) -> Option<i64> {
    match a {
        Value::Q8(v) => Some(i64::from(v.to_bits() >> 8)),
        Value::Q16(v) => Some(i64::from(v.to_bits() >> 16)),
        _ => None,
    }
}

/// `round`: nearest, half to even.
pub(crate) fn round_fx(a: &Value) -> Option<i64> {
    match a {
        Value::Q8(v) => Some(sat_i64(div_half_even(i128::from(v.to_bits()), 256))),
        Value::Q16(v) => Some(sat_i64(div_half_even(i128::from(v.to_bits()), 65_536))),
        _ => None,
    }
}

/// A fixed-point value of a type from exact bits, saturating (a table cell or a `neutral`
/// literal: `fx_from_fraction` of the reference).
#[must_use]
pub fn fixed_from_bits(ty: &Ty, bits: i128) -> Option<Value> {
    match ty {
        Ty::Q8 => Some(Value::Q8(Q8_8::from_bits(sat_i16(bits)))),
        Ty::Q16 => Some(Value::Q16(Q16_16::from_bits(sat_i32(bits)))),
        _ => None,
    }
}

/// Compiles a literal's text into the bits of `ty` (05.5 §3.2): the exact value, times
/// `2^FRAC_BITS`, rounded half to even with no float; `None` when it does not fit
/// (`E_LITERAL_RANGE`). An `int` takes the integer part.
pub(crate) fn literal_bits(ty: &Ty, text: &str) -> Option<i64> {
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let mut whole: u128 = 0;
    for d in int_part.bytes() {
        whole = whole
            .checked_mul(10)?
            .checked_add(u128::from(d.checked_sub(b'0')?))?;
        if whole > u128::from(u64::MAX) {
            return None;
        }
    }
    let (frac_bits, max) = match ty {
        Ty::Int => return i64::try_from(whole).ok(),
        Ty::Q8 => (8_u32, u128::from(i16::MAX.unsigned_abs())),
        Ty::Q16 => (16_u32, u128::from(i32::MAX.unsigned_abs())),
        _ => return None,
    };
    // The fraction's digits doubled frac_bits times: the carries out are floor(frac · 2^F), the
    // digits left are the remainder, compared with one half digit by digit.
    let mut digits: Vec<u8> = frac_part.bytes().map(|d| d.wrapping_sub(b'0')).collect();
    let mut carried: u128 = 0;
    for _ in 0..frac_bits {
        let mut carry = 0_u8;
        for d in digits.iter_mut().rev() {
            let v = *d * 2 + carry;
            *d = v % 10;
            carry = v / 10;
        }
        carried = carried * 2 + u128::from(carry);
    }
    let floor = (whole << frac_bits) + carried;
    let half = match digits.first() {
        None => Ordering::Less,
        Some(&first) if first > 5 => Ordering::Greater,
        Some(&first) if first < 5 => Ordering::Less,
        Some(_) => {
            if digits[1..].iter().any(|&d| d != 0) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    };
    let bits = match half {
        Ordering::Greater => floor + 1,
        Ordering::Equal => floor + (floor & 1),
        Ordering::Less => floor,
    };
    if bits > max {
        return None;
    }
    i64::try_from(bits).ok()
}

/// The value of compiled literal bits in a numeric type.
pub(crate) fn literal_value(ty: &Ty, bits: i64) -> Value {
    match ty {
        Ty::Q8 => Value::Q8(Q8_8::from_bits(sat_i16(i128::from(bits)))),
        Ty::Q16 => Value::Q16(Q16_16::from_bits(sat_i32(i128::from(bits)))),
        _ => Value::Int(bits),
    }
}

/// One element of a key's flat comparable form (§3.4): a list's end sorts below every scalar,
/// so a list that is a prefix of another sorts first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KeyAtom {
    /// The end of a list.
    End,
    /// A scalar, by its ordering integer.
    Scalar(i128),
}

/// The integer that orders a scalar inside a key: numbers by value, `false < true`, enums by
/// ordinal, entities by id.
pub(crate) fn scalar_int(v: &Value) -> i128 {
    match v {
        Value::Bool(b) => i128::from(*b),
        Value::Int(n) => i128::from(*n),
        Value::Q8(q) => i128::from(q.to_bits()),
        Value::Q16(q) => i128::from(q.to_bits()),
        Value::Enum(_, i) => i128::from(*i),
        Value::Ent(_, id) => i128::from(*id),
        _ => 0,
    }
}

/// The integer that orders a scalar (as [`scalar_int`]), for hosts that key rows by a column.
#[must_use]
pub fn scalar_int_pub(v: &Value) -> i128 {
    scalar_int(v)
}

/// Appends a key's flat form. Tuples have a fixed arity within one program, so flattening keeps
/// the lexicographic order; `prefix_last` is the §3.4 mutant (a list end above every scalar).
pub(crate) fn key_form(v: &Value, prefix_last: bool, out: &mut Vec<KeyAtom>) {
    match v {
        Value::Tuple(items) => {
            for i in items {
                key_form(i, prefix_last, out);
            }
        }
        Value::List(items) => {
            for i in items {
                key_form(i, prefix_last, out);
            }
            out.push(if prefix_last {
                KeyAtom::Scalar(i128::MAX)
            } else {
                KeyAtom::End
            });
        }
        scalar => out.push(KeyAtom::Scalar(scalar_int(scalar))),
    }
}

/// Renders a scalar in the golden file's text (`bool:true`, `q8_8:<bits>`, `person:none`, …).
#[must_use]
pub fn render_scalar(env: &Env, v: &Value) -> String {
    match v {
        Value::Bool(b) => format!("bool:{b}"),
        Value::Int(n) => format!("int:{n}"),
        Value::Q8(q) => format!("q8_8:{}", q.to_bits()),
        Value::Q16(q) => format!("q16_16:{}", q.to_bits()),
        Value::Enum(e, i) => {
            let def = env.enum_def(*e);
            let name = def.map_or("?", |d| d.name.as_str());
            let value = def
                .and_then(|d| usize::try_from(*i).ok().and_then(|i| d.values.get(i)))
                .map_or("?", String::as_str);
            format!("enum:{name}.{value}")
        }
        Value::Ent(k, 0) => format!("{}:none", env.kind_name(*k)),
        Value::Ent(k, id) => format!("{}:{id}", env.kind_name(*k)),
        Value::Event(EventArg::Own) => "event".into(),
        Value::Event(EventArg::None) => "0".into(),
        key => render_key(env, key),
    }
}

/// Renders a key: tuples in parentheses, lists in brackets.
#[must_use]
pub fn render_key(env: &Env, v: &Value) -> String {
    match v {
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(|i| render_scalar(env, i)).collect();
            format!("({})", parts.join(", "))
        }
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(|i| render_scalar(env, i)).collect();
            format!("[{}]", parts.join(", "))
        }
        scalar => render_scalar(env, scalar),
    }
}
