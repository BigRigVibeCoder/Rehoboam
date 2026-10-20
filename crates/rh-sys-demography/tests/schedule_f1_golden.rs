//! The F1 mortality schedule at construction against its exact reference (SPR-016 T-02;
//! `[algorithm.mortality_f1]` MR1–MR9).
//!
//! - **Bit-equality, read directly (D-6).** Every `lambda`, `schedule`, `knot` and `ell` line of
//!   `specs/demography/golden/m2_f1_mortality.txt` (which `selftest_m2_f1_mortality.py --check`
//!   keeps current) is reproduced: K, the tail's a and b, the zero knot, the 438 knots, l(x) at
//!   sampled minutes on both calendars. `RH_M2_F1_GOLDEN` names another file: the positive control
//!   (the golden file with its first knot one bit off, rendered by `bin/check_all.sh`) must fail.
//! - **Rung 0 is the β = 1 case.** At β = 1 the F1 compile's K, `L_0..L_70` and l(x) at 10^4 minutes
//!   in `[Y, 90 Y]` on both calendars equal rung 0's path and `tests/golden/m2_schedule_v1.txt`'s,
//!   for rung 0's seven check values and 64 pseudo-random e₀; `--features mutant-beta-one-power`
//!   must fail it.
//! - **INV-M2-21** at every compiled point, **the corners** and the spec's check values, **the
//!   refusals by name** (MR2, MR3, MR8 through the test hooks), and **reported**: the F1
//!   schedule's own e₀ against E at the spec's eight points.
#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    reason = "test helpers: a malformed vector file must fail the test loudly"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use rh_core::fixed::Q16_16;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_sys_demography::DemographyError;
use rh_sys_demography::schedule::f1::{
    F1Schedule, GRID_90, KNOT_TWELFTHS, KNOTS, LAMBDA, OddsGrid, Tail,
};
use rh_sys_demography::schedule::{GRID_POINTS, ONE, Schedule, solve_k, survivors};

const YEAR_360: i64 = 518_400;
const YEAR_365: i64 = 525_600;
const SAMPLES: usize = 10_000;
const RANDOM_E0: usize = 64;
const BOX_POINTS: usize = 64;

/// `SplitMix64` (Steele, Lea and Flood 2014), as the references.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[lo, hi]`.
    fn within(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_u64() % (hi.abs_diff(lo) + 1)).cast_signed()
    }
}

fn calendar(year: i64) -> Calendar {
    match year {
        YEAR_360 => Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four),
        YEAR_365 => Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four),
        _ => panic!("no calendar has {year} minutes per year"),
    }
}

fn int<T: std::str::FromStr>(token: &str) -> T {
    token
        .parse()
        .unwrap_or_else(|_| panic!("not an integer: {token}"))
}

/// The `Q16_16` bits of a decimal literal, rounded half to even (SPR-011 ruling 9).
fn q16(literal: &str) -> i32 {
    let (whole, frac) = literal.split_once('.').unwrap_or((literal, ""));
    let digits: u128 = int(&format!("{whole}{frac}"));
    let scale = 10_u128.pow(u32::try_from(frac.len()).expect("a short literal"));
    let (q, r) = ((digits << 16) / scale, (digits << 16) % scale);
    let bits = if 2 * r > scale || (2 * r == scale && q % 2 == 1) {
        q + 1
    } else {
        q
    };
    i32::try_from(bits).expect("a Q16_16 literal")
}

fn compile(e_bits: i32, beta_bits: i32, year: i64) -> F1Schedule {
    F1Schedule::new(
        Q16_16::from_bits(e_bits),
        Q16_16::from_bits(beta_bits),
        calendar(year),
    )
    .unwrap_or_else(|e| panic!("e0 {e_bits}, beta {beta_bits}: {e}"))
}

// LINT-ALLOW(forbidden-api): a test reads an environment override for its golden file; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its golden file"
)]
fn override_path() -> Option<PathBuf> {
    std::env::var_os("RH_M2_F1_GOLDEN").map(PathBuf::from)
}

fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The reference's golden file, or the file `RH_M2_F1_GOLDEN` names.
fn golden() -> String {
    match override_path() {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
        None => read("../../specs/demography/golden/m2_f1_mortality.txt"),
    }
}

