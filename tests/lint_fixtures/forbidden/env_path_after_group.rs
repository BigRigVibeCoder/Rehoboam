// Fixture (DEF-5): `use std::{env, io};` on line 4 and `env::current_dir` on line 5; the path must
// be reported on its own line, so the `env::<item>` pattern cannot be deleted unseen.

use std::{env, io};
pub fn cwd() -> io::Result<std::path::PathBuf> { env::current_dir() }
