//! Plane-A fixture. Mentions of HashMap in comments and strings are not findings:
/* nor in a block comment: HashMap, thread_rng, /* nested: SystemTime */ unsafe */

/// Doc comments may say HashMap too.
pub const NOTE: &str = "HashMap in a string";
pub const RAW: &str = r#"thread_rng() in a "raw" string"#;
pub const QUOTE: char = '"';

pub fn lookup(map: &std::collections::HashMap<u32, u32>, key: u32) -> Option<u32> {
    map.get(&key).copied()
}

pub fn fused(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c) // LINT-ALLOW(forbidden-api): fixture for a counted escape with a reason
}

pub fn label<'a>(s: &'a str) -> &'a str {
    s
}
