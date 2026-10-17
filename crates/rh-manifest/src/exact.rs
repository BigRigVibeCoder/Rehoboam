//! Authored numbers as exact decimals, and their compilation into fixed point (`05.5` §3.2).
//!
//! The loaders read a number twice: as the `f64` the Python reference validates (the load-time
//! gate), and as the exact decimal value of its literal ([`Decimal`]). Only the second reaches
//! plane A: [`Decimal::to_fixed`] rounds `value · 2^F` half to even (the `rh_core::fixed` rule)
//! from the literal's digits, never through an `f64`, and a value that does not fit is refused.
//!
//! **How the rounding is exact.** A literal is a coefficient `C` (decimal digits) and an exponent
//! `E`, `value = ±C · 10^E`. For `E < 0`, `C · 2^F` is computed by doubling its decimal digits `F`
//! times; the result keeps exactly `-E` fraction digits, so its integer part is `floor(value · 2^F)`
//! and its fraction digits are compared with one half digit by digit. Half to even is symmetric in
//! the sign, so a negative value rounds as its magnitude does. No division, no float.
//!
//! Literal forms: `PyYAML` floats (`1_000.5`, `.5`, `6.`, `1.0e+9`, sexagesimal `1:30.5`, `.inf`,
//! `.nan`), Python's `float(str)` for strings Pydantic coerces (`"30.5"`, `"1e3"`, `"inf"`), and the
//! `FromStr` form `toml` gives for a TOML float. Infinities and NaN are not decimals: they are
//! refused by name.
//!
//! REF: `docs/05_architecture/05.5_world_compiler_and_dsl.md` §3.2; SPR-011 T-04, ruling 9.

use std::fmt;

/// A decimal number exactly as written: `value = (-1)^negative · coefficient · 10^exponent`.
///
/// The coefficient is its decimal digits, most significant first, without leading zeros (empty for
/// zero, which is never negative). Two literals of one value compare equal (`0.50` = `.5`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    negative: bool,
    coefficient: Vec<u8>,
    exponent: i64,
}

/// Why a literal is not an exact decimal, or does not fit a fixed-point type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecimalError {
    /// Not a number in any accepted form.
    Syntax(String),
    /// An infinity or NaN: a float with no decimal value.
    NotFinite(String),
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(text) => write!(f, "`{text}` is not a decimal number"),
            Self::NotFinite(text) => write!(f, "`{text}` is not finite"),
        }
    }
}

impl std::error::Error for DecimalError {}

/// Exponents past this many digits from the coefficient make a value certainly out of range (above)
/// or certainly below half a step of any type here (below); they are clamped, never materialised.
const EXPONENT_CLAMP: i64 = 1 << 20;

