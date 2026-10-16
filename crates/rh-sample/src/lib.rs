//! `rh-sample`: the vendored distribution samplers, and plane A's only float zone.
//!
//! Plane A, determinism machinery. Systems draw binomial, Poisson, normal, gamma and beta variates
//! here, from `rh-ids`'s keyed streams, and get integers or `rh_core::fixed` values back. Floats are
//! used inside this crate only, under the rules of the Architect's ruling of 2026-10-16 (SPR-003,
//! "the sampler float zone"):
//!
//! 1. **The zone is this crate.** `bin/lint_arch.py` pins it to (A, machinery) by name, and
//!    `bin/lint_forbidden.py` exempts it, by that name, from the float rule only; every other rule
//!    of V-DET-19 still applies here. The clippy float lints are allowed by the one crate-level
//!    attribute below.
//! 2. **Transcendentals come from `libm` only.** `clippy.toml` disallows the `std` ones (`ln`, `exp`,
//!    `powf`, ...) workspace-wide; `sqrt` is allowed because IEEE 754 rounds it correctly.
//! 3. **No fused multiply-add.** `mul_add` is forbidden (V-DET-19), and rustc never contracts
//!    `a * b + c` into an FMA on its own: Rust has no fast-math and no `-ffp-contract=fast` mode.
//!    Every expression is evaluated in its written order, so the operation order is fixed by the
//!    source.
//! 4. **Floats never leave the crate.** The public API takes and returns integers or
//!    `rh_core::fixed` values. An internal `f64` becomes fixed point from its exact value, rounded
//!    half to even (the `rh_core::fixed` rule), without an `as` cast (`convert`).
//! 5. **Draws come from keyed streams.** A sampler reads `rh_ids::RngStream` sequentially from its
//!    current position; each sampler documents how many draws it consumes.
//!
//! REF: `CODEX/05_PROJECT/SPR-003_Samplers.md`; ADR-0006 items 3–4; `05.1` §6 items 3–4;
//! `docs/14` (`rand_distr`: PORT; `libm`: DEPEND); `docs/09` V-DET-09, V-DET-19.
#![forbid(unsafe_code)]
// The float zone (ruling item 2): the one crate-level allow of the workspace's float lints.
#![allow(
    clippy::float_arithmetic,
    clippy::float_cmp,
    reason = "rh-sample is plane A's float zone (SPR-003 Architect ruling, 2026-10-16): samplers \
              compute in f64, in a fixed order with libm, and return integers or fixed point"
)]
mod beta;
mod binomial;
mod convert;
mod error;
mod gamma;
mod normal;
mod poisson;
mod uniform;
mod ziggurat_tables;

pub use beta::Beta;
pub use binomial::Binomial;
pub use error::SampleError;
pub use gamma::Gamma;
pub use normal::Normal;
pub use poisson::Poisson;
