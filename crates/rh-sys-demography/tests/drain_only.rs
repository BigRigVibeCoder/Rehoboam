//! M2's drain-only binding against its spec (the review's K6; SPR-014 A-60 part).
//!
//! `Demography::access()` declares `Day`/`Vital` drain-only, so `rh-core` guards an empty drain by
//! its row counts alone. That holds only while the spec's `Day`/`Vital` `[[schedule]]` entry has
//! one step, the `lifecycle:` drain. A spec that gives the entry another step (SPR-015's F1 may)
//! fails here until the code drops the declaration on purpose. The same check on a copy of the
//! spec with a second step must fail (the control).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests: a broken fixture must fail loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::fixed::Q16_16;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_registry::ownership::SystemSpec;
use rh_sys_demography::Demography;

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);

fn spec_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/systems/M2.demography.odd.toml")
}

/// Every drain-only binding of M2's code must be a spec entry whose only step is the drain.
fn check(spec: &SystemSpec) -> Result<usize, String> {
    let m2 = Demography::new(Q16_16::from_int(30), None, Q16_16::ONE, 10_000_000, Q16_16::ONE, CALENDAR).unwrap();
    let declared = m2.access().drain_only;
    for b in &declared {
        let (pulse, phase) = (b.pulse.name(), b.phase.name());
        let i = spec
            .schedule
            .iter()
            .position(|(p, f)| p == pulse && f == phase)
            .ok_or_else(|| format!("{pulse}/{phase} is drain-only but not in the spec"))?;
        let steps = &spec.schedule_steps[i];
        if steps.len() != 1 || !steps[0].starts_with("lifecycle:") {
            return Err(format!(
                "{pulse}/{phase} is declared drain-only, but its [[schedule]] entry has {} steps \
                 ({:?}): drop the declaration in Demography::access or keep the drain alone",
                steps.len(),
                steps
                    .iter()
                    .map(|s| s.split(':').next().unwrap_or(""))
                    .collect::<Vec<_>>()
            ));
        }
    }
    Ok(declared.len())
}

#[test]
fn m2s_drain_only_binding_has_the_drain_as_its_only_step() {
    let path = spec_path();
    let text = std::fs::read_to_string(&path).unwrap();
    let spec = SystemSpec::from_toml(&path, &text).unwrap();
    let n = check(&spec).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(n, 1, "M2 declares Day/Vital drain-only");
    // The control: the same entry with a second step fails the check.
    let marker = "  \"lifecycle: the drain of M2's command queue";
    assert_eq!(text.matches(marker).count(), 1, "the drain step, once");
    let planted = text.replacen(
        marker,
        &format!("  \"census: a planted step\",\n{marker}"),
        1,
    );
    let refused = check(&SystemSpec::from_toml(&path, &planted).unwrap()).unwrap_err();
    assert!(
        refused.starts_with("Day/Vital is declared drain-only, but its [[schedule]] entry has 2"),
        "{refused}"
    );
    println!(
        "drain-only: M2's Day/Vital entry has the drain as its only step; a planted second step fails"
    );
}
