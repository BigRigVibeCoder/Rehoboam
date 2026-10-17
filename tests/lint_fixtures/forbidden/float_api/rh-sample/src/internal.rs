//! Floats behind pub(crate): the zone's own business, never reported.

/// The internal uniform.
pub(crate) fn uniform(x: u64) -> f64 {
    f64::from_bits(x)
}

/// An internal struct with a float field.
pub(crate) struct Inner {
    pub(crate) value: f64,
}
