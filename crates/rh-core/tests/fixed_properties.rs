//! Property tests for `rh_core::fixed` (SPR-010 T-02; GOV-002 §2 tier 3).
//!
//! Inputs come from a `SplitMix64` with fixed seeds, plus the edge values of each type, so every run
//! checks the same cases (a flaky determinism test is a determinism leak). `proptest` is not used:
//! it depends on `rand`, which `bin/lint_arch.py` forbids in plane-A crates even as a
//! dev-dependency (rng-crate rule).
//!
//! The rounding properties check the *definition* of round half to even (the result is within half
//! a step of the exact value, and an exact tie goes to the even neighbour) with integer arithmetic,
//! never by running the implementation's algorithm again.
//!
//! Refs: V-DET-19 (DET-3); `05.5` §2.1.
#![allow(
    clippy::expect_used,
    reason = "test helpers: an impossible input must fail the test loudly"
)]
#![allow(
    clippy::many_single_char_names,
    reason = "algebraic properties read best as a, b, c"
)]

use rh_core::fixed::{Money, Q8_8, Q16_16};

const CASES: usize = 200_000;

/// `SplitMix64` (Steele, Lea and Flood 2014): the same input generator as `bin/gen_fixed_vectors.py`.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A `Q16_16` that is either anywhere in range or within about +-4 (where products do not
    /// all saturate), alternating.
    fn q16(&mut self) -> Q16_16 {
        let raw = self.next_u64();
        let shift = if raw & 1 == 0 { 32 } else { 45 };
        Q16_16::from_bits(i32::try_from((raw.cast_signed()) >> shift).expect("fits"))
    }

    fn q8(&mut self) -> Q8_8 {
        let raw = self.next_u64();
        let shift = if raw & 1 == 0 { 48 } else { 53 };
        Q8_8::from_bits(i16::try_from((raw.cast_signed()) >> shift).expect("fits"))
    }

    fn money(&mut self) -> Money {
        let raw = self.next_u64().cast_signed();
        Money::from_minor(if raw & 1 == 0 { raw } else { raw >> 24 })
    }
}

fn q16_edges() -> Vec<Q16_16> {
    [
        i32::MIN,
        i32::MIN + 1,
        -65_537,
        -65_536,
        -32_768,
        -3,
        -1,
        0,
        1,
        3,
        32_768,
        65_536,
        i32::MAX,
    ]
    .into_iter()
    .map(Q16_16::from_bits)
    .collect()
}

/// Checks that `result` is `exact / 2^frac` rounded half to even, or its saturation.
///
/// `exact` is the exact numerator; the exact value of the result, in bits, is `exact / 2^frac`.
fn assert_rounded(exact: i128, scale: i128, result: i128, lo: i128, hi: i128, what: &str) {
    // The ideal (unclamped) result k satisfies |exact - k * scale| <= scale / 2, ties to even.
    if result == hi && exact >= hi * scale {
        return; // saturated high: the exact value is at or above MAX
    }
    if result == lo && exact <= lo * scale {
        return; // saturated low
    }
    let error = 2 * (exact - result * scale).abs(); // twice the rounding error, in 1/scale steps
    assert!(
        error <= scale.abs(),
        "{what}: {result} is not nearest to {exact}/{scale}"
    );
    if error == scale.abs() {
        assert_eq!(
            result % 2,
            0,
            "{what}: tie {exact}/{scale} rounded to odd {result}"
        );
    }
}

