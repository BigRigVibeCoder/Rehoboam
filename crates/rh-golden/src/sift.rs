//! `rh-cli sift <run-dir>` (SPR-013 T-11; SPR-011 ruling 14): sifts a run's sealed channel A with
//! the world's pattern packs (`narrative.pattern_packs`, from `specs/sifting/patterns.<pack>.toml`)
//! and rewrites its channel C. It never runs inside the engine process and touches nothing but
//! `stream/t…/C/` (V-PLN-04).
//!
//! REF: `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-11; `specs/stream/channel_c_v0.sql`.

use std::path::Path;

use rh_sifter::pattern::{Pattern, compile_pack};
use rh_sifter::sift::{SiftSummary, read_channel_c, sift_run};
use rh_stream::record::Body;

use crate::run::Error;
use crate::stream::{RunToml, TIMELINE};
use crate::tail::Annotations;

/// The pack file of `pack` under `repo`.
#[must_use]
pub fn pack_path(repo: &Path, pack: &str) -> std::path::PathBuf {
    repo.join("specs/sifting")
        .join(format!("patterns.{pack}.toml"))
}

/// The compiled patterns of `packs`, in order.
///
/// # Errors
/// [`Error`] if a pack file cannot be read or does not compile for v0.
pub fn patterns(repo: &Path, packs: &[String]) -> Result<Vec<Pattern>, Error> {
    let mut out = Vec::new();
    for pack in packs {
        let path = pack_path(repo, pack);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Error(format!("{}: {e}", path.display())))?;
        out.extend(compile_pack(pack, &text).map_err(|e| Error(e.to_string()))?);
    }
    Ok(out)
}

/// Sifts the run directory with its world's packs.
///
/// # Errors
/// [`Error`] if the run, the world, a pack or the stream cannot be read, or channel C cannot be
/// written.
pub fn sift(repo: &Path, run_dir: &Path) -> Result<(Vec<String>, SiftSummary), Error> {
    let run = RunToml::read(run_dir)?;
    let world = crate::world::setup(repo, &run.world, Some(run.seed), false)?;
    let pats = patterns(repo, &world.packs)?;
    let summary = sift_run(run_dir, TIMELINE, run.minutes_per_year, &pats, &world.packs)
        .map_err(|e| Error(e.to_string()))?;
    Ok((world.packs, summary))
}

/// The channel-C matches of a run as tail annotations, by the channel-A seq that closed them,
/// with the bound persons' display names (none before a sift).
///
/// # Errors
/// [`Error`] if channel C exists and does not read.
pub fn annotations(repo: &Path, run_dir: &Path) -> Result<Annotations, Error> {
    let segments = read_channel_c(run_dir, TIMELINE).map_err(|e| Error(e.to_string()))?;
    if segments.is_empty() {
        return Ok(Annotations::new());
    }
    let run = RunToml::read(run_dir)?;
    let names = crate::world::setup(repo, &run.world, Some(run.seed), false)?.names;
    let mut out = Annotations::new();
    for s in segments {
        for r in s.records {
            let Body::Match(m) = r.body else { continue };
            let bound: Vec<String> = m
                .bindings
                .iter()
                .map(|b| {
                    let who = if b.entity_kind == 1 {
                        names
                            .get(&b.entity_id)
                            .cloned()
                            .unwrap_or_else(|| format!("#{:016x}", b.entity_id))
                    } else {
                        format!("{}:{}", b.entity_kind, b.entity_id)
                    };
                    format!("{}={who}", b.role)
                })
                .collect();
            out.entry(m.closed_by_seq).or_default().push(format!(
                "match {}/{} {} (match {:016x})",
                m.pack,
                m.pattern_id,
                bound.join(" "),
                m.match_id
            ));
        }
    }
    Ok(out)
}
