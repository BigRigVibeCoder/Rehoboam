# Terrain v0: Worldgen-Lite to 3D Voxel Terrain Specification

*Status: v0, a C1 companion specification for SPR-033 T-03 (PRJ-003, ADR-0036). Defines the deterministic conversion of worldgen-lite rasters into discrete 3D voxel terrain (`terrain_v0`). Reference implementation: `specs/viewer/terrain_ref.py`; column golden: `specs/viewer/golden/terrain_v0_columns.txt`; viewer core implementation: `viewer/rh-view-core/src/terrain.rs`; terrain golden: `viewer/golden/terrain_v0.txt`.*

---

## 0. BLUF

`terrain_v0` defines the deterministic, bit-for-bit procedural mapping from Rehoboam's 2D worldgen-lite heightmap and region rasters into a discrete 3D voxel landscape (Plane B hydration).

- **Grid resolution:** Cell size is 30.0 m. Voxel cube size is 0.35 m (35 cm).
- **Scale ratio:** Exactly $30.0 / 0.35 = 600 / 7$ voxels per cell.
- **Vertical scale:** Elevation units are $1/64\text{ m}$ (64 units per metre).
- **Height ratio:** $64 \times 0.35 = 22.4 = 2240 / 100$ elevation units per voxel.
- **Interpolation:** Integer bilinear interpolation over the four nearest cell centres, clamped at raster borders, using exact rational cell centres carried as integers scaled by 7, with a single floor division at the end.
- **Materials:** Water below sea level over a sand seabed; sand beach within 2 m above sea level; rock on steep cliffs (> 3 voxels drop to any 4-neighbour); grass over 3 voxels of dirt elsewhere; stone plazas at settlement seats; packed earth draped along inter-settlement routes.
- **Sub-surface shell:** 6 solid voxels below ground surface, empty (air) beneath.

---

## 1. Scale Constants

Scale constants originate in the worldgen-lite export metadata (`target/viewer/exports/<world>.toml`):

| Constant | Value | Description |
|---|---|---|
| `METRES_PER_CELL` | `30` | Horizontal cell size in metres |
| `ELEVATION_UNITS_PER_METRE` | `64` | Vertical elevation precision |
| `METRES_PER_VOXEL_NUM` | `7` | Voxel size numerator ($7/20 = 0.35\text{ m}$) |
| `METRES_PER_VOXEL_DEN` | `20` | Voxel size denominator |
| `VOXELS_PER_CELL_NUM` | `600` | Voxels per cell numerator ($30 / 0.35 = 600 / 7$) |
| `VOXELS_PER_CELL_DEN` | `7` | Voxels per cell denominator |
| `ELEVATION_UNITS_PER_VOXEL_NUM` | `2240` | Elevation units per voxel numerator ($64 \times 0.35 = 22.4$) |
| `ELEVATION_UNITS_PER_VOXEL_DEN` | `100` | Elevation units per voxel denominator |
| `SHELL_DEPTH` | `6` | Thickness of the solid terrain crust in voxels |
| `PLAZA_SIZE` | `34` | Plaza width and height in voxels ($34 \times 0.35\text{ m} = 11.9\text{ m}$) |
| `ROUTE_WIDTH` | `4` | Route width in voxels ($4 \times 0.35\text{ m} = 1.4\text{ m}$) |

---

## 2. Cell-to-Voxel Mapping

A world voxel column is identified by non-negative integer coordinates $(vx, vz)$, where $vx \ge 0$ is the easting and $vz \ge 0$ is the northing.

The corresponding raster cell $(cx, cz)$ is:
$$\begin{aligned}
cx &= \lfloor vx \times 7 / 600 \rfloor \\
cz &= \lfloor vz \times 7 / 600 \rfloor
\end{aligned}$$

To avoid floating-point math, all horizontal calculations are performed in **scaled coordinates** multiplied by 7:
$$\begin{aligned}
Vx &= 7 \times vx \\
Vz &= 7 \times vz
\end{aligned}$$

Cell $i$ spans the scaled coordinate range $[600i, 600(i + 1) - 1]$. The geometric centre of cell $i$ is at:
$$C(i) = 600i + 300$$

---

## 3. Integer Bilinear Interpolation

For an elevation raster of resolution $R \times R$, cells are indexed by $cx, cz \in [0, R - 1]$, with linear raster index $cz \times R + cx$.

For a voxel column with scaled coordinates $(Vx, Vz)$:

