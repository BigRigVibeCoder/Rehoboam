//! SPR-005 against the real specs (`specs/systems`), loaded with this crate's spec reader.
//!
//! rh-core never depends on rh-registry, not even for tests (Architect ruling, 2026-10-16), so
//! every test that needs the real specs lives here; rh-core tests its rules on hand-built
//! `SpecTable`s.
//!
//! - T-03: the generated skeletons (`crates/rh-core/tests/generated/*.rs`, the unedited output of
//!   `lint_odd.py --rust <id>`, kept so by `bin/gen_skeletons.py --check`) compile against
//!   rh-core's `SystemModel` contract. Each module below brings `Fixed`, `Phase` and `Pulse` into
//!   scope, `include!`s its skeleton with a string literal (which `bin/lint_forbidden.py` follows
//!   and scans, BCK-002 A-27) and implements `SpecPurpose` for the generated `RngPurpose` with the
//!   generated `name()`: the only way a system names a purpose (ruling 2). Only the variant list
//!   (`SpecPurpose::ALL`) is written by hand, and registration checks it against the spec.
//!   Acceptance: the S5 and M2 skeletons compile, and so do S6 and S7.
//! - T-06: a purpose not in the spec fails registration; a spec purpose with no code use is
//!   reported.
//! - Ruling 3: the lint and the registration checker share one single-writer rule, and agree on
//!   the real specs.
//! - The RNG `system_id`s of the Registry systems are distinct.
//!
//! Refs: SPR-005 T-03, T-04, T-06, rulings 1–3; ADR-0033 §3; `05.1` §2, §6 item 2.
#![allow(
    clippy::type_complexity,
    clippy::expect_used,
    reason = "a table of test cases; test helpers fail loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::fixed::Q16_16;
use rh_core::ownership::{Declaration, Source, single_writer};
use rh_core::register::{register, system_id};
use rh_core::system::{
    AggregateSpec, Fixed, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose,
    StepCtx, SystemDescriptor, SystemModel,
};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_registry::ownership::{Ownership, SystemSpec, load_specs, registry_system_ids};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The ownership table of the real specs, as `rh-cli ownership` builds it.
fn real_ownership() -> Ownership {
    let ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    let specs = load_specs(&repo().join("specs/systems")).expect("specs");
    Ownership::build(&specs, &ids)
}

/// Implements `SpecPurpose` for the generated `RngPurpose` of the module it is invoked in. The
/// module itself `include!`s its skeleton with a string literal, which `bin/lint_forbidden.py`
/// follows and scans as source (BCK-002 A-27; SPR-008 T-00): a macro parameter there would not
/// be followed.
macro_rules! purposes {
    ($($variant:ident),* $(,)?) => {
        impl rh_core::system::SpecPurpose for RngPurpose {
            const ALL: &'static [Self] = &[$(Self::$variant),*];
            fn name(&self) -> &'static str {
                RngPurpose::name(*self) // the generated name (lint_odd.py --rust)
            }
        }
    };
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s5 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/S5_daily_life.rs");
    purposes!(ActivityBlock, EncounterOutcome);
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod m2 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/M2_demography.rs");
    purposes!(DeathTime);
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s6 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/S6_artifacts.rs");
    purposes!(LossOnDeath, TheftOutcome, SearchFind);
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s7 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/S7_vows.rs");
    purposes!(GrudgeForm, BreakMode);
}

/// A system built on a skeleton: everything the spec fixes comes from the generated module.
struct OnSkeleton {
    id: &'static str,
    spec_version: &'static str,
    schedule: &'static [(Pulse, Phase)],
    purposes: Vec<&'static str>,
    access: AccessSpec,
    params: Vec<&'static str>,
}

