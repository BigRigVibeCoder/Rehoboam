//! Palette v0: voxel material definitions and properties (SPR-033 T-02).

/// Air (empty space).
pub const AIR: u16 = 0;
/// Water (transparent surface).
pub const WATER: u16 = 1;
/// Sand.
pub const SAND: u16 = 2;
/// Grass.
pub const GRASS: u16 = 3;
/// Dirt.
pub const DIRT: u16 = 4;
/// Rock.
pub const ROCK: u16 = 5;
/// Packed earth (e.g. roads).
pub const PACKED_EARTH: u16 = 6;
/// Stone (e.g. plazas).
pub const STONE: u16 = 7;

// Reserved IDs for SPR-034's building materials:
pub const TIMBER: u16 = 8;
pub const PLASTER: u16 = 9;
pub const THATCH: u16 = 10;
pub const SHINGLE: u16 = 11;
pub const COBBLE: u16 = 12;

/// Checks if the material is air.
#[inline]
#[must_use]
pub const fn is_air(id: u16) -> bool {
    id == AIR
}

/// Checks if the material is water.
#[inline]
#[must_use]
pub const fn is_water(id: u16) -> bool {
    id == WATER
}

/// Checks if the material is transparent to the mesher.
/// Only water is transparent in `palette_v0`.
#[inline]
#[must_use]
pub const fn is_transparent(id: u16) -> bool {
    id == WATER
}

/// Checks if the material is opaque.
#[inline]
#[must_use]
pub const fn is_opaque(id: u16) -> bool {
    id != AIR && !is_transparent(id)
}
