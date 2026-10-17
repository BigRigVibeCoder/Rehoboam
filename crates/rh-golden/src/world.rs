//! The worlds a golden run can run (SPR-012 T-08; SPR-011 rulings 3, 15 and 16).
//!
//! - `smoke`: SPR-008's dummy world, exactly as before (its specs and Registry overlay under
//!   `worlds/smoke`, its synthetic inputs, its initial tables). Its ladder does not change.
//! - A **manifest world** `<id>`: `worlds/<id>/world.toml` names the manifest in
//!   `specs/manifest/examples/` and pins its SHA-256 (one copy, no drift; ruling 16), the Registry,
//!   and the committed input script (ruling 15). The runner compiles the manifest against the
//!   Registry with this build's systems ([`BUILT`]), runs `rh-worldgen`'s genesis, and registers
//!   the built systems against `specs/systems` (rule `spec`). A system the manifest names and this
//!   build lacks is `not_built`: skipped, never stubbed, and refused under `--strict` (ruling 3).
//!
//! The history tuple of a manifest world (DET-1): `engine_build` is `rh-golden <version>` with the
//! built set, `registry_version` the Registry's, `manifest_hash` the manifest's SHA-256, the seed
//! the manifest's (or `--seed`), and the journal the input script's rows. The ladder header
//! records the tuple and the built and not-built systems.
//!
//! The input script (`inputs.toml`): `[[input]]` tables with `tick` (minutes) and `founder` (the
//! founder's index in genesis order, whose `PersonId` is `founder(seed, k)` or its rehash). Each is
//! one scope-only material input (ruling 15), journaled before the first instant as SPR-008 does.
//! Inputs after the horizon's last minute are not part of a run of that horizon.
//!
//! REF: `CODEX/05_PROJECT/SPR-012_WalkingSkeletonB.md` T-08; SPR-011 rulings 3, 9, 10, 15, 16.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_core::crash::Provenance;
use rh_core::ownership::SpecTable;
use rh_core::register::system_id;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, SimTime};
use rh_core::world::WorldTables;
use rh_ids::person::PersonIdTable;
use rh_ids::{EntityRef, SystemId};
use rh_manifest::Registry;
use rh_manifest::specs::{load_specs, registry_system_ids};
use rh_registry::WorldSpec;
use rh_registry::ownership::Ownership;
use rh_sys_demography::Demography;
use serde::Deserialize;

use crate::run::Error;
use crate::sha256;

/// The systems this engine build has, by Registry id (ruling 3): part of `engine_build`.
pub const BUILT: [&str; 1] = ["M2.demography"];
/// The smoke world's name.
pub const SMOKE: &str = "smoke";

/// What a run needs of its world.
pub struct Setup {
    /// The world's name in ladders.
    pub name: String,
    /// The calendar.
    pub calendar: Calendar,
    /// The seed the run uses.
    pub seed: u64,
    /// Build, Registry version and manifest hash.
    pub provenance: Provenance,
    /// The spec table registration checks the systems against.
    pub specs: SpecTable,
    /// The tables at instant 0.
    pub tables: WorldTables,
    /// The systems, unregistered (called once per registration).
    pub systems: Box<dyn Fn(bool) -> Vec<Box<dyn SystemModel>>>,
    /// The material inputs of a run of `years`: `(instant, entity)`, in submission order.
    pub inputs: Box<dyn Fn(i64) -> Vec<(SimTime, EntityRef)>>,
    /// The header's history-tuple line, for a manifest world.
    pub tuple: Option<String>,
    /// The header's built and not-built systems line, for a manifest world.
    pub systems_line: Option<String>,
    /// The systems the manifest names that this build lacks.
    pub not_built: Vec<String>,
    /// What its World Stream carries (SPR-013 T-09): a manifest world's dictionaries and history
    /// tuple. The smoke world has none (no life-course log, no Registry verbs).
    pub stream: Option<StreamInfo>,
    /// The founders' display names, by `PersonId` (SPR-013 T-10; tool side only, the flavour
    /// rule of `05.5` §3): the cast's names, and each household's children and servants named
    /// after their head. Commoners have none.
    pub names: BTreeMap<u64, String>,
    /// The manifest's `narrative.pattern_packs`, in order (what `rh-cli sift` sifts with; T-11).
    pub packs: Vec<String>,
}