impl Decimal {
    /// Zero.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            negative: false,
            coefficient: Vec::new(),
            exponent: 0,
        }
    }

    /// The exact value of an integer.
    #[must_use]
    pub fn from_int(value: i128) -> Self {
        let digits = value.unsigned_abs().to_string().into_bytes();
        Self::normalised(value < 0, digits.iter().map(|d| d - b'0').collect(), 0)
    }

    /// Strips leading zeros (and zero's sign), and moves trailing zeros into the exponent.
    fn normalised(negative: bool, mut coefficient: Vec<u8>, mut exponent: i64) -> Self {
        let lead = coefficient.iter().take_while(|&&d| d == 0).count();
        coefficient.drain(..lead);
        while coefficient.last() == Some(&0) {
            coefficient.pop();
            exponent = exponent.saturating_add(1);
        }
        if coefficient.is_empty() {
            return Self::zero();
        }
        Self {
            negative,
            coefficient,
            exponent: exponent.clamp(-EXPONENT_CLAMP, EXPONENT_CLAMP),
        }
    }

    /// Parses `[+-]? digits [. digits] [(e|E) [+-]? digits]`, with at least one digit in the
    /// mantissa. `_` is not accepted here: callers remove it where their grammar allows it.
    fn parse_plain(text: &str, original: &str) -> Result<Self, DecimalError> {
        let syntax = || DecimalError::Syntax(original.to_owned());
        let (negative, body) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let (mantissa, exp) = match body.find(['e', 'E']) {
            Some(i) => (&body[..i], Some(&body[i + 1..])),
            None => (body, None),
        };
        let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if int.is_empty() && frac.is_empty()
            || !int.bytes().chain(frac.bytes()).all(|c| c.is_ascii_digit())
        {
            return Err(syntax());
        }
        let exponent = match exp {
            None => 0,
            Some(e) => {
                let (neg, digits) = match e.as_bytes().first() {
                    Some(b'-') => (true, &e[1..]),
                    Some(b'+') => (false, &e[1..]),
                    _ => (false, e),
                };
                if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(syntax());
                }
                // Saturating: past the clamp the value is out of range or below half a step.
                let magnitude = digits.bytes().fold(0_i64, |acc, d| {
                    acc.saturating_mul(10)
                        .saturating_add(i64::from(d - b'0'))
                        .min(EXPONENT_CLAMP * 4)
                });
                if neg { -magnitude } else { magnitude }
            }
        };
        let coefficient = int.bytes().chain(frac.bytes()).map(|d| d - b'0').collect();
        let frac_len = i64::try_from(frac.len()).map_err(|_| syntax())?;
        Ok(Self::normalised(negative, coefficient, exponent - frac_len))
    }

    /// The exact value of a `PyYAML` float literal (the text of a scalar resolved or tagged as a
    /// float): underscores dropped, case folded, base-60 groups (`1:30.5` = 90.5) summed exactly.
    ///
    /// # Errors
    /// [`DecimalError::NotFinite`] for `.inf` and `.nan`; [`DecimalError::Syntax`] otherwise.
    pub fn parse_yaml_float(text: &str) -> Result<Self, DecimalError> {
        let cleaned: String = text
            .chars()
            .filter(|&c| c != '_')
            .collect::<String>()
            .to_lowercase();
        let body = cleaned.trim_start_matches(['+', '-']);
        if matches!(body, ".inf" | ".nan") {
            return Err(DecimalError::NotFinite(text.to_owned()));
        }
        if !body.contains(':') {
            return Self::parse_plain(&cleaned, text);
        }
        // Sexagesimal: integer groups, the last of which may carry the fraction.
        let negative = cleaned.starts_with('-');
        let syntax = || DecimalError::Syntax(text.to_owned());
        let groups: Vec<&str> = body.split(':').collect();
        let (last, heads) = groups.split_last().ok_or_else(syntax)?;
        let (seconds, frac) = last.split_once('.').unwrap_or((last, ""));
        let mut whole: i128 = 0;
        for group in heads.iter().chain(std::iter::once(&seconds)) {
            let g: i128 = group.parse().map_err(|_| syntax())?;
            whole = whole
                .checked_mul(60)
                .and_then(|w| w.checked_add(g))
                .ok_or_else(syntax)?;
        }
        let sign = if negative { "-" } else { "" };
        Self::parse_plain(&format!("{sign}{whole}.{frac}0"), text)
    }

    /// The exact value of a string as Python's `float(str)` reads it (what Pydantic coerces a
    /// string field with): surrounding whitespace, `_` between digits, `inf`/`nan` refused.
    ///
    /// # Errors
    /// [`DecimalError::NotFinite`] for an infinity or NaN; [`DecimalError::Syntax`] otherwise.
    pub fn parse_python_float(text: &str) -> Result<Self, DecimalError> {
        let trimmed = text.trim();
        let lower = trimmed.to_lowercase();
        if matches!(
            lower.trim_start_matches(['+', '-']),
            "inf" | "infinity" | "nan"
        ) {
            return Err(DecimalError::NotFinite(text.to_owned()));
        }
        let bytes = trimmed.as_bytes();
        let underscores_ok = bytes.iter().enumerate().all(|(i, &c)| {
            c != b'_'
                || (i > 0
                    && i + 1 < bytes.len()
                    && bytes[i - 1].is_ascii_digit()
                    && bytes[i + 1].is_ascii_digit())
        });
        if !underscores_ok {
            return Err(DecimalError::Syntax(text.to_owned()));
        }
        Self::parse_plain(&trimmed.replace('_', ""), text)
    }

    /// The exact value of a TOML float, from the `FromStr` text `toml` keeps for it.
    ///
    /// # Errors
    /// [`DecimalError::NotFinite`] for `inf` and `nan`; [`DecimalError::Syntax`] otherwise.
    pub fn parse_toml_float(text: &str) -> Result<Self, DecimalError> {
        if matches!(text.trim_start_matches(['+', '-']), "inf" | "nan") {
            return Err(DecimalError::NotFinite(text.to_owned()));
        }
        Self::parse_plain(&text.replace('_', ""), text)
    }

    /// Whether the value is negative (zero never is).
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// `value · 2^frac_bits` rounded half to even, or `None` when its magnitude is at least
    /// `2^126` (no fixed-point type here comes near; the caller then checks its own range).
    #[must_use]
    pub fn to_fixed(&self, frac_bits: u32) -> Option<i128> {
        if self.coefficient.is_empty() {
            return Some(0);
        }
        let magnitude = if self.exponent >= 0 {
            // An integer: C · 10^E · 2^F, refused as soon as it passes 2^126.
            let limit = 1_i128 << 126;
            let mut v: i128 = 0;
            for &d in &self.coefficient {
                v = v.checked_mul(10)?.checked_add(i128::from(d))?;
            }
            for _ in 0..self.exponent {
                v = v.checked_mul(10).filter(|&x| x < limit)?;
            }
            v.checked_mul(1_i128 << frac_bits).filter(|&x| x < limit)?
        } else {
            let fraction_digits = usize::try_from(-self.exponent).ok()?;
            let scaled = double_digits(&self.coefficient, frac_bits);
            let split = scaled.len().saturating_sub(fraction_digits);
            let (int, frac) = scaled.split_at(split);
            let mut floor: i128 = 0;
            for &d in int {
                floor = floor.checked_mul(10)?.checked_add(i128::from(d))?;
            }
            if floor >= 1 << 126 {
                return None;
            }
            // The fraction has `fraction_digits` digits, of which the leading ones beyond
            // `scaled.len()` are zeros: then it is below one half.
            let half = if fraction_digits > scaled.len() {
                core::cmp::Ordering::Less
            } else {
                compare_with_half(frac)
            };
            let up = match half {
                core::cmp::Ordering::Greater => true,
                core::cmp::Ordering::Equal => floor & 1 == 1,
                core::cmp::Ordering::Less => false,
            };
            floor + i128::from(up)
        };
        Some(if self.negative { -magnitude } else { magnitude })
    }
}

