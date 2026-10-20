//! Worldgen-lite v0: the elevation raster, regions, seats and routes (SPR-018 T-01).
//!
//! Implements `M1.regions.odd.toml` `[algorithm.worldgen_lite]` steps W0–W10: a u16 elevation raster
//! from a heightmap template, K regions by keyed seeds and an integer Dijkstra flood, one seat per
//! region, one route per adjacency. Every quantity is an integer; every draw is keyed by
//! `H(world_seed, salt, system_id("rh-worldgen"), epoch 0, purpose, entity, t=0, k)`.
//!
//! The exact reference is `specs/worldgen/selftest_worldgen_lite.py`; the golden outputs are pinned
//! in `specs/worldgen/golden/worldgen_lite_v0.txt`.

use std::collections::{BinaryHeap, BTreeSet, HashSet};

use rh_ids::PurposeId;
use rh_ids::hash::{RngKey, h};
use rh_ids::salt::CANONICAL_SALT;

use crate::rng_purposes::RngPurpose;
use crate::WORLDGEN_ID;

// --------------------------------------------------------------------------- constants (W0)
const U16_MAX: i64 = 65535;
const PER_MILLE: i64 = 1000;
const SLOPE_SHIFT: u32 = 8;
const SMOOTH_CENTRE_WEIGHT: i64 = 4;
const SMOOTH_DIVISOR: i64 = 8;
const MIN_RADIUS_CELLS: i64 = 1;
const REGION_NONE: u16 = 0;
const MAX_SMOOTH_PASSES: u32 = 4;
const MAX_OP_COUNT: i64 = 64;

/// Per-mille position to cell coordinate: floor(v * (R - 1) / 1000).
fn pm_to_coord(v: i64, r: i64) -> i64 {
    v * (r - 1) / PER_MILLE
}

/// Per-mille length to cells: max(1, floor(v * R / 1000)).
fn pm_to_cells(v: i64, r: i64) -> i64 {
    (v * r / PER_MILLE).max(MIN_RADIUS_CELLS)
}

/// W0: pick(u, n) = floor(u * n / 2^64), an index in [0, n). n >= 1.
fn pick(u: u64, n: usize) -> usize {
    debug_assert!(n >= 1, "pick needs n >= 1");
    ((u as u128 * n as u128) >> 64) as usize
}

// --------------------------------------------------------------------------- draws (W0)
/// Make one keyed draw: H(world_seed, canonical salt, rh-worldgen, epoch 0, purpose, entity, t=0, k).
fn draw(world_seed: u64, purpose: RngPurpose, entity: u64, k: u64) -> u64 {
    let key = RngKey {
        world_seed,
        branch_salt: CANONICAL_SALT,
        system: rh_core::register::system_id(WORLDGEN_ID),
        purpose: PurposeId::of(purpose.name()),
        entity,
        epoch: 0,
        t: 0,
    };
    h(&key, k)
}

/// One keyed draw of blob b, k, mapped onto [lo, hi] inclusive.
fn ranged_draw(world_seed: u64, b: u64, k: u64, lo: i64, hi: i64) -> i64 {
    let u = draw(world_seed, RngPurpose::HeightmapBlob, b, k);
    lo + pick(u, (hi - lo + 1) as usize) as i64
}

