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
//!   Acceptance: the S5 and M2 skeletons compile, and so do S6 and S7, T1 and M7 (PR #32).
//! - T-06: a purpose not in the spec fails registration; a spec purpose with no code use is
//!   reported.
//! - Ruling 3: the lint and the registration checker share one single-writer rule, and agree on
//!   the real specs.
//! - The RNG `system_id`s of the Registry systems are distinct.
//! - PR #32 (Architect round 1, P5): M2, M7 and T1, declared exactly as their specs say, register
//!   together, so ownership, lags, commands and the phase-order rule are checked mechanically.
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
use rh_registry::ownership::{Ownership, SystemSpec};
use spec_files::{load_specs, registry_system_ids};

mod spec_files;

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
    purposes!(DeathTime, Conception, MarriageTime, BirthSex, HusbandPick);
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s6 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/S6_artifacts.rs");
    purposes!(LossOnDeath, TheftOutcome, SearchFind);
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod t1 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/T1_minds.rs");
    purposes!(
        TraitDraw,
        PollJitter,
        IausSelect,
        ActionOrder,
        BreakDelay,
        BreakAct,
        BreakOutcome,
        AttackOutcome,
        OutcomeDelay,
    );
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod m7 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/M7_dynasty.rs");
    purposes!(
        ExitMurder,
        ExitDeposition,
        ExitDay,
        MurderCulprit,
        ArbitrationTie,
        RegrantTie,
        Sortition,
        InterregnumDelay,
        Vocation,
    );
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s7 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/S7_vows.rs");
    purposes!(GrudgeForm, BreakMode);
}

// SPR-018: M1 F0 binds no pulse and draws nothing of its own (genesis only, through rh-worldgen), and S3 F0 draws
// nothing in a step: both skeletons have an empty `RngPurpose`, which `purposes!()` with no variant covers.
#[allow(
    dead_code,
    clippy::struct_field_names,
    reason = "generated items a stub does not use; the Registry names M1's F1 parameters route_cost_*"
)]
mod m1 {
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/M1_regions.rs");
    purposes!();
}

