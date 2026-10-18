//! SPR-017 T-06: the Registry's effect-language sections, the real environment and the
//! Registry's refusals, each planted on a copy of the real Registry or spec.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::path::Path;

use rh_manifest::Registry;
use rh_manifest::effect_registry::{compile_programs, effect_env, effect_problems};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn registry_text() -> String {
    std::fs::read_to_string(root().join("specs/manifest/registry_v0.toml")).unwrap()
}

fn specs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let dir = root().join("specs/systems");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".odd.toml") && !n.starts_with('_'))
        .collect();
    names.sort();
    for n in names {
        let text = std::fs::read_to_string(dir.join(&n)).unwrap();
        out.push((n, text));
    }
    out
}

#[test]
fn the_real_programs_compile() {
    let reg = Registry::from_toml(&registry_text()).unwrap();
    let env = effect_env(&reg, &registry_text(), &specs()).unwrap_or_else(|e| panic!("{e:#?}"));
    let programs = compile_programs(&reg, &env).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(programs.laws.len(), 7);
    assert_eq!(programs.count(), 21);
    assert!(effect_problems(&reg, &registry_text(), &specs()).is_empty());
}

/// `text` with `old` (which must occur exactly once) replaced by `new`.
fn edit(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "plant anchor {old:?}");
    text.replacen(old, new, 1)
}

/// Every problem a Registry text has: its load error, else its effect problems against `specs`.
fn problems_with(text: &str, specs: &[(String, String)]) -> Vec<String> {
    match Registry::from_toml(text) {
        Err(rh_manifest::RegistryError::Inconsistent(p)) => p,
        Err(e) => vec![e.to_string()],
        Ok(reg) => effect_problems(&reg, text, specs),
    }
}

fn problems(text: &str) -> Vec<String> {
    problems_with(text, &specs())
}

/// The plant is refused, and some problem contains every one of `needles`.
fn assert_refused(found: &[String], needles: &[&str]) {
    assert!(
        found.iter().any(|p| needles.iter().all(|n| p.contains(n))),
        "no problem names {needles:?}: {found:#?}"
    );
}

const PARTITION_ELIGIBLE: &str = "[succession_laws.partition]\ncandidates = 'children(holder)'\neligible   = 'person.alive(x) and x != holder'";
const VERBS_END: &str = "EXTINCTION         = { act_classes = [\"death\"], emitted_by = \"M2.demography\", tellability = 255 }";

#[test]
fn a_program_table_for_an_unknown_id_is_refused() {
    let t = edit(
        &registry_text(),
        "[succession_laws.ultimogeniture]",
        "[succession_laws.primogeniture_open]\ncandidates = 'children(holder)'\neligible = 'true'\norder = 'life_course.birth_tick(x)'\n\n[succession_laws.ultimogeniture]",
    );
    assert_refused(
        &problems(&t),
        &[
            "succession_laws.primogeniture_open",
            "not in succession_laws.ids",
        ],
    );
}

#[test]
fn a_law_without_its_order_is_refused() {
    let t = edit(
        &registry_text(),
        "order      = '(count(y in line_agnatic(common_agnatic(x, holder), x)) - up_agnatic(x, holder), life_course.birth_tick(x))'\n",
        "",
    );
    assert_refused(
        &problems(&t),
        &["succession_laws.seniority", "no order program"],
    );
}

#[test]
fn an_unknown_field_in_a_law_table_is_refused() {
    let t = edit(
        &registry_text(),
        PARTITION_ELIGIBLE,
        &format!("{PARTITION_ELIGIBLE}\nvariant    = 'equal'"),
    );
    assert_refused(&problems(&t), &["unknown field `variant`"]);
}

#[test]
fn a_language_version_the_evaluator_lacks_is_refused() {
    let t = edit(
        &registry_text(),
        "version = \"0.5.1\"",
        "version = \"0.6.0\"",
    );
    assert_refused(&problems(&t), &["effect_language.version '0.6.0'", "0.5.1"]);
}

#[test]
fn an_undeclared_read_is_refused_naming_the_law() {
    let t = edit(
        &registry_text(),
        PARTITION_ELIGIBLE,
        "[succession_laws.partition]\ncandidates = 'children(holder)'\neligible   = 'person.alive(x) and x != holder and person.residence(x) == 0'",
    );
    assert_refused(
        &problems(&t),
        &["succession_laws.partition.eligible", "E_UNDECLARED_READ"],
    );
}

#[test]
fn a_rate_read_without_its_neutral_is_refused() {
    let law = "[succession_laws.partition]\ncandidates = 'children(holder)'\neligible   = 'person.alive(x) and x != holder and polity.exec_constraint(none) >= 0.0'";
    let t = edit(&registry_text(), PARTITION_ELIGIBLE, law);
    // Against the real M7 the law compiles: the read carries `neutral = 0.0`.
    assert_eq!(problems(&t), Vec::<String>::new());
    let specs: Vec<(String, String)> = specs()
        .into_iter()
        .map(|(name, text)| {
            if name.starts_with("M7.") {
                let text = edit(
                    &text,
                    "lag = \"lagged\"\nuse = \"rate\"\nneutral = 0.0",
                    "lag = \"lagged\"\nuse = \"rate\"",
                );
                (name, text)
            } else {
                (name, text)
            }
        })
        .collect();
    assert_refused(
        &problems_with(&t, &specs),
        &["succession_laws.partition.eligible", "E_NO_NEUTRAL"],
    );
}