/// Integer square root (floor).
fn integer_sqrt(n: i64) -> i64 {
    if n <= 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// The 4-neighbours of a cell (up, left, right, down).
fn neighbours(i: usize, r: i64) -> Vec<usize> {
    let x = i % r as usize;
    let y = i / r as usize;
    let r_usize = r as usize;
    let mut out = Vec::with_capacity(4);
    if y > 0 { out.push(i - r_usize); }
    if x > 0 { out.push(i - 1); }
if x < r_usize - 1 { out.push(i + 1); }
    if y < r_usize - 1 { out.push(i + r_usize); }
    out
}

// --------------------------------------------------------------------------- templates
/// One template operation: count blobs of one kind with parameter ranges.
#[derive(Clone, Debug)]
pub struct Op {
    kind: &'static str,
    count: i64,
    height: (i64, i64),
    radius: (i64, i64),
    x: (i64, i64),
    y: (i64, i64),
    width: Option<(i64, i64)>,
}

/// A heightmap template.
#[derive(Clone, Debug)]
pub struct Template {
    /// Template name.
    pub name: String,
    /// Base elevation level.
    pub base: i64,
    /// Sea level threshold.
    pub sea_level: i64,
    /// Number of smoothing passes (0..4).
    pub smooth_passes: u32,
    /// The blob operations.
    pub ops: Vec<Op>,
}

/// Parse a [lo, hi] integer pair.
fn parse_pair(raw: &toml::Value, lo: i64, hi: i64, what: &str) -> Result<(i64, i64), String> {
    let arr = raw.as_array().ok_or_else(|| format!("{what} must be an array"))?;
    if arr.len() != 2 {
        return Err(format!("{what} must be [lo, hi]"));
    }
    let a = arr[0].as_integer().ok_or_else(|| format!("{what}[0] must be an integer"))?;
    let b = arr[1].as_integer().ok_or_else(|| format!("{what}[1] must be an integer"))?;
    if a < lo || a > b || b > hi {
        return Err(format!("{what} = [{a}, {b}] must satisfy {lo} <= lo <= hi <= {hi}"));
    }
    Ok((a, b))
}

/// Parse one operation from a TOML table.
fn parse_op(raw: &toml::Table, where_: &str) -> Result<Op, String> {
    let kind_str = raw.get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{where_}: kind must be a string"))?;
    if kind_str != "dome" && kind_str != "ring" {
        return Err(format!("{where_}: kind must be dome or ring"));
    }
    let count = raw.get("count")
        .and_then(|v| v.as_integer())
        .ok_or_else(|| format!("{where_}: count must be an integer"))?;
    if count < 1 || count > MAX_OP_COUNT {
        return Err(format!("{where_}: count must be in [1, {MAX_OP_COUNT}]"));
    }
    let height = parse_pair(raw.get("height").unwrap_or(&toml::Value::Array(vec![])), -U16_MAX, U16_MAX, &format!("{where_}.height"))?;
    let radius = parse_pair(raw.get("radius").unwrap_or(&toml::Value::Array(vec![])), 0, PER_MILLE, &format!("{where_}.radius"))?;
    let x = parse_pair(raw.get("x").unwrap_or(&toml::Value::Array(vec![])), 0, PER_MILLE, &format!("{where_}.x"))?;
    let y = parse_pair(raw.get("y").unwrap_or(&toml::Value::Array(vec![])), 0, PER_MILLE, &format!("{where_}.y"))?;
    let width = if kind_str == "ring" {
        Some(parse_pair(raw.get("width").ok_or_else(|| format!("{where_}: a ring needs a width"))?, 0, PER_MILLE, &format!("{where_}.width"))?)
    } else {
        None
    };
    let kind: &'static str = if kind_str == "dome" { "dome" } else { "ring" };
    Ok(Op { kind, count, height, radius, x, y, width })
}

/// Load templates from `specs/worldgen/templates_v0.toml`.
pub fn load_templates(data: &str) -> Result<Vec<Template>, String> {
    let root: toml::Table = toml::from_str(data).map_err(|e| format!("template TOML: {e}"))?;
    let templates = root.get("templates")
        .and_then(|v| v.as_table())
        .ok_or_else(|| "missing [templates] table".to_string())?;
    let mut out = Vec::new();
    for (name, t) in templates {
        let t = t.as_table().ok_or_else(|| format!("{name}: must be a table"))?;
        for key in &["base", "sea_level", "smooth_passes"] {
            let v = t.get(*key).and_then(|v| v.as_integer())
                .ok_or_else(|| format!("{name}.{key} must be an integer"))?;
            if v < 0 || v > U16_MAX {
                return Err(format!("{name}.{key} must be in [0, {U16_MAX}]"));
            }
        }
        let smooth_passes = t["smooth_passes"].as_integer().unwrap() as u32;
        if smooth_passes > MAX_SMOOTH_PASSES {
            return Err(format!("{name}.smooth_passes must be at most {MAX_SMOOTH_PASSES}"));
        }
        let ops_arr = t.get("ops").and_then(|v| v.as_array())
            .ok_or_else(|| format!("{name}.ops must be an array"))?;
        let mut ops = Vec::new();
        for (i, op) in ops_arr.iter().enumerate() {
            let tbl = op.as_table().ok_or_else(|| format!("{name}.ops[{i}]: must be a table"))?;
            ops.push(parse_op(tbl, &format!("{name}.ops[{i}]"))?);
        }
        out.push(Template {
            name: name.clone(),
            base: t["base"].as_integer().unwrap(),
            sea_level: t["sea_level"].as_integer().unwrap(),
            smooth_passes,
            ops,
        });
    }
    Ok(out)
}

/// Find a template by name.
pub fn find_template<'a>(templates: &'a [Template], name: &str) -> Option<&'a Template> {
    templates.iter().find(|t| t.name == name)
}