fn same<T: PartialEq + std::fmt::Display>(got: T, want: T, what: &dyn std::fmt::Display) {
    assert!(
        got == want,
        "golden line differs: {what}: the reference gives {want}, rh-sys-demography {got}"
    );
}

#[test]
fn every_lambda_schedule_knot_and_ell_line_is_reproduced() {
    let text = golden();
    let mut current: Option<(String, [F1Schedule; 2])> = None;
    let mut knot = 0_usize;
    let mut counts = BTreeMap::<&str, usize>::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let t: Vec<&str> = line.split(' ').collect();
        match t[0] {
            "lambda" => {
                let i: usize = int(t[1]);
                same(LAMBDA[i - 1], int(t[2]), &format_args!("lambda {i}"));
            }
            "schedule" => {
                let (e, b) = (q16(t[1]), q16(t[2]));
                let both = [compile(e, b, YEAR_360), compile(e, b, YEAR_365)];
                let name = format!("schedule {} {}", t[1], t[2]);
                let s = &both[0];
                same(s.k(), int(t[3]), &format_args!("K of {name}"));
                same(s.tail().a, int(t[4]), &format_args!("a of {name}"));
                same(s.tail().b, int(t[5]), &format_args!("b of {name}"));
                same(
                    s.zero_twelfths(),
                    int(t[6]),
                    &format_args!("zero knot of {name}"),
                );
                assert_eq!(
                    both[1].knots(),
                    both[0].knots(),
                    "the calendars' knots of {name}"
                );
                current = Some((name, both));
                knot = 0;
            }
            "knot" => {
                let (name, both) = current.as_ref().expect("a knot after a schedule");
                same(
                    KNOT_TWELFTHS[knot],
                    int(t[1]),
                    &format_args!("knot age {knot} of {name}"),
                );
                same(
                    both[0].knots()[knot],
                    int(t[2]),
                    &format_args!("knot {knot} of {name}"),
                );
                knot += 1;
            }
            "ell" => {
                let (name, both) = current.as_ref().expect("an ell after a schedule");
                let (year, x): (i64, i64) = (int(t[1]), int(t[2]));
                let s = &both[usize::from(year == YEAR_365)];
                let got = s.survivors_at(x).expect("a non-negative age");
                same(got, int(t[3]), &format_args!("l({x}) on {year} of {name}"));
            }
            // MR10-MR12's vectors: SPR-016 T-03.
            "q" | "modifier" | "qmod" | "qnew" | "qresample" => {}
            other => panic!("unknown line kind {other}: {line}"),
        }
        *counts.entry(t[0]).or_default() += 1;
    }
    let count = |k: &str| counts.get(k).copied().unwrap_or(0);
    assert_eq!(
        count("lambda"),
        GRID_POINTS - 2,
        "the file pins every lambda"
    );
    assert!(
        count("schedule") >= 42,
        "the design points, corners and 32 box points"
    );
    assert_eq!(
        count("knot"),
        KNOTS * count("schedule"),
        "438 knots per schedule"
    );
    assert!(
        count("ell") >= 40 * count("schedule"),
        "l(x) samples per schedule"
    );
    println!(
        "T-02: every lambda, schedule, knot and ell line reproduced bit for bit ({} lambda, {} \
         schedules, {} knots, {} ell)",
        count("lambda"),
        count("schedule"),
        count("knot"),
        count("ell")
    );
}

/// Rung 0's golden lines of one e0: K, the grid, and `(year, x, l)` samples.
type Rung0 = (u64, Vec<u64>, Vec<(i64, i64, u64)>);

/// Rung 0's golden file: `schedule E K T`, `grid E L_0..L_70`, `ell E year x l`.
fn rung0_golden() -> BTreeMap<i32, Rung0> {
    let mut out = BTreeMap::<i32, Rung0>::new();
    for line in read("tests/golden/m2_schedule_v1.txt").lines() {
        let t: Vec<&str> = line.split(' ').collect();
        match t[0] {
            "schedule" => out.entry(int(t[1])).or_default().0 = int(t[2]),
            "grid" => out.entry(int(t[1])).or_default().1 = t[2..].iter().map(|v| int(v)).collect(),
            "ell" => out
                .entry(int(t[1]))
                .or_default()
                .2
                .push((int(t[2]), int(t[3]), int(t[4]))),
            _ => {}
        }
    }
    out
}

