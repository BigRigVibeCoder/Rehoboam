//! `rh_manifest::compile`: manifests into the plane-A `WorldSpec` (SPR-011 T-04, rulings 3, 8, 9,
//! 10; `05.5` §3.2). The reference worlds compile; every refusal names its path.
#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "tests: a failure must stop the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::fixed::Q16_16;
use rh_core::register::system_id;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_manifest::manifest::{Outcome, Verdict, validate_str};
use rh_manifest::{CompileErrors, Registry, compile, compile_file};
use rh_registry::{FounderRole, HouseholdSpec, Sex, WorldSpec};

const BUILT: [&str; 1] = ["M2.demography"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry() -> Registry {
    Registry::load(&repo().join("specs/manifest/registry_v0.toml")).expect("the Registry loads")
}

fn example(name: &str) -> Result<WorldSpec, CompileErrors> {
    let path = repo().join(format!("specs/manifest/examples/{name}.yaml"));
    compile_file(&path, &registry(), &BUILT)
}

/// Compiles a manifest text, which must validate first.
fn text(manifest: &str, reg: &Registry) -> Result<WorldSpec, CompileErrors> {
    match validate_str(manifest, reg) {
        Outcome::Checked { manifest, .. } => compile(&manifest, reg, &BUILT),
        other => panic!("{:?}: {}", other.verdict(), other.render("test")),
    }
}

fn solo_text() -> String {
    std::fs::read_to_string(repo().join("specs/manifest/examples/solo.yaml")).unwrap()
}

fn roles(spec: &WorldSpec) -> Vec<(FounderRole, usize)> {
    let mut out: Vec<(FounderRole, usize)> = Vec::new();
    for f in &spec.founders {
        match out.last_mut() {
            Some((r, n)) if *r == f.role => *n += 1,
            _ => out.push((f.role, 1)),
        }
    }
    out
}

#[test]
fn solo_compiles_with_the_registry_defaults() {
    let spec = example("solo").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(spec.seed, 1);
    assert_eq!(
        spec.calendar,
        Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four)
    );
    assert_eq!(spec.horizon_years, 80);
    let demography = spec.demography.expect("M2 is built and named");
    assert_eq!(demography.e0, Q16_16::from_int(30));
    assert_eq!(demography.brass_beta, Q16_16::ONE);
    assert_eq!(spec.founders.len(), 1);
    assert_eq!(spec.founders[0].sex, Some(Sex::Female));
    assert_eq!(spec.founders[0].age_years, Some(25));
    // M2 is built; M4 and S5 are named by the fidelity profile but not built (ruling 3).
    for (id, built) in [
        ("M2.demography", true),
        ("M4.economy", false),
        ("S5.daily_life", false),
    ] {
        let named = spec.systems.iter().find(|s| s.id == system_id(id));
        assert_eq!(named.map(|s| s.built), Some(built), "{id}");
    }
    assert!(
        spec.systems.windows(2).all(|w| w[0].id < w[1].id),
        "ascending SystemId"
    );
}

#[test]
fn uz_job_household_and_hamlet_compile_with_their_founders_in_genesis_order() {
    let uz = example("uz_job").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(uz.demography.map(|d| d.e0), Some(Q16_16::from_int(30)));
    assert_eq!(
        roles(&uz),
        [
            (FounderRole::Cast, 6),
            (FounderRole::Child, 10),
            (FounderRole::Servant, 30),
            (FounderRole::Commoner, 14),
        ]
    );
    let children: Vec<_> = uz.founders[6..16].iter().map(|f| f.sex).collect();
    assert_eq!(children[..7], [Some(Sex::Male); 7]);
    assert_eq!(children[7..], [Some(Sex::Female); 3]);
    // the couple, their three children and the unrelated founder Wulf (SPR-015, REV-SPR-015 R8)
    let household = example("household").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        roles(&household),
        [(FounderRole::Cast, 3), (FounderRole::Child, 3)]
    );
    let hamlet = example("hamlet").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(roles(&hamlet), roles(&uz));
    assert_eq!(
        hamlet.demography.map(|d| (d.e0, d.brass_beta)),
        uz.demography.map(|d| (d.e0, d.brass_beta))
    );
    // uz_job authors tfr 6.0; hamlet takes the Registry default 4.9 (SPR-015 F1).
    assert_eq!(
        uz.demography.map(|d| d.tfr),
        Some(Q16_16::from_bits(393_216))
    );
    assert_eq!(
        hamlet.demography.map(|d| d.tfr),
        Some(Q16_16::from_bits(321_126))
    );
}

