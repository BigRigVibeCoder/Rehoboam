//! Correctness of the samplers against the exact distributions (SPR-003 T-01..T-04).
//!
//! Golden vectors (`golden.rs`) only freeze outputs; these tests show the outputs are right. The
//! exact reference, `tests/golden/reference.txt`, is computed independently by
//! `bin/gen_sampler_reference.py` (Python `fractions`/`decimal`); `bin/check_all.sh` checks it is
//! current. Each statistic runs in two tiers: `pr` (the PR tier, here) and `nightly` (64 times the
//! samples, `#[ignore]`d, run by `.github/workflows/nightly.yml`).
//!
//! - **chi2:** binomial (T-01) and Poisson (T-02) counts in the reference's bins; the statistic must
//!   stay below the 99.9% quantile of χ²(df).
//! - **moments:** Poisson (T-02), gamma and beta (T-04), normal (T-03): the sample mean and the
//!   sample variance, each as a z-score against the exact moments (the variance's standard error
//!   from the exact fourth central moment); `|z| < Z_LIMIT`.
//! - **tail:** the standard normal (T-03) beyond `t` = 1 to 4 standard deviations, both tails and
//!   inside and beyond the ziggurat's base box (`R ≈ 3.654`): each tail count, and their sum, as a
//!   binomial z-score against the exact probability.
//!
//! The draws come from a keyed stream per case, so the tests are deterministic: a failure is a
//! real one, never a flake. Floats are allowed in `tests/` (V-DET-19 DECISION, SPR-010 T-04).

#![allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::expect_used,
    clippy::panic,
    clippy::many_single_char_names,
    reason = "test statistics are computed in f64 with their textbook names; a malformed reference \
              file must fail loudly"
)]

use rh_core::fixed::Q16_16;
use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};
use rh_sample::{Beta, Binomial, Gamma, Normal, Poisson};

const REFERENCE: &str = include_str!("golden/reference.txt");
/// Two-sided z bound for each moment and tail test: P(|Z| > 4.5) ≈ 7·10⁻⁶ per test.
const Z_LIMIT: f64 = 4.5;

/// The stream of one case: the case's line in the reference file keys it.
fn stream(case: u64) -> RngStream {
    RngStream::new(&RngKey {
        world_seed: 0x05A3_D1E5,
        branch_salt: BranchSalt::from_raw(0),
        system: SystemId::from_raw(0),
        purpose: PurposeId::of("sampler_statistics"),
        entity: case,
        epoch: 0,
        t: 0,
    })
}

fn bits(s: &str) -> Q16_16 {
    Q16_16::from_bits(s.parse().expect("Q16_16 bits"))
}

fn num<T: std::str::FromStr>(s: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    s.parse().expect("number")
}

/// The reference lines as (line number, fields).
fn lines() -> impl Iterator<Item = (u64, Vec<&'static str>)> {
    REFERENCE
        .lines()
        .zip(1_u64..)
        .filter(|(l, _)| !l.starts_with('#'))
        .map(|(l, n)| (n, l.split_whitespace().collect()))
}

fn q_value(x: Q16_16) -> f64 {
    f64::from(x.to_bits()) / 65_536.0
}

/// A sampler of one reference case, as a closure over a stream returning `f64`.
fn sampler(dist: &str, params: &[&str]) -> Box<dyn Fn(&mut RngStream) -> f64> {
    match dist {
        "binomial" => {
            let b = Binomial::new(num(params[0]), bits(params[1])).expect("binomial");
            Box::new(move |s| f64::from(b.sample(s)))
        }
        "poisson" => {
            let p = Poisson::new(bits(params[0])).expect("poisson");
            Box::new(move |s| f64::from(p.sample(s)))
        }
        "gamma" => {
            let g = Gamma::new(bits(params[0]), bits(params[1])).expect("gamma");
            Box::new(move |s| q_value(g.sample(s)))
        }
        "beta" => {
            let b = Beta::new(bits(params[0]), bits(params[1])).expect("beta");
            Box::new(move |s| q_value(b.sample(s)))
        }
        "normal" => {
            let n = Normal::new(bits(params[0]), bits(params[1])).expect("normal");
            Box::new(move |s| q_value(n.sample(s)))
        }
        other => panic!("unknown distribution {other}"),
    }
}

fn param_count(dist: &str) -> usize {
    match dist {
        "poisson" => 1,
        _ => 2,
    }
}

