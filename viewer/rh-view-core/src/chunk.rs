//! 62³ voxel chunks stored padded to 64³ in ZXY order (SPR-033 T-02).

use crate::palette;

/// The interior dimension of a chunk (voxels per axis).
pub const CS: usize = 62;
/// The padded dimension of a chunk (with 1 voxel of padding on each boundary).
pub const CS_P: usize = 64;
/// The area of a padded slice.
pub const CS_P2: usize = CS_P * CS_P;
/// The total number of voxels in a padded chunk buffer.
pub const CS_P3: usize = CS_P * CS_P * CS_P;
/// The total number of voxels in a chunk interior.
pub const CS_3: usize = CS * CS * CS;

/// Chunk coordinate in the world hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkCoord {
    /// Level of detail (0 = 0.35m, 1 = 0.70m, 2 = 1.40m).
    pub level: u8,
    /// Chunk grid coordinate X.
    pub x: i32,
    /// Chunk grid coordinate Y.
    pub y: i32,
    /// Chunk grid coordinate Z.
    pub z: i32,
}

impl ChunkCoord {
    /// Creates a new chunk coordinate.
    #[must_use]
    pub const fn new(level: u8, x: i32, y: i32, z: i32) -> Self {
        Self { level, x, y, z }
    }
}

/// A 62³ voxel chunk with 1 voxel of padding on each side (64³ total buffer).
/// Stored in ZXY order: index = z + x * 64 + y * 64 * 64.
pub struct Chunk {
    /// The coordinate of this chunk.
    pub coord: ChunkCoord,
    /// Padded voxel data (64³).
    voxels: Box<[u16; CS_P3]>,
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self {
            coord: self.coord,
            voxels: self.voxels.clone(),
        }
    }
}

impl Chunk {
    /// Creates an empty chunk filled with AIR (0).
    #[must_use]
    pub fn new(coord: ChunkCoord) -> Self {
        let buf = vec![palette::AIR; CS_P3].into_boxed_slice();
        let Ok(voxels) = buf.try_into() else {
            unreachable!()
        };
        Self { coord, voxels }
    }

    /// Padded buffer linear index in ZXY order.
    #[inline]
    #[must_use]
    pub const fn pad_index(px: usize, py: usize, pz: usize) -> usize {
        pz + px * CS_P + py * CS_P * CS_P
    }

    /// Interior linear index in ZXY order (px = x + 1, py = y + 1, pz = z + 1).
    #[inline]
    #[must_use]
    pub const fn interior_index(x: usize, y: usize, z: usize) -> usize {
        Self::pad_index(x + 1, y + 1, z + 1)
    }

    /// Gets a voxel from the chunk interior (x, y, z in 0..62).
    #[inline]
    #[must_use]
    pub fn get(&self, x: usize, y: usize, z: usize) -> u16 {
        debug_assert!(
            x < CS && y < CS && z < CS,
            "interior coordinates out of bounds"
        );
        self.voxels[Self::interior_index(x, y, z)]
    }

    /// Sets a voxel in the chunk interior (x, y, z in 0..62).
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, material: u16) {
        debug_assert!(
            x < CS && y < CS && z < CS,
            "interior coordinates out of bounds"
        );
        let idx = Self::interior_index(x, y, z);
        self.voxels[idx] = material;
    }

    /// Gets a voxel from the padded buffer (px, py, pz in 0..64).
    #[inline]
    #[must_use]
    pub fn get_padded(&self, px: usize, py: usize, pz: usize) -> u16 {
        debug_assert!(
            px < CS_P && py < CS_P && pz < CS_P,
            "padded coordinates out of bounds"
        );
        self.voxels[Self::pad_index(px, py, pz)]
    }

    /// Sets a voxel in the padded buffer (px, py, pz in 0..64).
    #[inline]
    pub fn set_padded(&mut self, px: usize, py: usize, pz: usize, material: u16) {
        debug_assert!(
            px < CS_P && py < CS_P && pz < CS_P,
            "padded coordinates out of bounds"
        );
        let idx = Self::pad_index(px, py, pz);
        self.voxels[idx] = material;
    }

    /// Returns a slice of the entire padded voxel buffer.
    #[inline]
    #[must_use]
    pub fn padded_voxels(&self) -> &[u16; CS_P3] {
        &self.voxels
    }

    /// Checks if the entire chunk interior is air.
    #[must_use]
    pub fn is_interior_all_air(&self) -> bool {
        for y in 0..CS {
            for x in 0..CS {
                for z in 0..CS {
                    if self.get(x, y, z) != palette::AIR {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Fills the padding boundary voxels from neighbouring chunks.
    pub fn fill_padding(&mut self, neighbours: &ChunkNeighbours<'_>) {
        if let Some(c) = neighbours.neg_x {
            for y in 0..CS {
                for z in 0..CS {
                    self.set_padded(0, y + 1, z + 1, c.get(CS - 1, y, z));
                }
            }
        }
        if let Some(c) = neighbours.pos_x {
            for y in 0..CS {
                for z in 0..CS {
                    self.set_padded(CS_P - 1, y + 1, z + 1, c.get(0, y, z));
                }
            }
        }
        if let Some(c) = neighbours.neg_y {
            for x in 0..CS {
                for z in 0..CS {
                    self.set_padded(x + 1, 0, z + 1, c.get(x, CS - 1, z));
                }
            }
        }
        if let Some(c) = neighbours.pos_y {
            for x in 0..CS {
                for z in 0..CS {
                    self.set_padded(x + 1, CS_P - 1, z + 1, c.get(x, 0, z));
                }
            }
        }
        if let Some(c) = neighbours.neg_z {
            for x in 0..CS {
                for y in 0..CS {
                    self.set_padded(x + 1, y + 1, 0, c.get(x, y, CS - 1));
                }
            }
        }
        if let Some(c) = neighbours.pos_z {
            for x in 0..CS {
                for y in 0..CS {
                    self.set_padded(x + 1, y + 1, CS_P - 1, c.get(x, y, 0));
                }
            }
        }
    }
}

/// The 6 face neighbours of a chunk.
#[derive(Default)]
pub struct ChunkNeighbours<'a> {
    /// Neighbour at -X (x - 1).
    pub neg_x: Option<&'a Chunk>,
    /// Neighbour at +X (x + 1).
    pub pos_x: Option<&'a Chunk>,
    /// Neighbour at -Y (y - 1).
    pub neg_y: Option<&'a Chunk>,
    /// Neighbour at +Y (y + 1).
    pub pos_y: Option<&'a Chunk>,
    /// Neighbour at -Z (z - 1).
    pub neg_z: Option<&'a Chunk>,
    /// Neighbour at +Z (z + 1).
    pub pos_z: Option<&'a Chunk>,
}
