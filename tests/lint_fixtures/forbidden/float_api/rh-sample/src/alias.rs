//! A private type alias carries a float through a public signature without naming it there.

type Rate = f64;

/// The rate, as a float, behind an alias.
#[must_use]
pub fn rate() -> Rate {
    0.25
}
