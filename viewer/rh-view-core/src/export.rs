//! Worldgen-lite export parser and validator (SPR-033 T-03).
//!
//! Parses `<world>.toml` and the paired little-endian u16 binary rasters
//! (`<world>_elevation.bin`, `<world>_regions.bin`).

use std::fs;
use std::path::Path;

/// Expected format identifier in `<world>.toml`.
pub const EXPECTED_FORMAT: &str = "rehoboam-worldgen-lite-export-v0";

/// Scale constants defined in `terrain_v0.md`.
pub const METRES_PER_CELL: i64 = 30;
pub const ELEVATION_UNITS_PER_METRE: i64 = 64;
pub const METRES_PER_VOXEL_NUM: i64 = 7;
pub const METRES_PER_VOXEL_DEN: i64 = 20;
pub const VOXELS_PER_CELL_NUM: i64 = 600;
pub const VOXELS_PER_CELL_DEN: i64 = 7;
pub const ELEVATION_UNITS_PER_VOXEL_NUM: i64 = 2240;
pub const ELEVATION_UNITS_PER_VOXEL_DEN: i64 = 100;

/// Parsed worldgen-lite export data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldgenExport {
    pub format: String,
    pub templates_version: String,
    pub case: String,
    pub world: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    pub raster_resolution: usize,
    pub sea_level: u16,
    pub land_count: usize,
    pub seats: Vec<usize>,
    pub edges: Vec<[usize; 2]>,
    pub routes: Vec<usize>,
    pub elevation: Vec<u16>,
    pub regions: Vec<u16>,
}

