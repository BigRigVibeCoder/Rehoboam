//! Fixture: rh-registry outside the loaders' home (review MAJ-2). A re-export under another name
//! and an exported macro that expands to a call would carry the loaders to any crate under names
//! the token rule never sees there, so both are reported here, at their source.
pub mod ownership;

pub use ownership::load_specs as read_dir;

#[macro_export]
macro_rules! ids {
    () => {
        $crate::ownership::registry_system_ids()
    };
}
