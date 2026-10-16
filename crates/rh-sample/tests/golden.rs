//! V-DET-09: the samplers' golden vectors, bit-equal across releases, machines and opt-levels.
//!
//! `tests/golden/samplers_v1.txt` pins, for every sampler and a spread of parameters (each method,
//! both orientations, the degenerate and saturating cases), the first `DRAWS_PER_KEY` samples of
//! several keyed streams and the stream position after them (so the number of draws consumed is
//! pinned too). Values are integers: counts, or `Q16_16` raw bits.
//!
//! The file is the Rust output itself, recorded once: no independent implementation can reproduce a
//! float algorithm's exact bits (correctness comes from `stats.rs` instead). Any change to it is a
//! C3 change (`docs/13` §4.2) and needs its own justification; it is regenerated, never edited, with
//! `RH_SAMPLE_GOLDEN_WRITE=1 cargo test -p rh-sample --test golden`. `RH_SAMPLE_GOLDEN=<path>` checks
//! another file instead (the positive control: `tests/lint_fixtures/golden/altered_samplers_v1.txt`,
//! one altered value, must fail).
//!
//! T-05: `bin/check_all.sh` runs this test at opt-level 0 (the dev profile) and 3 (`--release`),
//! and CI runs it on GitHub's runner: all must match the one committed file.
//!
//! REF: SPR-003 T-01..T-05; ADR-0006 item 3; `docs/09` V-DET-09.

#![allow(
    clippy::expect_used,
    reason = "test helpers: an unwritable or malformed golden file must fail the test loudly"
)]

use std::fmt::Write as _;

use rh_core::fixed::Q16_16;
use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};
use rh_sample::{Beta, Binomial, Gamma, Normal, Poisson};

const GOLDEN: &str = "tests/golden/samplers_v1.txt";
const KEYS: u64 = 4;
const DRAWS_PER_KEY: usize = 12;

fn stream(entity: u64) -> RngStream {
    RngStream::new(&RngKey {
        world_seed: 0x601D,
        branch_salt: BranchSalt::from_raw(0),
        system: SystemId::from_raw(3),
        purpose: PurposeId::of("sampler_golden_vectors"),
        entity,
        epoch: 0,
        t: 7,
    })
}

fn q(bits: i32) -> Q16_16 {
    Q16_16::from_bits(bits)
}

const ONE: i32 = 1 << 16;

/// One golden case: its label (sampler and parameter bits) and a function drawing one sample as an
/// integer.
type Case = (String, Box<dyn Fn(&mut RngStream) -> i64>);

fn cases() -> Vec<Case> {
    let mut out: Vec<Case> = Vec::new();
    for (n, p) in [
        (10, 0),
        (10, ONE),
        (0, ONE / 2),
        (30, 6554),
        (5, 58982),
        (20, ONE / 2),
        (1000, 49152),
        (100_000, 19661),
        (u32::MAX, 1),
        (u32::MAX, ONE / 2),
    ] {
        let b = Binomial::new(n, q(p)).expect("binomial");
        out.push((
            format!("binomial {n} {p}"),
            Box::new(move |s| i64::from(b.sample(s))),
        ));
    }
    for lam in [
        0,
        1,
        ONE / 2,
        3 * ONE,
        10 * ONE - 1,
        10 * ONE,
        300 * ONE,
        i32::MAX,
    ] {
        let p = Poisson::new(q(lam)).expect("poisson");
        out.push((
            format!("poisson {lam}"),
            Box::new(move |s| i64::from(p.sample(s))),
        ));
    }
    for (mean, sd) in [
        (0, ONE),
        (5 * ONE, 3 * ONE),
        (-1000 * ONE, ONE / 8),
        (7, 0),
        (32_000 * ONE, 1000 * ONE),
    ] {
        let d = Normal::new(q(mean), q(sd)).expect("normal");
        out.push((
            format!("normal {mean} {sd}"),
            Box::new(move |s| i64::from(d.sample(s).to_bits())),
        ));
    }
    for (shape, scale) in [
        (1, ONE),
        (ONE / 4, 2 * ONE),
        (ONE, ONE),
        (5 * ONE / 2, ONE / 2),
        (30 * ONE, 3 * ONE),
        (30_000 * ONE, 30_000 * ONE),
    ] {
        let d = Gamma::new(q(shape), q(scale)).expect("gamma");
        out.push((
            format!("gamma {shape} {scale}"),
            Box::new(move |s| i64::from(d.sample(s).to_bits())),
        ));
    }
    for (a, b) in [
        (1, 1),
        (ONE / 2, ONE / 2),
        (2 * ONE, 5 * ONE),
        (1000 * ONE, 1),
        (ONE / 4, 3 * ONE),
    ] {
        let d = Beta::new(q(a), q(b)).expect("beta");
        out.push((
            format!("beta {a} {b}"),
            Box::new(move |s| i64::from(d.sample(s).to_bits())),
        ));
    }
    out
}

/// The golden file's text: one line per (case, key): `<case> key=<k> pos=<draws> <values...>`.
fn render() -> String {
    let mut text = String::from(
        "# rh-sample golden vectors (V-DET-09; SPR-003). Recorded from the Rust samplers; never edit.\n\
         # <sampler> <parameter bits...> key=<entity> pos=<draws consumed> <first samples...>\n\
         # Counts are integers; normal, gamma and beta values are Q16_16 raw bits.\n",
    );
    for (label, draw) in cases() {
        for entity in 0..KEYS {
            let mut s = stream(entity);
            let values: Vec<String> = (0..DRAWS_PER_KEY)
                .map(|_| draw(&mut s).to_string())
                .collect();
            writeln!(
                text,
                "{label} key={entity} pos={} {}",
                s.position(),
                values.join(" ")
            )
            .expect("writing to a String");
        }
    }
    text
}

#[test]
fn samplers_reproduce_the_golden_vectors() {
    let path = std::env::var_os("RH_SAMPLE_GOLDEN").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN),
        std::path::PathBuf::from,
    );
    let text = render();
    if std::env::var_os("RH_SAMPLE_GOLDEN_WRITE").is_some() {
        std::fs::write(&path, &text).expect("golden file is writable");
    }
    let golden = std::fs::read_to_string(&path).expect("golden file is readable");
    for (want, got) in golden.lines().zip(text.lines()) {
        assert_eq!(got, want, "golden vector differs ({})", path.display());
    }
    assert_eq!(
        golden.lines().count(),
        text.lines().count(),
        "golden line count"
    );
    let vectors = text.lines().filter(|l| !l.starts_with('#')).count();
    println!(
        "V-DET-09: {vectors} golden vectors of {DRAWS_PER_KEY} samples match ({})",
        path.display()
    );
}