#[test]
fn rung_0_is_the_beta_one_case() {
    // [algorithm.mortality_rung0]'s seven check values (e0 literal, K), then 64 pseudo-random e0.
    let checks = [
        ("20", 18_743_037_470_u64),
        ("23.4", 14_691_140_731),
        ("30", 9_582_773_732),
        ("36.6", 6_442_600_637),
        ("43.44", 4_299_869_756),
        ("60", 1_466_952_214),
        ("85", 73_327_863),
    ];
    let mut rng = SplitMix64(0x5016_7021);
    let mut points: Vec<(i32, Option<u64>)> =
        checks.iter().map(|&(l, k)| (q16(l), Some(k))).collect();
    for _ in 0..RANDOM_E0 {
        let e = rng.within(20 << 16, 85 << 16);
        points.push((i32::try_from(e).expect("in the box"), None));
    }
    let rung0 = rung0_golden();
    let (mut minutes, mut golden_ells) = (0_usize, 0_usize);
    for (e, k) in points {
        for year in [YEAR_360, YEAR_365] {
            let f1 = compile(e, 1 << 16, year);
            let r0 = Schedule::new(Q16_16::from_bits(e), calendar(year)).expect("rung 0 compiles");
            let differs =
                |what: String| format!("rung 0 differs at beta = 1: {what} at e0 bits {e}");
            assert_eq!(f1.k(), r0.k(), "{}", differs("K".into()));
            assert_eq!(Ok(f1.k()), solve_k(e), "{}", differs("K (S5)".into()));
            if let Some(k) = k {
                assert_eq!(
                    f1.k(),
                    k,
                    "{}",
                    differs("K against the spec's check value".into())
                );
            }
            assert_eq!(
                f1.grid(),
                &survivors(f1.k()),
                "{}",
                differs("L_0..L_70".into())
            );
            assert_eq!(f1.grid(), r0.grid(), "{}", differs("L_0..L_70".into()));
            for _ in 0..SAMPLES / 2 {
                let x = rng.within(year, 90 * year);
                let (a, b) = (f1.survivors_at(x), r0.survivors_at(x));
                assert_eq!(a, b, "{}", differs(format!("l({x}) on {year}")));
                minutes += 1;
            }
            if let Some((k0, grid, ells)) = rung0.get(&e) {
                assert_eq!(f1.k(), *k0, "{}", differs("K of m2_schedule_v1.txt".into()));
                assert_eq!(f1.grid().as_slice(), grid, "{}", differs("its grid".into()));
                for &(y, x, l) in ells
                    .iter()
                    .filter(|&&(y, x, _)| y == year && (y..=90 * y).contains(&x))
                {
                    let got = f1.survivors_at(x).expect("an age");
                    assert_eq!(got, l, "{}", differs(format!("its l({x}) on {y}")));
                    golden_ells += 1;
                }
            }
        }
    }
    assert!(
        golden_ells > 100,
        "only {golden_ells} of m2_schedule_v1.txt's l(x) lines in [1, 90]"
    );
    println!(
        "T-02: rung 0 is the beta = 1 case: K, L_0..L_70 and {minutes} sampled minutes of l(x) on \
         [1, 90] years equal rung 0's at {} e0 on both calendars; {golden_ells} lines of \
         m2_schedule_v1.txt",
        checks.len() + RANDOM_E0
    );
}

