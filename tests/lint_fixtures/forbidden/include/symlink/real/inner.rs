// Reached as ../link.rs (a symlink to this file). rustc joins "gen.rs" to link.rs's directory, so
// the include is ../gen.rs, whose HashMap must FAIL (VER-SPR-008 MIN-3). Never compiled.
include!("gen.rs");