#[test]
fn an_enum_default_outside_its_values_is_refused() {
    let t = edit(
        &registry_text(),
        "title_tiers       = { registry = \"title_tiers\", default = \"minor\" }",
        "title_tiers       = { registry = \"title_tiers\", default = \"baron\" }",
    );
    assert_refused(
        &problems(&t),
        &["effect_language.enums.title_tiers", "default 'baron'"],
    );
}

#[test]
fn a_constant_death_cause_none_to_schedule_death_is_refused() {
    let verb = "FIXTURE_SLAY = { act_classes = [\"harm\"], emitted_by = \"T1.minds\", tellability = 1, payload = { success = \"bool\" }, effects = 'when success: ScheduleDeath(person: target, cause: death_cause.none, culprit: actor, cause_event: event)' }";
    let t = edit(&registry_text(), VERBS_END, &format!("{VERBS_END}\n{verb}"));
    assert_refused(
        &problems(&t),
        &["verbs.FIXTURE_SLAY", "death_cause.none", "(Q13)"],
    );
    // The same program with a real cause compiles.
    let t = edit(
        &registry_text(),
        VERBS_END,
        &format!(
            "{VERBS_END}\n{}",
            verb.replace("death_cause.none", "death_cause.murder")
        ),
    );
    assert_eq!(problems(&t), Vec::<String>::new());
}

#[test]
fn effects_need_a_registry_system_as_emitter() {
    let verb = "FIXTURE_RESIGN = { act_classes = [\"harm\"], emitted_by = \"rh-minds\", tellability = 1, effects = 'Resign(person: actor, office: none)' }";
    let t = edit(&registry_text(), VERBS_END, &format!("{VERBS_END}\n{verb}"));
    assert_refused(
        &problems(&t),
        &["verbs.FIXTURE_RESIGN", "'rh-minds'", "(Q8)"],
    );
    // Without effects, a non-system emitter stays allowed (the seven rh-minds verbs; Q8 scoped).
    assert!(Registry::from_toml(&registry_text()).is_ok());
}

#[test]
fn two_verb_programs_of_one_system_sharing_a_draw_site_are_refused() {
    let program = "'when chance(attack_outcome, actor, target, params.attempt_success): Resign(person: actor, office: none)'";
    let verbs = format!(
        "{VERBS_END}\nFIXTURE_A = {{ act_classes = [\"harm\"], emitted_by = \"T1.minds\", tellability = 1, effects = {program} }}\nFIXTURE_B = {{ act_classes = [\"harm\"], emitted_by = \"T1.minds\", tellability = 1, effects = {program} }}"
    );
    let t = edit(&registry_text(), VERBS_END, &verbs);
    assert_refused(
        &problems(&t),
        &["verbs.FIXTURE_A", "verbs.FIXTURE_B", "E_RNG_REUSE"],
    );
}

#[test]
fn two_laws_sharing_a_draw_site_compile() {
    let t = edit(
        &registry_text(),
        "order      = '(min(t in titles(x): ordinal(title.tier(t)) else 3), life_course.birth_tick(x))'",
        "order      = 'draw(sortition, x, title)'",
    );
    assert_eq!(problems(&t), Vec::<String>::new());
}

fn golden_registry() -> toml::Table {
    let text =
        std::fs::read_to_string(root().join("specs/registry/effect_language_golden_v0.toml"))
            .unwrap();
    let golden: toml::Table = toml::from_str(&text).unwrap();
    golden["registry"].as_table().unwrap().clone()
}

/// D37: the Registry's 21 law texts are the golden file's, byte for byte.
#[test]
fn the_law_texts_are_the_golden_files() {
    let reg = Registry::from_toml(&registry_text()).unwrap();
    let golden = golden_registry();
    let laws = golden["programs"]["succession_laws"].as_table().unwrap();
    let mut compared = 0;
    for id in &reg.succession_laws.ids {
        let ours = &reg.succession_laws.programs[id];
        let theirs = laws[id].as_table().unwrap();
        for (name, text) in [
            ("candidates", &ours.candidates),
            ("eligible", &ours.eligible),
            ("order", &ours.order),
        ] {
            assert_eq!(
                text.as_deref(),
                theirs[name].as_str(),
                "succession_laws.{id}.{name}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 21);
}

/// §3.1: every enum the real environment shares with the golden slice has its values, in its
/// order, and its default. `vow_kinds` and `opinion_modifiers` are keyed tables whose source
/// order is not sorted, so a loader that took `toml::Table`'s order would fail here.
#[test]
fn the_real_enums_are_the_golden_slices() {
    let reg = Registry::from_toml(&registry_text()).unwrap();
    let env = effect_env(&reg, &registry_text(), &specs()).unwrap();
    let golden = golden_registry();
    let mut shared = Vec::new();
    for (name, def) in golden["enums"].as_table().unwrap() {
        let Some(id) = env.enum_id(name) else {
            continue;
        };
        let ours = env.enum_def(id).unwrap();
        let values: Vec<&str> = def["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(ours.values, values, "enum {name}: values");
        assert_eq!(
            ours.values[ours.default as usize],
            def["default"].as_str().unwrap(),
            "enum {name}: default"
        );
        shared.push(name.as_str());
    }
    for keyed in ["vow_kinds", "opinion_modifiers"] {
        assert!(shared.contains(&keyed), "{keyed} not compared");
        let order = &rh_manifest::effect_registry::key_order(&registry_text()).unwrap()[keyed];
        let mut sorted = order.clone();
        sorted.sort();
        assert_ne!(
            order, &sorted,
            "{keyed}: the source order must not be sorted for this test to bite"
        );
    }
    assert!(shared.len() >= 13, "only {shared:?} compared");
}