impl SystemModel for OnSkeleton {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            fidelity: "F1",
            spec_version: self.spec_version,
        }
    }
    fn access(&self) -> AccessSpec {
        self.access.clone()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        PulseBinding::from_schedule(self.schedule)
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.purposes.clone()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema {
            names: self.params.clone(),
        }
    }
    fn step(
        &self,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn read(column: &'static str, owner: &'static str) -> ColumnRead {
    ColumnRead {
        column,
        owner,
        lag: Lag::StartOfDay,
    }
}

fn write(column: &'static str) -> ColumnWrite {
    ColumnWrite {
        column,
        plane: Plane::A,
    }
}

/// S5 as its spec declares it: one owned column, six start-of-day reads.
fn s5_system(purposes: Vec<&'static str>) -> OnSkeleton {
    OnSkeleton {
        id: s5::SYSTEM_ID,
        spec_version: s5::SPEC_VERSION,
        schedule: s5::SCHEDULE,
        purposes,
        access: AccessSpec {
            reads: vec![
                read("person.birth_tick", "M2.demography"),
                read("person.sex", "M2.demography"),
                read("person.alive", "M2.demography"),
                read("person.residence", "S3.migration"),
                read("person.occupation", "M4.economy"),
                read("region.conditions", "M6.polity"),
            ],
            writes: vec![write("encounter_log.day_index")],
            ..AccessSpec::default()
        },
        params: vec!["blocks_per_day", "encounter_tie_rate"],
    }
}

/// M2 as its spec declares it: four owned columns, no reads.
fn m2_system() -> OnSkeleton {
    OnSkeleton {
        id: m2::SYSTEM_ID,
        spec_version: m2::SPEC_VERSION,
        schedule: m2::SCHEDULE,
        purposes: m2::RngPurpose::names(),
        access: AccessSpec {
            reads: vec![],
            writes: vec![
                write("person.birth_tick"),
                write("person.sex"),
                write("person.alive"),
                write("person.death_tick"),
            ],
            ..AccessSpec::default()
        },
        params: vec![
            "e0",
            "brass_alpha",
            "brass_beta",
            "tfr",
            "eps_f",
            "eps_m",
            "elite_fertility_mult",
        ],
    }
}

#[test]
fn the_generated_params_are_q16_16() {
    // Ruling 1: `Fixed` is `Q16_16`, and the spec ranges fit it (e0 up to about 85).
    let p = m2::Params {
        e0: Q16_16::from_int(85),
        brass_alpha: Fixed::from_ratio(-136, 100).unwrap(),
        brass_beta: Fixed::ONE,
        tfr: Fixed::from_int(8),
        eps_f: Fixed::from_ratio(1, 10).unwrap(),
        eps_m: Fixed::from_int(3),
        elite_fertility_mult: Fixed::from_ratio(16, 10).unwrap(),
    };
    assert_eq!(p.e0.to_bits(), 85 << 16);
    let _ = s5::Params {
        blocks_per_day: Fixed::from_int(6),
        encounter_tie_rate: Fixed::EPSILON,
    };
}

#[test]
fn every_skeleton_matches_its_spec() {
    let t = real_ownership();
    let cases: [(&str, &[(Pulse, Phase)], Vec<&'static str>); 4] = [
        (s5::SYSTEM_ID, s5::SCHEDULE, s5::RngPurpose::names()),
        (m2::SYSTEM_ID, m2::SCHEDULE, m2::RngPurpose::names()),
        (s6::SYSTEM_ID, s6::SCHEDULE, s6::RngPurpose::names()),
        (s7::SYSTEM_ID, s7::SCHEDULE, s7::RngPurpose::names()),
    ];
    for (id, schedule, purposes) in cases {
        let spec = &t.specs[id];
        assert_eq!(
            spec.rng, purposes,
            "{id}: the SpecPurpose names are the spec's [[rng]]"
        );
        assert!(!schedule.is_empty(), "{id}");
    }
    assert_eq!(
        s7::SCHEDULE.len(),
        3,
        "S7 binds Day twice (Vital, Agenda) and Month once"
    );
}

#[test]
fn s5_and_m2_register_on_their_skeletons() {
    let t = real_ownership();
    let r = register(
        vec![
            Box::new(s5_system(s5::RngPurpose::names())),
            Box::new(m2_system()),
        ],
        &t.spec_table(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.ids(), ["M2.demography", "S5.daily_life"]);
    assert_eq!(r.order(Phase::Vital), ["M2.demography"]);
    assert_eq!(r.order(Phase::Activity), ["S5.daily_life"]);
    // S5 reads columns whose owners have no code yet: reported, not refused.
    assert!(
        r.reports().iter().any(|n| n
            == "S5.daily_life: reads person.residence from S3.migration, which is not registered")
    );
    // The code declarations are what `rh-cli ownership` will count "from code".
    assert_eq!(r.accesses().len(), 2);
}

/// SPR-006: the real specs' `[[commands]]` and `[[emits]]` reach registration. M2 declares the
/// `DEATH` it emits; S5's spec sends `TieDelta` to S1. S5 declaring that submission is refused
/// while S1 is not registered (G5); an undeclared kind or a wrong owner is refused too.
#[test]
fn commands_and_event_kinds_are_checked_against_the_real_specs() {
    use rh_core::access::CommandSend;
    let t = real_ownership();
    let with =
        |emits: Vec<&'static str>, target: Option<&'static str>| -> Vec<Box<dyn SystemModel>> {
            let mut s5 = s5_system(s5::RngPurpose::names());
            if let Some(target) = target {
                s5.access.sends.push(CommandSend {
                    command: "TieDelta",
                    target,
                });
            }
            let mut m2 = m2_system();
            m2.access.emits = emits;
            vec![Box::new(s5), Box::new(m2)]
        };
    let r = register(with(vec!["DEATH"], None), &t.spec_table()).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        r.reports()
            .iter()
            .any(|n| n == "S5.daily_life: spec event kind ENCOUNTER has no use in code")
    );
    let Err(e) = register(with(vec!["DEATH"], Some("S1.culture")), &t.spec_table()) else {
        panic!("a command to an unregistered owner must be refused (G5)");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [command] S5.daily_life: submits command TieDelta to S1.culture, which is not registered; a world that registers a sender registers the command's owner (ruling 5)"
        ]
    );
    let Err(e) = register(with(vec!["BIRTH"], Some("M2.demography")), &t.spec_table()) else {
        panic!("an undeclared kind and a wrong owner must be refused");
    };
    let rules: Vec<(&str, &str)> = e
        .violations
        .iter()
        .map(|v| (v.rule, v.system.as_str()))
        .collect();
    assert_eq!(
        rules,
        [
            ("command", "S5.daily_life"),
            ("command", "S5.daily_life"),
            ("event-kind", "M2.demography"),
        ],
        "{e}"
    );
}

/// An enum whose names drift from the spec, as a stale or hand-written one would.
#[allow(dead_code, reason = "only the names matter")]
enum Misspelt {
    ActivityBlock,
    Encounter,
}

impl SpecPurpose for Misspelt {
    const ALL: &'static [Self] = &[Self::ActivityBlock, Self::Encounter];
    fn name(&self) -> &'static str {
        match self {
            Self::ActivityBlock => "activity_block",
            Self::Encounter => "encounter",
        }
    }
}

