//! Level-of-detail surface-preserving reduction (SPR-033 T-02, A7).

use crate::palette;

/// Reduces a 2x2x2 block of voxels into one coarse voxel.
///
/// Children are provided as a slice of 8 voxels ordered by:
/// `children[dx + dz * 2 + dy * 4]` where dx, dz, dy in {0, 1}.
/// - Bottom layer: dy = 0 (indices 0..4).
/// - Top layer: dy = 1 (indices 4..8).
#[must_use]
pub fn reduce_voxel(children: &[u16; 8]) -> u16 {
    #[cfg(feature = "mutant-lod-majority")]
    {
        // Mutant: plain majority across all 8 children, ties broken by lowest id.
        let mut best_id = palette::AIR;
        let mut best_count = 0;

        let mut unique: Vec<u16> = children.to_vec();
        unique.sort_unstable();
        unique.dedup();

        for id in unique {
            if id == palette::AIR {
                continue;
            }
            let count = children.iter().filter(|&&c| c == id).count();
            if count > best_count {
                best_count = count;
                best_id = id;
            }
        }
        best_id
    }

    #[cfg(not(feature = "mutant-lod-majority"))]
    {
        // 1. If all 8 children are air, coarse voxel is air.
        let non_air_count = children.iter().filter(|&&c| c != palette::AIR).count();
        if non_air_count == 0 {
            return palette::AIR;
        }

        // 2. Check top layer (dy = 1, indices 4..8).
        let top_layer = &children[4..8];
        let top_non_air: Vec<u16> = top_layer
            .iter()
            .copied()
            .filter(|&c| c != palette::AIR)
            .collect();

        let candidates = if top_non_air.is_empty() {
            let bottom_layer = &children[0..4];
            bottom_layer
                .iter()
                .copied()
                .filter(|&c| c != palette::AIR)
                .collect()
        } else {
            top_non_air
        };

        // 3. Most frequent among candidate non-air children, ties to lowest id.
        let mut best_id = candidates[0];
        let mut best_count = 0;

        let mut unique = candidates.clone();
        unique.sort_unstable();
        unique.dedup();

        for id in unique {
            let count = candidates.iter().filter(|&&c| c == id).count();
            if count > best_count {
                best_count = count;
                best_id = id;
            }
        }

        best_id
    }
}

/// Reduces 8 child chunks (2x2x2) at level L into 1 chunk at level L+1.
///
/// Children are ordered by `dx + dz * 2 + dy * 4` where dx, dz, dy in {0, 1}.
#[must_use]
pub fn reduce_chunk(
    children: [&crate::chunk::Chunk; 8],
    coord: crate::chunk::ChunkCoord,
) -> crate::chunk::Chunk {
    let mut coarse = crate::chunk::Chunk::new(coord);
    for y in 0..crate::chunk::CS {
        let sub_y = y / 31;
        let ly = (y % 31) * 2;
        for z in 0..crate::chunk::CS {
            let sub_z = z / 31;
            let lz = (z % 31) * 2;
            for x in 0..crate::chunk::CS {
                let sub_x = x / 31;
                let lx = (x % 31) * 2;
                let child_chunk_idx = sub_x + sub_z * 2 + sub_y * 4;
                let child = children[child_chunk_idx];

                let mut child_voxels = [palette::AIR; 8];
                for dy in 0..2 {
                    for dz in 0..2 {
                        for dx in 0..2 {
                            let idx = dx + dz * 2 + dy * 4;
                            child_voxels[idx] = child.get(lx + dx, ly + dy, lz + dz);
                        }
                    }
                }
                let mat = reduce_voxel(&child_voxels);
                coarse.set(x, y, z, mat);
            }
        }
    }
    coarse
}