/// Every chi-square case of `tier`; returns the failures.
fn chi2(tier: &str) -> (usize, Vec<String>) {
    let all: Vec<_> = lines().collect();
    let (mut cases, mut failures) = (0, Vec::new());
    for (i, (line, f)) in all.iter().enumerate() {
        if f[0] != "chi2" || f[1] != tier {
            continue;
        }
        cases += 1;
        let dist = f[2];
        let k = param_count(dist);
        let params = &f[3..3 + k];
        let samples: u64 = num(f[3 + k]);
        let n_bins: usize = num(f[4 + k]);
        let crit: f64 = num(f[6 + k]);
        let bins: Vec<(u32, f64)> = all[i + 1..=i + n_bins]
            .iter()
            .map(|(_, b)| {
                assert_eq!(b[0], "bin", "line {line}: expected {n_bins} bins");
                (num(b[1]), num(b[3]))
            })
            .collect();
        let draw = sampler(dist, params);
        let mut s = stream(*line);
        let mut observed = vec![0_u64; n_bins];
        for _ in 0..samples {
            let x = draw(&mut s);
            // Counts are integers below 2^32, exactly representable; find the bin by its lower edge.
            let at = bins.partition_point(|&(lo, _)| f64::from(lo) <= x) - 1;
            observed[at] += 1;
        }
        let n = samples as f64;
        let stat: f64 = observed
            .iter()
            .zip(&bins)
            .map(|(&o, &(_, p))| {
                let e = n * p;
                let d = o as f64 - e;
                d * d / e
            })
            .sum();
        let verdict = if stat < crit { "ok" } else { "FAIL" };
        println!("chi2 {tier} {dist} {params:?}: {stat:.2} vs {crit:.2} ({verdict})");
        if stat >= crit {
            failures.push(format!(
                "line {line}: {dist} {params:?} chi2 {stat:.3} >= {crit:.3}"
            ));
        }
    }
    (cases, failures)
}

/// Every moments case of `tier`; returns the failures.
fn moments(tier: &str) -> (usize, Vec<String>) {
    let (mut cases, mut failures) = (0, Vec::new());
    for (line, f) in lines() {
        if f[0] != "moments" || f[1] != tier {
            continue;
        }
        cases += 1;
        let dist = f[2];
        let k = param_count(dist);
        let params = &f[3..3 + k];
        let samples: u64 = num(f[3 + k]);
        let (mean, var, mu4): (f64, f64, f64) = (num(f[4 + k]), num(f[5 + k]), num(f[6 + k]));
        let draw = sampler(dist, params);
        let mut s = stream(line);
        let xs: Vec<f64> = (0..samples).map(|_| draw(&mut s)).collect();
        let n = samples as f64;
        let m = xs.iter().sum::<f64>() / n;
        let s2 = xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0);
        let z_mean = (m - mean) / (var / n).sqrt();
        let z_var = (s2 - var) / ((mu4 - var * var) / n).sqrt();
        println!("moments {tier} {dist} {params:?}: z_mean {z_mean:+.2}, z_var {z_var:+.2}");
        if z_mean.abs() >= Z_LIMIT || z_var.abs() >= Z_LIMIT {
            failures.push(format!(
                "line {line}: {dist} {params:?} mean {m} (exact {mean}, z {z_mean:.2}), \
                 variance {s2} (exact {var}, z {z_var:.2})"
            ));
        }
    }
    (cases, failures)
}

/// Every tail case of `tier`; returns the failures.
fn tails(tier: &str) -> (usize, Vec<String>) {
    let (mut cases, mut failures) = (0, Vec::new());
    let normal = Normal::new(Q16_16::ZERO, Q16_16::ONE).expect("standard normal");
    for (line, f) in lines() {
        if f[0] != "tail" || f[1] != tier {
            continue;
        }
        cases += 1;
        let t: i32 = num(f[2]);
        let samples: u64 = num(f[3]);
        let p: f64 = num(f[4]);
        let mut s = stream(line);
        let (mut upper, mut lower) = (0_u64, 0_u64);
        for _ in 0..samples {
            let z = normal.sample(&mut s).to_bits();
            upper += u64::from(z > t);
            lower += u64::from(z < -t);
        }
        let n = samples as f64;
        let z_of =
            |count: u64, prob: f64| (count as f64 - n * prob) / (n * prob * (1.0 - prob)).sqrt();
        let zs = [z_of(upper, p), z_of(lower, p), z_of(upper + lower, 2.0 * p)];
        println!(
            "tail {tier} t={}: upper {upper}, lower {lower}, expected {:.1}; z {zs:+.2?}",
            q_value(Q16_16::from_bits(t)),
            n * p
        );
        if zs.iter().any(|z| z.abs() >= Z_LIMIT) {
            failures.push(format!("line {line}: t bits {t}: z {zs:?}"));
        }
    }
    (cases, failures)
}

fn check(name: &str, expected_cases: usize, (cases, failures): (usize, Vec<String>)) {
    assert_eq!(cases, expected_cases, "{name}: reference cases");
    assert!(failures.is_empty(), "{name}:\n{}", failures.join("\n"));
}

#[test]
fn binomial_and_poisson_match_the_exact_pmf_chi2_pr() {
    check("chi2 pr", 18, chi2("pr"));
}

#[test]
fn moments_match_the_exact_moments_pr() {
    check("moments pr", 18, moments("pr"));
}

#[test]
fn normal_tails_match_the_exact_tail_probabilities_pr() {
    check("tail pr", 6, tails("pr"));
}

#[test]
#[ignore = "nightly tier: 64 times the PR samples (.github/workflows/nightly.yml)"]
fn binomial_and_poisson_match_the_exact_pmf_chi2_nightly() {
    check("chi2 nightly", 18, chi2("nightly"));
}

#[test]
#[ignore = "nightly tier: 64 times the PR samples (.github/workflows/nightly.yml)"]
fn moments_match_the_exact_moments_nightly() {
    check("moments nightly", 18, moments("nightly"));
}

#[test]
#[ignore = "nightly tier: 64 times the PR samples (.github/workflows/nightly.yml)"]
fn normal_tails_match_the_exact_tail_probabilities_nightly() {
    check("tail nightly", 6, tails("nightly"));
}