// --------------------------------------------------------------------------- blobs and heightmap (W1-W4)
/// One drawn blob.
#[derive(Clone, Debug)]
struct Blob {
    kind: &'static str,
    cx: i64,
    cy: i64,
    height: i64,
    radius: i64,
    width: i64,
}

/// W1: generate all blobs for a template at a resolution, with keyed draws.
fn generate_blobs(tpl: &Template, world_seed: u64, r: i64) -> Vec<Blob> {
    let mut out = Vec::new();
    let mut b: u64 = 0;
    for op in &tpl.ops {
        for _ in 0..op.count {
            let cx = ranged_draw(world_seed, b, 0, pm_to_coord(op.x.0, r), pm_to_coord(op.x.1, r));
            let cy = ranged_draw(world_seed, b, 1, pm_to_coord(op.y.0, r), pm_to_coord(op.y.1, r));
            let h = ranged_draw(world_seed, b, 2, op.height.0, op.height.1);
            let rad = ranged_draw(
                world_seed, b, 3,
                pm_to_cells(op.radius.0, r),
                pm_to_cells(op.radius.1, r),
            );
            let w = if let Some(ref width) = op.width {
                ranged_draw(world_seed, b, 4, pm_to_cells(width.0, r), pm_to_cells(width.1, r))
            } else {
                0
            };
            out.push(Blob { kind: op.kind, cx, cy, height: h, radius: rad, width: w });
            b += 1;
        }
    }
    out
}

/// W2: one blob's contribution at squared distance d2 from its centre.
fn contribution(blob: &Blob, d2: i64) -> i64 {
    let mag = blob.height.abs();
    let sign = if blob.height < 0 { -1 } else { 1 };
    if blob.kind == "dome" {
        let r2 = blob.radius * blob.radius;
        if d2 < r2 { sign * (mag * (r2 - d2) / r2) } else { 0 }
    } else {
        let q = (integer_sqrt(d2) - blob.radius).abs();
        if q < blob.width { sign * (mag * (blob.width - q) / blob.width) } else { 0 }
    }
}

/// W2: add one blob to the accumulator over its bounding box.
fn apply_blob(acc: &mut [i64], blob: &Blob, r: i64) {
    let reach = blob.radius + if blob.kind == "ring" { blob.width } else { 0 };
    let x0 = (blob.cx - reach).max(0);
    let x1 = (blob.cx + reach).min(r - 1);
    let y0 = (blob.cy - reach).max(0);
    let y1 = (blob.cy + reach).min(r - 1);
    for y in y0..=y1 {
        let dy2 = (y - blob.cy) * (y - blob.cy);
        let row = (y * r) as usize;
        for x in x0..=x1 {
            let d2 = (x - blob.cx) * (x - blob.cx) + dy2;
            acc[row + x as usize] += contribution(blob, d2);
        }
    }
}

/// W3: clamp to [0, 65535].
fn clamp(v: i64) -> u16 {
    if v < 0 { 0 } else if v > U16_MAX { u16::MAX } else { v as u16 }
}

/// W4: one 5-point smoothing pass, floor((4*c + n + s + e + w) / 8), edges use themselves.
fn smooth(e: &[u16], r: i64) -> Vec<u16> {
    let n = (r * r) as usize;
    let mut out = vec![0u16; n];
    let r_usize = r as usize;
    for y in 0..r_usize {
        let row = y * r_usize;
        let up = if y > 0 { y - 1 } else { y } * r_usize;
        let down = if y < r_usize - 1 { y + 1 } else { y } * r_usize;
        for x in 0..r_usize {
            let left = if x > 0 { x - 1 } else { x };
            let right = if x < r_usize - 1 { x + 1 } else { x };
            let s = SMOOTH_CENTRE_WEIGHT * e[row + x] as i64
                + e[up + x] as i64
                + e[down + x] as i64
                + e[row + left] as i64
                + e[row + right] as i64;
            out[row + x] = (s / SMOOTH_DIVISOR) as u16;
        }
    }
    out
}