impl fmt::Display for Decimal {
    /// The value in plain decimal notation (`40000`, `-0.25`) when its exponent is within 40 of
    /// the digits, else `±C e E`; for messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.coefficient.is_empty() {
            return f.write_str("0");
        }
        let digits: String = self
            .coefficient
            .iter()
            .map(|d| char::from(b'0' + d))
            .collect();
        let sign = if self.negative { "-" } else { "" };
        let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        match self.exponent {
            0 => write!(f, "{sign}{digits}"),
            e @ 1..=40 => write!(f, "{sign}{digits}{}", "0".repeat(small(e))),
            e if e < 0 && -e < len => {
                let (int, frac) = digits.split_at(small(len + e));
                write!(f, "{sign}{int}.{frac}")
            }
            e if e < 0 && -e <= len + 40 => {
                let zeros = "0".repeat(small(-e - len));
                write!(f, "{sign}0.{zeros}{digits}")
            }
            e => write!(f, "{sign}{digits}e{e}"),
        }
    }
}

/// A count from 0 to a few hundred, for [`Decimal`]'s display.
fn small(n: i64) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// `digits · 2^times` as decimal digits, most significant first.
fn double_digits(digits: &[u8], times: u32) -> Vec<u8> {
    let mut little: Vec<u8> = digits.iter().rev().copied().collect();
    for _ in 0..times {
        let mut carry = 0;
        for d in &mut little {
            let v = *d * 2 + carry;
            *d = v % 10;
            carry = v / 10;
        }
        if carry > 0 {
            little.push(carry);
        }
    }
    little.reverse();
    little
}

