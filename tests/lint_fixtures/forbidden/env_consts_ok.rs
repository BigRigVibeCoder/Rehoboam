// Fixture (DEF-5): std::env::consts is the one path into std::env that is allowed (the target's
// constants, fixed by the build), and env! and option_env! are compile-time, out of the rule's
// scope. Never compiled; scanned as plane-A code by bin/check_all.sh: it must pass.

use std::env::consts;
use std::env::consts::FAMILY;

pub fn target() -> [&'static str; 3] { [consts::ARCH, std::env::consts::OS, FAMILY] }
pub fn built() -> (&'static str, Option<&'static str>) { (env!("CARGO_PKG_NAME"), option_env!("RH_BUILD")) }
