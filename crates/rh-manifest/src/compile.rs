//! The authored-number boundary: a validated manifest compiled into a plane-A [`WorldSpec`]
//! (SPR-011 T-04, rulings 3, 8, 9 and 10; `05.5` §3.2).
//!
//! Every number that enters plane A is the exact decimal value of its literal as written (the
//! manifest's YAML text or the Registry's TOML text, [`crate::exact`]), rounded half to even to the
//! fixed-point type, never through an `f64`. A value that does not fit is a load error naming it.
//! A parameter a built system reads comes from the manifest, else from the Registry's `defaults`
//! (ruling 8); a parameter with neither is a load error naming it. Names stay on the tool side: the
//! `WorldSpec` holds `SystemId`s, enums and fixed-point values (the flavour rule, `05.5` §3).
//!
//! What P1.0 compiles: the seed, the calendar, the horizon, the systems the manifest names (each
//! built or not, ruling 3), `M2.demography`'s rung-0 parameters (`e0`, `brass_beta`; `brass_alpha`
//! and `brass_beta != 1` are refused until P1.3, ruling 7), and the founders in genesis order
//! (ruling 10). Other parameters and sections are not read by any built system yet and are not
//! compiled; each joins with the sprint whose system reads it.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-04; `docs/05_architecture/05.5` §3.2.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use rh_core::fixed::Q16_16;
use rh_core::register::system_id;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_registry::{DemographyParams, Founder, FounderRole, NamedSystem, Sex, WorldSpec};

use crate::exact::{Decimal, DecimalError};
use crate::manifest::{Manifest, Outcome, People, Verdict, validate_file};
use crate::registry::Registry;

/// The demography system's Registry id.
pub const DEMOGRAPHY: &str = "M2.demography";
/// The parameters rung 0 of `M2.demography` reads (ruling 7), in spec order.
pub const DEMOGRAPHY_READS: [&str; 2] = ["e0", "brass_beta"];
/// Parameters a manifest may not set until P1.3 adds fixed-point `exp` and `ln` (ruling 7).
pub const DEMOGRAPHY_REFUSED: [&str; 1] = ["brass_alpha"];

/// One reason a manifest does not compile, at a manifest path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    /// Where: a manifest path (`systems.M2.demography.e0`) or a Registry one.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for CompileError {}

/// Every reason a manifest does not compile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileErrors(pub Vec<CompileError>);

impl fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(
            f,
            "the manifest does not compile:\n  {}",
            lines.join("\n  ")
        )
    }
}

impl std::error::Error for CompileErrors {}

/// Errors gathered while compiling, so every problem is reported at once.
#[derive(Default)]
struct Errors(Vec<CompileError>);

impl Errors {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(CompileError {
            path: path.into(),
            message: message.into(),
        });
    }
}

/// `exact · 2^16` rounded half to even, if it fits a `Q16_16`.
///
/// # Errors
/// Why not, for a message: the literal has no decimal value, or the value is out of range.
pub fn to_q16_16(exact: &Result<Decimal, DecimalError>) -> Result<Q16_16, String> {
    let decimal = exact.as_ref().map_err(ToString::to_string)?;
    decimal
        .to_fixed(Q16_16::FRAC_BITS)
        .and_then(|bits| i32::try_from(bits).ok())
        .map(Q16_16::from_bits)
        .ok_or_else(|| {
            format!("{decimal} does not fit a Q16_16 (range [-32768, 32768 - 2^-16], step 2^-16)")
        })
}

/// Compiles a validated manifest for an engine build that has the systems `built`.
///
/// # Errors
/// [`CompileErrors`] listing every number that does not fit, every parameter a built system needs
/// that has no value, every refused parameter, and every structure plane A cannot represent.
pub fn compile(
    manifest: &Manifest,
    registry: &Registry,
    built: &[&str],
) -> Result<WorldSpec, CompileErrors> {
    let mut errors = Errors::default();
    let world = &manifest.world;
    let seed = u64::try_from(world.seed).unwrap_or_else(|_| {
        errors.push("world.seed", format!("{} is negative", world.seed));
        0
    });
    let calendar = calendar(manifest, &mut errors);
    let horizon_years = u32::try_from(world.horizon.years).unwrap_or_else(|_| {
        errors.push("world.horizon.years", "does not fit a u32");
        0
    });
    let systems = named_systems(manifest, registry, built, &mut errors);
    let demography = if built.contains(&DEMOGRAPHY) && named(manifest, registry, DEMOGRAPHY) {
        demography(manifest, registry, &mut errors)
    } else {
        None
    };
    let founders = founders(manifest, &mut errors);
    if errors.0.is_empty() {
        Ok(WorldSpec {
            seed,
            calendar,
            horizon_years,
            systems,
            demography,
            founders,
        })
    } else {
        Err(CompileErrors(errors.0))
    }
}

