//! `exp`, `ln_ratio` and `prob_from_hazard` against their exact reference (SPR-016 T-01; `05.1` §6
//! item 4; format `rehoboam/fixed/exp-ln/v0`).
//!
//! - **The golden file, read directly (D-6).** `specs/numerics/golden/exp_ln_v0.txt`, which
//!   `specs/numerics/exp_ln_v0.py --check` keeps current, is read from the repository, never copied:
//!   every `const` line equals the compiled constant, every `exp`, `ln` and `hazard` line is
//!   reproduced bit for bit (or refused by name where it says `refused`), and a file whose first line
//!   does not name the format is refused. `RH_EXP_LN_GOLDEN` names another file: the positive
//!   control `tests/lint_fixtures/golden/stale_exp_ln_v0.txt` (`exp 0` one bit off) must fail.
//! - **Beyond the committed vectors.** The ignored test replays a file of `exp_ln_v0.py --emit N`
//!   (`RH_EXP_LN_EMIT`): 2^14 of each function in the PR tier, 2^20 nightly.
//! - **Properties** on `SplitMix64` inputs (10^4 or more cases each): monotone, the exact anchors,
//!   the hazard's saturation, `exp(ln(n / d))` on the Brass standard's 69 odds, the refusals.
#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::many_single_char_names,
    reason = "test helpers: a malformed vector file must fail the test loudly"
)]

use std::path::PathBuf;

use rh_core::fixed::{
    EXP_LN_C, EXP_LN_VERSION, FixedError, INV_LN2_62, LN2_64, Prob32, Q32_32, exp, ln_ratio,
    prob_from_hazard,
};

const CASES: usize = 16_384;
const ONE: i64 = 1 << 32;
/// The Brass General Standard's four-digit survivors (`specs/demography/brass_general_standard.toml`).
const STANDARD_SURVIVORS: [u64; 71] = [
    10000, 8499, 8070, 7876, 7762, 7691, 7642, 7601, 7564, 7532, 7502, 7477, 7452, 7425, 7396,
    7362, 7328, 7287, 7241, 7188, 7130, 7069, 7005, 6943, 6884, 6826, 6764, 6703, 6643, 6584, 6525,
    6466, 6405, 6345, 6284, 6223, 6160, 6097, 6032, 5966, 5898, 5830, 5759, 5686, 5612, 5535, 5454,
    5371, 5285, 5197, 5106, 4857, 4585, 4291, 3965, 3602, 3210, 2801, 2380, 1945, 1500, 1090, 760,
    490, 290, 155, 70, 30, 10, 1, 0,
];

/// `SplitMix64` (Steele, Lea and Flood 2014), as `specs/numerics/exp_ln_v0.py`.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    /// A `Q32_32` in `[lo, hi]`.
    fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = hi.abs_diff(lo) + 1;
        lo.wrapping_add(self.below(span).cast_signed())
    }
}

// LINT-ALLOW(forbidden-api): a test reads an environment override for its vector file; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its vector file"
)]
fn path_from(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

/// The reference's golden file, or the file `RH_EXP_LN_GOLDEN` names.
fn golden_text() -> (PathBuf, String) {
    let path = path_from("RH_EXP_LN_GOLDEN").unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs/numerics/golden/exp_ln_v0.txt")
    });
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    (path, text)
}

fn int<T: std::str::FromStr>(token: &str) -> T {
    token
        .parse()
        .unwrap_or_else(|_| panic!("not an integer: {token}"))
}

/// The bits of a result that must not be refused.
fn ok(result: Result<Q32_32, FixedError>) -> i64 {
    result.map(Q32_32::to_bits).expect("inside the domain")
}

/// The bits of a probability that must not be refused.
fn okp(result: Result<Prob32, FixedError>) -> u64 {
    result.map(Prob32::to_bits).expect("inside the domain")
}

fn text<T: std::fmt::Display>(result: Result<T, FixedError>) -> String {
    result.map_or_else(|_| "refused".to_owned(), |v| v.to_string())
}

