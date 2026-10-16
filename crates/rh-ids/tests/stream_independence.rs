//! V-DET-20: statistical independence of RNG streams (SPR-002 T-06; RT-2026-10-16).
//!
//! `docs/09` V-DET-20: streams for distinct (system, purpose, entity, t, k) keys and for sibling
//! branch salts; pairwise correlation over 10⁶ draws per stream, chi-square on bytes, and a runs
//! test, over at least 10³ key pairs including adjacent entities and adjacent t. Pass: every
//! statistic inside its 99.9% band, with no more exceedances across all pairs than a binomial
//! 99.9% allows.
//!
//! # Design
//!
//! - **Pairs.** [`PAIRS`] key pairs, in [`Kind`]s that each differ in one field by the smallest
//!   step (adjacent entity, t, system, purpose identifier, world seed, entity epoch; sibling
//!   rollout salts; disjoint `k` ranges of one stream), plus adjacent entity, t and epoch and
//!   sibling salts for [`LightStream`]. Base keys are drawn
//!   from a keyed stream, so the sweep is deterministic and a failure is reproducible.
//! - **Statistics per pair** (stream A, stream B, `n` draws each):
//!   1. Pearson correlation of the draws as uniforms on [0, 1): `z = r·√n`, ~N(0, 1);
//!   2. chi-square of the bytes of `A ⊕ B` (256 cells, 255 df): a dependence shows as bias;
//!   3. chi-square of the joint top nibbles `(A >> 60, B >> 60)` (16 × 16 cells, 255 df);
//!   4. Wald–Wolfowitz runs test on the signs of `A − B`, `z` ~N(0, 1);
//!   5. and 6. chi-square of the bytes of A, and of B, alone (uniformity; 255 df).
//! - **Bands.** Two-sided 99.9%: `|z| ≤ z₀.₉₉₉₅` for the normal statistics, and the 0.0005 and
//!   0.9995 quantiles of χ²(255) (Wilson–Hilferty) for the chi-squares.
//! - **Verdict.** With `N = 6 · PAIRS` tests at false-alarm rate 0.001, the number of
//!   exceedances must not exceed the 99.9% quantile of Binomial(N, 0.001).
//!
//! The PR tier runs [`PR_DRAWS`] draws per stream; the full sweep with 10⁶ draws per stream
//! (`docs/09`) is the ignored test `v_det_20_full_sweep`, run with
//! `cargo nextest run -p rh-ids --run-ignored only --test stream_independence`.

#![allow(
    clippy::cast_precision_loss,
    reason = "statistics on counts below 2^53: the f64 conversion is exact"
)]
#![allow(
    clippy::expect_used,
    reason = "test helpers: an impossible key must fail loudly"
)]
#![allow(
    clippy::many_single_char_names,
    reason = "statistical notation: a, b streams; n draws; r, z, t as in the formulas"
)]

use rh_ids::{
    BranchSalt, CANONICAL_SALT, ForkPolicy, LightStream, PurposeId, RngKey, RngStream, SaltLineage,
    SystemId, TimelineId,
};

/// Key pairs per sweep (`docs/09`: at least 10³).
const PAIRS: usize = 1_000;
/// Draws per stream in the PR tier (`docs/09` asks 10⁶; the full sweep runs nightly).
const PR_DRAWS: usize = 1 << 16;
/// Draws per stream in the full sweep (`docs/09` V-DET-20).
const FULL_DRAWS: usize = 1_000_000;
/// Two-sided false-alarm rate per statistic (the 99.9% band).
const ALPHA: f64 = 0.001;
/// Φ⁻¹(1 − ALPHA/2), the two-sided 99.9% normal quantile.
/// REF: standard normal tables (Abramowitz and Stegun 26.2); Φ(3.290527) = 0.9995.
const Z_BAND: f64 = 3.290_527;
/// Degrees of freedom of every chi-square here (256 cells, expectation fully specified).
const CHI_DF: f64 = 255.0;
/// Statistics computed per pair.
const STATS_PER_PAIR: usize = 6;

/// What differs between the two streams of a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    AdjacentEntity,
    AdjacentT,
    AdjacentSystem,
    AdjacentPurpose,
    AdjacentSeed,
    AdjacentEpoch,
    SiblingSalts,
    DisjointK,
    LightAdjacentEntity,
    LightAdjacentT,
    LightAdjacentEpoch,
    LightSiblingSalts,
}