/// Validates a manifest file as the reference does, then compiles it: an invalid manifest never
/// reaches plane A.
///
/// # Errors
/// The validation's verdict and first problem, or [`compile`]'s errors.
pub fn compile_file(
    path: &Path,
    registry: &Registry,
    built: &[&str],
) -> Result<WorldSpec, CompileErrors> {
    let outcome = validate_file(path, registry);
    match (&outcome, outcome.verdict()) {
        (Outcome::Checked { manifest, .. }, Verdict::Valid) => compile(manifest, registry, built),
        _ => Err(CompileErrors(vec![CompileError {
            path: path.display().to_string(),
            message: format!(
                "does not validate ({:?}), so it is not compiled:\n{}",
                outcome.verdict(),
                outcome.render(&path.display().to_string())
            ),
        }])),
    }
}

fn calendar(manifest: &Manifest, errors: &mut Errors) -> Calendar {
    let c = &manifest.world.calendar;
    let days = match c.days_per_year {
        365 => DaysPerYear::D365,
        360 => DaysPerYear::D360,
        n => {
            errors.push(
                "world.calendar.days_per_year",
                format!("{n}: the engine's calendars have 360 or 365 days (05.1 §4.1)"),
            );
            DaysPerYear::D360
        }
    };
    if c.months != 12 {
        errors.push(
            "world.calendar.months",
            format!(
                "{}: the engine's calendars have 12 months (05.1 §4.1)",
                c.months
            ),
        );
    }
    let seasons = match c.seasons {
        2 => SeasonsPerYear::Two,
        4 => SeasonsPerYear::Four,
        n => {
            errors.push(
                "world.calendar.seasons",
                format!("{n}: the engine's calendars have 2 or 4 seasons (05.1 §4.1)"),
            );
            SeasonsPerYear::Four
        }
    };
    Calendar::new(days, seasons)
}

/// Whether the manifest names a system: its template's defaults, its fidelity profile or its
/// `systems` section.
fn named(manifest: &Manifest, registry: &Registry, id: &str) -> bool {
    let template = registry.templates.get(&manifest.world.template);
    template.is_some_and(|t| t.default_systems.iter().any(|s| s == id))
        || manifest.world.fidelity_profile.contains_key(id)
        || manifest.systems.contains_key(id)
}

fn named_systems(
    manifest: &Manifest,
    registry: &Registry,
    built: &[&str],
    errors: &mut Errors,
) -> Vec<NamedSystem> {
    let mut names: Vec<&str> = registry
        .templates
        .get(&manifest.world.template)
        .map(|t| t.default_systems.iter().map(String::as_str).collect())
        .unwrap_or_default();
    names.extend(manifest.world.fidelity_profile.keys());
    names.extend(manifest.systems.keys());
    let mut by_id: BTreeMap<rh_ids::SystemId, (&str, NamedSystem)> = BTreeMap::new();
    for name in names {
        let fidelity = manifest.world.fidelity_profile.get(name).and_then(|level| {
            let parsed = level.strip_prefix('F').and_then(|n| n.parse::<u8>().ok());
            if parsed.is_none() {
                errors.push(
                    format!("world.fidelity_profile.{name}"),
                    format!("{level} is not a fidelity level F<n>"),
                );
            }
            parsed
        });
        let entry = NamedSystem {
            id: system_id(name),
            fidelity,
            built: built.contains(&name),
        };
        match by_id.get(&entry.id) {
            Some((other, _)) if *other != name => errors.push(
                format!("systems.{name}"),
                format!("its SystemId collides with {other}'s (05.1 §6 item 2)"),
            ),
            _ => {
                by_id.insert(entry.id, (name, entry));
            }
        }
    }
    by_id.into_values().map(|(_, entry)| entry).collect()
}