### 3.1 Clamping and Nearest Centres (X axis)
- If $Vx \le 300$: $i_0 = 0$, $i_1 = 0$, $w_{x0} = 600$, $w_{x1} = 0$.
- If $Vx \ge 600(R - 1) + 300$: $i_0 = R - 1$, $i_1 = R - 1$, $w_{x0} = 600$, $w_{x1} = 0$.
- Otherwise ($300 < Vx < 600(R - 1) + 300$):
  $$\begin{aligned}
  X' &= Vx - 300 \\
  i_0 &= \lfloor X' / 600 \rfloor \\
  i_1 &= i_0 + 1 \\
  w_{x1} &= X' \pmod{600} \\
  w_{x0} &= 600 - w_{x1}
  \end{aligned}$$

### 3.2 Clamping and Nearest Centres (Z axis)
- If $Vz \le 300$: $j_0 = 0$, $j_1 = 0$, $w_{z0} = 600$, $w_{z1} = 0$.
- If $Vz \ge 600(R - 1) + 300$: $j_0 = R - 1$, $j_1 = R - 1$, $w_{z0} = 600$, $w_{z1} = 0$.
- Otherwise ($300 < Vz < 600(R - 1) + 300$):
  $$\begin{aligned}
  Z' &= Vz - 300 \\
  j_0 &= \lfloor Z' / 600 \rfloor \\
  j_1 &= j_0 + 1 \\
  w_{z1} &= Z' \pmod{600} \\
  w_{z0} &= 600 - w_{z1}
  \end{aligned}$$

### 3.3 Four Samples and Weighted Interpolation
Read the four grid elevation samples from the raster:
$$\begin{aligned}
E_{00} &= \text{raster}[j_0 \times R + i_0] \\
E_{10} &= \text{raster}[j_0 \times R + i_1] \\
E_{01} &= \text{raster}[j_1 \times R + i_0] \\
E_{11} &= \text{raster}[j_1 \times R + i_1]
\end{aligned}$$

Compute the 64-bit weighted numerator:
$$\text{num} = E_{00} \cdot w_{x0} \cdot w_{z0} + E_{10} \cdot w_{x1} \cdot w_{z0} + E_{01} \cdot w_{x0} \cdot w_{z1} + E_{11} \cdot w_{x1} \cdot w_{z1}$$

The denominator is fixed:
$$\text{den} = 600 \times 600 = 360{,}000$$

The interpolated continuous elevation is the floor division:
$$e(vx, vz) = \lfloor \text{num} / 360{,}000 \rfloor$$

---

## 4. Heights and Sea Level

- **Surface height:**
  $$h(vx, vz) = \lfloor e(vx, vz) \times 100 / 2240 \rfloor$$
- **Sea level height:**
  $$h_{\text{sea}} = \lfloor \text{sea\_level} \times 100 / 2240 \rfloor$$

---

## 5. Material Palette and Surface Determination

Materials use the constants defined in `palette_v0`:
- `AIR = 0`
- `WATER = 1`
- `SAND = 2`
- `GRASS = 3`
- `DIRT = 4`
- `ROCK = 5`
- `PACKED_EARTH = 6`
- `STONE = 7`

### 5.1 Base Surface Material
For an unmodified column $(vx, vz)$ with elevation $e = e(vx, vz)$ and height $h = h(vx, vz)$:
1. **Below sea level ($e < \text{sea\_level}$):**
   - Water layer: voxels from $y = h + 1$ to $y = h_{\text{sea}}$ are `WATER` (1).
   - Seabed surface ($y = h$): `SAND` (2).
2. **Beach ($e \ge \text{sea\_level}$ and $h - h_{\text{sea}} \le 5$):**
   - Land within 2 m ($\le 5$ voxels) above sea level: surface ($y = h$) is `SAND` (2).
3. **Cliff / Rock:**
   - If $h - h_{\text{nb}} > 3$ for any 4-connected neighbour $h_{\text{nb}} \in \{h(vx \pm 1, vz), h(vx, vz \pm 1)\}$:
   - Surface ($y = h$) is `ROCK` (5).
4. **Meadow / Soil:**
   - Otherwise, surface ($y = h$) is `GRASS` (3).

### 5.2 The 6-Voxel Shell
Every column has a solid shell extending downwards 6 voxels from the ground surface, occupying vertical indices $y \in [h - 5, h]$. All voxels at $y < h - 5$ are empty (`AIR`).

The shell composition depends on the column's classification:
- **Cliff (`ROCK`):** all 6 voxels are `ROCK` (5).
- **Meadow (`GRASS`):**
  - $y = h$: `GRASS` (3)
  - $y \in [h - 3, h - 1]$: `DIRT` (4) (3 voxels)
  - $y \in [h - 5, h - 4]$: `ROCK` (5) (2 voxels)
