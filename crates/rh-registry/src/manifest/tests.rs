//! Unit tests for manifest loading. The live comparison with the Python reference is
//! `bin/manifest_parity.py` (V-WC-04); these pin the same expectations inside `cargo nextest`.

use std::path::{Path, PathBuf};

use super::*;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry() -> Registry {
    Registry::load(&repo().join("specs/manifest/registry_v0.toml")).unwrap()
}

fn example(name: &str) -> Outcome {
    validate_file(
        &repo()
            .join("specs/manifest/examples")
            .join(format!("{name}.yaml")),
        &registry(),
    )
}

/// V-WC-04: verdicts and counts the Python reference gives on every example manifest.
#[test]
fn examples_match_the_reference_verdicts() {
    let expected = [
        ("solo", Verdict::Valid, 0, 0),
        ("val_khor", Verdict::Valid, 0, 1),
        ("aethelgard", Verdict::Valid, 0, 0),
        ("uz_job", Verdict::Valid, 0, 0),
        ("invalid_draft_style", Verdict::Invalid, 8, 3),
    ];
    for (name, verdict, errors, warnings) in expected {
        let o = example(name);
        assert_eq!(o.verdict(), verdict, "{name}:\n{}", o.render(name));
        assert_eq!(
            (o.error_count(), o.warning_count()),
            (errors, warnings),
            "{name}:\n{}",
            o.render(name)
        );
    }
}

/// The reference's exact lines for the invalid example (`manifest_v0.py` output, 2026-10-16).
#[test]
fn invalid_example_messages_match_the_reference() {
    let want = "\
[ERROR] world.horizon: template 'modern_metropolis' supports ≤50 historical years; set horizon.label='speculative' (and declare a tech/institution trajectory) for 1000
[ERROR] systems.M5.contagion.r0: value 12.0 outside Registry bounds [1.0, 8.0]
[ERROR] systems.M9.free_strings: unknown system
[ERROR] world.fidelity_profile.M5.contagion: fidelity F3 not implemented (available: ['F0', 'F1'])
[ERROR] geography.climate.harvest_sigma: value 0.5 outside Registry bounds [0.04, 0.35]
[ERROR] polities[0].titles: missing required role 'successor'
[WARNING] catalysts[0]: covert outbreak without a testing technology: detection only via behavioural tells
[WARNING] catalysts[0].seed_infected: seed >1% of population: the 'covert' phase may be skipped
[ERROR] systems: creature peoples require system S4.ecology (prey, grazing, predator energetics)
[WARNING] peoples[1].params.mass_kg: 5000 kg is beyond the 0.14–310 kg range behind the Carbone ratio; the energetics bound is extrapolated (metabolism_multiplier = field metabolic rate relative to a mammalian carnivore of equal mass)
[ERROR] peoples[1]: energetics: 45×5000 kg = 225 t of predator, but standing prey 15000 t supports only 135 t at metabolism ×1.0 (Carbone ratio 0.009). Lower population, lower metabolism_multiplier, or declare wild_prey_tonnes.
INVALID: x (8 errors, 3 warnings) — next gate: liveness burn-in (engine)
";
    assert_eq!(example("invalid_draft_style").render("x"), want);
}

#[test]
fn exit_codes_follow_the_reference() {
    assert_eq!(example("solo").exit_code(), 0);
    assert_eq!(example("invalid_draft_style").exit_code(), 1);
    assert_eq!(validate_str("[]", &registry()).exit_code(), 2);
    assert_eq!(validate_str("a: [", &registry()).exit_code(), 1);
}

fn schema_errors(text: &str) -> Vec<(String, &'static str)> {
    match validate_str(text, &registry()) {
        Outcome::SchemaInvalid(e) => e.into_iter().map(|e| (e.loc.join("."), e.kind)).collect(),
        other => panic!("expected SCHEMA INVALID, got {other:?}"),
    }
}

/// Error lists observed from Pydantic 2.14 on the same inputs (see the parity corpus).
#[test]
fn schema_errors_are_counted_like_pydantic() {
    let empty = schema_errors("{}");
    assert_eq!(empty.len(), 8);
    assert!(empty.iter().all(|(_, k)| *k == "missing"));
    assert_eq!(schema_errors("[]"), vec![(String::new(), "model_type")]);

    let unions = schema_errors(
        "peoples: [{population: 5}, {archetype: elf}, 5, {archetype: human, extra: 1}]\n\
         catalysts: [{kind: artifact, effects: []}]\n\
         geography: {climate: {zone: x}, heightmap_template: flat, raster_resolution: 100}\n",
    );
    let kinds: Vec<&str> = unions.iter().map(|(_, k)| *k).collect();
    assert_eq!(
        kinds,
        [
            "missing",
            "missing",
            "missing",
            "literal_error",
            "string_pattern_mismatch",
            "union_tag_not_found",
            "union_tag_invalid",
            "model_attributes_type",
            "missing",
            "missing",
            "missing",
            "missing",
            "extra_forbidden",
            "missing",
            "missing",
            "missing",
            "missing",
            "too_short",
            "missing",
            "missing",
        ]
    );
    assert_eq!(unions[12].0, "peoples.3.human.extra");

    let after = schema_errors(
        "peoples: [{archetype: human, population: 0, cultures: [], faiths: [], \
         class_shares: {commoner: 0.5, burgher: 0.1, elite: 0.1}}]\n\
         world: {id: ab, display_name: x, template: ab, seed: 1, horizon: {years: 10, label: speculative}}\n",
    );
    assert_eq!(after.len(), 11);
    assert!(after.contains(&("world".to_owned(), "value_error")));
    assert!(after.contains(&("peoples.0.human.class_shares".to_owned(), "value_error")));
}

#[test]
fn lax_coercion_matches_pydantic() {
    // A valid manifest whose numbers arrive as strings and integral floats, as Pydantic accepts.
    let base = std::fs::read_to_string(repo().join("specs/manifest/examples/solo.yaml")).unwrap();
    let text = base
        .replace("seed: 1\n", "seed: \"1\"\n")
        .replace("population: 1\n", "population: 1.0\n");
    assert_eq!(validate_str(&text, &registry()).verdict(), Verdict::Valid);
    let bad = base.replace("population: 1\n", "population: 1.5\n");
    let Outcome::SchemaInvalid(e) = validate_str(&bad, &registry()) else {
        panic!("accepted 1.5 people")
    };
    assert_eq!(e[0].kind, "int_from_float");
}