#[allow(dead_code, reason = "generated items a stub does not use")]
mod s3 {
    use rh_core::{Phase, Pulse}; // S3 F0 has no parameter, so its skeleton names no `Fixed`
    include!("../../rh-core/tests/generated/S3_migration.rs");
    purposes!();
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
    type Index = ();

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
            applies: rh_core::access::CommandApply::from_drains(m2::DRAINS),
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

/// The violations of `e`, as text, other than `absent-owner`: S5 (a P1a draft whose identity
/// reads name S3 and M4) never registers in P1a (SPR-014 T-04; REV-EVO-001 R12), so the tests that
/// use it for another rule look at that rule's violations only.
fn besides_absent_owner(e: &rh_core::register::RegistrationError) -> Vec<String> {
    e.violations
        .iter()
        .filter(|v| v.rule != "absent-owner")
        .map(ToString::to_string)
        .collect()
}

/// The `absent-owner` violations S5 alone gets (SPR-014 T-04): its identity reads of S3 and M4,
/// and its mode read of M6 with no typed neutral; M2's columns are refused too when M2 is absent.
const S5_ABSENT: [&str; 3] = [
    "VIOLATION [absent-owner] S5.daily_life: reads person.residence as an identity read from S3.migration, which is not registered: a world that registers S5.daily_life registers S3.migration (05.1 §2 rule 2)",
    "VIOLATION [absent-owner] S5.daily_life: reads person.occupation as an identity read from M4.economy, which is not registered: a world that registers S5.daily_life registers M4.economy (05.1 §2 rule 2)",
    "VIOLATION [absent-owner] S5.daily_life: reads region.conditions as a mode read from M6.polity, which is not registered, and its spec gives no typed neutral (05.1 §2 rule 2: the type's zero is never a fallback)",
];

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
        density_capacity: 10_000_000,
        density_mortality_slope: Fixed::ONE,
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
    let cases: [(&str, &[(Pulse, Phase)], Vec<&'static str>); 6] = [
        (s5::SYSTEM_ID, s5::SCHEDULE, s5::RngPurpose::names()),
        (m2::SYSTEM_ID, m2::SCHEDULE, m2::RngPurpose::names()),
        (s6::SYSTEM_ID, s6::SCHEDULE, s6::RngPurpose::names()),
        (s7::SYSTEM_ID, s7::SCHEDULE, s7::RngPurpose::names()),
        (t1::SYSTEM_ID, t1::SCHEDULE, t1::RngPurpose::names()),
        (m7::SYSTEM_ID, m7::SCHEDULE, m7::RngPurpose::names()),
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

/// SPR-014 T-04 (REV-EVO-001 R12): M2 registers alone on its skeleton; S5 does not register
/// beside it, refused under `absent-owner` by name for every identity read of an owner the world
/// lacks (S3, M4) and for a mode read with no typed neutral (M6).
#[test]
fn m2_registers_on_its_skeleton_and_s5_is_refused_without_its_owners() {
    let t = real_ownership();
    let r =
        register(vec![Box::new(m2_system())], &t.spec_table()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.ids(), ["M2.demography"]);
    assert_eq!(r.order(Phase::Vital), ["M2.demography", "M2.demography"]);
    assert_eq!(r.accesses().len(), 1);
    let Err(e) = register(
        vec![
            Box::new(s5_system(s5::RngPurpose::names())),
            Box::new(m2_system()),
        ],
        &t.spec_table(),
    ) else {
        panic!("S5 without S3, M4 and M6 must be refused");
    };
    let shown: Vec<String> = e.violations.iter().map(ToString::to_string).collect();
    assert_eq!(shown, S5_ABSENT, "{e}");
}

/// SPR-006: the real specs' `[[commands]]` and `[[emits]]` reach registration. M2 declares the
/// `DEATH` it emits; S5's spec sends `TieDelta` to S1. Since SPR-014 (T-04) a submission to an
/// owner the world does not register is reported, not refused; an undeclared kind or a wrong owner
/// is still refused. (S5's own absent owners are refused too; `besides_absent_owner` sets them aside.)
#[test]
fn commands_and_event_kinds_are_checked_against_the_real_specs() {
    use rh_core::access::CommandSend;
    let t = real_ownership();
    let with = |emits: Vec<&'static str>,
                target: Option<&'static str>|
     -> Vec<Box<dyn SystemModel<Index = ()>>> {
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
    let Err(e) = register(with(vec!["DEATH"], None), &t.spec_table()) else {
        panic!("S5 has absent owners");
    };
    assert!(besides_absent_owner(&e).is_empty(), "{e}");
    assert!(
        e.reports
            .iter()
            .any(|n| n == "S5.daily_life: spec event kind ENCOUNTER has no use in code")
    );
    let Err(e) = register(with(vec!["DEATH"], Some("S1.culture")), &t.spec_table()) else {
        panic!("S5 has absent owners");
    };
    assert!(besides_absent_owner(&e).is_empty(), "{e}");
    assert!(
        e.reports.iter().any(|n| n
            == "S5.daily_life: submits command TieDelta to S1.culture, which is not registered: each submission is discarded and counted under dropped{S5.daily_life, TieDelta, absent_owner} (05.1 §2 rule 2)"),
        "{:?}",
        e.reports
    );
    let Err(e) = register(
        with(vec!["MURDER_ATTEMPT"], Some("M2.demography")),
        &t.spec_table(),
    ) else {
        panic!("an undeclared kind and a wrong owner must be refused");
    };
    let rules: Vec<(&str, &str)> = e
        .violations
        .iter()
        .filter(|v| v.rule != "absent-owner")
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
    let Err(e) = register(
        vec![Box::new(s5_system(vec!["activity_block"]))],
        &t.spec_table(),
    ) else {
        panic!("S5 has absent owners");
    };
    assert!(
        besides_absent_owner(&e).is_empty(),
        "only S5's absent owners: {e}"
    );
    assert!(
        e.reports
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
        [
            PulseBinding {
                pulse: Pulse::Month,
                phase: Phase::Vital
            },
            PulseBinding {
                pulse: Pulse::Day,
                phase: Phase::Vital
            }
        ]
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
    type Index = ();

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
                applies: spec.map_or_else(Vec::new, spec_applies),
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
        ("M2.demography", &["title.holder"], &["person.alive"]),
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
        // `absent-owner` is a rule of the world, not of one declaration: the lint, which judges a
        // declaration alone, has no such rule (SPR-014 T-04), so it is set aside here.
        let mut reg_rules: Vec<&str> = match &registered {
            Ok(_) => Vec::new(),
            Err(e) => e
                .violations
                .iter()
                .map(|v| v.rule)
                .filter(|r| *r != "absent-owner")
                .collect(),
        };
        lint_rules.sort_unstable();
        lint_rules.dedup();
        reg_rules.sort_unstable();
        reg_rules.dedup();
        assert_eq!(
            lint.is_empty(),
            reg_rules.is_empty(),
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
        besides_absent_owner(&e),
        [
            "VIOLATION [spec] S5.daily_life: reads person.sex at lag lagged, but its spec's [[reads]] declares 'start_of_day'"
        ]
    );
    let mut wrong_schedule = m2_system();
    wrong_schedule.schedule = &[(Pulse::Day, Phase::Vital)]; // the Month binding left out
    let Err(e) = register(vec![Box::new(wrong_schedule)], &t.spec_table()) else {
        panic!("a binding that differs from the spec must fail registration");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [spec] M2.demography: binds Day/Vital but its spec's [[schedule]] declares Day/Vital, Month/Vital"
        ]
    );
}

/// The commands a spec owns, each drained by the binding its `[[applies]] drained_by` names, as a
/// skeleton's `DRAINS` declares them (SPR-014 T-01).
fn spec_applies(spec: &SystemSpec) -> Vec<rh_core::access::CommandApply> {
    spec.applies
        .iter()
        .map(|a| {
            let (pulse, phase) = a
                .drained_by
                .as_deref()
                .and_then(|d| d.split_once('/'))
                .expect("a drained_by");
            rh_core::access::CommandApply::new(
                a.command.clone().leak(),
                Pulse::from_name(pulse).expect("a pulse"),
                Phase::from_name(phase).expect("a phase"),
            )
        })
        .collect()
}

/// `system` declared exactly as its spec says, in a world that registers `world`: every read with its
/// spec lag, declared unconditionally (REV-P1a round 4, NEW-4; SPR-014 T-04: registration refuses
/// an identity read of an absent owner and drops a rate or mode read, serving its neutral), every
/// owned column written, the event kinds it emits, every command it submits, declared
/// unconditionally too (a send to an absent owner is reported and its submissions discarded), and
/// the commands it owns with the bindings that drain them.
fn as_specified(t: &Ownership, system: &'static str, _world: &[&'static str]) -> Declared {
    let spec = &t.specs[system];
    let leak = |s: &String| -> &'static str { s.clone().leak() };
    // The commands it owns, each drained by the binding its spec names (SPR-014 T-01): as a
    // skeleton's DRAINS declares them, whatever the world registers.
    let applies = spec_applies(spec);
    Declared {
        id: system,
        access: AccessSpec {
            reads: spec
                .reads
                .iter()
                .map(|r| ColumnRead {
                    column: leak(&r.column),
                    owner: leak(&r.owner),
                    lag: Lag::from_name(&r.lag).expect("a spec lag"),
                })
                .collect(),
            writes: spec.owns.iter().map(|o| write(leak(&o.column))).collect(),
            sends: spec
                .commands
                .iter()
                .map(|c| rh_core::access::CommandSend {
                    command: leak(&c.command),
                    target: leak(&c.target),
                })
                .collect(),
            applies,
            emits: spec.emits.iter().map(leak).collect(),
            drain_only: Vec::new(),
        },
        bindings: spec
            .schedule
            .iter()
            .map(|(p, f)| PulseBinding {
                pulse: Pulse::from_name(p).expect("a pulse"),
                phase: Phase::from_name(f).expect("a phase"),
            })
            .collect(),
        purposes: spec.rng.iter().map(leak).collect(),
    }
}

/// PR #32 (Architect round 1, P5): the P1a trio M2, M7 and T1, each declared as its spec says,
/// registers together: no two owners, every same-instant read in phase order, no scheduler cycle
/// (T1 and M7 read each other only with lags), every command with a registered owner. A planted
/// same-instant read of a column M7 writes in a later phase is refused (the positive control).
#[test]
fn m2_m7_and_t1_register_together_as_specified() {
    let t = real_ownership();
    let world = ["M2.demography", "M7.dynasty", "T1.minds"];
    let trio = || -> Vec<Box<dyn SystemModel<Index = ()>>> {
        world
            .iter()
            .map(|&id| Box::new(as_specified(&t, id, &world)) as Box<dyn SystemModel<Index = ()>>)
            .collect()
    };
    let r = register(trio(), &t.spec_table()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.ids(), world);
    // M2 binds Vital at Month and, since 0.2.3, at Day (its drain, SPR-014); M7 0.3.2 binds
    // Day/Vital for its murder timer (EVO-001 R9): after M2 (it reads M2 at same_instant), before
    // T1 (no edge either way; ties by id).
    assert_eq!(
        r.order(Phase::Vital),
        ["M2.demography", "M2.demography", "M7.dynasty", "T1.minds"]
    );
    // One entry per binding (Year, Month, Day), coarser pulse first within a level.
    assert_eq!(
        r.order(Phase::Polity),
        ["M7.dynasty", "M7.dynasty", "M7.dynasty"]
    );
    // No same-instant edge between M7 and T1 in Agenda: one level, T1's Month binding first.
    assert_eq!(
        r.order(Phase::Agenda),
        ["T1.minds", "M7.dynasty", "T1.minds"]
    );
    // Positive control: T1 reading title.holder at same_instant (M7 writes it at Polity, after
    // T1's Vital binding) is a phase-order violation.
    let mut planted = as_specified(&t, "T1.minds", &world);
    for r in &mut planted.access.reads {
        if r.column == "title.holder" {
            r.lag = Lag::SameInstant;
        }
    }
    let mut systems = trio();
    systems.retain(|s| s.descriptor().id != "T1.minds");
    systems.push(Box::new(planted));
    let Err(e) = register(systems, &t.spec_table()) else {
        panic!("a same_instant read of a column written in a later phase must be refused");
    };
    assert!(
        e.violations.iter().any(|v| v.rule == "phase-order"
            && v.system == "T1.minds"
            && v.message.contains("title.holder")),
        "{e}"
    );
}

/// SPR-020 T-01: a system whose skeleton's role constants are those of another Registry. It is M2
/// on its skeleton, declaring `roles` instead of the skeleton's `ROLES`.
struct WithRoles(OnSkeleton, &'static [(&'static str, rh_core::RoleId)]);

impl SystemModel for WithRoles {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        self.0.descriptor()
    }
    fn access(&self) -> AccessSpec {
        self.0.access()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.0.schedule()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.0.rng_purposes()
    }
    fn roles(&self) -> &'static [(&'static str, rh_core::RoleId)] {
        self.1
    }
    fn params_schema(&self) -> ParamSchema {
        self.0.params_schema()
    }
    fn step(
        &self,
        b: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        self.0.step(b, v, c)
    }
    fn invariants(&self, w: &WorldTables) -> Vec<InvariantResult> {
        self.0.invariants(w)
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        self.0.declared_aggregates()
    }
}