/// W1-W4: the u16 elevation raster of a template at a seed and resolution.
pub fn heightmap(tpl: &Template, world_seed: u64, r: i64) -> Vec<u16> {
    let n = (r * r) as usize;
    let mut acc = vec![tpl.base; n];
    for blob in generate_blobs(tpl, world_seed, r) {
        apply_blob(&mut acc, &blob, r);
    }
    let mut e: Vec<u16> = acc.into_iter().map(clamp).collect();
    for _ in 0..tpl.smooth_passes {
        e = smooth(&e, r);
    }
    e
}

// --------------------------------------------------------------------------- land (W5)
/// W5: land cells (elevation >= sea_level) in index order.
pub fn land_cells(e: &[u16], sea_level: u16) -> Vec<usize> {
    e.iter()
        .enumerate()
.filter(|&(_, v)| *v >= sea_level)
        .map(|(i, _)| i)
        .collect()
}

// --------------------------------------------------------------------------- land components (W6)
/// W6: 4-connected land components in order of their lowest cell index.
fn land_components(land: &[usize], r: i64) -> Vec<Vec<usize>> {
    let members: HashSet<usize> = land.iter().copied().collect();
    let mut seen: HashSet<usize> = HashSet::new();
    let mut out = Vec::new();
    for &start in land {
        if seen.contains(&start) { continue; }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        seen.insert(start);
        while let Some(c) = stack.pop() {
            comp.push(c);
            for nb in neighbours(c, r) {
                if members.contains(&nb) && !seen.contains(&nb) {
                    seen.insert(nb);
                    stack.push(nb);
                }
            }
        }
        comp.sort_unstable();
        out.push(comp);
    }
    out
}

// --------------------------------------------------------------------------- region seeds (W6)
/// W6: K region seeds on land cells.
pub fn region_seeds(land: &[usize], world_seed: u64, k: u16, r: i64) -> Vec<usize> {
    let k = k as usize;
    if land.len() < k {
        return Vec::new();
    }
    let u = draw(world_seed, RngPurpose::RegionSeed, 0, 0);
    let first = land[pick(u, land.len())];
    let mut seeds = vec![first];

    let mut dmin: Vec<i64> = land.iter()
        .map(|&i| {
            let dx = (i % r as usize) as i64 - (first % r as usize) as i64;
            let dy = (i / r as usize) as i64 - (first / r as usize) as i64;
            dx * dx + dy * dy
        })
        .collect();

    let add_seed = |seeds: &mut Vec<usize>, dmin: &mut [i64], cells: &[usize]| {
        let mut best_pos: Option<usize> = None;
        let mut best_dmin: i64 = -1;
        let mut best_neg_cell: i64 = 0;
        for &cell in cells {
            let pos = land.iter().position(|&l| l == cell).unwrap();
            let d = dmin[pos];
            let neg_cell = -(cell as i64);
            if d > best_dmin || (d == best_dmin && neg_cell > best_neg_cell) {
                best_dmin = d;
                best_neg_cell = neg_cell;
                best_pos = Some(pos);
            }
        }
        if let Some(pos) = best_pos {
            let best = land[pos];
            seeds.push(best);
            let bx = (best % r as usize) as i64;
            let by = (best / r as usize) as i64;
            for (j, &i) in land.iter().enumerate() {
                let dx = (i % r as usize) as i64 - bx;
                let dy = (i / r as usize) as i64 - by;
                let d2 = dx * dx + dy * dy;
                if d2 < dmin[j] { dmin[j] = d2; }
            }
        }
    };

    let components = land_components(land, r);
    for comp in &components {
        if seeds.len() >= k { break; }
        if !comp.iter().any(|&i| seeds.contains(&i)) {
            add_seed(&mut seeds, &mut dmin, comp);
        }
    }
    while seeds.len() < k {
        add_seed(&mut seeds, &mut dmin, land);
    }
    seeds
}

