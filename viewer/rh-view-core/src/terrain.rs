//! Terrain v0: deterministic voxelization of worldgen-lite rasters (SPR-033 T-03).
//!
//! Implements `specs/viewer/terrain_v0.md`:
//! - Bilinear interpolation of cell centres with 7-scaled integers.
//! - Elevation to height mapping ($h = \lfloor e \times 100 / 2240 \rfloor$).
//! - Materials: water, sand, rock, grass over dirt, 6-voxel solid shell.
//! - 34x34 plazas levelled at seat centres with stone paving.
//! - 4-voxel routes along integer Bresenham lines with packed earth.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::similar_names,
    reason = "grid coordinate conversions and 3D coordinate naming"
)]

use std::collections::BTreeMap;

use crate::chunk::{CS, Chunk, ChunkCoord};
use crate::digest::BubbleDef;
use crate::export::WorldgenExport;
use crate::palette;

pub const GENERATOR_VERSION: &str = "terrain_v0";

pub const METRES_PER_CELL: i64 = 30;
pub const ELEVATION_UNITS_PER_METRE: i64 = 64;
pub const METRES_PER_VOXEL_NUM: i64 = 7;
pub const METRES_PER_VOXEL_DEN: i64 = 20;
pub const VOXELS_PER_CELL_NUM: i64 = 600;
pub const VOXELS_PER_CELL_DEN: i64 = 7;
pub const ELEVATION_UNITS_PER_VOXEL_NUM: i64 = 2240;
pub const ELEVATION_UNITS_PER_VOXEL_DEN: i64 = 100;
pub const SHELL_DEPTH: usize = 6;
pub const PLAZA_SIZE: i64 = 34;
pub const PLAZA_HALF_EXTENT: i64 = 17;
pub const ROUTE_WIDTH: i64 = 4;

/// Integer bilinear interpolation of elevation raster over cell centres clamped at raster edge.
#[must_use]
pub fn interpolate_elevation(raster: &[u16], r: usize, vx: i64, vz: i64) -> i64 {
    let Ok(r_i64) = i64::try_from(r) else {
        return 0;
    };
    let vx_scaled = 7 * vx;
    let vz_scaled = 7 * vz;

    // X axis interpolation weights
    let (i0, i1, wx0, wx1) = if vx_scaled <= 300 {
        (0, 0, 600, 0)
    } else if vx_scaled >= 600 * (r_i64 - 1) + 300 {
        (r_i64 - 1, r_i64 - 1, 600, 0)
    } else {
        let x_prime = vx_scaled - 300;
        let i0 = x_prime / 600;
        let wx1 = x_prime % 600;
        (i0, i0 + 1, 600 - wx1, wx1)
    };

    // Z axis interpolation weights
    let (j0, j1, wz0, wz1) = if vz_scaled <= 300 {
        (0, 0, 600, 0)
    } else if vz_scaled >= 600 * (r_i64 - 1) + 300 {
        (r_i64 - 1, r_i64 - 1, 600, 0)
    } else {
        let z_prime = vz_scaled - 300;
        let j0 = z_prime / 600;
        let wz1 = z_prime % 600;
        (j0, j0 + 1, 600 - wz1, wz1)
    };

    let idx00 = (j0 * r_i64 + i0) as usize;
    let idx10 = (j0 * r_i64 + i1) as usize;
    let idx01 = (j1 * r_i64 + i0) as usize;
    let idx11 = (j1 * r_i64 + i1) as usize;

    let e00 = i64::from(raster[idx00]);
    let e10 = i64::from(raster[idx10]);
    let e01 = i64::from(raster[idx01]);
    let e11 = i64::from(raster[idx11]);

    let num = e00 * wx0 * wz0 + e10 * wx1 * wz0 + e01 * wx0 * wz1 + e11 * wx1 * wz1;
    let den = 360_000;
    num / den
}

/// Converts elevation units to voxel height: floor(e * 100 / 2240).
#[must_use]
pub fn height_from_elevation(e: i64) -> i64 {
    #[cfg(not(feature = "mutant-terrain-off-by-one"))]
    {
        (e * 100) / 2240
    }
    #[cfg(feature = "mutant-terrain-off-by-one")]
    {
        (e * 100) / 2240 + 1
    }
}