/// Checks one `exp`, `ln` or `hazard` vector line; returns whether it was one.
fn check_vector(line: &str) -> bool {
    let words: Vec<&str> = line.split(' ').collect();
    let got = match words.as_slice() {
        ["exp", y, _] => text(exp(Q32_32::from_bits(int(y))).map(Q32_32::to_bits)),
        ["ln", n, d, _] => text(ln_ratio(int(n), int(d)).map(Q32_32::to_bits)),
        ["hazard", h, _] => text(prob_from_hazard(Q32_32::from_bits(int(h))).map(Prob32::to_bits)),
        _ => return false,
    };
    let want = words[words.len() - 1];
    assert_eq!(
        got,
        want,
        "golden vector differs: {}: the reference gives {want}, rh-core {got}",
        words[..words.len() - 1].join(" ")
    );
    true
}

#[test]
fn every_constant_and_vector_of_the_golden_file_is_reproduced() {
    let (path, text) = golden_text();
    let first = text.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(&format!("# Golden vectors of {EXP_LN_VERSION}.")),
        "{} is not a {EXP_LN_VERSION} golden file: {first}",
        path.display()
    );
    let (mut constants, mut vectors) = (0, 0);
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["const", "LN2_64", v] => assert_eq!(LN2_64, int::<u64>(v), "const LN2_64 differs"),
            ["const", "INV_LN2_62", v] => {
                assert_eq!(INV_LN2_62, int::<u64>(v), "const INV_LN2_62 differs");
            }
            ["const", "C", k, v] => {
                let k: usize = int(k);
                assert_eq!(EXP_LN_C[k - 1], int::<u64>(v), "const C {k} differs");
            }
            _ => {
                assert!(check_vector(line), "unknown line: {line}");
                vectors += 1;
                continue;
            }
        }
        constants += 1;
    }
    assert_eq!(constants, 34, "the file pins 34 constants");
    assert!(
        vectors > 9_000,
        "only {vectors} vectors in {}",
        path.display()
    );
    println!(
        "T-01: every constant and vector reproduced bit for bit ({constants} constants, {vectors} \
         vectors of {})",
        path.display()
    );
}

/// The PR tier runs 2^14 of each function, the nightly tier 2^20: `exp_ln_v0.py --emit N`.
#[test]
#[ignore = "needs RH_EXP_LN_EMIT, a file of exp_ln_v0.py --emit (bin/check_all.sh, golden_nightly.sh)"]
fn emitted_vectors_are_bit_equal() {
    let path = path_from("RH_EXP_LN_EMIT").expect("RH_EXP_LN_EMIT names an --emit file");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let header = text.lines().next().unwrap_or_default();
    assert!(
        header.starts_with(&format!("# Emitted vectors of {EXP_LN_VERSION}:")),
        "{} is not an --emit file of {EXP_LN_VERSION}: {header}",
        path.display()
    );
    let (mut exps, mut lns) = (0_usize, 0_usize);
    for line in text.lines().skip(1) {
        assert!(check_vector(line), "unknown line: {line}");
        if line.starts_with("exp ") {
            exps += 1;
        } else {
            lns += 1;
        }
    }
    assert!(exps > 0 && exps == lns, "{exps} exp and {lns} ln vectors");
    println!("T-01: emitted vectors bit-equal ({exps} exp, {lns} ln; {header})");
}

#[test]
fn exp_is_monotone_and_exact_at_zero() {
    assert_eq!(exp(Q32_32::ZERO), Ok(Q32_32::ONE));
    let mut rng = SplitMix64(0x5016_0001);
    for _ in 0..CASES {
        let y = rng.in_range(-24 * ONE, 16 * ONE - 1);
        let step = 1 + rng.below(1 << 30).cast_signed();
        let z = (y + step).min(16 * ONE);
        let (a, b) = (ok(exp(Q32_32::from_bits(y))), ok(exp(Q32_32::from_bits(z))));
        assert!(a <= b, "exp({y}) = {a} > exp({z}) = {b}");
        assert!(
            a <= ok(exp(Q32_32::from_bits(y + 1))),
            "exp({y}) > exp({y} + 1)"
        );
    }
}