// --------------------------------------------------------------------------- Dijkstra flood (W7)
/// W7: multi-source integer Dijkstra over land; priority (dist, cell, region); 0 = sea/unreached.
pub fn regions(e: &[u16], land: &[usize], seeds: &[usize], r: i64) -> Vec<u16> {
    let n = (r * r) as usize;
    let mut is_land = vec![false; n];
    for &i in land { is_land[i] = true; }
    let mut region = vec![REGION_NONE; n];
    let mut heap: BinaryHeap<std::cmp::Reverse<(i64, usize, u16)>> = BinaryHeap::new();
    for (j, &s) in seeds.iter().enumerate() {
        heap.push(std::cmp::Reverse((0, s, (j + 1) as u16)));
    }
    while let Some(std::cmp::Reverse((dist, cell, reg))) = heap.pop() {
        if region[cell] != REGION_NONE { continue; }
        region[cell] = reg;
        for nb in neighbours(cell, r) {
            if is_land[nb] && region[nb] == REGION_NONE {
                let de = (e[cell] as i64 - e[nb] as i64).abs();
                let cost = 1 + (de >> SLOPE_SHIFT);
                heap.push(std::cmp::Reverse((dist + cost, nb, reg)));
            }
        }
    }
    region
}

// --------------------------------------------------------------------------- seats (W8)
/// W8: per region its land-cell count and seat, the cell nearest the floor centroid.
pub fn seats(region: &[u16], k: u16, r: i64) -> (Vec<u32>, Vec<usize>) {
    let k_usize = k as usize;
    let mut sx = vec![0u64; k_usize + 1];
    let mut sy = vec![0u64; k_usize + 1];
    let mut n = vec![0u32; k_usize + 1];
    for (i, &reg) in region.iter().enumerate() {
        if reg != REGION_NONE {
            let reg_idx = reg as usize;
            sx[reg_idx] += (i % r as usize) as u64;
            sy[reg_idx] += (i / r as usize) as u64;
            n[reg_idx] += 1;
        }
    }
    let mut best: Vec<Option<(i64, usize)>> = vec![None; k_usize + 1];
    for (i, &reg) in region.iter().enumerate() {
        if reg == REGION_NONE { continue; }
        let reg_idx = reg as usize;
        if n[reg_idx] == 0 { continue; }
        let cx = sx[reg_idx] / n[reg_idx] as u64;
        let cy = sy[reg_idx] / n[reg_idx] as u64;
        let dx = (i % r as usize) as i64 - cx as i64;
        let dy = (i / r as usize) as i64 - cy as i64;
        let key = (dx * dx + dy * dy, i);
        match best[reg_idx] {
            None => best[reg_idx] = Some(key),
            Some(prev) if key < prev => best[reg_idx] = Some(key),
            _ => {}
        }
    }
    let land_count: Vec<u32> = n[1..].to_vec();
    let seat: Vec<usize> = best[1..].iter().map(|b| b.unwrap().1).collect();
    (land_count, seat)
}

// --------------------------------------------------------------------------- edges and routes (W9)
/// W9: the region graph: (a, b) with a < b iff a land cell of a is 4-adjacent to one of b.
pub fn region_edges(region: &[u16], r: i64) -> Vec<(u16, u16)> {
    let mut found: BTreeSet<(u16, u16)> = BTreeSet::new();
    let r_usize = r as usize;
    for (i, &a) in region.iter().enumerate() {
        if a == REGION_NONE { continue; }
        if i % r_usize < r_usize - 1 {
            let b = region[i + 1];
            if b != REGION_NONE && b != a {
                found.insert((a.min(b), a.max(b)));
            }
        }
        if i / r_usize < r_usize - 1 {
            let b = region[i + r_usize];
            if b != REGION_NONE && b != a {
                found.insert((a.min(b), a.max(b)));
            }
        }
    }
    found.into_iter().collect()
}

/// Number of connected components of the region graph.
pub fn graph_components(k: u16, edge_list: &[(u16, u16)]) -> usize {
    let k = k as usize;
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); k + 1];
    for &(a, b) in edge_list {
        adj[a as usize].push(b as usize);
        adj[b as usize].push(a as usize);
    }
    let mut seen = vec![false; k + 1];
    let mut count = 0;
    for start in 1..=k {
        if seen[start] { continue; }
        count += 1;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(v) = stack.pop() {
            for &nb in &adj[v] {
                if !seen[nb] { seen[nb] = true; stack.push(nb); }
            }
        }
    }
    count
}

/// W9: one route per edge, length = isqrt of squared distance between the two seats.
pub fn routes(edge_list: &[(u16, u16)], seat: &[usize], r: i64) -> Vec<u32> {
    edge_list.iter().map(|&(a, b)| {
        let sa = seat[a as usize - 1];
        let sb = seat[b as usize - 1];
        let dx = (sa % r as usize) as i64 - (sb % r as usize) as i64;
        let dy = (sa / r as usize) as i64 - (sb / r as usize) as i64;
        integer_sqrt(dx * dx + dy * dy) as u32
    }).collect()
}

