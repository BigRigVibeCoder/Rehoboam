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

use std::fmt::Write as _;
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
    type Index = ();
    
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

/// The real specs plus a reader's, registered with the real M2 and the reader.
fn registered(reader: Box<dyn SystemModel>, id: &str, spec: &str) -> rh_core::register::Registered {
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
    specs.push(SystemSpec::from_toml(Path::new("reader.odd.toml"), spec).unwrap());
    let registry = std::fs::read_to_string(repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let mut ids = parse_registry_system_ids("registry_v0.toml", &registry).unwrap();
    ids.insert(id.to_owned());
    let table = Ownership::build(&specs, &ids).spec_table();
    let m2 = Demography::new(Q16_16::from_int(30), None, Q16_16::ONE, CALENDAR).unwrap();
    register(vec![Box::new(m2), reader], &table).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_reader_beside_the_real_m2_reads_its_columns_through_the_contract() {
    let registered = registered(Box::new(Reader), "Z1.reader", READER_SPEC);
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

/// What every accessor must give on one row, decoded from the spec alone: its `[[owns]]` type
/// and `values` (`Err` for an enum code outside them).
type Expected = Vec<(&'static str, usize, Result<Option<m2::Value>, ()>)>;

/// A reader of all nineteen columns that calls every [`m2::ACCESSORS`] entry on every seeded row
/// and compares it with the spec's decode (the Tester's DEF-1: a wrong column, a swapped enum
/// decode, a scaled time or a wrapped code in any accessor fails here).
struct AllReader {
    expected: Expected,
}

impl SystemModel for AllReader {
    type Index = ();
    
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: "Z2.all",
            fidelity: "F0",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: m2::COLUMNS
                .iter()
                .map(|&(column, _)| ColumnRead {
                    column,
                    owner: "M2.demography",
                    lag: Lag::SameInstant,
                })
                .collect(),
            writes: vec![ColumnWrite {
                column: "all.checked",
                plane: Plane::A,
            }],
            ..AccessSpec::default()
        }
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Agenda,
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
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        if c.now().minutes() != 0 {
            return Ok(());
        }
        let mut checked = 0;
        for &(column, row, ref want) in &self.expected {
            let (_, accessor) = m2::ACCESSORS
                .iter()
                .find(|(c, _)| *c == column)
                .unwrap_or_else(|| panic!("no accessor for {column}"));
            match (accessor(v, row), want) {
                (Ok(got), Ok(want)) => assert_eq!(got, *want, "{column} row {row}"),
                (Err(e), Err(())) => assert!(
                    e.to_string().contains(column) && e.to_string().contains("outside"),
                    "{column} row {row}: {e}"
                ),
                (got, want) => panic!("{column} row {row}: got {got:?}, the spec gives {want:?}"),
            }
            checked += 1;
        }
        v.write("all.checked")?.push(checked);
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

/// The spec's decode of one raw value.
fn decode(ty: &str, values: Option<&Vec<String>>, raw: i64) -> Result<m2::Value, ()> {
    Ok(match ty {
        "u64" => m2::Value::Person(rh_ids::PersonId::from_raw(raw.cast_unsigned())),
        "i64" => m2::Value::Time(SimTime::from_minutes(raw)),
        "bool" => m2::Value::Bool(raw != 0),
        "enum" => {
            let name = usize::try_from(raw)
                .ok()
                .and_then(|i| values.and_then(|v| v.get(i)))
                .ok_or(())?;
            m2::Value::Code(name.clone().leak())
        }
        other => panic!("type {other} has no accessor decode"),
    })
}

/// Seeded M2 columns: three ledger rows and seven life-course rows, every column's values
/// distinct from its neighbours', every enum code used, and one closed row (person 77) whose
/// origin (7) and sex (-1) are outside their codes. M2's invariants and the ledger's hold.
fn seeded() -> Vec<(&'static str, Vec<i64>)> {
    let max = i64::MAX;
    vec![
        ("person.id", vec![11, 22, 33]),
        ("person.birth_tick", vec![-101, -202, -303]),
        ("person.sex", vec![0, 1, 0]),
        ("person.alive", vec![1, 1, 1]),
        ("person.death_tick", vec![max, max, max]),
        ("person.mother", vec![41, 42, 43]),
        ("person.father", vec![51, 52, 53]),
        ("person.spouse", vec![61, 62, 63]),
        ("person.death_cause", vec![0, 2, 5]),
        ("life_course.person_id", vec![11, 22, 33, 44, 55, 66, 77]),
        ("life_course.origin", vec![1, 1, 1, 0, 2, 1, 7]),
        ("life_course.mother", vec![71, 72, 73, 74, 75, 76, 79]),
        ("life_course.father", vec![81, 82, 83, 84, 85, 86, 89]),
        ("life_course.sex", vec![0, 1, 0, 1, 0, 1, -1]),
        (
            "life_course.birth_tick",
            vec![-101, -202, -303, -404, -505, -606, -707],
        ),
        ("life_course.entry_tick", vec![-1, -2, -3, -4, -5, -6, -7]),
        ("life_course.exit_kind", vec![0, 0, 0, 1, 2, 1, 1]),
        (
            "life_course.exit_tick",
            vec![max, max, max, -10, -20, -30, -40],
        ),
        ("life_course.death_cause", vec![0, 0, 0, 1, 0, 3, 4]),
    ]
}

#[test]
fn every_accessor_decodes_its_spec_column_on_seeded_rows() {
    let path = repo().join("specs/systems/M2.demography.odd.toml");
    let spec = SystemSpec::from_toml(&path, &std::fs::read_to_string(&path).unwrap()).unwrap();
    let columns = seeded();
    let mut expected: Expected = Vec::new();
    for owns in &spec.owns {
        let rows = &columns
            .iter()
            .find(|(c, _)| *c == owns.column)
            .unwrap_or_else(|| panic!("{} is not seeded", owns.column))
            .1;
        let column: &'static str = owns.column.clone().leak();
        for (row, &raw) in rows.iter().enumerate() {
            let want = decode(&owns.ty, owns.values.as_ref(), raw).map(Some);
            expected.push((column, row, want));
        }
        expected.push((column, rows.len(), Ok(None)));
    }
    let n = expected.len();
    assert_eq!(
        m2::ACCESSORS.len(),
        spec.owns.len(),
        "one accessor per owned column"
    );
    let mut reader_spec = String::from(
        "[system]\nid = \"Z2.all\"\n[[owns]]\ncolumn = \"all.checked\"\nentity = \"all\"\ntype = \"i64\"\nplane = \"A\"\n",
    );
    for (column, _) in m2::COLUMNS {
        writeln!(
            reader_spec,
            "[[reads]]\ncolumn = \"{column}\"\nowner = \"M2.demography\"\nlag = \"same_instant\"\nuse = \"identity\""
        )
        .unwrap();
    }
    reader_spec.push_str("[[schedule]]\npulse = \"Day\"\nphase = \"Agenda\"\n");
    let registered = registered(Box::new(AllReader { expected }), "Z2.all", &reader_spec);
    let mut t = WorldTables::new();
    for (column, rows) in columns {
        t.insert(column, rows);
    }
    t.insert("all.checked", Vec::new());
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
    assert_eq!(
        e.tables().get("all.checked").unwrap(),
        &vec![i64::try_from(n).unwrap()],
        "every expectation checked"
    );
    println!(
        "rh-contracts consumer: {} accessors, {n} seeded values decoded as the spec says",
        m2::ACCESSORS.len()
    );
}