/// Standard 2D integer Bresenham line algorithm.
#[must_use]
pub fn bresenham_line(x0: i64, z0: i64, x1: i64, z1: i64) -> Vec<[i64; 2]> {
    let mut pts = Vec::new();
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dz = -(z1 - z0).abs();
    let sz = if z0 < z1 { 1 } else { -1 };
    let mut err = dx + dz;
    let mut x = x0;
    let mut z = z0;

    loop {
        pts.push([x, z]);
        if x == x1 && z == z1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dz {
            err += dz;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            z += sz;
        }
    }
    pts
}

/// Plaza configuration.
#[derive(Debug, Clone, Copy)]
pub struct Plaza {
    pub seat_idx: usize,
    pub svx: i64,
    pub svz: i64,
    pub h_plaza: i64,
}

impl Plaza {
    #[must_use]
    pub fn contains(&self, vx: i64, vz: i64) -> bool {
        vx >= self.svx - PLAZA_HALF_EXTENT
            && vx < self.svx + PLAZA_HALF_EXTENT
            && vz >= self.svz - PLAZA_HALF_EXTENT
            && vz < self.svz + PLAZA_HALF_EXTENT
    }
}

/// Context for evaluating terrain on a loaded export.
pub struct TerrainContext<'a> {
    pub export: &'a WorldgenExport,
    pub plazas: Vec<Plaza>,
    pub routes: BTreeMap<[i64; 2], usize>,
    pub max_voxels: i64,
}

/// Column evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    pub vx: i64,
    pub vz: i64,
    pub e: i64,
    pub h: i64,
    pub surface: u16,
    pub stack: [u16; 6],
    pub flag: String,
}

impl<'a> TerrainContext<'a> {
    #[must_use]
    pub fn new(export: &'a WorldgenExport) -> Self {
        let r = i64::try_from(export.raster_resolution).unwrap_or(0);
        let max_voxels = (600 * r) / 7;

        // 1. Calculate seats and plazas
        let mut seats = Vec::new();
        for &s_cell in &export.seats {
            let scx = i64::try_from(s_cell % export.raster_resolution).unwrap_or(0);
            let scz = i64::try_from(s_cell / export.raster_resolution).unwrap_or(0);
            let svx = (600 * scx + 300) / 7;
            let svz = (600 * scz + 300) / 7;
            seats.push([svx, svz]);
        }

        let mut plazas = Vec::new();
        for (seat_idx, &[svx, svz]) in seats.iter().enumerate() {
            let e_seat =
                interpolate_elevation(&export.elevation, export.raster_resolution, svx, svz);
            let h_plaza = height_from_elevation(e_seat);
            plazas.push(Plaza {
                seat_idx,
                svx,
                svz,
                h_plaza,
            });
        }

        // 2. Precompute routes
        let mut routes = BTreeMap::new();
        for (edge_idx, &[a, b]) in export.edges.iter().enumerate() {
            let sa = seats[a - 1];
            let sb = seats[b - 1];
            let dx = (sb[0] - sa[0]).abs();
            let dz = -(sb[1] - sa[1]).abs();
            let line = bresenham_line(sa[0], sa[1], sb[0], sb[1]);
            for [lx, lz] in line {
                if dx >= -dz {
                    for k in [-1, 0, 1, 2] {
                        routes.insert([lx, lz + k], edge_idx);
                    }
                } else {
                    for k in [-1, 0, 1, 2] {
                        routes.insert([lx + k, lz], edge_idx);
                    }
                }
            }
        }

        Self {
            export,
            plazas,
            routes,
            max_voxels,
        }
    }

    /// Evaluates properties of a single voxel column.
    #[must_use]
    pub fn eval_column(&self, vx: i64, vz: i64) -> ColumnInfo {
        let e = interpolate_elevation(
            &self.export.elevation,
            self.export.raster_resolution,
            vx,
            vz,
        );
        let mut h = height_from_elevation(e);
        let sea_level = i64::from(self.export.sea_level);
        let h_sea = height_from_elevation(sea_level);

        // 1. Cliff detection
        let mut cliff = false;
        for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
            let nb_x = (vx + dx).clamp(0, self.max_voxels - 1);
            let nb_z = (vz + dz).clamp(0, self.max_voxels - 1);
            let nb_e = interpolate_elevation(
                &self.export.elevation,
                self.export.raster_resolution,
                nb_x,
                nb_z,
            );
            let nb_h = height_from_elevation(nb_e);
            if h - nb_h > 3 {
                cliff = true;
                break;
            }
        }