/// The outputs of worldgen-lite for one (template, seed, resolution, region count).
#[derive(Clone, Debug)]
pub struct World {
    /// Template name.
    pub template: String,
    /// World seed.
    pub seed: u64,
    /// Raster resolution (R).
    pub r: i64,
    /// Region count (K).
    pub k: u16,
    /// Elevation raster (u16, R*R cells, row-major).
    pub elevation: Vec<u16>,
    /// Land cell indices, in index order.
    pub land: Vec<usize>,
    /// Seed cell indices (K entries).
    pub seeds: Vec<usize>,
    /// Region assignment per cell (0 = sea/unreached).
    pub region: Vec<u16>,
    /// Per-region land cell counts (K entries).
    pub land_count: Vec<u32>,
    /// Per-region seat cell indices (K entries).
    pub seat: Vec<usize>,
    /// Region graph edges (a, b) with a < b.
    pub edges: Vec<(u16, u16)>,
    /// Route lengths per edge.
    pub route_length: Vec<u32>,
}

/// W1-W10: generate a world.
pub fn generate(
    templates: &[Template],
    name: &str,
    seed: u64,
    r: i64,
    k: u16,
) -> Result<World, String> {
    let tpl = find_template(templates, name)
        .ok_or_else(|| format!("unknown template: {name}"))?;
    let e = heightmap(tpl, seed, r);
    let land = land_cells(&e, tpl.sea_level as u16);
    if land.len() < k as usize {
        return Err(format!(
            "too few land cells for {k} regions: {} (template {name}, seed {seed}, R={r})",
            land.len()
        ));
    }
    let sd = region_seeds(&land, seed, k, r);
    let reg = regions(&e, &land, &sd, r);
    let (counts, seat) = seats(&reg, k, r);
    let ed = region_edges(&reg, r);
    let rl = routes(&ed, &seat, r);
    Ok(World {
        template: name.to_string(), seed, r, k, elevation: e,
        land, seeds: sd, region: reg, land_count: counts, seat,
        edges: ed, route_length: rl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATES_TOML: &str = include_str!("../../../specs/worldgen/templates_v0.toml");

    /// Minimal SHA-256 for tests (adapted from rh-golden/src/sha256.rs).
    fn sha256_u16_le(data: &[u16]) -> String {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut h: [u32; 8] = [
            0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a,
            0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19,
        ];
        let mut padded = bytes.to_vec();
        padded.push(0x80);
        while padded.len() % 64 != 56 { padded.push(0); }
        let bits = (bytes.len() as u64).wrapping_mul(8);
        padded.extend_from_slice(&bits.to_be_bytes());
        const K: [u32; 64] = [
            0x428a_2f98,0x7137_4491,0xb5c0_fbcf,0xe9b5_dba5,0x3956_c25b,0x59f1_11f1,0x923f_82a4,0xab1c_5ed5,
            0xd807_aa98,0x1283_5b01,0x2431_85be,0x550c_7dc3,0x72be_5d74,0x80de_b1fe,0x9bdc_06a7,0xc19b_f174,
            0xe49b_69c1,0xefbe_4786,0x0fc1_9dc6,0x240c_a1cc,0x2de9_2c6f,0x4a74_84aa,0x5cb0_a9dc,0x76f9_88da,
            0x983e_5152,0xa831_c66d,0xb003_27c8,0xbf59_7fc7,0xc6e0_0bf3,0xd5a7_9147,0x06ca_6351,0x1429_2967,
            0x27b7_0a85,0x2e1b_2138,0x4d2c_6dfc,0x5338_0d13,0x650a_7354,0x766a_0abb,0x81c2_c92e,0x9272_2c85,
            0xa2bf_e8a1,0xa81a_664b,0xc24b_8b70,0xc76c_51a3,0xd192_e819,0xd699_0624,0xf40e_3585,0x106a_a070,
            0x19a4_c116,0x1e37_6c08,0x2748_774c,0x34b0_bcb5,0x391c_0cb3,0x4ed8_aa4a,0x5b9c_ca4f,0x682e_6ff3,
            0x748f_82ee,0x78a5_636f,0x84c8_7814,0x8cc7_0208,0x90be_fffa,0xa450_6ceb,0xbef9_a3f7,0xc671_78f2,
        ];
        for block in padded.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (i, word) in block.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..64 {
                let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
                let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
                w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a;
                a = t1.wrapping_add(t2);
            }
            for (x, y) in h.iter_mut().zip([a,b,c,d,e,f,g,hh]) { *x = x.wrapping_add(y); }
        }
        h.iter().flat_map(|w| w.to_be_bytes()).map(|b| format!("{b:02x}")).collect()
    }

    const CASES: &[(&str, u64, i64, u16)] = &[
        ("flat", 1, 64, 1),
        ("flat", 7, 64, 1),
        ("desert_fringe", 42, 256, 4),
        ("desert_fringe", 42, 128, 4),
        ("desert_fringe", 7, 128, 4),
        ("archipelago", 7, 96, 6),
        ("mountain_valleys", 7, 128, 5),
        ("caldera_with_passes", 1, 128, 3),
        ("continental_plains", 3, 64, 8),
        ("river_delta_city", 11, 128, 4),
    ];

    #[test]
    fn templates_load() {
        let tpls = load_templates(TEMPLATES_TOML).expect("templates must load");
        assert!(!tpls.is_empty());
        assert!(find_template(&tpls, "flat").is_some());
    }

    #[test]
    fn golden_reproduction() {
        let tpls = load_templates(TEMPLATES_TOML).expect("templates must load");
        let golden = include_str!("../../../specs/worldgen/golden/worldgen_lite_v0.txt");
        let golden_lines: Vec<&str> = golden.lines().collect();

        for &(name, seed, r, k) in CASES {
            let w = generate(&tpls, name, seed, r, k)
                .unwrap_or_else(|e| panic!("generate {name}/{seed}/{r}/{k}: {e}"));
            let case_start = format!("case {name} {seed} {r} {k}");

            // Collect golden block for this case
            let mut block: Vec<&str> = Vec::new();
            let mut in_block = false;
            for line in &golden_lines {
                if line.trim() == case_start { in_block = true; block.push(line); continue; }
                if in_block {
                    if line.starts_with("case ") || line.starts_with("bench4 ")
                        || line.starts_with("templates ") || line.starts_with('#') { break; }
                    block.push(line);
                }
            }
            assert!(!block.is_empty(), "case not found in golden: {case_start}");

            let elev_sha = sha256_u16_le(&w.elevation);
            assert!(block.iter().any(|l| l.trim() == format!("elevation_sha256 {elev_sha}")),
                "{case_start}: elevation sha mismatch: {elev_sha}");

            assert!(block.iter().any(|l| l.trim() == format!("land {}", w.land.len())),
                "{case_start}: land mismatch: {}", w.land.len());

            let seeds_str = w.seeds.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
            assert!(block.iter().any(|l| l.trim() == format!("seeds {seeds_str}")),
                "{case_start}: seeds mismatch: {seeds_str}");

            let reg_sha = sha256_u16_le(&w.region);
            assert!(block.iter().any(|l| l.trim() == format!("regions_sha256 {reg_sha}")),
                "{case_start}: regions sha mismatch: {reg_sha}");

            let lc_str = w.land_count.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
            assert!(block.iter().any(|l| l.trim() == format!("land_cells {lc_str}")),
                "{case_start}: land_cells mismatch: {lc_str}");

            let seats_str = w.seat.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
            assert!(block.iter().any(|l| l.trim() == format!("seats {seats_str}")),
                "{case_start}: seats mismatch: {seats_str}");

            let edges_str = if w.edges.is_empty() {
                "edges 0 ".to_string()
            } else {
                format!("edges {} {}", w.edges.len(),
                    w.edges.iter().map(|(a,b)| format!("{a}-{b}")).collect::<Vec<_>>().join(" "))
            };
            assert!(block.iter().any(|l| l.trim() == edges_str.trim()),
                "{case_start}: edges mismatch: {edges_str}");

            let routes_str = if w.route_length.is_empty() {
                "routes ".to_string()
            } else {
                format!("routes {}",
                    w.route_length.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" "))
            };
            assert!(block.iter().any(|l| l.trim() == routes_str.trim()),
                "{case_start}: routes mismatch: {routes_str}");

            let comps = graph_components(k, &w.edges);
            let conn = if comps == 1 { "yes" } else { "no" };
            assert!(block.iter().any(|l| l.trim() == format!("graph components {comps} connected {conn}")),
                "{case_start}: graph components mismatch");
        }
    }
}