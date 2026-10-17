//! The one error type of `rh-sample`: a sampler built with parameters outside its domain.
//!
//! REF: GOV-004 (typed errors); GOV-003 §2 (no `unwrap` in library code).

use core::fmt;

/// A sampler's parameters are outside its domain. Returned by the constructors, never by `sample`:
/// once built, a sampler always returns a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleError {
    sampler: &'static str,
    reason: &'static str,
}

impl SampleError {
    /// An error for `sampler` (its type name) with a one-line `reason`.
    pub(crate) const fn new(sampler: &'static str, reason: &'static str) -> Self {
        Self { sampler, reason }
    }

    /// The sampler that refused its parameters.
    #[must_use]
    pub const fn sampler(&self) -> &'static str {
        self.sampler
    }

    /// Why the parameters were refused.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.sampler, self.reason)
    }
}

impl std::error::Error for SampleError {}