/// Errors returned when parsing or validating an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Io(String),
    MissingField(String),
    InvalidFormat(String),
    InvalidScaleConstant {
        name: String,
        expected: i64,
        found: i64,
    },
    InvalidRasterLength {
        name: String,
        expected: usize,
        found: usize,
    },
    ParseIntError(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {msg}"),
            Self::MissingField(field) => write!(f, "missing field: {field}"),
            Self::InvalidFormat(fmt) => write!(f, "invalid format: {fmt}"),
            Self::InvalidScaleConstant {
                name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "invalid scale constant {name}: expected {expected}, found {found}"
                )
            }
            Self::InvalidRasterLength {
                name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "invalid raster length for {name}: expected {expected} bytes, found {found}"
                )
            }
            Self::ParseIntError(msg) => write!(f, "parse int error: {msg}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl WorldgenExport {
    /// Loads and validates a worldgen export from a directory given the world name.
    ///
    /// # Errors
    /// Returns [`ExportError`] if files cannot be read, format is wrong,
    /// scale constants disagree, or raster lengths mismatch.
    pub fn load_from_dir(dir: &Path, world_name: &str) -> Result<Self, ExportError> {
        let toml_path = dir.join(format!("{world_name}.toml"));
        let elev_path = dir.join(format!("{world_name}_elevation.bin"));
        let reg_path = dir.join(format!("{world_name}_regions.bin"));

        Self::load(&toml_path, &elev_path, &reg_path)
    }

    /// Loads and validates an export given paths to the TOML and binary raster files.
    ///
    /// # Errors
    /// Returns [`ExportError`] on any validation failure.
    pub fn load(toml_path: &Path, elev_path: &Path, reg_path: &Path) -> Result<Self, ExportError> {
        let toml_text = fs::read_to_string(toml_path)
            .map_err(|e| ExportError::Io(format!("{}: {e}", toml_path.display())))?;
        let elev_bytes = fs::read(elev_path)
            .map_err(|e| ExportError::Io(format!("{}: {e}", elev_path.display())))?;
        let reg_bytes = fs::read(reg_path)
            .map_err(|e| ExportError::Io(format!("{}: {e}", reg_path.display())))?;

        Self::parse_and_validate(&toml_text, &elev_bytes, &reg_bytes)
    }

    /// Parses export text and validates rasters.
    ///
    /// # Errors
    /// Returns [`ExportError`] on validation or parsing failure.
    #[allow(
        clippy::too_many_lines,
        reason = "straightforward sequential parsing of TOML export keys"
    )]
    pub fn parse_and_validate(
        toml_text: &str,
        elev_bytes: &[u8],
        reg_bytes: &[u8],
    ) -> Result<Self, ExportError> {
        let mut format = None;
        let mut templates_version = None;
        let mut case = None;
        let mut world = None;
        let mut manifest_path = None;
        let mut manifest_sha256 = None;
        let mut raster_resolution = None;
        let mut sea_level = None;
        let mut land_count = None;
        let mut seats = Vec::new();
        let mut edges = Vec::new();
        let mut routes = Vec::new();

        let mut in_scale = false;
        let mut scale_metres_per_cell = None;
        let mut scale_elev_per_m = None;
        let mut scale_m_per_vox_num = None;
        let mut scale_m_per_vox_den = None;
        let mut scale_vox_per_cell_num = None;
        let mut scale_vox_per_cell_den = None;
        let mut scale_elev_per_vox_num = None;
        let mut scale_elev_per_vox_den = None;

        for line in toml_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "[scale]" {
                in_scale = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_scale = false;
                continue;
            }

            let Some((key, val_str)) = trimmed.split_once('=') else {
                continue;
            };
            let k = key.trim();
            let v = val_str.trim().trim_matches('"');

            if in_scale {
                let parsed_num: i64 = v
                    .parse()
                    .map_err(|_| ExportError::ParseIntError(v.to_string()))?;
                match k {
                    "metres_per_cell" => scale_metres_per_cell = Some(parsed_num),
                    "elevation_units_per_metre" => scale_elev_per_m = Some(parsed_num),
                    "metres_per_voxel_num" => scale_m_per_vox_num = Some(parsed_num),
                    "metres_per_voxel_den" => scale_m_per_vox_den = Some(parsed_num),
                    "voxels_per_cell_num" => scale_vox_per_cell_num = Some(parsed_num),
                    "voxels_per_cell_den" => scale_vox_per_cell_den = Some(parsed_num),
                    "elevation_units_per_voxel_num" => scale_elev_per_vox_num = Some(parsed_num),
                    "elevation_units_per_voxel_den" => scale_elev_per_vox_den = Some(parsed_num),
                    _ => {}
                }
            } else {
                match k {
                    "format" => format = Some(v.to_string()),
                    "templates_version" => templates_version = Some(v.to_string()),
                    "case" => case = Some(v.to_string()),
                    "world" => world = Some(v.to_string()),
                    "manifest_path" => manifest_path = Some(v.to_string()),
                    "manifest_sha256" => manifest_sha256 = Some(v.to_string()),
                    "raster_resolution" => {
                        let res: usize = v
                            .parse()
                            .map_err(|_| ExportError::ParseIntError(v.to_string()))?;
                        raster_resolution = Some(res);
                    }
                    "sea_level" => {
                        let sl: u16 = v
                            .parse()
                            .map_err(|_| ExportError::ParseIntError(v.to_string()))?;
                        sea_level = Some(sl);
                    }
                    "land_count" => {
                        let lc: usize = v
                            .parse()
                            .map_err(|_| ExportError::ParseIntError(v.to_string()))?;
                        land_count = Some(lc);
                    }
                    "seats" => {
                        seats = parse_int_array(val_str.trim())?;
                    }
                    "edges" => {
                        edges = parse_edge_array(val_str.trim())?;
                    }
                    "routes" => {
                        routes = parse_int_array(val_str.trim())?;
                    }
                    _ => {}
                }
            }
        }

        // 1. Validate header format
        let fmt = format.ok_or_else(|| ExportError::MissingField("format".into()))?;
        if fmt != EXPECTED_FORMAT {
            return Err(ExportError::InvalidFormat(fmt));
        }

        // 2. Validate scale constants
        check_scale("metres_per_cell", scale_metres_per_cell, METRES_PER_CELL)?;
        check_scale(
            "elevation_units_per_metre",
            scale_elev_per_m,
            ELEVATION_UNITS_PER_METRE,
        )?;
        check_scale(
            "metres_per_voxel_num",
            scale_m_per_vox_num,
            METRES_PER_VOXEL_NUM,
        )?;
        check_scale(
            "metres_per_voxel_den",
            scale_m_per_vox_den,
            METRES_PER_VOXEL_DEN,
        )?;
        check_scale(
            "voxels_per_cell_num",
            scale_vox_per_cell_num,
            VOXELS_PER_CELL_NUM,
        )?;
        check_scale(
            "voxels_per_cell_den",
            scale_vox_per_cell_den,
            VOXELS_PER_CELL_DEN,
        )?;
        check_scale(
            "elevation_units_per_voxel_num",
            scale_elev_per_vox_num,
            ELEVATION_UNITS_PER_VOXEL_NUM,
        )?;
        check_scale(
            "elevation_units_per_voxel_den",
            scale_elev_per_vox_den,
            ELEVATION_UNITS_PER_VOXEL_DEN,
        )?;

        // 3. Validate raster lengths
        let r = raster_resolution
            .ok_or_else(|| ExportError::MissingField("raster_resolution".into()))?;
        let expected_bytes = r * r * 2;
        if elev_bytes.len() != expected_bytes {
            return Err(ExportError::InvalidRasterLength {
                name: "elevation".into(),
                expected: expected_bytes,
                found: elev_bytes.len(),
            });
        }
        if reg_bytes.len() != expected_bytes {
            return Err(ExportError::InvalidRasterLength {
                name: "regions".into(),
                expected: expected_bytes,
                found: reg_bytes.len(),
            });
        }

        // Decode u16 little-endian rasters
        let elevation = decode_u16_le(elev_bytes);
        let regions = decode_u16_le(reg_bytes);

        Ok(Self {
            format: fmt,
            templates_version: templates_version
                .ok_or_else(|| ExportError::MissingField("templates_version".into()))?,
            case: case.ok_or_else(|| ExportError::MissingField("case".into()))?,
            world: world.ok_or_else(|| ExportError::MissingField("world".into()))?,
            manifest_path: manifest_path
                .ok_or_else(|| ExportError::MissingField("manifest_path".into()))?,
            manifest_sha256: manifest_sha256
                .ok_or_else(|| ExportError::MissingField("manifest_sha256".into()))?,
            raster_resolution: r,
            sea_level: sea_level.ok_or_else(|| ExportError::MissingField("sea_level".into()))?,
            land_count: land_count.ok_or_else(|| ExportError::MissingField("land_count".into()))?,
            seats,
            edges,
            routes,
            elevation,
            regions,
        })
    }
}