#[test]
fn an_undeclared_purpose_fails_registration() {
    let t = real_ownership();
    let Err(e) = register(
        vec![Box::new(s5_system(Misspelt::names()))],
        &t.spec_table(),
    ) else {
        panic!("a purpose missing from S5's [[rng]] must fail registration");
    };
    let rng: Vec<_> = e
        .violations
        .iter()
        .filter(|v| v.rule == "rng-purpose")
        .collect();
    assert_eq!(rng.len(), 1, "{e}");
    assert_eq!(
        rng[0].to_string(),
        "VIOLATION [rng-purpose] S5.daily_life: draws under RNG purpose encounter, which its spec's [[rng]] table does not declare (ruling 2)"
    );
    // And the spec purpose the code no longer names is reported.
    assert!(
        e.reports
            .iter()
            .any(|n| n == "S5.daily_life: spec RNG purpose encounter_outcome has no use in code")
    );
}

#[test]
fn a_spec_purpose_with_no_code_use_is_reported() {
    let t = real_ownership();
    let r = register(
        vec![Box::new(s5_system(vec!["activity_block"]))],
        &t.spec_table(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        r.reports()
            .iter()
            .any(|n| n == "S5.daily_life: spec RNG purpose encounter_outcome has no use in code")
    );
    let dup = register(
        vec![Box::new(s5_system(vec![
            "activity_block",
            "activity_block",
        ]))],
        &t.spec_table(),
    );
    assert!(dup.is_err_and(|e| {
        e.violations
            .iter()
            .any(|v| v.message == "declares RNG purpose activity_block twice")
    }));
}

#[test]
fn a_system_on_a_skeleton_reports_its_spec_data() {
    let s5 = s5_system(s5::RngPurpose::names());
    let m2 = m2_system();
    assert_eq!(s5.descriptor().id, "S5.daily_life");
    assert_eq!(s5.descriptor().spec_version, "0.1.0");
    assert_eq!(
        s5.schedule(),
        [PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Activity
        }]
    );
    assert_eq!(
        m2.schedule(),
        [PulseBinding {
            pulse: Pulse::Month,
            phase: Phase::Vital
        }]
    );
    assert_eq!(s5.rng_purposes(), ["activity_block", "encounter_outcome"]);
    assert_eq!(m2.params_schema().names.len(), 7);
    assert_eq!(s5.access().plane(), "A");
    assert!(m2.access().writes_column("person.alive"));
    assert_eq!(
        s5.access().read_of("person.sex").map(|r| r.lag),
        Some(Lag::StartOfDay)
    );
    assert!(s5.invariants(&WorldTables::new()).is_empty() && m2.declared_aggregates().is_empty());
}

