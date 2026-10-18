//! `rh-view-core`: Bevy-free core voxel processing, greedy meshing, terrain voxelization, and digests (SPR-033).

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

pub mod chunk;
pub mod digest;
pub mod export;
pub mod lod;
pub mod mesh;
pub mod palette;
pub mod terrain;

#[cfg(feature = "mutant-unsafe")]
fn _planted_unsafe() {
    unsafe {}
}