        // 2. Base terrain material
        let (mut surf, mut is_cliff) = if e < sea_level || h - h_sea <= 5 {
            (palette::SAND, false)
        } else if cliff {
            (palette::ROCK, true)
        } else {
            (palette::GRASS, false)
        };

        let mut flag = "none".to_string();

        // 3. Plazas
        for plaza in &self.plazas {
            if plaza.contains(vx, vz) {
                h = plaza.h_plaza;
                surf = palette::STONE;
                is_cliff = false;
                flag = format!("plaza:{}", plaza.seat_idx);
                break;
            }
        }

        // 4. Routes
        if let Some(&edge_idx) = self.routes.get(&[vx, vz]) {
            surf = palette::PACKED_EARTH;
            is_cliff = false;
            flag = format!("route:{edge_idx}");
        }

        // Build 6-voxel solid shell [y=h down to y=h-5]
        let stack = if is_cliff {
            [
                surf,
                palette::ROCK,
                palette::ROCK,
                palette::ROCK,
                palette::ROCK,
                palette::ROCK,
            ]
        } else {
            [
                surf,
                palette::DIRT,
                palette::DIRT,
                palette::DIRT,
                palette::ROCK,
                palette::ROCK,
            ]
        };

        ColumnInfo {
            vx,
            vz,
            e,
            h,
            surface: surf,
            stack,
            flag,
        }
    }

    /// Evaluates a single L0 voxel at world coordinates (vx, vy, vz).
    #[must_use]
    pub fn eval_l0_voxel(&self, vx: i64, vy: i64, vz: i64) -> u16 {
        if vx < 0 || vz < 0 || vx >= self.max_voxels || vz >= self.max_voxels {
            return palette::AIR;
        }

        let col = self.eval_column(vx, vz);
        let col_h = col.h;
        let col_e = col.e;
        let sea_level = i64::from(self.export.sea_level);
        let h_sea = height_from_elevation(sea_level);

        if vy > h_sea && vy > col_h {
            palette::AIR
        } else if vy > col_h && vy <= h_sea && col_e < sea_level {
            palette::WATER
        } else if vy <= col_h && vy >= col_h - 5 {
            let layer_idx = (col_h - vy) as usize;
            col.stack[layer_idx]
        } else {
            palette::AIR
        }
    }

    /// Generates a single chunk on-demand at the specified LOD level.
    #[must_use]
    pub fn generate_chunk(&self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::new(coord);
        let cs_i64 = CS as i64;
        let origin_x = i64::from(coord.x) * cs_i64;
        let origin_y = i64::from(coord.y) * cs_i64;
        let origin_z = i64::from(coord.z) * cs_i64;

        if coord.level == 0 {
            let sea_level = i64::from(self.export.sea_level);
            let h_sea = height_from_elevation(sea_level);

            for px in 0..64 {
                let wx = origin_x - 1 + px;
                for pz in 0..64 {
                    let wz = origin_z - 1 + pz;
                    if wx < 0 || wz < 0 || wx >= self.max_voxels || wz >= self.max_voxels {
                        continue;
                    }
                    let col = self.eval_column(wx, wz);
                    let col_e = col.e;
                    let col_h = col.h;

                    for py in 0..64 {
                        let wy = origin_y - 1 + py;
                        let mat = if wy > h_sea && wy > col_h {
                            palette::AIR
                        } else if wy > col_h && wy <= h_sea && col_e < sea_level {
                            palette::WATER
                        } else if wy <= col_h && wy >= col_h - 5 {
                            let layer_idx = (col_h - wy) as usize;
                            col.stack[layer_idx]
                        } else {
                            palette::AIR
                        };
                        chunk.set_padded(px as usize, py as usize, pz as usize, mat);
                    }
                }
            }
        } else if coord.level == 1 {
            for px in 0..64 {
                let cx = origin_x - 1 + px;
                for pz in 0..64 {
                    let cz = origin_z - 1 + pz;
                    for py in 0..64 {
                        let cy = origin_y - 1 + py;

                        let mut children = [palette::AIR; 8];
                        for dy in 0..2 {
                            for dz in 0..2 {
                                for dx in 0..2 {
                                    let idx = dx + dz * 2 + dy * 4;
                                    children[idx] = self.eval_l0_voxel(
                                        2 * cx + dx as i64,
                                        2 * cy + dy as i64,
                                        2 * cz + dz as i64,
                                    );
                                }
                            }
                        }
                        let mat = crate::lod::reduce_voxel(&children);
                        chunk.set_padded(px as usize, py as usize, pz as usize, mat);
                    }
                }
            }
        }

        chunk
    }

    /// Generates non-empty chunks within the focus bubble for the given LOD level.
    #[must_use]
    pub fn generate_bubble_chunks(&self, bubble: &BubbleDef, level: u8) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let cs_i64 = CS as i64;

        let centre_l = [
            bubble.centre[0] >> level,
            bubble.centre[1] >> level,
            bubble.centre[2] >> level,
        ];
        let mid_cx = centre_l[0] / cs_i64;
        let mid_cz = centre_l[2] / cs_i64;

        let scale_factor: i64 = if level == 0 { 20 } else { 10 };
        let r_scaled = i64::from(bubble.radius) * scale_factor;

        let span_chunks = if level == 0 { 6 } else { 4 };

        for dcx in -span_chunks..=span_chunks {
            let cx = mid_cx + dcx;
            let x_min = cx * cs_i64;
            let x_max = cx * cs_i64 + cs_i64 - 1;
            let dx = if centre_l[0] < x_min {
                x_min - centre_l[0]
            } else if centre_l[0] > x_max {
                centre_l[0] - x_max
            } else {
                0
            };

            for dcz in -span_chunks..=span_chunks {
                let cz = mid_cz + dcz;
                let z_min = cz * cs_i64;
                let z_max = cz * cs_i64 + cs_i64 - 1;
                let dz = if centre_l[2] < z_min {
                    z_min - centre_l[2]
                } else if centre_l[2] > z_max {
                    centre_l[2] - z_max
                } else {
                    0
                };

                if (7 * dx) * (7 * dx) + (7 * dz) * (7 * dz) > r_scaled * r_scaled {
                    continue;
                }

                // Sample corners in L0 coordinates to find height range
                let l0_xmin = x_min << level;
                let l0_xmax = x_max << level;
                let l0_zmin = z_min << level;
                let l0_zmax = z_max << level;

                let h0 = self
                    .eval_column(
                        l0_xmin.clamp(0, self.max_voxels - 1),
                        l0_zmin.clamp(0, self.max_voxels - 1),
                    )
                    .h;
                let h1 = self
                    .eval_column(
                        l0_xmax.clamp(0, self.max_voxels - 1),
                        l0_zmin.clamp(0, self.max_voxels - 1),
                    )
                    .h;
                let h2 = self
                    .eval_column(
                        l0_xmin.clamp(0, self.max_voxels - 1),
                        l0_zmax.clamp(0, self.max_voxels - 1),
                    )
                    .h;
                let h3 = self
                    .eval_column(
                        l0_xmax.clamp(0, self.max_voxels - 1),
                        l0_zmax.clamp(0, self.max_voxels - 1),
                    )
                    .h;

                let min_h = (h0.min(h1).min(h2).min(h3) - 6) >> level;
                let max_h = (h0.max(h1).max(h2).max(h3) + 1) >> level;
                let min_cy = min_h.div_euclid(cs_i64);
                let max_cy = max_h.div_euclid(cs_i64);

                for cy in min_cy..=max_cy {
                    let coord = ChunkCoord::new(level, cx as i32, cy as i32, cz as i32);
                    let chunk = self.generate_chunk(coord);
                    if !chunk.is_interior_all_air() {
                        chunks.push(chunk);
                    }
                }
            }
        }

        chunks.sort_by_key(|c| c.coord);
        chunks
    }
}
