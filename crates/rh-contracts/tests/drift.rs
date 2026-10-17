//! The spec-drift test of `rh_contracts::m2` (SPR-014 T-06; `docs/13` §4.1): every accessor's
//! column, type and enum codes equal `M2.demography`'s `[[owns]]`, in order, and every owned
//! column has an accessor.
//!
//! `RH_CONTRACTS_M2_SPEC` points the test at another spec: `bin/check_all.sh` plants one with
//! `person.sex`'s values reordered (`tests/lint_fixtures/contracts/M2_sex_reordered.odd.toml`),
//! which must FAIL naming the column.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

use std::path::{Path, PathBuf};

use rh_contracts::m2::{COLUMNS, ColumnType};
use rh_registry::ownership::SystemSpec;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The spec type an accessor's type stands for, with the enum codes.
fn spec_type(t: ColumnType) -> (&'static str, Option<Vec<String>>) {
    match t {
        ColumnType::Person => ("u64", None),
        ColumnType::Time => ("i64", None),
        ColumnType::Bool => ("bool", None),
        ColumnType::Enum(values) => (
            "enum",
            Some(values.iter().map(|v| (*v).to_owned()).collect()),
        ),
    }
}

/// The spec under test: M2's, or the override's.
// LINT-ALLOW(forbidden-api): a test reads an environment override for its planted control; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its planted control"
)]
fn spec_path() -> PathBuf {
    std::env::var_os("RH_CONTRACTS_M2_SPEC").map_or_else(
        || repo().join("specs/systems/M2.demography.odd.toml"),
        PathBuf::from,
    )
}

#[test]
fn every_m2_accessor_matches_the_spec() {
    let path = spec_path();
    let text = std::fs::read_to_string(&path).unwrap();
    let spec = SystemSpec::from_toml(&path, &text).unwrap();
    assert_eq!(spec.id, rh_contracts::m2::OWNER);
    let owned: Vec<&str> = spec.owns.iter().map(|o| o.column.as_str()).collect();
    let read: Vec<&str> = COLUMNS.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        read, owned,
        "the accessors cover exactly the spec's [[owns]], in order"
    );
    for ((column, ty), owns) in COLUMNS.iter().zip(&spec.owns) {
        let (want, codes) = spec_type(*ty);
        assert_eq!(
            owns.ty, want,
            "DRIFT in {column}: the spec types it {}, the accessor {want}",
            owns.ty
        );
        assert_eq!(
            owns.values, codes,
            "DRIFT in {column}: the spec's values are {:?}, the accessor's {codes:?}",
            owns.values
        );
    }
    println!(
        "rh-contracts drift: {} M2 accessors match the spec",
        COLUMNS.len()
    );
}