const KINDS: [Kind; 12] = [
    Kind::AdjacentEntity,
    Kind::AdjacentT,
    Kind::AdjacentSystem,
    Kind::AdjacentPurpose,
    Kind::AdjacentSeed,
    Kind::AdjacentEpoch,
    Kind::SiblingSalts,
    Kind::DisjointK,
    Kind::LightAdjacentEntity,
    Kind::LightAdjacentT,
    Kind::LightAdjacentEpoch,
    Kind::LightSiblingSalts,
];

/// Two sibling rollout salts: timelines `n` and `n + 1` forked from the canonical timeline at `t`.
fn sibling_salts(t: i64, n: u32) -> (BranchSalt, BranchSalt) {
    let fork = |id| {
        SaltLineage::canonical()
            .fork(t, TimelineId::from_raw(id), ForkPolicy::ROLLOUT)
            .expect("rollout fork")
            .branch_salt(t)
    };
    (fork(n), fork(n + 1))
}

/// The two draw sequences of pair `i`.
fn pair(i: usize, n: usize) -> (Kind, Vec<u64>, Vec<u64>) {
    let kind = KINDS[i % KINDS.len()];
    let pick = LightStream::new(&RngKey {
        world_seed: 0xD7_20,
        branch_salt: CANONICAL_SALT,
        system: SystemId::from_raw(0),
        purpose: PurposeId::of("v_det_20_keys"),
        entity: i as u64,
        epoch: 0,
        t: 0,
    });
    // Small t and entity values, as in real worlds, so that "adjacent" is realistic.
    let t = i64::try_from(pick.draw(5) % 10_000_000).expect("t") - 5_000_000;
    let a = RngKey {
        world_seed: pick.draw(0),
        branch_salt: CANONICAL_SALT,
        system: SystemId::from_raw(u32::try_from(pick.draw(1) % 64).expect("system")),
        purpose: PurposeId::from_raw(pick.draw(2)),
        entity: pick.draw(3) % 1_000_000,
        // Epochs are small in practice: most entities are never re-sampled by an input.
        epoch: u32::try_from(pick.draw(6) % 4).expect("epoch"),
        t,
    };
    let mut b = a;
    let chacha = |key: &RngKey, skip: usize| {
        let mut s = RngStream::new(key);
        s.seek(skip as u64);
        (0..n).map(|_| s.next_u64()).collect::<Vec<u64>>()
    };
    let light = |key: &RngKey| {
        let s = LightStream::new(key);
        (0..n as u64).map(|k| s.draw(k)).collect::<Vec<u64>>()
    };
    let timeline = u32::try_from(pick.draw(4) % 1_000_000).expect("timeline");
    match kind {
        Kind::AdjacentEntity | Kind::LightAdjacentEntity => b.entity += 1,
        Kind::AdjacentT | Kind::LightAdjacentT => b.t += 1,
        Kind::AdjacentSystem => b.system = SystemId::from_raw(a.system.to_raw() + 1),
        Kind::AdjacentPurpose => b.purpose = PurposeId::from_raw(a.purpose.to_raw() ^ 1),
        Kind::AdjacentSeed => b.world_seed ^= 1,
        Kind::AdjacentEpoch | Kind::LightAdjacentEpoch => b.epoch += 1,
        Kind::SiblingSalts | Kind::LightSiblingSalts => {
            let mut a2 = a;
            (a2.branch_salt, b.branch_salt) = sibling_salts(a.t, timeline);
            return finish(kind, &a2, &b, chacha, light);
        }
        Kind::DisjointK => return (kind, chacha(&a, 0), chacha(&a, n)),
    }
    finish(kind, &a, &b, chacha, light)
}

fn finish(
    kind: Kind,
    a: &RngKey,
    b: &RngKey,
    chacha: impl Fn(&RngKey, usize) -> Vec<u64>,
    light: impl Fn(&RngKey) -> Vec<u64>,
) -> (Kind, Vec<u64>, Vec<u64>) {
    match kind {
        Kind::LightAdjacentEntity
        | Kind::LightAdjacentT
        | Kind::LightAdjacentEpoch
        | Kind::LightSiblingSalts => (kind, light(a), light(b)),
        _ => (kind, chacha(a, 0), chacha(b, 0)),
    }
}