#[test]
fn addition_and_multiplication_commute_and_have_identities() {
    let mut rng = SplitMix64(1);
    for _ in 0..CASES {
        let (a, b) = (rng.q16(), rng.q16());
        assert_eq!(a + b, b + a);
        assert_eq!(a * b, b * a);
        assert_eq!(a + Q16_16::ZERO, a);
        assert_eq!(a - Q16_16::ZERO, a);
        assert_eq!(a * Q16_16::ONE, a);
        assert_eq!(a * Q16_16::ZERO, Q16_16::ZERO);
        assert_eq!(a.checked_div(Q16_16::ONE), Some(a));
        let (c, d) = (rng.q8(), rng.q8());
        assert_eq!(c + d, d + c);
        assert_eq!(c * d, d * c);
        assert_eq!(c * Q8_8::ONE, c);
        assert_eq!(c + Q8_8::ZERO, c);
        let (m, n) = (rng.money(), rng.money());
        assert_eq!(m + n, n + m);
        assert_eq!(m + Money::ZERO, m);
        assert_eq!(m.mul_rate(Q16_16::ONE), m);
        assert_eq!(m.saturating_mul_int(1), m);
        assert_eq!(m.checked_div_int(1), Some(m));
    }
}

#[test]
fn addition_and_subtraction_saturate_at_both_bounds() {
    let mut rng = SplitMix64(2);
    let clamp = |v: i128| v.clamp(i32::MIN.into(), i32::MAX.into());
    for _ in 0..CASES {
        let (a, b) = (rng.q16(), rng.q16());
        let (x, y) = (i128::from(a.to_bits()), i128::from(b.to_bits()));
        assert_eq!(i128::from((a + b).to_bits()), clamp(x + y));
        assert_eq!(i128::from((a - b).to_bits()), clamp(x - y));
    }
    assert_eq!(Q16_16::MAX + Q16_16::EPSILON, Q16_16::MAX);
    assert_eq!(Q16_16::MIN - Q16_16::EPSILON, Q16_16::MIN);
    assert_eq!(Q8_8::MAX + Q8_8::MAX, Q8_8::MAX);
    assert_eq!(Q8_8::MIN + Q8_8::MIN, Q8_8::MIN);
    assert_eq!(Q16_16::MAX * Q16_16::from_int(2), Q16_16::MAX);
    assert_eq!(Q16_16::MIN * Q16_16::from_int(2), Q16_16::MIN);
    assert_eq!(Q16_16::MIN * Q16_16::MIN, Q16_16::MAX);
    assert_eq!(-Q16_16::MIN, Q16_16::MAX);
    assert_eq!(Money::MAX + Money::MAX, Money::MAX);
    assert_eq!(Money::MIN - Money::MAX, Money::MIN);
    assert_eq!(Money::MIN.mul_rate(Q16_16::from_int(3)), Money::MIN);
}

#[test]
fn q16_multiplication_and_division_round_half_to_even() {
    let mut rng = SplitMix64(3);
    let (lo, hi) = (i128::from(i32::MIN), i128::from(i32::MAX));
    let edges = q16_edges();
    let mut pairs: Vec<(Q16_16, Q16_16)> = (0..CASES).map(|_| (rng.q16(), rng.q16())).collect();
    pairs.extend(
        edges
            .iter()
            .flat_map(|&a| edges.iter().map(move |&b| (a, b))),
    );
    for (a, b) in pairs {
        let (x, y) = (i128::from(a.to_bits()), i128::from(b.to_bits()));
        let product = i128::from((a * b).to_bits());
        assert_rounded(x * y, 1 << 16, product, lo, hi, "q16 mul");
        match a.checked_div(b) {
            None => assert_eq!(y, 0, "q16 div: None for a non-zero divisor"),
            Some(q) => {
                let (num, den) = if y < 0 {
                    (-(x << 16), -y)
                } else {
                    (x << 16, y)
                };
                assert_rounded(num, den, i128::from(q.to_bits()), lo, hi, "q16 div");
            }
        }
    }
}

