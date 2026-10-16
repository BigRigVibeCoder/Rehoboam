//! Fixed-point and money types for every threshold quantity in plane A (DET-3).
//!
//! - `Q8_8` (i16 storage) and `Q16_16` (i32 storage): the `q8_8` and `q16_16` column types of the
//!   system specs (`specs/systems/_template.odd.toml`).
//! - `Money`: i64 minor units.
//!
//! There is no conversion from or to `f32`/`f64` anywhere in this module, under any feature: a
//! plane-A crate cannot reach one (`05.1` §6 item 4). A tool that wants to plot a value divides
//! `to_bits()` by `2^FRAC_BITS` itself, outside plane A. The three types never mix implicitly; the
//! compile-fail controls in `tests/lint_fixtures/compile_fail/fixed_point/` prove both rules.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md`; `05.1` §6 item 4; `05.5` §2.1; ADR-0006;
//! GOV-003 §2.

mod money;
mod q;
mod round;

pub use money::Money;
pub use q::{Q8_8, Q16_16};