/// INV-M2-21: 438 knots strictly increasing in age, non-increasing in l, l(90) = `L_66`, l(x) = 0
/// from the zero knot and at every x >= 120 Y, l > 0 at the knot before it.
fn inv_m2_21(s: &F1Schedule, rng: &mut SplitMix64, year: i64) {
    let name = format!("e0 bits {}, beta bits {}", s.e_bits(), s.beta_bits());
    assert!(
        KNOT_TWELFTHS.windows(2).all(|p| p[0] < p[1]),
        "INV-M2-21: knot ages, {name}"
    );
    assert!(
        s.knots().windows(2).all(|p| p[0] >= p[1]),
        "INV-M2-21: l rises, {name}"
    );
    assert_eq!(s.knots()[0], ONE, "INV-M2-21: l(0) = 1, {name}");
    let at = |x: i64| s.survivors_at(x).expect("an age");
    assert_eq!(
        at(90 * year),
        s.grid()[GRID_90],
        "INV-M2-21: l(90) = L_66, {name}"
    );
    let zero = i64::from(s.zero_twelfths()) * year / 12;
    assert_eq!(at(zero), 0, "INV-M2-21: l at the zero knot, {name}");
    assert!(
        at(zero - year / 12) > 0,
        "INV-M2-21: l before the zero knot, {name}"
    );
    for _ in 0..64 {
        assert_eq!(
            at(rng.within(zero, 200 * year)),
            0,
            "INV-M2-21: l past zero, {name}"
        );
        assert_eq!(
            at(rng.within(120 * year, i64::MAX / 2)),
            0,
            "INV-M2-21: the cap, {name}"
        );
        let (x, y) = (rng.within(0, 121 * year), rng.within(0, 121 * year));
        assert!(
            at(x.min(y)) >= at(x.max(y)),
            "INV-M2-21: l(x) non-increasing, {name}"
        );
    }
}

#[test]
fn inv_m2_21_holds_at_every_compiled_point_and_the_corners_compile() {
    let mut rng = SplitMix64(0x5016_7022);
    let corners = [("20", "0.6"), ("20", "1.65"), ("85", "0.6"), ("85", "1.65")];
    let mut points: Vec<(i32, i32)> = corners.iter().map(|&(e, b)| (q16(e), q16(b))).collect();
    for line in golden().lines().filter(|l| l.starts_with("schedule ")) {
        let t: Vec<&str> = line.split(' ').collect();
        points.push((q16(t[1]), q16(t[2])));
    }
    for _ in 0..BOX_POINTS {
        let e = rng.within(20 << 16, 85 << 16);
        let b = rng.within(39_322, 108_134);
        points.push((
            i32::try_from(e).expect("in the box"),
            i32::try_from(b).expect("in the box"),
        ));
    }
    for &(e, b) in &points {
        for year in [YEAR_360, YEAR_365] {
            inv_m2_21(&compile(e, b, year), &mut rng, year);
        }
    }
    // The spec's check values: K, a, b and the zero knot (1314 at (20, 1.65), 1361 at (60, 1.65),
    // 1381 at (85, 1.65), 1408 at (30, 1.3), 1440 elsewhere).
    let spec = spec_check_values();
    assert_eq!(spec.len(), 8, "the spec's eight check values");
    for (e0, beta, k, a, b, zero) in &spec {
        let s = compile(q16(e0), q16(beta), YEAR_360);
        let got = (s.k(), s.tail().a, s.tail().b, s.zero_twelfths());
        assert_eq!(
            got,
            (*k, *a, *b, *zero),
            "the spec's check value at e0 {e0}, beta {beta}"
        );
    }
    println!(
        "T-02: INV-M2-21 at {} compiled points on both calendars; the four corners compile; the \
         spec's eight check values hold",
        points.len()
    );
}

/// The `check_values` of the M2 spec's `[algorithm.mortality_f1]` (the next-version draft while it
/// exists, else the frozen spec it is promoted into): `(e0, beta, K, a, b, l_zero_twelfths)`.
fn spec_check_values() -> Vec<(String, String, u64, i64, i64, u16)> {
    let next = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/systems/next/M2.demography.odd.toml");
    let text = if next.exists() {
        read("../../specs/systems/next/M2.demography.odd.toml")
    } else {
        read("../../specs/systems/M2.demography.odd.toml")
    };
    let table = text
        .split("\n[algorithm.mortality_f1]\n")
        .nth(1)
        .expect("the spec has [algorithm.mortality_f1]");
    let values = table
        .split("check_values = [")
        .nth(1)
        .and_then(|v| v.split("\n]").next())
        .expect("the table has check_values");
    let field = |row: &str, key: &str| -> String {
        let rest = row
            .split(&format!(" {key} = "))
            .nth(1)
            .unwrap_or_else(|| panic!("{key} in {row}"));
        rest.split([',', ' ', '}'])
            .next()
            .unwrap_or_default()
            .trim_matches('"')
            .to_owned()
    };
    values
        .lines()
        .filter(|l| l.trim_start().starts_with('{'))
        .map(|row| {
            (
                field(row, "e0"),
                field(row, "beta"),
                int(&field(row, "K")),
                int(&field(row, "tail_a")),
                int(&field(row, "tail_b")),
                int(&field(row, "l_zero_twelfths")),
            )
        })
        .collect()
}

