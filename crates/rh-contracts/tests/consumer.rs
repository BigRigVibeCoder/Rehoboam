//! The consumer contract test of `rh_contracts::m2` (SPR-014 T-06; `docs/13` §4.3): a system that
//! is not M2 reads M2's columns only through the crate, beside the real `M2.demography`, registered
//! against the real specs plus its own. Its reads go through its own `AccessSpec`: a column it does
//! not declare is refused through the accessor too.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

use std::path::{Path, PathBuf};

use rh_contracts::m2::{self, DeathCause, ExitKind, Origin, Sex};
use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::engine::Engine;
use rh_core::fixed::Q16_16;
use rh_core::register::register;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, StepCtx, SystemDescriptor,
    SystemModel,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::BranchSalt;
use rh_registry::ownership::{Ownership, SystemSpec, parse_registry_system_ids, parse_specs};
use rh_sys_demography::Demography;

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The reader's spec: it reads six M2 columns at `same_instant` and owns one column.
const READER_SPEC: &str = r#"
[system]
id = "Z1.reader"
[[owns]]
column = "reader.seen"
entity = "reader"
type = "i64"
plane = "A"
[[reads]]
column = "person.id"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "person.alive"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "person.sex"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "person.death_cause"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "life_course.origin"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "life_course.exit_kind"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[schedule]]
pulse = "Day"
phase = "Polity"
"#;

const READS: [&str; 6] = [
    "person.id",
    "person.alive",
    "person.sex",
    "person.death_cause",
    "life_course.origin",
    "life_course.exit_kind",
];

struct Reader;

impl SystemModel for Reader {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: "Z1.reader",
            fidelity: "F0",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: READS
                .iter()
                .map(|&column| ColumnRead {
                    column,
                    owner: "M2.demography",
                    lag: Lag::SameInstant,
                })
                .collect(),
            writes: vec![ColumnWrite {
                column: "reader.seen",
                plane: Plane::A,
            }],
            ..AccessSpec::default()
        }
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Polity,
        }]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let mut seen = Vec::new();
        let person = m2::person_id(v, 1)?.expect("row 1");
        let row = v.person_row(person)?.expect("its ledger row");
        assert_eq!(m2::alive(v, row)?, Some(true));
        assert_eq!(m2::sex(v, row)?, Some(Sex::Female));
        assert_eq!(m2::death_cause(v, row)?, Some(DeathCause::None));
        assert_eq!(m2::lc_origin(v, row)?, Some(Origin::Founder));
        assert_eq!(m2::lc_exit_kind(v, row)?, Some(ExitKind::Open));
        assert_eq!(m2::alive(v, 99)?, None, "a row the column does not have");
        // A column the reader does not declare is refused through the accessor as well.
        let refused = m2::birth_tick(v, row).unwrap_err();
        assert!(
            matches!(refused, SimError::UndeclaredRead { .. }),
            "{refused}"
        );
        seen.push(i64::try_from(row).unwrap());
        v.write("reader.seen")?.extend(seen);
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

#[test]
fn a_reader_beside_the_real_m2_reads_its_columns_through_the_contract() {
    let mut files: Vec<(PathBuf, String)> = std::fs::read_dir(repo().join("specs/systems"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect();
    files.sort();
    let mut specs = parse_specs(files).unwrap();
    specs.push(SystemSpec::from_toml(Path::new("Z1.reader.odd.toml"), READER_SPEC).unwrap());
    let registry = std::fs::read_to_string(repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let mut ids = parse_registry_system_ids("registry_v0.toml", &registry).unwrap();
    ids.insert("Z1.reader".to_owned());
    let table = Ownership::build(&specs, &ids).spec_table();
    let m2 = Demography::new(Q16_16::from_int(30), None, Q16_16::ONE, CALENDAR).unwrap();
    let registered =
        register(vec![Box::new(m2), Box::new(Reader)], &table).unwrap_or_else(|e| panic!("{e}"));
    // Three founders, sorted by id; row 1 is a woman.
    let ids: Vec<i64> = vec![11, 22, 33];
    let n = ids.len();
    let mut t = WorldTables::new();
    for (column, rows) in [
        ("person.id", ids.clone()),
        (
            "person.birth_tick",
            vec![-20 * CALENDAR.minutes_per_year(); n],
        ),
        ("person.sex", vec![0, 1, 0]),
        ("person.alive", vec![1; n]),
        ("person.death_tick", vec![i64::MAX; n]),
        ("person.mother", vec![0; n]),
        ("person.father", vec![0; n]),
        ("person.spouse", vec![0; n]),
        ("person.death_cause", vec![0; n]),
        ("life_course.person_id", ids),
        ("life_course.origin", vec![1; n]),
        ("life_course.mother", vec![0; n]),
        ("life_course.father", vec![0; n]),
        ("life_course.sex", vec![0, 1, 0]),
        (
            "life_course.birth_tick",
            vec![-20 * CALENDAR.minutes_per_year(); n],
        ),
        ("life_course.entry_tick", vec![0; n]),
        ("life_course.exit_kind", vec![0; n]),
        ("life_course.exit_tick", vec![i64::MAX; n]),
        ("life_course.death_cause", vec![0; n]),
        ("reader.seen", Vec::new()),
    ] {
        t.insert(column, rows);
    }
    let mut e = Engine::new(
        CALENDAR,
        3,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(1_439), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(e.tables().get("reader.seen").unwrap(), &vec![1]);
}