#[test]
fn q8_multiplication_rounds_half_to_even_for_every_left_operand() {
    // Exhaustive in the left operand (all 65,536 values) against the edges and random right operands.
    let mut rng = SplitMix64(4);
    let (lo, hi) = (i128::from(i16::MIN), i128::from(i16::MAX));
    let mut rights: Vec<i16> = vec![
        i16::MIN,
        -257,
        -256,
        -128,
        -3,
        -1,
        0,
        1,
        3,
        128,
        256,
        i16::MAX,
    ];
    rights.extend((0..12).map(|_| rng.q8().to_bits()));
    for left in i16::MIN..=i16::MAX {
        for &right in &rights {
            let (a, b) = (Q8_8::from_bits(left), Q8_8::from_bits(right));
            let exact = i128::from(left) * i128::from(right);
            assert_rounded(
                exact,
                1 << 8,
                i128::from((a * b).to_bits()),
                lo,
                hi,
                "q8 mul",
            );
            if right != 0 {
                let q = a.checked_div(b).expect("non-zero divisor");
                let (num, den) = (i128::from(left) << 8, i128::from(right));
                let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
                assert_rounded(num, den, i128::from(q.to_bits()), lo, hi, "q8 div");
            }
        }
        let narrowed = Q8_8::from_bits(left).widen().narrow_saturating();
        assert_eq!(
            narrowed.to_bits(),
            left,
            "widen then narrow is the identity"
        );
    }
}

#[test]
fn money_rates_and_splits_round_half_to_even() {
    let mut rng = SplitMix64(5);
    let (lo, hi) = (i128::from(i64::MIN), i128::from(i64::MAX));
    for _ in 0..CASES {
        let (m, r) = (rng.money(), rng.q16());
        let exact = i128::from(m.to_minor()) * i128::from(r.to_bits());
        assert_rounded(
            exact,
            1 << 16,
            i128::from(m.mul_rate(r).to_minor()),
            lo,
            hi,
            "mul_rate",
        );
        let count = i64::from(r.to_bits() >> 8);
        match m.checked_div_int(count) {
            None => assert_eq!(count, 0),
            Some(share) => {
                let (num, den) = (i128::from(m.to_minor()), i128::from(count));
                let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
                assert_rounded(num, den, i128::from(share.to_minor()), lo, hi, "div_int");
            }
        }
    }
}

#[test]
fn ordering_is_monotone_under_the_operations() {
    let mut rng = SplitMix64(6);
    for _ in 0..CASES {
        let (mut a, mut b, c) = (rng.q16(), rng.q16(), rng.q16());
        if a > b {
            (a, b) = (b, a);
        }
        assert!(a + c <= b + c, "addition preserves order");
        assert!(a - c <= b - c, "subtraction preserves order");
        assert!(-a >= -b, "negation reverses order");
        if c >= Q16_16::ZERO {
            assert!(a * c <= b * c, "multiplying by c >= 0 preserves order");
        } else {
            assert!(a * c >= b * c, "multiplying by c < 0 reverses order");
        }
        if c > Q16_16::ZERO {
            assert!(
                a.checked_div(c) <= b.checked_div(c),
                "dividing by c > 0 preserves order"
            );
        }
        assert!(
            a.narrow_saturating() <= b.narrow_saturating(),
            "narrowing preserves order"
        );
        let (mut p, mut q) = (rng.q8(), rng.q8());
        if p > q {
            (p, q) = (q, p);
        }
        assert!(p.widen() <= q.widen(), "widening preserves order");
        let (mut m, mut n) = (rng.money(), rng.money());
        if m > n {
            (m, n) = (n, m);
        }
        if c >= Q16_16::ZERO {
            assert!(
                m.mul_rate(c) <= n.mul_rate(c),
                "a non-negative rate preserves order"
            );
        }
    }
}

#[test]
fn division_is_none_exactly_for_a_zero_divisor() {
    let mut rng = SplitMix64(7);
    for _ in 0..CASES {
        let a = rng.q16();
        assert_eq!(a.checked_div(Q16_16::ZERO), None);
        assert_eq!(rng.q8().checked_div(Q8_8::ZERO), None);
        assert_eq!(rng.money().checked_div_int(0), None);
        let b = rng.q16();
        assert_eq!(a.checked_div(b).is_none(), b == Q16_16::ZERO);
    }
}
