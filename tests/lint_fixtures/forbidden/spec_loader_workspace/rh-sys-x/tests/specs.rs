//! Fixture: a system crate's tests may build the spec table from the files (never reported).
#[test]
fn registers() {
    let _ = rh_registry::ownership::load_specs(std::path::Path::new("specs/systems"));
}