/// Ruling 3: on every real spec, the lint's rule and the checker's rule are the same function and
/// agree: the specs' declarations pass both, and a planted second owner fails both.
#[test]
fn the_checker_and_the_lint_share_one_rule() {
    let t = real_ownership();
    assert_eq!(t.problems, vec![], "the real specs are clean");
    let declarations: Vec<Declaration> = t.specs.values().map(SystemSpec::declaration).collect();
    let (spec_owners, spec_problems) = single_writer(&declarations, Source::Spec);
    let (code_owners, code_problems) = single_writer(&declarations, Source::Code);
    assert_eq!(spec_owners, t.owners);
    assert_eq!(code_owners, t.owners);
    assert!(spec_problems.is_empty() && code_problems.is_empty());
    let mut planted = declarations.clone();
    planted[0]
        .writes
        .push(("person.alive".to_owned(), "A".to_owned()));
    planted.rotate_left(1); // the rightful owner (M2) comes first
    let (_, spec_problems) = single_writer(&planted, Source::Spec);
    let (_, code_problems) = single_writer(&planted, Source::Code);
    assert_eq!(spec_problems.len(), 1);
    assert_eq!(code_problems.len(), 1);
    assert_eq!(spec_problems[0].rule, code_problems[0].rule);
    assert_eq!(spec_problems[0].system, code_problems[0].system);
}

#[test]
fn system_ids_are_name_derived_and_distinct_for_the_registry() {
    let ids = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let raw: std::collections::BTreeSet<u32> =
        ids.iter().map(|id| system_id(id).to_raw()).collect();
    assert_eq!(
        raw.len(),
        ids.len(),
        "no two Registry systems share an RNG system_id"
    );
    assert_eq!(system_id("M2.demography"), system_id("M2.demography"));
}

/// A system that declares a given access with its spec's lags, schedule and RNG purposes: so that
/// registration judges only the access (Tester DEF-3).
struct Declared {
    id: &'static str,
    access: AccessSpec,
    bindings: Vec<PulseBinding>,
    purposes: Vec<&'static str>,
}