#[test]
fn manifest_households_resolve_to_founder_indices() {
    let household = example("household").unwrap_or_else(|e| panic!("{e}"));
    // Oswin (head) and Maren (spouse) are cast members 0 and 1; the three children follow, then
    // Wulf alone.
    assert_eq!(
        household.households,
        [
            HouseholdSpec {
                head: 0,
                spouse: Some(1),
                children: 3..6,
                servants: 6..6,
            },
            HouseholdSpec {
                head: 2,
                spouse: None,
                children: 6..6,
                servants: 6..6,
            },
        ]
    );

    let hamlet = example("hamlet").unwrap_or_else(|e| panic!("{e}"));
    // Job (head) and his wife (spouse) are cast members 0 and 1; the ten children are 6..16 and
    // the thirty servants 16..46. The other cast members and the fourteen commoners name no
    // household, so they get one-person households in `rh-worldgen`, not here.
    assert_eq!(
        hamlet.households,
        [HouseholdSpec {
            head: 0,
            spouse: Some(1),
            children: 6..16,
            servants: 16..46,
        }]
    );

    let solo = example("solo").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        solo.households,
        [HouseholdSpec {
            head: 0,
            spouse: None,
            children: 1..1,
            servants: 1..1,
        }]
    );
}

#[test]
fn without_m2_in_the_build_nothing_of_m2_is_compiled() {
    let path = repo().join("specs/manifest/examples/solo.yaml");
    let spec = compile_file(&path, &registry(), &[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(spec.demography, None);
    assert!(spec.systems.iter().all(|s| !s.built));
}

#[test]
fn a_manifest_value_is_compiled_from_its_literal() {
    // 30.00001525878906250 is 30 + 2^-16 exactly; 30.000007629394531250 is a tie (to even: 30).
    for (literal, bits) in [
        ("30.00001525878906250", (30 << 16) + 1),
        ("30.000007629394531250", 30 << 16),
        ("30.0000228881835937500", (30 << 16) + 2),
        ("3_0.5", (30 << 16) + 32_768),
        ("30", 30 << 16),
    ] {
        let manifest = solo_text().replace(
            "systems: {}",
            &format!("systems:\n  M2.demography: {{e0: {literal}}}"),
        );
        let spec = text(&manifest, &registry()).unwrap_or_else(|e| panic!("{literal}: {e}"));
        assert_eq!(
            spec.demography.map(|d| d.e0.to_bits()),
            Some(bits),
            "{literal}"
        );
    }
}

fn refused(manifest: &str, reg: &Registry) -> Vec<String> {
    text(manifest, reg)
        .expect_err("must not compile")
        .0
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn brass_alpha_and_a_slope_other_than_one_are_refused_by_name() {
    let manifest = solo_text().replace(
        "systems: {}",
        "systems:\n  M2.demography: {brass_alpha: 0.1, brass_beta: 1.5}",
    );
    let errors = refused(&manifest, &registry());
    assert!(
        errors
            .iter()
            .any(|e| e.starts_with("systems.M2.demography.brass_alpha: refused until P1.3")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e
            .starts_with("systems.M2.demography.brass_beta: rung 0 runs Brass at beta = 1 only")),
        "{errors:?}"
    );
}

/// The Registry with M2's bounds and defaults replaced.
fn registry_with(m2: &str) -> Registry {
    let text = std::fs::read_to_string(repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let start = text.find("[systems.\"M2.demography\"]").unwrap();
    let end = start + text[start..].find("\n[systems.\"M3").unwrap();
    let replaced = format!(
        "{}[systems.\"M2.demography\"]\n{m2}\n{}",
        &text[..start],
        &text[end..]
    );
    Registry::from_toml(&replaced).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_value_that_does_not_fit_q16_16_is_refused_by_name() {
    let reg = registry_with(
        "fidelity_levels = [\"F0\", \"F1\", \"F2\"]\nparams = { e0 = [0.0, 1.0e9], brass_beta = [0.6, 1.65], tfr = [1.2, 7.5], elite_fertility_mult = [1.0, 1.6] }\ndefaults = { brass_beta = 1.0, tfr = 4.9, elite_fertility_mult = 1.0 }",
    );
    let manifest = solo_text().replace("systems: {}", "systems:\n  M2.demography: {e0: 40000.0}");
    let errors = refused(&manifest, &reg);
    assert_eq!(
        errors,
        [
            "systems.M2.demography.e0: 40000 does not fit a Q16_16 (range [-32768, 32768 - 2^-16], step 2^-16)"
        ]
    );
}

#[test]
fn a_parameter_with_no_value_and_no_default_is_refused_by_name() {
    let reg = registry_with(
        "fidelity_levels = [\"F0\", \"F1\", \"F2\"]\nparams = { e0 = [20.0, 85.0], brass_beta = [0.6, 1.65], tfr = [1.2, 7.5], elite_fertility_mult = [1.0, 1.6] }\ndefaults = { brass_beta = 1.0, tfr = 4.9, elite_fertility_mult = 1.0 }",
    );
    let errors = refused(&solo_text(), &reg);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("systems.M2.demography.e0: a built system reads it, and neither"),
        "{errors:?}"
    );
}

#[test]
fn a_registry_default_is_compiled_from_its_literal() {
    // 30.0000076293945312500001 is just above the tie: it rounds up, where its f64 (30.0000076...)
    // would round to even. The literal decides.
    let reg = registry_with(
        "fidelity_levels = [\"F0\", \"F1\", \"F2\"]\nparams = { e0 = [20.0, 85.0], brass_beta = [0.6, 1.65], tfr = [1.2, 7.5], elite_fertility_mult = [1.0, 1.6] }\ndefaults = { e0 = 30.0000076293945312500001, brass_beta = 1, tfr = 4.9, elite_fertility_mult = 1.0 }",
    );
    let spec = text(&solo_text(), &reg).unwrap_or_else(|e| panic!("{e}"));
    let d = spec.demography.unwrap();
    assert_eq!(d.e0.to_bits(), (30 << 16) + 1);
    assert_eq!(d.brass_beta, Q16_16::ONE);
}

#[test]
fn an_invalid_manifest_is_never_compiled() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("rh-manifest-compile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.yaml");
    std::fs::write(&path, solo_text().replace("seed: 1", "seed: -1")).unwrap();
    let reg = registry();
    let verdict = rh_manifest::manifest::validate_file(&path, &reg).verdict();
    let result = compile_file(&path, &reg, &BUILT);
    std::fs::remove_dir_all(&dir).unwrap();
    if verdict == Verdict::Valid {
        // The schema allows a negative seed: compile itself refuses it.
        let errors = result.expect_err("a negative seed").0;
        assert_eq!(errors[0].path, "world.seed");
    } else {
        assert!(
            result.expect_err("invalid").0[0]
                .message
                .contains("does not validate")
        );
    }
}

/// VER-SPR-011-TEST GAP-9: a literal whose exponent is past the stated bound is refused once at
/// its path, not by the bounds check and again by the conversion.
#[test]
fn an_exponent_past_the_bound_is_reported_once() {
    let reg = registry_with(
        "fidelity_levels = [\"F0\", \"F1\", \"F2\"]\nparams = { e0 = [0.0, 1.0e9], brass_beta = [0.6, 1.65], tfr = [1.2, 7.5], elite_fertility_mult = [1.0, 1.6] }\ndefaults = { brass_beta = 1.0, tfr = 4.9, elite_fertility_mult = 1.0 }",
    );
    let manifest = solo_text().replace(
        "systems: {}",
        "systems:\n  M2.demography: {e0: 1.0e-1048577}",
    );
    let errors = refused(&manifest, &reg);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("systems.M2.demography.e0: ")
            && errors[0].contains("has an exponent beyond"),
        "{errors:?}"
    );
}

#[test]
fn g_2_bounds_are_checked_on_the_exact_decimal_not_the_float() {
    // 85.0000000000000001 is one unit past e0's upper bound (85) in its last digit; as an f64 it
    // is exactly 85.0, so the reference's float check passes it. The compiled value must not.
    for (literal, ok) in [
        ("85.0000000000000001", false),
        ("19.9999999999999999", false),
        ("85", true),
        ("20.0", true),
    ] {
        let manifest = solo_text().replace(
            "systems: {}",
            &format!("systems:\n  M2.demography: {{e0: {literal}}}"),
        );
        let result = text(&manifest, &registry());
        if ok {
            assert!(result.is_ok(), "{literal}: {result:?}");
        } else {
            let errors: Vec<String> = result
                .expect_err("one unit past the bound")
                .0
                .iter()
                .map(ToString::to_string)
                .collect();
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert!(
                errors[0].starts_with("systems.M2.demography.e0: ")
                    && errors[0]
                        .contains(" is outside its Registry bounds [20, 85], compared exactly"),
                "{errors:?}"
            );
        }
    }
}
