// Positive control (VER-SPR-008): include_str!/include_bytes! of a file outside the repository
// FAILS; an in-repository literal is allowed (python/tests). Never compiled.
pub const HOST: &str = include_str!("/etc/hostname");
