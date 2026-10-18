//! Sealing segments on disk and reading them back: the one file of `rh-stream` allowed file I/O
//! (`bin/lint_forbidden.py` `FILE_IO_FILES`; the Architect's ruling on T-09). Everything else in
//! the crate encodes to bytes and parses from bytes.
//!
//! **Layout** (R5): `<root>/t<timeline, 10 digits>/<channel>/y<sim_year, 6 digits>/<table>.parquet`,
//! with `<root>` the run directory's `stream/`.
//!
//! **Sealing is atomic** (ruling 13; T-09's `kill -9` acceptance). A segment is built in a hidden
//! sibling directory, `.y<year>.partial`: each file is written under a temporary name, fsynced and
//! renamed to its own name, `segment.parquet` last; the directory is fsynced, renamed to
//! `y<year>`, and the channel directory is fsynced. A directory named `y<year>` is therefore
//! always complete, and a crash leaves at most a `.partial` directory, which no reader lists and
//! the next writer removes. R5's rule "complete when `segment.parquet` exists" holds inside the
//! partial directory too.
//!
//! A [`SealHook`] sees every file as it lands; the host's test hooks stall there to be killed
//! mid-seal (GOV-002 tier 15). It never changes a byte.
//!
//! REF: `specs/stream/channel_a_v0.sql` R5; `specs/stream/channel_c_v0.sql` R7; SPR-011 ruling 13.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::StreamError;
use crate::record::Channel;
use crate::segment::SEGMENT_FILE;

/// Observes a seal (test hooks only).
pub trait SealHook {
    /// After file `index` (from 0) of the segment `y<year>` has been written and renamed.
    fn file_written(&mut self, _channel: Channel, _year: i64, _index: usize, _name: &str) {}
    /// After the segment `y<year>` is sealed (renamed into place).
    fn sealed(&mut self, _channel: Channel, _year: i64) {}
}

/// The hook that does nothing.
pub struct NoHook;

impl SealHook for NoHook {}

fn io(path: &Path) -> impl Fn(std::io::Error) -> StreamError + '_ {
    move |e| StreamError::Io(format!("{}: {e}", path.display()))
}

/// The directory of a timeline's channel under `root`.
#[must_use]
pub fn channel_dir(root: &Path, timeline_id: u32, channel: Channel) -> PathBuf {
    root.join(format!("t{timeline_id:010}"))
        .join(channel.letter())
}

/// The name of the segment directory of `year`.
#[must_use]
pub fn segment_name(year: i64) -> String {
    format!("y{year:06}")
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), StreamError> {
    let mut f = File::create(path).map_err(io(path))?;
    f.write_all(bytes).map_err(io(path))?;
    f.sync_all().map_err(io(path))
}

fn sync_dir(path: &Path) -> Result<(), StreamError> {
    File::open(path)
        .and_then(|d| d.sync_all())
        .map_err(io(path))
}

/// Seals one segment of `year` from its `files` (in sealing order; `segment.parquet` must be the
/// last) into the channel directory `dir` (module docs). A `.partial` left by a crash is removed
/// first; an existing sealed segment of that year is refused.
///
/// # Errors
/// [`StreamError::Io`], or [`StreamError::Segment`] for a malformed file list or a sealed segment
/// already in place.
pub fn seal(
    dir: &Path,
    channel: Channel,
    year: i64,
    files: &[(String, Vec<u8>)],
    hook: &mut dyn SealHook,
) -> Result<(), StreamError> {
    if files.last().is_none_or(|(name, _)| name != SEGMENT_FILE) {
        return Err(StreamError::Segment(
            "a segment's files end with segment.parquet (R5)".to_owned(),
        ));
    }
    if files
        .iter()
        .any(|(n, _)| n.contains(['/', '\\']) || n.starts_with('.'))
    {
        return Err(StreamError::Segment(
            "a segment file name holds a path".to_owned(),
        ));
    }
    fs::create_dir_all(dir).map_err(io(dir))?;
    let name = segment_name(year);
    let target = dir.join(&name);
    if target.exists() {
        return Err(StreamError::Segment(format!(
            "{} is already sealed",
            target.display()
        )));
    }
    let partial = dir.join(format!(".{name}.partial"));
    if partial.exists() {
        fs::remove_dir_all(&partial).map_err(io(&partial))?;
    }
    fs::create_dir(&partial).map_err(io(&partial))?;
    for (i, (file, bytes)) in files.iter().enumerate() {
        let tmp = partial.join(format!(".{file}.tmp"));
        write_synced(&tmp, bytes)?;
        let path = partial.join(file);
        fs::rename(&tmp, &path).map_err(io(&path))?;
        hook.file_written(channel, year, i, file);
    }
    sync_dir(&partial)?;
    fs::rename(&partial, &target).map_err(io(&target))?;
    sync_dir(dir)?;
    hook.sealed(channel, year);
    Ok(())
}

/// Removes every `.partial` directory a crash left in the channel directory `dir`, and returns how
/// many. A missing directory has none.
///
/// # Errors
/// [`StreamError::Io`].
pub fn remove_partials(dir: &Path) -> Result<usize, StreamError> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut n = 0;
    for entry in fs::read_dir(dir).map_err(io(dir))? {
        let path = entry.map_err(io(dir))?.path();
        let partial = path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with('.') && s.ends_with(".partial"));
        if partial {
            fs::remove_dir_all(&path).map_err(io(&path))?;
            n += 1;
        }
    }
    Ok(n)
}

/// The sealed segments of the channel directory `dir`, by sim-year: every `y<6 digits>` directory
/// that holds `segment.parquet`. A missing directory has none; hidden directories are skipped.
///
/// # Errors
/// [`StreamError::Io`], or [`StreamError::Segment`] for a `y…` directory with no
/// `segment.parquet` (sealing never leaves one).
pub fn sealed_segments(dir: &Path) -> Result<BTreeMap<i64, PathBuf>, StreamError> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir).map_err(io(dir))? {
        let path = entry.map_err(io(dir))?.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(digits) = name.strip_prefix('y') else {
            continue;
        };
        if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if !path.join(SEGMENT_FILE).is_file() {
            return Err(StreamError::Segment(format!(
                "{} has no {SEGMENT_FILE}: not a sealed segment",
                path.display()
            )));
        }
        let year: i64 = digits
            .parse()
            .map_err(|_| StreamError::Segment(format!("{name} is not a sim-year")))?;
        out.insert(year, path);
    }
    Ok(out)
}

/// Every regular file of a sealed segment directory, by name. Only a sealed segment is read: a
/// directory named `y` and six digits that holds `segment.parquet` (REV-SPR-013 MIN-3: the crate's
/// file reads stay segment reads).
///
/// # Errors
/// [`StreamError::Segment`] for a directory that is not a sealed segment; [`StreamError::Io`].
pub fn read_segment(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, StreamError> {
    let named = dir
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix('y'))
        .is_some_and(|y| y.len() == 6 && y.bytes().all(|b| b.is_ascii_digit()));
    if !named || !dir.join(SEGMENT_FILE).is_file() {
        return Err(StreamError::Segment(format!(
            "{} is not a sealed segment directory",
            dir.display()
        )));
    }
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(io(dir))? {
        let path = entry.map_err(io(dir))?.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                StreamError::Segment(format!("{}: a file name that is not UTF-8", path.display()))
            })?
            .to_owned();
        out.insert(name, fs::read(&path).map_err(io(&path))?);
    }
    Ok(out)
}
