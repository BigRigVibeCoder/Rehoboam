//! Fixture: a system crate reaching for the spec loaders (the spec-loader rule must report each).
use rh_registry::ownership::{load_specs, registry_system_ids as ids};

pub fn step() {
    let _ = rh_registry::ownership::load_specs(std::path::Path::new("specs/systems"));
    let loader = registry_system_ids;
    let _ = (loader, ids, load_specs);
}