/// SPR-020 T-01: registration compares a skeleton's `ROLE_*` constants with the Registry it runs
/// under, as it compares `SCHEDULE`: M2's generated `ROLES` pass, and a stale constant (or a role
/// the Registry lacks) is refused under rule `spec`, by the constant's name.
#[test]
fn a_skeleton_role_constant_is_checked_against_the_registry() {
    static STALE: [(&str, rh_core::RoleId); 2] = [
        ("actor", rh_core::RoleId(0)),
        ("target", rh_core::RoleId(2)),
    ];
    static UNKNOWN: [(&str, rh_core::RoleId); 1] = [("partner", rh_core::RoleId(2))];
    let registry = repo().join("specs/manifest/registry_v0.toml");
    let text = std::fs::read_to_string(&registry).expect("registry");
    let table = rh_registry::ownership::parse_registry_table("registry_v0.toml", &text)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        table.event_roles,
        ["actor", "target", "witness", "instrument"]
    );
    let specs = real_ownership().spec_table_with(table);
    assert_eq!(m2::ROLE_TARGET, rh_core::RoleId(1));
    let generated = WithRoles(m2_system(), m2::ROLES);
    register(vec![Box::new(generated)], &specs).unwrap_or_else(|e| panic!("{e}"));
    let Err(e) = register(vec![Box::new(WithRoles(m2_system(), &STALE))], &specs) else {
        panic!("a stale ROLE_TARGET must be refused");
    };
    let shown: Vec<String> = e.violations.iter().map(ToString::to_string).collect();
    assert_eq!(
        shown,
        [
            "VIOLATION [spec] M2.demography: skeleton constant ROLE_TARGET is 2, but the Registry's [event_roles] gives target the code 1: regenerate the skeleton (bin/gen_skeletons.py)"
        ]
    );
    let Err(e) = register(vec![Box::new(WithRoles(m2_system(), &UNKNOWN))], &specs) else {
        panic!("a role the Registry lacks must be refused");
    };
    assert!(
        e.violations.iter().any(|v| v
            .to_string()
            .contains("ROLE_PARTNER names the role partner")),
        "{e}"
    );
    // With no Registry passed, nothing is compared.
    register(
        vec![Box::new(WithRoles(m2_system(), &STALE))],
        &real_ownership().spec_table(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

/// Registers `world`, each system declared as its spec says (every read, every send).
fn register_world(
    t: &Ownership,
    world: &[&'static str],
) -> Result<rh_core::register::Registered, rh_core::register::RegistrationError> {
    let systems: Vec<Box<dyn SystemModel<Index = ()>>> = world
        .iter()
        .map(|&id| Box::new(as_specified(t, id, world)) as Box<dyn SystemModel<Index = ()>>)
        .collect();
    register(systems, &t.spec_table())
}

/// SPR-014 T-04, V-INV-04's absent-owner rows on the real specs (`05.1` §2 rule 2; REV-P1a NEW-4,
/// NEW-6; REV-EVO-001 R4, R12, R16):
///
/// | World | Outcome |
/// |---|---|
/// | {M2} | registers |
/// | {M2, M7} | registers; M7's rate and mode reads of M6 and T1 are dropped and reported |
/// | {M2, T1} | refused under `absent-owner`, naming T1, an M7 column and M7 |
/// | {M2, M7, T1} | registers; M2 before M7 and T1 in Vital; every `life_course.*` read is `same_instant` and no `life_course.*` column is lag-snapshotted; T1's send to the absent M4 is reported |
#[test]
fn v_inv_04_absent_owners_on_the_real_specs() {
    let t = real_ownership();
    // {M2}: M2 reads nothing foreign.
    let r = register_world(&t, &["M2.demography"]).unwrap_or_else(|e| panic!("{e}"));
    let dropped = r.dropped_reads("M2.demography");
    assert!(dropped.contains(&("house_member.person", 0)), "{dropped:?}");
    assert!(dropped.contains(&("person.named", 0)), "{dropped:?}");
    assert!(dropped.contains(&("person.residence", 0)), "{dropped:?}");
    assert!(dropped.contains(&("person.vocation", 0)), "{dropped:?}");
    // {M2, M7}: M7's rate read of M6 (exec_constraint) and mode read of T1 (person.named) are
    // dropped with their typed neutrals; nothing refused.
    let r = register_world(&t, &["M2.demography", "M7.dynasty"]).unwrap_or_else(|e| panic!("{e}"));
    let dropped = r.dropped_reads("M7.dynasty");
    assert!(
        dropped.contains(&("polity.exec_constraint", 0)),
        "{dropped:?}"
    );
    assert!(dropped.contains(&("person.named", 0)), "{dropped:?}");
    assert!(
        r.reports().iter().any(|n| n
            == "M7.dynasty: reads polity.exec_constraint from M6.polity, which is not registered: a rate read, dropped; its neutral is 0"),
        "{:?}",
        r.reports()
    );
    assert!(
        r.reports().iter().any(|n| n
            == "M7.dynasty: reads person.named from T1.minds, which is not registered: a mode read, dropped; its neutral is 0"),
        "{:?}",
        r.reports()
    );
    // {M2, T1}: T1 reads M7's columns as identity reads, so the world is refused, by name.
    let Err(e) = register_world(&t, &["M2.demography", "T1.minds"]) else {
        panic!("T1 without M7 must be refused");
    };
    assert!(!e.violations.is_empty());
    assert!(
        e.violations.iter().all(|v| v.rule == "absent-owner"
            && v.system == "T1.minds"
            && v.message
                .contains("from M7.dynasty, which is not registered")),
        "{e}"
    );
    assert!(
        e.violations.iter().any(|v| v
            .message
            .starts_with("reads title.holder as an identity read from M7.dynasty")),
        "{e}"
    );
    // {M2, M7, T1}: registers; NEW-6: life_course.* is read at same_instant and never snapshotted.
    let world = ["M2.demography", "M7.dynasty", "T1.minds"];
    let r = register_world(&t, &world).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        r.order(Phase::Vital),
        ["M2.demography", "M2.demography", "M7.dynasty", "T1.minds"]
    );
    let life_course: Vec<(&str, Lag)> = r
        .accesses()
        .iter()
        .flat_map(|(_, a)| a.reads.iter())
        .filter(|rd| rd.column.starts_with("life_course."))
        .map(|rd| (rd.column, rd.lag))
        .collect();
    assert!(!life_course.is_empty());
    assert!(
        life_course.iter().all(|(_, lag)| *lag == Lag::SameInstant),
        "{life_course:?}"
    );
    assert!(
        r.lagged_reads()
            .iter()
            .all(|c| !c.starts_with("life_course.")),
        "{:?}",
        r.lagged_reads()
    );
    assert!(
        r.reports().iter().any(|n| n
            == "T1.minds: submits command BurnHoldings to M4.economy, which is not registered: each submission is discarded and counted under dropped{T1.minds, BurnHoldings, absent_owner} (05.1 §2 rule 2)"),
        "{:?}",
        r.reports()
    );
    println!(
        "V-INV-04 real specs: {{M2}} and {{M2, M7}} register, {{M2, T1}} is refused by name, {{M2, M7, T1}} registers with no life_course.* snapshot"
    );
}

/// SPR-014 T-01 (REV-EVO-001 NIT-10): registration compares a skeleton's `DRAINS` with its spec's
/// `[[applies]] drained_by` under rule `spec`, as it compares `SCHEDULE`, and refuses a mismatch by
/// name: the command and both bindings. A spec drain the code does not declare is refused too.
#[test]
fn a_drain_that_differs_from_its_spec_is_refused_by_name() {
    let t = real_ownership();
    let mut moved = m2_system();
    moved.access.applies[0].drained_by = PulseBinding {
        pulse: Pulse::Month,
        phase: Phase::Vital,
    };
    let Err(e) = register(vec![Box::new(moved)], &t.spec_table()) else {
        panic!("a drain that differs from the spec must be refused");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [spec] M2.demography: drains command ScheduleDeath at Month/Vital, but its spec's [[applies]] says drained_by = Day/Vital"
        ]
    );
    let mut missing = m2_system();
    missing.access.applies.retain(|a| a.command != "Marry");
    let Err(e) = register(vec![Box::new(missing)], &t.spec_table()) else {
        panic!("a spec drain the code leaves out must be refused");
    };
    assert_eq!(
        e.violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "VIOLATION [spec] M2.demography: its spec's [[applies]] declares command Marry drained_by = Day/Vital, but its code drains nothing for it"
        ]
    );
    println!("DRAINS: a drain that differs from its spec, or is missing, is refused by name");
}

