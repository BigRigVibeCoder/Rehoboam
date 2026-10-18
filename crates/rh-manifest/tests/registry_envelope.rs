//! SPR-020 T-01: the envelope sections of the Registry (`envelope_v0.md` §8; BCK-002 A-24, the
//! envelope part). The real Registry passes, `attr_id` is the index in the name-sorted
//! `[attributes]` (`stress` is 32 on today's Registry), and one planted Registry per refusal of §8
//! is refused by its named message. The plants are text edits of the real Registry, in
//! `tests/lint_fixtures/registry_envelope/plants.toml`, which the Python gate
//! (`specs/manifest/registry_envelope_plants.py`) runs through `lint_patterns.py` too.
#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "tests: a failure must stop the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_manifest::{Registry, RegistryError};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry_text() -> String {
    std::fs::read_to_string(repo().join("specs/manifest/registry_v0.toml"))
        .expect("the Registry reads")
}

#[test]
fn the_real_registry_types_the_envelope_sections() {
    let reg = Registry::from_toml(&registry_text()).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        reg.event_roles.ids,
        ["actor", "target", "witness", "instrument"]
    );
    assert_eq!(reg.role_id("target"), Some(1));
    assert_eq!(reg.motive_edges.ids.len(), 9);
    assert_eq!(reg.motive_index("FEAR_OF"), Some(4));
    assert_eq!(reg.motive_index("CORRUPTION_BY"), Some(8));
    assert_eq!(
        reg.role_domains["authority"].column, "title.holder",
        "the typed [role_domains]"
    );
    assert!(
        !reg.sections.contains_key("event_roles") && !reg.sections.contains_key("role_domains"),
        "the typed sections leave the generic map"
    );
    // envelope §6: stress moved from 22 to 32 with the ten domain attributes.
    assert_eq!(reg.attr_id("stress"), Some(32));
    assert_eq!(reg.attr_id("no_such_attribute"), None);
    // The index is a function of the sorted names: every attribute's id is the number of names
    // that sort before it, byte-wise.
    let names: Vec<&String> = reg.attributes.keys().collect();
    for name in &names {
        let below = names
            .iter()
            .filter(|n| n.as_bytes() < name.as_bytes())
            .count();
        assert_eq!(reg.attr_id(name).map(usize::from), Some(below), "{name}");
    }
    // An attribute that sorts first moves every id up by one, and no name.
    let added = registry_text().replace(
        "[attributes]",
        "[attributes]\naaa_first = { type = \"bool\", args = [\"actor\"] }",
    );
    let grown = Registry::from_toml(&added).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(grown.attr_id("aaa_first"), Some(0));
    assert_eq!(grown.attr_id("stress"), Some(33));
}

#[derive(serde::Deserialize)]
struct Plants {
    plant: Vec<Plant>,
}

#[derive(serde::Deserialize)]
struct Plant {
    name: String,
    find: String,
    replace: String,
    rust: String,
}

#[test]
fn every_planted_registry_is_refused_by_name() {
    let text = registry_text();
    let plants: Plants = toml::from_str(
        &std::fs::read_to_string(repo().join("tests/lint_fixtures/registry_envelope/plants.toml"))
            .expect("the plants read"),
    )
    .expect("the plants parse");
    assert!(plants.plant.len() >= 15, "every refusal of §8 is planted");
    for p in &plants.plant {
        assert_eq!(
            text.matches(&p.find).count(),
            1,
            "{}: the edit's text occurs once",
            p.name
        );
        let planted = text.replacen(&p.find, &p.replace, 1);
        let message = match Registry::from_toml(&planted) {
            Ok(_) => panic!("{}: the planted Registry loads", p.name),
            Err(RegistryError::Inconsistent(problems)) => problems.join("\n"),
            Err(e) => e.to_string(),
        };
        assert!(
            message.contains(&p.rust),
            "{}: refused for another reason: {message}",
            p.name
        );
    }
}