fn check_scale(name: &str, val: Option<i64>, expected: i64) -> Result<(), ExportError> {
    let found = val.ok_or_else(|| ExportError::MissingField(name.to_string()))?;
    if found != expected {
        return Err(ExportError::InvalidScaleConstant {
            name: name.to_string(),
            expected,
            found,
        });
    }
    Ok(())
}

fn decode_u16_le(bytes: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    out
}

fn parse_int_array(s: &str) -> Result<Vec<usize>, ExportError> {
    let inner = s
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        if !p.is_empty() {
            let val = p
                .parse()
                .map_err(|_| ExportError::ParseIntError(p.to_string()))?;
            out.push(val);
        }
    }
    Ok(out)
}

fn parse_edge_array(s: &str) -> Result<Vec<[usize; 2]>, ExportError> {
    let inner = s
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for sub in inner.split("],") {
        let clean = sub
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        if clean.is_empty() {
            continue;
        }
        let parts: Vec<&str> = clean.split(',').map(str::trim).collect();
        if parts.len() == 2 {
            let a = parts[0]
                .parse()
                .map_err(|_| ExportError::ParseIntError(parts[0].to_string()))?;
            let b = parts[1]
                .parse()
                .map_err(|_| ExportError::ParseIntError(parts[1].to_string()))?;
            out.push([a, b]);
        }
    }
    Ok(out)
}