impl SystemModel for Declared {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            fidelity: "F1",
            spec_version: "0.1.0",
        }
    }
    fn access(&self) -> AccessSpec {
        self.access.clone()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.bindings.clone()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.purposes.clone()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

impl Declared {
    /// `system` reading `reads` and writing `writes` (all plane A), with the lags, schedule and RNG
    /// purposes its spec declares (`start_of_day` for a read the spec does not list).
    fn from_spec(
        t: &Ownership,
        system: &'static str,
        reads: &[&'static str],
        writes: &[&'static str],
    ) -> Self {
        let spec = t.specs.get(system);
        let leak = |s: &String| -> &'static str { s.clone().leak() };
        let lag = |column: &str| {
            spec.and_then(|s| s.reads.iter().find(|r| r.column == column))
                .and_then(|r| Lag::from_name(&r.lag))
                .unwrap_or(Lag::StartOfDay)
        };
        Declared {
            id: system,
            access: AccessSpec {
                reads: reads
                    .iter()
                    .map(|&column| ColumnRead {
                        column,
                        owner: t.owners.get(column).map_or("M9.ghost", |o| leak(&o.system)),
                        lag: lag(column),
                    })
                    .collect(),
                writes: writes
                    .iter()
                    .map(|&column| ColumnWrite {
                        column,
                        plane: Plane::A,
                    })
                    .collect(),
                ..AccessSpec::default()
            },
            bindings: spec.map_or_else(Vec::new, |s| {
                s.schedule
                    .iter()
                    .map(|(p, f)| PulseBinding {
                        pulse: Pulse::from_name(p).expect("a pulse"),
                        phase: Phase::from_name(f).expect("a phase"),
                    })
                    .collect()
            }),
            purposes: spec.map_or_else(Vec::new, |s| s.rng.iter().map(leak).collect()),
        }
    }
}

/// Tester DEF-3: the ownership lint (`Ownership::check`, what `rh-cli ownership` runs on an access
/// file) and registration give the same verdict, with the same rules, on the same declaration.
#[test]
fn the_lint_and_registration_agree_on_one_fixture_set() {
    type Case = (
        &'static str,
        &'static [&'static str],
        &'static [&'static str],
    );
    let cases: [Case; 8] = [
        (
            "M2.demography",
            &[],
            &["person.birth_tick", "person.alive", "person.alive"],
        ),
        ("M2.demography", &[], &["person.birth_tick", "person.sex"]),
        ("M2.demography", &["person.residence"], &["person.alive"]),
        (
            "S5.daily_life",
            &["person.sex"],
            &["encounter_log.day_index", "person.alive"],
        ),
        (
            "S5.daily_life",
            &["person.sex", "person.residence"],
            &["encounter_log.day_index"],
        ),
        ("S5.daily_life", &["person.mood"], &[]),
        ("S5.daily_life", &["person.sex", "person.sex"], &[]),
        ("M9.ghost", &[], &["ghost.x"]),
    ];
    let t = real_ownership();
    let mut verdicts = (0, 0);
    for (system, reads, writes) in cases {
        let lint = t.check(&rh_registry::ownership::AccessSpec {
            system: system.to_owned(),
            reads: reads.iter().map(ToString::to_string).collect(),
            writes: writes.iter().map(ToString::to_string).collect(),
        });
        let declared = Declared::from_spec(&t, system, reads, writes);
        let registered = register(vec![Box::new(declared)], &t.spec_table());
        let mut lint_rules: Vec<&str> = lint.iter().map(|v| v.rule).collect();
        let mut reg_rules: Vec<&str> = match &registered {
            Ok(_) => Vec::new(),
            Err(e) => e.violations.iter().map(|v| v.rule).collect(),
        };
        lint_rules.sort_unstable();
        lint_rules.dedup();
        reg_rules.sort_unstable();
        reg_rules.dedup();
        assert_eq!(
            lint.is_empty(),
            registered.is_ok(),
            "{system} {reads:?} {writes:?}: lint {lint:?}"
        );
        assert_eq!(lint_rules, reg_rules, "{system} {reads:?} {writes:?}");
        if lint.is_empty() {
            verdicts.0 += 1;
        } else {
            verdicts.1 += 1;
        }
    }
    assert_eq!(verdicts, (2, 6), "the fixture set has both verdicts");
}

/// G-a on the real specs: a binding or a read lag that differs from the spec fails registration.
#[test]
fn bindings_and_lags_must_match_the_spec() {
    let t = real_ownership();
    let mut wrong_lag = s5_system(s5::RngPurpose::names());
    wrong_lag.access.reads[1].lag = Lag::Lagged; // person.sex: the spec says start_of_day
    let Err(e) = register(vec![Box::new(wrong_lag)], &t.spec_table()) else {
        panic!("a lag that differs from the spec must fail registration");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [spec] S5.daily_life: reads person.sex at lag lagged, but its spec's [[reads]] declares 'start_of_day'"
        ]
    );
    let mut wrong_schedule = m2_system();
    wrong_schedule.schedule = &[(Pulse::Day, Phase::Vital)];
    let Err(e) = register(vec![Box::new(wrong_schedule)], &t.spec_table()) else {
        panic!("a binding that differs from the spec must fail registration");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [spec] M2.demography: binds Day/Vital but its spec's [[schedule]] declares Month/Vital"
        ]
    );
}