/// A draw as a uniform on [0, 1): its top 53 bits.
fn unit(x: u64) -> f64 {
    (x >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
}

/// Pearson correlation of the two sequences as uniforms, as `z = r·√n`.
fn correlation_z(a: &[u64], b: &[u64]) -> f64 {
    let n = a.len() as f64;
    let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&x, &y) in a.iter().zip(b) {
        let (u, v) = (unit(x), unit(y));
        sa += u;
        sb += v;
        saa += u * u;
        sbb += v * v;
        sab += u * v;
    }
    let cov = sab - sa * sb / n;
    let r = cov / ((saa - sa * sa / n) * (sbb - sb * sb / n)).sqrt();
    r * n.sqrt()
}

/// Pearson's chi-square of 256 counts against a uniform expectation.
fn chi_square(counts: &[u64; 256]) -> f64 {
    let total: u64 = counts.iter().sum();
    let expected = total as f64 / 256.0;
    counts
        .iter()
        .map(|&c| (c as f64 - expected).powi(2) / expected)
        .sum()
}

fn byte_counts(xs: impl Iterator<Item = u64>) -> [u64; 256] {
    let mut counts = [0_u64; 256];
    for x in xs {
        for byte in x.to_le_bytes() {
            counts[usize::from(byte)] += 1;
        }
    }
    counts
}

fn joint_nibbles(a: &[u64], b: &[u64]) -> [u64; 256] {
    let mut counts = [0_u64; 256];
    for (&x, &y) in a.iter().zip(b) {
        counts[usize::try_from((x >> 60) * 16 + (y >> 60)).expect("cell")] += 1;
    }
    counts
}

/// Wald–Wolfowitz runs test on the signs of `a − b`, as a z-score.
fn runs_z(a: &[u64], b: &[u64]) -> f64 {
    let signs: Vec<bool> = a.iter().zip(b).map(|(x, y)| x < y).collect();
    let n1 = signs.iter().filter(|&&s| s).count() as f64;
    let n2 = signs.len() as f64 - n1;
    let n = n1 + n2;
    let runs = 1 + signs.windows(2).filter(|w| w[0] != w[1]).count();
    let mu = 2.0 * n1 * n2 / n + 1.0;
    let var = (mu - 1.0) * (mu - 2.0) / (n - 1.0);
    (runs as f64 - mu) / var.sqrt()
}

/// The `z`-quantile of χ²(df) by the Wilson–Hilferty approximation (relative error < 10⁻³ at
/// 255 df). REF: Wilson and Hilferty (1931); Johnson, Kotz and Balakrishnan, vol. 1, §18.5.
fn chi_square_quantile(df: f64, z: f64) -> f64 {
    let c = 2.0 / (9.0 * df);
    df * (1.0 - c + z * c.sqrt()).powi(3)
}

/// The smallest `m` with P(Binomial(trials, p) ≤ m) ≥ 0.999.
fn binomial_allowance(trials: usize, p: f64) -> usize {
    let mut pmf = (1.0 - p).powf(trials as f64);
    let mut cdf = pmf;
    let mut m = 0;
    while cdf < 1.0 - ALPHA {
        pmf *= (trials - m) as f64 / (m + 1) as f64 * p / (1.0 - p);
        m += 1;
        cdf += pmf;
    }
    m
}

/// One statistic outside its band.
#[derive(Debug)]
struct Exceedance {
    pair: usize,
    kind: Kind,
    statistic: &'static str,
    value: f64,
}

