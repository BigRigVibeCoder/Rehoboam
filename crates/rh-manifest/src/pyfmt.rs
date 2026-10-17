//! Python-compatible text formatting for diagnostics.
//!
//! The Rust loader must say what the Python reference (`specs/manifest/manifest_v0.py`) says, so
//! numbers and lists in messages are rendered the way Python renders them: `repr(float)`,
//! `f"{n:,}"`, `repr(list[str])`. Fixed-precision floats (`f"{x:.2f}"`) need no helper: Rust's
//! `{:.2}` rounds the exact binary value half to even, as Python does.

use std::fmt::Write as _;

/// Python's `repr(x)` for a float: the shortest round-trip digits, positional between 1e-4 and
/// 1e16, otherwise scientific with a signed, at-least-two-digit exponent (`1e+16`, `1.5e-07`).
#[must_use]
pub fn float_repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0" } else { "0.0" }.to_owned();
    }
    // `{:e}` gives the shortest round-trip mantissa: "-1.25e-7", "1e9".
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let sign = if x < 0.0 { "-" } else { "" };
    if (-4..16).contains(&exp) {
        let n = i32::try_from(digits.len()).unwrap_or(i32::MAX);
        let body = if exp < 0 {
            let zeros = usize::try_from(-exp - 1).unwrap_or(0);
            format!("0.{}{digits}", "0".repeat(zeros))
        } else if exp + 1 >= n {
            let zeros = usize::try_from(exp + 1 - n).unwrap_or(0);
            format!("{digits}{}.0", "0".repeat(zeros))
        } else {
            let point = usize::try_from(exp + 1).unwrap_or(0);
            format!("{}.{}", &digits[..point], &digits[point..])
        };
        format!("{sign}{body}")
    } else {
        let tail = &digits[1..];
        let mantissa = if tail.is_empty() {
            digits[..1].to_owned()
        } else {
            format!("{}.{tail}", &digits[..1])
        };
        let esign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{mantissa}e{esign}{:02}", exp.abs())
    }
}

/// Python's `f"{n:,}"`: an integer with comma thousands separators.
#[must_use]
pub fn int_grouped(n: i128) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Python's `repr(s)` for a string: single quotes unless the text holds a single quote and no
/// double quote; backslashes and control characters escaped.
#[must_use]
pub fn str_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Python's `repr(list[str])`: `['F0', 'F1']`.
#[must_use]
pub fn str_list_repr<S: AsRef<str>>(items: &[S]) -> String {
    let inner: Vec<String> = items.iter().map(|s| str_repr(s.as_ref())).collect();
    format!("[{}]", inner.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_repr_matches_python() {
        let cases = [
            (12.0, "12.0"),
            (0.5, "0.5"),
            (1.0e9, "1000000000.0"),
            (1.0e16, "1e+16"),
            (1.5e16, "1.5e+16"),
            (1.5e-7, "1.5e-07"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (-2.1, "-2.1"),
            (123_456.789, "123456.789"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::INFINITY, "inf"),
            (-0.0, "-0.0"),
        ];
        for (x, want) in cases {
            assert_eq!(float_repr(x), want, "repr({x})");
        }
    }

    #[test]
    fn grouping_and_lists() {
        assert_eq!(int_grouped(1_000_000), "1,000,000");
        assert_eq!(int_grouped(999), "999");
        assert_eq!(int_grouped(-1234), "-1,234");
        assert_eq!(str_list_repr(&["F0", "F1"]), "['F0', 'F1']");
        assert_eq!(str_repr("it's"), "\"it's\"");
    }
}