fn demography(
    manifest: &Manifest,
    registry: &Registry,
    errors: &mut Errors,
) -> Option<DemographyParams> {
    let set = manifest.systems.get(DEMOGRAPHY);
    for refused in DEMOGRAPHY_REFUSED {
        if set.is_some_and(|s| s.contains_key(refused)) {
            errors.push(
                format!("systems.{DEMOGRAPHY}.{refused}"),
                "refused until P1.3 brings fixed-point exp and ln (SPR-011 ruling 7); set e0",
            );
        }
    }
    let def = registry.systems.get(DEMOGRAPHY);
    let mut values = Vec::new();
    for name in DEMOGRAPHY_READS {
        let (path, value) = if let Some(a) = set.and_then(|s| s.get(name)) {
            if let Some(b) = def.and_then(|d| d.params.get(name))
                && !b.contains(a.value)
            {
                errors.push(
                    format!("systems.{DEMOGRAPHY}.{name}"),
                    "outside its Registry bounds",
                );
            }
            (format!("systems.{DEMOGRAPHY}.{name}"), to_q16_16(&a.exact))
        } else if let Some(exact) = def.and_then(|d| d.default_literals.get(name)) {
            (
                format!("registry systems.{DEMOGRAPHY}.defaults.{name}"),
                to_q16_16(exact),
            )
        } else {
            errors.push(
                format!("systems.{DEMOGRAPHY}.{name}"),
                "a built system reads it, and neither the manifest nor the Registry's defaults \
                 set it (SPR-011 ruling 8)",
            );
            continue;
        };
        match value {
            Ok(q) => values.push((name, q)),
            Err(message) => errors.push(path, message),
        }
    }
    let get = |n: &str| values.iter().find(|(name, _)| *name == n).map(|(_, q)| *q);
    let (e0, brass_beta) = (get("e0")?, get("brass_beta")?);
    if brass_beta != Q16_16::ONE {
        errors.push(
            format!("systems.{DEMOGRAPHY}.brass_beta"),
            "rung 0 runs Brass at beta = 1 only until P1.3 (SPR-011 ruling 7)",
        );
    }
    Some(DemographyParams { e0, brass_beta })
}

fn founders(manifest: &Manifest, errors: &mut Errors) -> Vec<Founder> {
    let mut out = Vec::new();
    for (i, member) in manifest.cast.iter().enumerate() {
        let sex = match member.sex.as_str() {
            "female" => Some(Sex::Female),
            "male" => Some(Sex::Male),
            other => {
                errors.push(
                    format!("cast[{i}].sex"),
                    format!("{other} is not male or female"),
                );
                None
            }
        };
        let age = u8::try_from(member.age_years).ok().filter(|&a| a <= 120);
        if age.is_none() {
            errors.push(format!("cast[{i}].age_years"), "outside [0, 120]");
        }
        out.push(Founder {
            role: FounderRole::Cast,
            sex,
            age_years: age,
        });
    }
    for (i, household) in manifest.households.iter().enumerate() {
        let count = |n: i64, what: &str, errors: &mut Errors| {
            usize::try_from(n).unwrap_or_else(|_| {
                errors.push(format!("households[{i}].{what}"), "is negative");
                0
            })
        };
        let sons = count(household.children.sons, "children.sons", errors);
        let daughters = count(household.children.daughters, "children.daughters", errors);
        let servants = count(household.servants, "servants", errors);
        let child = |sex| Founder {
            role: FounderRole::Child,
            sex: Some(sex),
            age_years: None,
        };
        out.extend(std::iter::repeat_n(child(Sex::Male), sons));
        out.extend(std::iter::repeat_n(child(Sex::Female), daughters));
        let servant = Founder {
            role: FounderRole::Servant,
            sex: None,
            age_years: None,
        };
        out.extend(std::iter::repeat_n(servant, servants));
    }
    let population: i64 = manifest
        .peoples
        .iter()
        .map(|p| match p {
            People::Human(h) => h.population,
            People::Creature(_) => 0,
        })
        .sum();
    let named = i64::try_from(out.len()).unwrap_or(i64::MAX);
    match usize::try_from(population - named) {
        Ok(rest) => out.extend(std::iter::repeat_n(
            Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: None,
            },
            rest,
        )),
        Err(_) => errors.push(
            "peoples",
            format!(
                "the human population, {population}, is smaller than the {named} founders the cast \
                 and the households name"
            ),
        ),
    }
    out
}