#[test]
fn ln_ratio_is_monotone_and_zero_at_one() {
    let mut rng = SplitMix64(0x5016_0002);
    let top = (1_u64 << 63) - 1;
    for _ in 0..CASES {
        let n = rng.below(top) + 1;
        assert_eq!(ln_ratio(n, n), Ok(Q32_32::ZERO), "ln({n} / {n})");
        let d = rng.below(top) + 1;
        let m = n.saturating_add(1 + rng.below(1 << 40)).min(top);
        let (a, b) = (ok(ln_ratio(n, d)), ok(ln_ratio(m, d)));
        assert!(a <= b, "ln({n}/{d}) = {a} > ln({m}/{d}) = {b}");
        // A larger denominator gives a smaller logarithm.
        let e = d.saturating_add(1 + rng.below(1 << 20)).min(top);
        assert!(ok(ln_ratio(n, e)) <= a, "ln({n}/{e}) > ln({n}/{d})");
    }
}

#[test]
fn prob_from_hazard_is_zero_at_zero_one_from_23_and_non_decreasing() {
    assert_eq!(prob_from_hazard(Q32_32::ZERO), Ok(Prob32::ZERO));
    let mut rng = SplitMix64(0x5016_0003);
    for _ in 0..CASES {
        let h = rng.in_range(23 * ONE, i64::MAX);
        assert_eq!(
            prob_from_hazard(Q32_32::from_bits(h)),
            Ok(Prob32::ONE),
            "h = {h}"
        );
        let h = rng.in_range(0, 24 * ONE);
        let g = h + rng.below(1 << 24).cast_signed();
        let (p, q) = (
            okp(prob_from_hazard(Q32_32::from_bits(h))),
            okp(prob_from_hazard(Q32_32::from_bits(g))),
        );
        assert!(p <= q, "hazard {h} -> {p} above {g} -> {q}");
    }
}

/// `exp(ln(n / d))` against `n · 2^32 / d` on the standard's 69 interior odds `(10^4 − s_i) / s_i`.
///
/// The bound is 2 ulp of the result relative to the odds, `2 · max(1, n / d)` ulp: `ln`'s error of
/// up to ½ ulp absolute becomes a relative error of `2^-33` in `exp`'s result, which is about
/// `n / (2 d)` ulp absolute (4,603 ulp at the odds 9999 / 1). The plan's 2 ulp absolute holds only up
/// to odds of about 4.
#[test]
fn exp_of_ln_returns_the_standard_odds() {
    let mut worst = 0_u128;
    for s in STANDARD_SURVIVORS
        .into_iter()
        .filter(|&s| s > 0 && s < 10_000)
    {
        let (n, d) = (10_000 - s, s);
        let back = ln_ratio(n, d).and_then(exp).expect("inside both domains");
        let exact = (u128::from(n) << 32) / u128::from(d);
        let error = u128::from(back.to_bits().cast_unsigned()).abs_diff(exact);
        assert!(
            error * u128::from(d) <= 2 * u128::from(n.max(d)) + u128::from(d),
            "exp(ln({n} / {d})) = {} against {exact}",
            back.to_bits()
        );
        worst = worst.max(error);
    }
    println!("T-01: exp(ln) on the 69 standard odds within 2 max(1, n/d) ulp (worst {worst} ulp)");
}

#[test]
fn each_domain_is_refused_by_name() {
    let mut rng = SplitMix64(0x5016_0004);
    for _ in 0..CASES {
        let y = rng.in_range(16 * ONE + 1, i64::MAX);
        let e = exp(Q32_32::from_bits(y));
        assert_eq!(e, Err(FixedError::ExpDomain { y }));
        assert!(e.is_err_and(|e| e.to_string().starts_with("ExpDomain")));
        assert_eq!(
            exp(Q32_32::from_bits(rng.in_range(i64::MIN, -23 * ONE - 1))),
            Ok(Q32_32::ZERO)
        );
        let big = (1 << 63) + rng.below(1 << 63);
        let n = rng.below(1 << 62) + 1;
        for (a, b) in [(0, n), (n, 0), (big, n), (n, big)] {
            let e = ln_ratio(a, b);
            assert_eq!(e, Err(FixedError::LnDomain { n: a, d: b }));
            assert!(e.is_err_and(|e| e.to_string().starts_with("LnDomain")));
        }
        let h = rng.in_range(i64::MIN, -1);
        let e = prob_from_hazard(Q32_32::from_bits(h));
        assert_eq!(e, Err(FixedError::HazardDomain { h }));
        assert!(e.is_err_and(|e| e.to_string().starts_with("HazardDomain")));
    }
}