fn construction(e: &DemographyError) -> (&'static str, &str) {
    match e {
        DemographyError::Construction { step, reason } => (step, reason.as_str()),
        other => panic!("not a construction refusal: {other}"),
    }
}

#[test]
fn each_refusal_names_its_step() {
    let cal = calendar(YEAR_360);
    // MR2: beta outside the Registry box; e0 outside it is the schedule's refusal (S5).
    let e =
        F1Schedule::new(Q16_16::from_int(30), Q16_16::from_bits(108_135), cal).expect_err("1.65+");
    assert_eq!(construction(&e).0, "MR2", "{e}");
    assert!(e.to_string().contains("construction refused at MR2"), "{e}");
    let e = F1Schedule::new(Q16_16::from_int(19), Q16_16::ONE, cal).expect_err("e0 19");
    assert!(matches!(e, DemographyError::Schedule(_)), "{e}");
    // MR3: exp refuses beta lambda_69 > 16 outside the box (beta = 2: 18.4).
    let e = OddsGrid::new(2 << 16).expect_err("beta 2");
    let (step, reason) = construction(&e);
    assert_eq!(step, "MR3", "{e}");
    assert!(reason.contains("ExpDomain"), "{e}");
    // MR8's three preconditions through the test hook, and its ln refusal.
    let half = ONE / 2;
    for (l87, l90, l92, text) in [
        (half, half, half / 2, "strictly decreasing"),
        (ONE, half, 0, "strictly decreasing"),
        (ONE, half, half / 2, "must rise through 90"),
        (ONE, half, 1_000, "below one per year"),
        (ONE, 4_188_914_000, 1_200_000_000, "30 b <= 16"),
    ] {
        let e = Tail::from_anchor(l87, l90, l92).expect_err(text);
        let (step, reason) = construction(&e);
        assert_eq!(step, "MR8", "{e}");
        assert!(reason.contains(text), "{l87}, {l90}, {l92}: {e}");
    }
    let wide = Tail {
        m1: 1,
        m2: 2,
        mu: 1,
        a: i64::MAX - i64::from(u32::MAX),
        b: 1 << 28,
    };
    let e = wide.knots(half).expect_err("1 + a above 2^63");
    let (step, reason) = construction(&e);
    assert_eq!(step, "MR8", "{e}");
    assert!(reason.contains("LnDomain"), "{e}");
    // brass_alpha: refused by M2 at construction (rh_manifest::compile refuses it at load).
    let e = rh_sys_demography::Demography::new(
        Q16_16::from_int(30),
        Some(Q16_16::ZERO),
        Q16_16::ONE,
        Q16_16::from_int(4),
        Q16_16::ONE,
        10_000_000,
        Q16_16::ONE,
        cal,
    )
    .expect_err("brass_alpha");
    assert_eq!(e, DemographyError::AlphaRefused { bits: 0 });
    println!("T-02: each refusal names its step (MR2, MR3, MR8 x 6, brass_alpha)");
}

/// Check 8's figures, reported: the trapezoid e₀ over the F1 knots against E at the spec's points
/// (the reference: −0.06 years at e0 30, +0.5 at e0 85).
#[test]
fn the_realized_e0_is_reported() {
    for (e0, beta, ..) in spec_check_values() {
        let s = compile(q16(&e0), q16(&beta), YEAR_360);
        // e0 in micro-years: area / (24 · 2^32) and E / 2^16.
        let realized = s.realized_e0_units() * 1_000_000 / (24 << 32);
        let input = (u128::from(s.e_bits().cast_unsigned()) * 1_000_000) >> 16;
        let diff = i128::try_from(realized).expect("small") - i128::try_from(input).expect("small");
        let sign = if diff < 0 { "-" } else { "+" };
        let (whole, micro) = (
            diff.unsigned_abs() / 1_000_000,
            diff.unsigned_abs() % 1_000_000,
        );
        println!(
            "T-02: e0 {e0}, beta {beta}: the F1 schedule's own e0 is E {sign}{whole}.{micro:06} years"
        );
    }
}