/// The review's K10: a payload that does not parse is kept as malformed (registration refuses it
/// by name), never read as an untyped command; a well-formed one is typed.
#[test]
fn a_malformed_applies_payload_is_kept_as_malformed() {
    let text = |payload: &str| {
        format!(
            "[system]\nid = \"M2.demography\"\n[[applies]]\ncommand = \"Kill\"\npayload = \"{payload}\"\ndrained_by = \"Day/Vital\"\nkey = \"living\"\nkey_field = \"victim\"\n"
        )
    };
    let entry = |payload: &str| {
        let spec = SystemSpec::from_toml(Path::new("M2.odd.toml"), &text(payload)).unwrap();
        let ids = std::iter::once("M2.demography".to_owned()).collect();
        let table = Ownership::build(&[spec], &ids).spec_table();
        table.specs["M2.demography"].applies[0].clone()
    };
    let bad = entry("victim PersonId");
    assert_eq!(bad.payload, None);
    assert_eq!(bad.payload_malformed.as_deref(), Some("victim PersonId"));
    let good = entry("{ victim: PersonId }");
    assert_eq!(
        good.payload,
        Some(vec![("victim".to_owned(), "PersonId".to_owned())])
    );
    assert_eq!(good.payload_malformed, None);
}

/// The review's K11: a rate read of a column whose owner has no spec yet gets its neutral typed by
/// the literal alone: a zero is zero in every type; a non-zero integer is left untyped (it might be
/// a `q8_8` or `q16_16` value).
#[test]
fn a_non_zero_neutral_for_a_column_with_no_spec_is_left_untyped() {
    let neutral = |value: &str| {
        let text = format!(
            "[system]\nid = \"M3.agriculture\"\n[[reads]]\ncolumn = \"era.level\"\nowner = \"M6.polity\"\nlag = \"lagged\"\nuse = \"rate\"\nneutral = {value}\n"
        );
        let spec = SystemSpec::from_toml(Path::new("M3.odd.toml"), &text).unwrap();
        let ids = ["M3.agriculture", "M6.polity"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let table = Ownership::build(&[spec], &ids).spec_table();
        table.specs["M3.agriculture"].read_uses[0].neutral
    };
    assert_eq!(neutral("0"), Some(0));
    assert_eq!(neutral("5"), None);
}