- **Beach or Seabed (`SAND`):**
  - $y = h$: `SAND` (2)
  - $y \in [h - 3, h - 1]$: `DIRT` (4) (3 voxels)
  - $y \in [h - 5, h - 4]$: `ROCK` (5) (2 voxels)
- **Plaza (`STONE`):**
  - $y = h$: `STONE` (7)
  - $y \in [h - 3, h - 1]$: `DIRT` (4) (3 voxels)
  - $y \in [h - 5, h - 4]$: `ROCK` (5) (2 voxels)
- **Route (`PACKED_EARTH`):**
  - $y = h$: `PACKED_EARTH` (6)
  - $y \in [h - 3, h - 1]$: `DIRT` (4) (3 voxels)
  - $y \in [h - 5, h - 4]$: `ROCK` (5) (2 voxels)

---

## 6. Plazas

Each settlement seat cell index $s_k$ has cell coordinates $(scx, scz) = (s_k \pmod R, \lfloor s_k / R \rfloor)$.
The seat cell's centre voxel coordinate is:
$$\begin{aligned}
c_{\text{vx}} &= \lfloor (600 \times scx + 300) / 7 \rfloor \\
c_{\text{vz}} &= \lfloor (600 \times scz + 300) / 7 \rfloor
\end{aligned}$$

The plaza level is the base terrain height at this centre:
$$h_{\text{plaza}} = h(c_{\text{vx}}, c_{\text{vz}})$$

The plaza is a square of $34 \times 34$ voxels centred at $(c_{\text{vx}}, c_{\text{vz}})$, covering:
$$\begin{aligned}
vx &\in [c_{\text{vx}} - 17, c_{\text{vx}} + 16] \\
vz &\in [c_{\text{vz}} - 17, c_{\text{vz}} + 16]
\end{aligned}$$

For every column inside the plaza:
- The surface height is levelled to $h = h_{\text{plaza}}$.
- The surface material is replaced with `STONE` (7).
- The 6-voxel solid shell is positioned at $[h_{\text{plaza}} - 5, h_{\text{plaza}}]$.
- The column flag is marked `plaza:<seat_id>`.

---

## 7. Routes

Routes connect settlement seats along the edges declared in the export file.
For each edge connecting seat $a$ to seat $b$ (with $a < b$):
- Start voxel: $(x_0, z_0) = (c_{\text{vx}}(a), c_{\text{vz}}(a))$.
- End voxel: $(x_1, z_1) = (c_{\text{vx}}(b), c_{\text{vz}}(b))$.

### 7.1 Integer Bresenham Algorithm
The digital line is generated using standard integer Bresenham:
$$\begin{aligned}
\Delta x &= |x_1 - x_0|, \quad s_x = (x_0 < x_1 \ ? \ 1 : -1) \\
\Delta z &= -|z_1 - z_0|, \quad s_z = (z_0 < z_1 \ ? \ 1 : -1) \\
\text{err} &= \Delta x + \Delta z
\end{aligned}$$

At each step $(px, pz)$:
A transverse strip 4 voxels wide is generated:
- If $\Delta x \ge -\Delta z$ (horizontal-dominant):
  Columns are $(px, pz + k)$ for $k \in \{-1, 0, 1, 2\}$.
- If $\Delta x < -\Delta z$ (vertical-dominant):
  Columns are $(px + k, pz)$ for $k \in \{-1, 0, 1, 2\}$.

Advance line:
If $(px, pz) == (x_1, z_1)$, terminate.
Let $e_2 = 2 \times \text{err}$.
If $e_2 \ge \Delta z$: $\text{err} += \Delta z$; $px += s_x$.
If $e_2 \le \Delta x$: $\text{err} += \Delta x$; $pz += s_z$.

### 7.2 Route Draping
For each column in the route strip:
- Height is unchanged ($h$ is preserved).
- Surface material is replaced with `PACKED_EARTH` (6).
- Column flag is marked `route:<edge_idx>`.
- When routes overlap (e.g. at junction seats), the later edge overwrites the route flag and surface material.

---

## 8. Drawing Pipeline & Precedence

Procedural hydration evaluates columns in the following deterministic sequence:
1. **Base terrain:** Continuous elevation $e(vx, vz)$, height $h(vx, vz)$, base material, and 6-voxel shell.
2. **Plazas:** Levelled and paved with `STONE` (7) over the $34 \times 34$ seat footprints, with flag `plaza:<seat_idx>`.
3. **Routes:** Draped in ascending order of export edges ($0, 1, \dots, E - 1$), and along the Bresenham line from seat $a$ to seat $b$. Route surface material is `PACKED_EARTH` (6) with height preserved. Overlapping routes at junctions overwrite the route flag and material in edge order.