/// A manifest world's stream dictionaries and the history tuple its segments carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamInfo {
    /// The Registry's verbs and act classes.
    pub dictionaries: rh_stream::segment::Dictionaries,
    /// The history tuple.
    pub provenance: rh_stream::segment::Provenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldToml {
    world: String,
    manifest: PathBuf,
    manifest_sha256: String,
    registry: PathBuf,
    inputs: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputsToml {
    #[serde(default)]
    input: Vec<ScriptedInput>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptedInput {
    tick: i64,
    founder: usize,
}

fn read(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|e| Error(format!("{}: {e}", path.display())))
}

/// The prefix of a test fixture world: `fixture-<name>` lives in `tests/worlds/<name>`, not under
/// `worlds/` (it has no baseline; SPR-013 T-11's 10,000-founder fixture).
pub const FIXTURE_PREFIX: &str = "fixture-";

/// The directory of world `name` under `repo`.
#[must_use]
pub fn world_dir(repo: &Path, name: &str) -> PathBuf {
    match name.strip_prefix(FIXTURE_PREFIX) {
        Some(fixture) => repo.join("tests/worlds").join(fixture),
        None => repo.join("worlds").join(name),
    }
}

/// The SHA-256 check of `world.toml`'s pin: `Ok(hex)` when the manifest's bytes hash to it.
///
/// # Errors
/// [`Error`] naming the pinned and the actual digest.
pub fn check_pin(repo: &Path, name: &str) -> Result<String, Error> {
    let toml = world_toml(repo, name)?;
    let path = repo.join(&toml.manifest);
    let bytes = std::fs::read(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
    let actual = sha256::hex(&bytes);
    if actual != toml.manifest_sha256 {
        return Err(Error(format!(
            "MANIFEST PIN: {} hashes to sha256 {actual}, but worlds/{name}/world.toml pins {}: \
             one copy of a manifest, no drift (SPR-011 ruling 16); re-pin it only with its \
             baseline, in a [C3] PR",
            toml.manifest.display(),
            toml.manifest_sha256
        )));
    }
    Ok(actual)
}

fn world_toml(repo: &Path, name: &str) -> Result<WorldToml, Error> {
    let path = world_dir(repo, name).join("world.toml");
    let toml: WorldToml =
        toml::from_str(&read(&path)?).map_err(|e| Error(format!("{}: {e}", path.display())))?;
    if toml.world != name {
        return Err(Error(format!(
            "{}: world = {}, not {name}",
            path.display(),
            toml.world
        )));
    }
    Ok(toml)
}

/// The setup of world `name` for a run with `seed` (`None`: the world's own) under `repo`.
///
/// # Errors
/// [`Error`]: an unknown world, a manifest that does not match its pin or does not compile, a
/// genesis or a construction refusal, or `strict` with a system not built (ruling 3).
#[allow(
    clippy::too_many_lines,
    reason = "a manifest world's setup in its order: pin, compile, names, genesis, systems, stream"
)]
pub fn setup(repo: &Path, name: &str, seed: Option<u64>, strict: bool) -> Result<Setup, Error> {
    if name == SMOKE {
        return smoke(repo, seed.unwrap_or(1));
    }
    let toml = world_toml(repo, name)?;
    let manifest_hash = check_pin(repo, name)?;
    let registry_path = repo.join(&toml.registry);
    let registry = Registry::load(&registry_path).map_err(|e| Error(format!("registry: {e}")))?;
    let mut spec = rh_manifest::compile_file(&repo.join(&toml.manifest), &registry, &BUILT)
        .map_err(|e| Error(format!("compile {name}: {e}")))?;
    if let Some(s) = seed {
        spec.seed = s;
    }
    // A template may name a system the Registry's [systems] does not list yet (pastoral_household's
    // S1.culture): its name comes from the template, so the header never shows a bare id.
    let names: BTreeMap<SystemId, &str> = registry
        .systems
        .keys()
        .chain(registry.templates.values().flat_map(|t| &t.default_systems))
        .map(|n| (system_id(n), n.as_str()))
        .collect();
    let name_of = |id: SystemId| {
        names
            .get(&id)
            .map_or_else(|| id.to_string(), |n| (*n).to_owned())
    };
    let built: Vec<String> = spec
        .systems
        .iter()
        .filter(|s| s.built)
        .map(|s| name_of(s.id))
        .collect();
    let not_built: Vec<String> = spec
        .systems
        .iter()
        .filter(|s| !s.built)
        .map(|s| name_of(s.id))
        .collect();
    // Names in name order: the header is read by people (WorldSpec orders by SystemId).
    let (mut built, mut not_built) = (built, not_built);
    built.sort();
    not_built.sort();
    if strict && !not_built.is_empty() {
        return Err(Error(format!(
            "--strict refuses {name}: the manifest names systems this build lacks ({}); \
             a not-built system is skipped, never stubbed (SPR-011 ruling 3)",
            not_built.join(", ")
        )));
    }
    let ids = registry_system_ids(&registry_path).map_err(|e| Error(format!("registry: {e}")))?;
    let specs_loaded =
        load_specs(&repo.join("specs/systems")).map_err(|e| Error(format!("specs: {e}")))?;
    let specs = Ownership::build(&specs_loaded, &ids).spec_table();
    let tables = rh_worldgen::genesis(&spec, &specs).map_err(|e| Error(e.to_string()))?;
    // Constructed once here, so a refusal (alpha, beta) fails the setup, not a registration.
    let m2 = Demography::from_world(&spec).map_err(|e| Error(e.to_string()))?;
    let script = scripted_inputs(repo, &toml.inputs, &spec)?;
    let (names, packs) = display_names(&repo.join(&toml.manifest), &registry, &spec)?;
    let calendar = spec.calendar;
    let provenance = Provenance {
        build: format!(
            "rh-golden {} built:{}",
            env!("CARGO_PKG_VERSION"),
            BUILT.join(",")
        ),
        registry_version: registry.registry_version.clone(),
        manifest_hash: format!("sha256:{manifest_hash}"),
    };
    let (tuple, systems_line) = header_lines(&provenance, spec.seed, &built, &not_built);
    let stream = StreamInfo {
        dictionaries: crate::stream::dictionaries(&registry)?,
        provenance: rh_stream::segment::Provenance {
            engine_build: provenance.build.clone(),
            manifest_hash: hex_bytes(&manifest_hash)?,
            world_seed: spec.seed,
            built_systems: built.clone(),
            not_built: not_built.clone(),
        },
    };
    Ok(Setup {
        name: name.to_owned(),
        calendar,
        seed: spec.seed,
        provenance,
        specs,
        tables,
        systems: Box::new(move |_leak| {
            m2.clone()
                .map(|m| Box::new(m) as Box<dyn SystemModel>)
                .into_iter()
                .collect()
        }),
        inputs: Box::new(move |years| {
            let end = years.saturating_mul(calendar.minutes_per_year());
            script
                .iter()
                .copied()
                .filter(|(t, _)| t.minutes() < end)
                .collect()
        }),
        tuple: Some(tuple),
        systems_line: Some(systems_line),
        not_built,
        stream: Some(stream),
        names,
        packs,
    })
}

