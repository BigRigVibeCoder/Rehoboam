//! Fixture: plane-A engine code reading the environment (the env rule must report line 5 only).

pub fn build(b: Builder) -> Builder { b.args(&[1, 2]) }

pub fn seed() -> Option<String> { std::env::var("SEED").ok() }