/// Runs the sweep; returns the number of tests and the exceedances.
fn sweep(draws: usize) -> (usize, Vec<Exceedance>) {
    let chi_lo = chi_square_quantile(CHI_DF, -Z_BAND);
    let chi_hi = chi_square_quantile(CHI_DF, Z_BAND);
    let mut out = Vec::new();
    for i in 0..PAIRS {
        let (kind, a, b) = pair(i, draws);
        let xor = byte_counts(a.iter().zip(&b).map(|(x, y)| x ^ y));
        let stats: [(&'static str, f64, bool); STATS_PER_PAIR] = [
            ("correlation z", correlation_z(&a, &b), false),
            ("xor-byte chi2", chi_square(&xor), true),
            (
                "joint-nibble chi2",
                chi_square(&joint_nibbles(&a, &b)),
                true,
            ),
            ("runs z", runs_z(&a, &b), false),
            (
                "byte chi2 (A)",
                chi_square(&byte_counts(a.iter().copied())),
                true,
            ),
            (
                "byte chi2 (B)",
                chi_square(&byte_counts(b.iter().copied())),
                true,
            ),
        ];
        for (statistic, value, is_chi) in stats {
            let inside = if is_chi {
                (chi_lo..=chi_hi).contains(&value)
            } else {
                value.abs() <= Z_BAND
            };
            if !inside {
                out.push(Exceedance {
                    pair: i,
                    kind,
                    statistic,
                    value,
                });
            }
        }
    }
    (PAIRS * STATS_PER_PAIR, out)
}

fn check(draws: usize) {
    let (tests, exceedances) = sweep(draws);
    let allowed = binomial_allowance(tests, ALPHA);
    println!(
        "V-DET-20: {PAIRS} pairs x {draws} draws, {tests} tests, {} outside the 99.9% band \
         (expected {:.1}, allowed {allowed})",
        exceedances.len(),
        tests as f64 * ALPHA
    );
    for e in &exceedances {
        println!(
            "  pair {} {:?}: {} = {:.4}",
            e.pair, e.kind, e.statistic, e.value
        );
    }
    assert!(
        exceedances.len() <= allowed,
        "V-DET-20: {} exceedances > {allowed} allowed: {exceedances:#?}",
        exceedances.len()
    );
}

/// V-DET-20, PR tier.
#[test]
fn v_det_20_pr_tier() {
    check(PR_DRAWS);
}

/// V-DET-20, full sweep (10⁶ draws per stream; `docs/09`). Nightly or on demand.
#[test]
#[ignore = "full V-DET-20 sweep: run with --run-ignored only (nightly)"]
fn v_det_20_full_sweep() {
    check(FULL_DRAWS);
}

/// The harness itself must catch dependence (a positive control): copies, shifted copies and
/// complements of one stream, and a biased stream, all fail.
#[test]
fn harness_detects_planted_dependence() {
    let key = RngKey {
        world_seed: 1,
        branch_salt: CANONICAL_SALT,
        system: SystemId::from_raw(1),
        purpose: PurposeId::of("control"),
        entity: 1,
        epoch: 0,
        t: 0,
    };
    let mut s = RngStream::new(&key);
    let a: Vec<u64> = (0..PR_DRAWS).map(|_| s.next_u64()).collect();
    let chi_hi = chi_square_quantile(CHI_DF, Z_BAND);
    // An identical stream: correlation 1 and every xor byte zero.
    assert!(correlation_z(&a, &a) > Z_BAND);
    assert!(chi_square(&byte_counts(a.iter().map(|x| x ^ x))) > chi_hi);
    // A complemented stream: correlation -1.
    let complement: Vec<u64> = a.iter().map(|x| !x).collect();
    assert!(correlation_z(&a, &complement) < -Z_BAND);
    // A stream that differs only in its low byte: the joint top nibbles are on the diagonal.
    let low_byte: Vec<u64> = a.iter().map(|x| x ^ 0xFF).collect();
    assert!(chi_square(&joint_nibbles(&a, &low_byte)) > chi_hi);
    // A shifted copy, b_i = a_(i+1): the signs of a_i - a_(i+1) mark descents, and a descent is
    // followed by an ascent with probability 2/3, not 1/2, so there are too many runs.
    let shifted: Vec<u64> = a[1..].iter().copied().chain([a[0]]).collect();
    assert!(runs_z(&a, &shifted).abs() > Z_BAND);
    // A biased stream: every byte's top bit cleared.
    let biased = a.iter().map(|x| x & 0x7F7F_7F7F_7F7F_7F7F);
    assert!(chi_square(&byte_counts(biased)) > chi_hi);
}

#[test]
fn band_constants_are_sound() {
    // Wilson–Hilferty against the tabulated median of chi2(255) (254.33) and the binomial
    // allowance against its definition.
    assert!((chi_square_quantile(CHI_DF, 0.0) - 254.33).abs() < 0.1);
    let lo = chi_square_quantile(CHI_DF, -Z_BAND);
    let hi = chi_square_quantile(CHI_DF, Z_BAND);
    assert!(lo > 180.0 && lo < 200.0, "{lo}");
    assert!(hi > 325.0 && hi < 340.0, "{hi}");
    assert_eq!(binomial_allowance(1, 0.5), 1);
    assert!(binomial_allowance(PAIRS * STATS_PER_PAIR, ALPHA) >= 6);
}