/// The founders' display names (see [`Setup::names`]), in genesis order (ruling 10: the cast,
/// then each household's sons, daughters and servants, then the rest), and the manifest's pattern
/// packs.
fn display_names(
    manifest: &Path,
    registry: &Registry,
    spec: &WorldSpec,
) -> Result<(BTreeMap<u64, String>, Vec<String>), Error> {
    let rh_manifest::manifest::Outcome::Checked { manifest: m, .. } =
        rh_manifest::manifest::validate_file(manifest, registry)
    else {
        return Err(Error(format!("{}: does not load", manifest.display())));
    };
    let ids = rh_worldgen::founder_ids(spec.seed, spec.founders.len(), &mut PersonIdTable::new())
        .map_err(|e| Error(e.to_string()))?;
    let mut labels: Vec<String> = m.cast.iter().map(|c| c.name.clone()).collect();
    for h in &m.households {
        let head = m
            .cast
            .iter()
            .find(|c| c.id == h.head)
            .map_or_else(|| h.head.clone(), |c| c.name.clone());
        let n = |x: i64| usize::try_from(x).unwrap_or(0);
        labels.extend((1..=n(h.children.sons)).map(|i| format!("{head}'s son {i}")));
        labels.extend((1..=n(h.children.daughters)).map(|i| format!("{head}'s daughter {i}")));
        labels.extend((1..=n(h.servants)).map(|i| format!("{head}'s servant {i}")));
    }
    let names = ids
        .iter()
        .zip(labels)
        .map(|(id, name)| (id.to_raw(), name))
        .collect();
    Ok((names, m.narrative.pattern_packs.clone()))
}