/// Compares the fraction `0.d1 d2 d3 …` with one half.
fn compare_with_half(frac: &[u8]) -> core::cmp::Ordering {
    match frac.first() {
        None => core::cmp::Ordering::Less,
        Some(&d) if d > 5 => core::cmp::Ordering::Greater,
        Some(&d) if d < 5 => core::cmp::Ordering::Less,
        Some(_) if frac[1..].iter().any(|&d| d != 0) => core::cmp::Ordering::Greater,
        Some(_) => core::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::{Decimal, DecimalError};

    fn q16(text: &str) -> Option<i128> {
        Decimal::parse_yaml_float(text).ok()?.to_fixed(16)
    }

    #[test]
    fn exact_values_round_half_to_even() {
        assert_eq!(q16("1.0"), Some(65_536));
        assert_eq!(q16("0.5"), Some(32_768));
        // 2^-17 is half a Q16_16 step: ties go to even.
        assert_eq!(q16("0.0000076293945312500"), Some(0));
        assert_eq!(q16("0.0000228881835937500"), Some(2)); // 3 · 2^-17: 1.5 steps -> 2
        assert_eq!(q16("0.00000762939453125001"), Some(1)); // just above the tie
        assert_eq!(q16("-0.0000228881835937500"), Some(-2));
        // 0.1 · 2^16 = 6553.6 exactly: 6554, where f64's 0.1 would be the same here but not in
        // general; the literal decides, not the float.
        assert_eq!(q16("0.1"), Some(6_554));
        assert_eq!(q16("30"), Some(30 << 16));
        assert_eq!(Decimal::from_int(30).to_fixed(16), Some(30 << 16));
    }

    #[test]
    fn literal_forms_parse_to_one_value() {
        let half = Decimal::parse_yaml_float("0.5").ok();
        for text in [".5", "0.50", "5_0.0e-2", "+.5", "0.5e+0"] {
            assert_eq!(Decimal::parse_yaml_float(text).ok(), half, "{text}");
        }
        assert_eq!(
            Decimal::parse_yaml_float("1:30.5").ok(),
            Decimal::parse_yaml_float("90.5").ok()
        );
        assert_eq!(
            Decimal::parse_yaml_float("-1:00:00.25").ok(),
            Decimal::parse_yaml_float("-3600.25").ok()
        );
        assert_eq!(
            Decimal::parse_python_float(" 1_000.5 ").ok(),
            Decimal::parse_yaml_float("1000.5").ok()
        );
        assert_eq!(
            Decimal::parse_toml_float("1e9").ok(),
            Some(Decimal::from_int(1_000_000_000))
        );
        assert_eq!(
            Decimal::parse_yaml_float("6.").ok(),
            Some(Decimal::from_int(6))
        );
    }

    #[test]
    fn infinities_and_garbage_are_refused_by_name() {
        for text in [".inf", "-.Inf", ".NaN"] {
            assert!(matches!(
                Decimal::parse_yaml_float(text),
                Err(DecimalError::NotFinite(_))
            ));
        }
        for text in ["inf", "-Infinity", "nan"] {
            assert!(matches!(
                Decimal::parse_python_float(text),
                Err(DecimalError::NotFinite(_))
            ));
        }
        for text in ["", ".", "e5", "1e", "1.2.3", "0x10", "1__0", "_1"] {
            assert!(Decimal::parse_python_float(text).is_err(), "{text}");
        }
    }

    #[test]
    fn display_is_plain_decimal() {
        let shown = |t: &str| Decimal::parse_python_float(t).map(|d| d.to_string());
        assert_eq!(shown("4e4"), Ok("40000".to_owned()));
        assert_eq!(shown("-0.250"), Ok("-0.25".to_owned()));
        assert_eq!(shown("0.001"), Ok("0.001".to_owned()));
        assert_eq!(shown("12.5"), Ok("12.5".to_owned()));
        assert_eq!(shown("1e-99"), Ok("1e-99".to_owned()));
    }

    #[test]
    fn huge_and_tiny_exponents_never_materialise() {
        assert_eq!(
            Decimal::parse_toml_float("1e999999999999").map(|d| d.to_fixed(16)),
            Ok(None)
        );
        assert_eq!(
            Decimal::parse_toml_float("1e-999999999999").map(|d| d.to_fixed(32)),
            Ok(Some(0))
        );
        assert_eq!(
            Decimal::parse_toml_float("-0.0").map(|d| (d.is_negative(), d.to_fixed(16))),
            Ok((false, Some(0)))
        );
    }
}
