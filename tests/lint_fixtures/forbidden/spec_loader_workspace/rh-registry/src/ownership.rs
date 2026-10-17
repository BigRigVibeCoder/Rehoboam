//! Fixture: the loaders' home, which defines and calls them on its load path (never reported).
pub fn load_specs() {}
pub fn registry_system_ids() {}
pub fn load() {
    load_specs();
    registry_system_ids();
}