/// The bytes of a lowercase hex digest.
fn hex_bytes(hex: &str) -> Result<Vec<u8>, Error> {
    (0..hex.len() / 2)
        .map(|i| {
            hex.get(2 * i..2 * i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| Error(format!("{hex} is not a hex digest")))
        })
        .collect()
}

/// The header's `tuple …` and `systems …` lines (without their prefixes) of a manifest world.
fn header_lines(
    provenance: &Provenance,
    seed: u64,
    built: &[String],
    not_built: &[String],
) -> (String, String) {
    let list = |v: &[String]| {
        if v.is_empty() {
            "-".to_owned()
        } else {
            v.join(",")
        }
    };
    (
        format!(
            "engine_build={} registry_version={} manifest_hash={} seed={seed}",
            provenance.build.replace(' ', "-"),
            provenance.registry_version,
            provenance.manifest_hash,
        ),
        format!("built {} not_built {}", list(built), list(not_built)),
    )
}

/// The input script's inputs, with each founder index resolved to its `PersonId`.
fn scripted_inputs(
    repo: &Path,
    path: &Path,
    spec: &WorldSpec,
) -> Result<Vec<(SimTime, EntityRef)>, Error> {
    let full = repo.join(path);
    let script: InputsToml =
        toml::from_str(&read(&full)?).map_err(|e| Error(format!("{}: {e}", full.display())))?;
    let ids = rh_worldgen::founder_ids(spec.seed, spec.founders.len(), &mut PersonIdTable::new())
        .map_err(|e| Error(e.to_string()))?;
    let mut out = Vec::new();
    for i in &script.input {
        let id = ids.get(i.founder).ok_or_else(|| {
            Error(format!(
                "{}: founder {} does not exist ({} founders)",
                full.display(),
                i.founder,
                ids.len()
            ))
        })?;
        out.push((SimTime::from_minutes(i.tick), EntityRef::person(*id)));
    }
    if out.windows(2).any(|w| w[0].0 > w[1].0) {
        return Err(Error(format!(
            "{}: inputs are not in tick order",
            full.display()
        )));
    }
    Ok(out)
}

fn smoke(repo: &Path, seed: u64) -> Result<Setup, Error> {
    Ok(Setup {
        name: SMOKE.to_owned(),
        calendar: rh_sys_smoke::CALENDAR,
        seed,
        provenance: crate::run::provenance(),
        specs: crate::run::spec_table(repo)?,
        tables: rh_sys_smoke::initial_tables(),
        systems: Box::new(crate::run::smoke_systems),
        inputs: Box::new(rh_sys_smoke::synthetic_inputs),
        tuple: None,
        systems_line: None,
        not_built: Vec::new(),
        stream: None,
        names: BTreeMap::new(),
        packs: Vec::new(),
    })
}
