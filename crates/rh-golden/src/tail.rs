//! `rh-cli tail <run-dir>` (SPR-013 T-10): the sealed World Stream as text.
//!
//! It reads the run directory's sealed channel-A segments in `seq` order (`rh-stream`'s reader,
//! which checks every segment), and follows new ones as they seal (`--wait`). For each record it
//! prints either the **display line** (the date in the world's calendar, the kind, the roles and
//! causes, with the cast's names from the manifest; tool side, never plane A) or, with
//! `--canonical`, the record's canonical line, after the canonical file header, so that the output
//! of one segment is a canonical file whose digest is the segment's `canonical_digest`. Channel-C
//! matches, when a sift has written channel C, are printed as annotation lines after the event that
//! completed them (`^ match …`; `#^ match …` comments in canonical mode, which no digest reads).
//!
//! The status line of `05.10` §2 ends the output (and follows each new segment under `--wait`):
//! the newest sealed sim-year on disk, its last tick and seq, the cursor (the next seq tail would
//! print) and the lag (the sealed records after the cursor). It is a `#` comment in canonical mode.
//!
//! Filters: `--from-seq N` and `--to-seq M` bound the seq range (`--to-seq` is SPR-026's, for the
//! wire's catch-up); `--follow P` keeps the records about one person (a cast name or a person id:
//! its role, its life records, an input scoped to it); `--kind K` (repeatable) keeps one record kind
//! (`DEATH` and other verbs name events). Digests and keyframe markers are about no one, so
//! `--follow` drops them.
//!
//! Tail never writes: it opens no file for writing and creates nothing in the run directory (a
//! read-only test, T-10). A stopped tail cannot slow the run: the writer never waits on a reader
//! (`05.10` §7 item 3) and tail holds no lock.
//!
//! REF: `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-10; `05.10` §2, §5, §6; V-STR-05.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rh_core::time::{Calendar, SimTime};
use rh_stream::canonical::{self, FileHeader};
use rh_stream::record::{Body, Channel, Record};
use rh_stream::seal::{channel_dir, read_segment, sealed_segments};
use rh_stream::segment::{self, Dictionaries, FORMAT_A};

use crate::run::Error;
use crate::stream::{RunToml, TIMELINE};

/// What to print.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "CLI argument flags are boolean"
)]
pub struct TailArgs {
    /// The run directory.
    pub run_dir: PathBuf,
    /// The repository (for the world's manifest: calendar and names).
    pub repo: PathBuf,
    /// The first seq to print.
    pub from_seq: u64,
    /// The last seq to print.
    pub to_seq: Option<u64>,
    /// Keep the records about this person (a name or an id).
    pub follow: Option<String>,
    /// Keep these kinds (record kinds or verbs); empty: all.
    pub kinds: Vec<String>,
    /// Print canonical lines.
    pub canonical: bool,
    /// Follow new segments until the run's last sim-year is sealed.
    pub wait: bool,
    /// Bounded wait for run.toml in seconds (default 60 s; C9).
    pub timeout: Option<u64>,
    /// Read channel C (sift matches) instead of channel A (`--channel C`; SPR-026 T-04).
    pub channel_c: bool,
    /// Keep records with this place id on `EVENT.place_id` (SPR-027 T-04).
    pub place: Option<String>,
    /// Keep matches with this pattern id on channel-C `MATCH` lines (SPR-027 T-04).
    pub pattern: Option<String>,
    /// Keep events with at least this tellability on `EVENT.tellability` (SPR-027 T-04).
    pub min_tellability: Option<u8>,
    /// Print only `STEP_DIGEST` rows (SPR-027 T-04).
    pub digest: bool,
}

/// The names and the calendar a display needs (tool side).
pub struct Display {
    calendar: Calendar,
    names: BTreeMap<u64, String>,
}

impl Display {
    fn name(&self, person: u64) -> String {
        self.names
            .get(&person)
            .cloned()
            .unwrap_or_else(|| format!("#{person:016x}"))
    }

    fn date(&self, tick: i64) -> String {
        let d = self.calendar.to_date(SimTime::from_minutes(tick));
        let sign = if d.year < 0 { "-" } else { "" };
        format!(
            "{sign}{:04}-{:02}-{:02} {:02}:{:02}",
            d.year.unsigned_abs(),
            d.month,
            d.day,
            d.hour,
            d.minute
        )
    }

    fn entity(&self, kind: u8, id: u64) -> String {
        match kind {
            1 => self.name(id),
            0 => "the world".to_owned(),
            k => format!("entity {k}:{id}"),
        }
    }
}

const DEATH_CAUSES: [&str; 6] = [
    "none",
    "natural",
    "murder",
    "battle",
    "execution",
    "accident",
];
const ROLES: [&str; 4] = ["actor", "target", "witness", "instrument"];
const ORIGINS: [&str; 3] = ["born", "founder", "immigrant"];
const SEXES: [&str; 2] = ["male", "female"];

fn label<'a>(table: &[&'a str], code: impl Into<u64>) -> &'a str {
    usize::try_from(code.into())
        .ok()
        .and_then(|i| table.get(i))
        .copied()
        .unwrap_or("?")
}

/// The display line of a record.
#[must_use]
#[allow(
    clippy::many_single_char_names,
    reason = "one short binding per record kind, as the canonical module does"
)]
pub fn display_line(r: &Record, d: &Display) -> String {
    let mut s = format!("{:>7} {} ", r.header.seq, d.date(r.header.tick));
    match &r.body {
        Body::Input(i) => {
            let on = match (i.scope.as_str(), i.scope_id) {
                ("person", Some(p)) => format!(" on {}", d.name(p)),
                (scope, Some(id)) => format!(" on {scope} {id}"),
                (_, None) => String::new(),
            };
            let _ = write!(
                s,
                "INPUT {}{on} by {} (journal #{})",
                i.input_kind, i.producer, i.journal_seq
            );
        }
        Body::Event(e) => {
            let _ = write!(s, "{}", e.verb);
            for role in &e.roles {
                let _ = write!(
                    s,
                    " {} {}",
                    label(&ROLES, role.role),
                    d.entity(role.entity_kind, role.entity_id)
                );
            }
            if e.verb == "DEATH"
                && let Some(&cause) = e.payload.first()
            {
                let _ = write!(s, " ({})", label(&DEATH_CAUSES, cause.cast_unsigned()));
            } else if !e.payload.is_empty() {
                let _ = write!(s, " {:?}", e.payload);
            }
            let _ = write!(s, " [event #{}]", e.event_id);
            for c in &e.causes {
                let _ = write!(s, " <- #{}", c.cause_id);
            }
        }
        Body::LifeEntry(e) => {
            let born = d.date(e.birth_tick);
            let _ = write!(
                s,
                "ENTERS {}: {}, {}, born {}",
                d.name(e.person_id),
                label(&ORIGINS, e.origin),
                label(&SEXES, e.sex),
                born.split(' ').next().unwrap_or_default()
            );
        }
        Body::LifeExit(x) => {
            let how = if x.exit_kind == 0 {
                format!("death ({})", label(&DEATH_CAUSES, x.death_cause))
            } else {
                "emigration".to_owned()
            };
            let _ = write!(s, "LEAVES {}: {how}", d.name(x.person_id));
            if let Some(e) = x.event_id {
                let _ = write!(s, " <- #{e}");
            }
        }
        Body::StepDigest(g) => {
            let _ = write!(
                s,
                "DIGEST living {} births {} deaths {} state {:016x}",
                g.living, g.births, g.deaths, g.state_hash
            );
        }
        Body::Keyframe(k) => {
            let _ = write!(
                s,
                "KEYFRAME {} state {:016x}",
                k.keyframe_path, k.state_hash
            );
        }
        Body::Match(m) => {
            let _ = write!(s, "MATCH {}/{}", m.pack, m.pattern_id);
        }
    }
    s
}

/// The person a record is about, for `--follow`: its target or actor, its life record, an input
/// scoped to a person.
fn about(r: &Record, person: u64) -> bool {
    match &r.body {
        Body::Input(i) => i.scope == "person" && i.scope_id == Some(person),
        Body::Event(e) => e
            .roles
            .iter()
            .any(|x| x.entity_kind == 1 && x.entity_id == person),
        Body::LifeEntry(e) => e.person_id == person,
        Body::LifeExit(x) => x.person_id == person,
        Body::StepDigest(_) | Body::Keyframe(_) | Body::Match(_) => false,
    }
}

fn kind_matches(r: &Record, kinds: &[String]) -> bool {
    kinds.is_empty()
        || kinds
            .iter()
            .any(|k| k == r.kind().name() || matches!(&r.body, Body::Event(e) if &e.verb == k))
}

/// Predicate filters applied to records (sealed segments or live stream lines).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    /// The first sequence number to include.
    pub from_seq: u64,
    /// The optional last sequence number to include.
    pub to_seq: Option<u64>,
    /// Optional person id to follow.
    pub follow: Option<u64>,
    /// Record kinds or verbs to include (empty means all).
    pub kinds: Vec<String>,
    /// Optional place id to match on `EVENT.place_id`.
    pub place: Option<u32>,
    /// Optional pattern id to match on `MATCH.pattern_id`.
    pub pattern: Option<String>,
    /// Optional minimum tellability threshold on `EVENT.tellability`.
    pub min_tellability: Option<u8>,
    /// Whether to print only `STEP_DIGEST` rows.
    pub digest: bool,
}

impl Filters {
    /// Tests a decoded record against all configured filters.
    #[must_use]
    pub fn matches_record(&self, r: &Record) -> bool {
        let seq = r.header.seq;
        if seq < self.from_seq || self.to_seq.is_some_and(|m| seq > m) {
            return false;
        }
        if self.digest && !matches!(&r.body, Body::StepDigest(_)) {
            return false;
        }
        if let Some(ref pat) = self.pattern
            && !matches!(&r.body, Body::Match(m) if &m.pattern_id == pat)
        {
            return false;
        }
        if let Some(pid) = self.place
            && !matches!(&r.body, Body::Event(e) if e.place_id == Some(pid))
        {
            return false;
        }
        if let Some(min_t) = self.min_tellability
            && !matches!(&r.body, Body::Event(e) if e.tellability >= min_t)
        {
            return false;
        }
        if !self.kinds.is_empty() && !kind_matches(r, &self.kinds) {
            return false;
        }
        if let Some(person) = self.follow
            && !about(r, person)
        {
            return false;
        }
        true
    }

    /// Fast prefix split for live canonical lines (W36: split frozen prefix before full parse).
    #[must_use]
    pub fn matches_live_line(&self, line: &str) -> bool {
        let mut tokens = line.split(' ');
        let Some(kind_str) = tokens.next() else {
            return false;
        };
        let Some(_timeline) = tokens.next() else {
            return false;
        };
        let Some(seq_str) = tokens.next() else {
            return false;
        };
        let Ok(seq) = seq_str.parse::<u64>() else {
            return false;
        };
        if seq < self.from_seq || self.to_seq.is_some_and(|m| seq > m) {
            return false;
        }
        if self.digest && kind_str != "STEP_DIGEST" {
            return false;
        }
        if let Some(ref pat) = self.pattern {
            if kind_str != "MATCH" {
                return false;
            }
            // MATCH: kind(0), timeline(1), seq(2), tick(3), match_id(4), pack(5), pattern_id(6)
            let _tick = tokens.next();
            let _match_id = tokens.next();
            let _pack = tokens.next();
            let pattern_id = tokens.next();
            if pattern_id != Some(pat.as_str()) {
                return false;
            }
        }
        if (self.place.is_some() || self.min_tellability.is_some()) && kind_str != "EVENT" {
            return false;
        }
        if kind_str == "EVENT" {
            let _tick = tokens.next();
            let _event_id = tokens.next();
            let _phase = tokens.next();
            let _verb_id = tokens.next();
            let verb = tokens.next(); // index 7
            if !self.kinds.is_empty() {
                let verb_match = verb.is_some_and(|v| self.kinds.iter().any(|k| k == v));
                let kind_match = self.kinds.iter().any(|k| k == "EVENT");
                if !verb_match && !kind_match {
                    return false;
                }
            }
            let _act_classes = tokens.next(); // index 8
            let place_tok = tokens.next(); // index 9
            if let Some(pid) = self.place {
                let pid_str = pid.to_string();
                if place_tok != Some(pid_str.as_str()) {
                    return false;
                }
            }
            let _magnitude = tokens.next(); // index 10
            let tell_tok = tokens.next(); // index 11
            if let Some(min_t) = self.min_tellability {
                let Ok(t_val) = tell_tok.unwrap_or("").parse::<u8>() else {
                    return false;
                };
                if t_val < min_t {
                    return false;
                }
            }
            if let Some(p) = self.follow {
                let _secrecy = tokens.next();
                let _origin = tokens.next();
                let _payload = tokens.next();
                let roles_tok = tokens.next(); // index 15
                let target = format!("1/{p}/");
                if !roles_tok.is_some_and(|r| r.contains(&target)) {
                    return false;
                }
            }
            return true;
        }
        if !self.kinds.is_empty() && !self.kinds.iter().any(|k| k == kind_str) {
            return false;
        }
        if let Some(p) = self.follow {
            let _tick = tokens.next();
            match kind_str {
                "INPUT" => {
                    let _journal_seq = tokens.next();
                    let _class = tokens.next();
                    let _input_kind = tokens.next();
                    let scope = tokens.next();
                    let scope_id = tokens.next();
                    let p_str = p.to_string();
                    if scope != Some("person") || scope_id != Some(p_str.as_str()) {
                        return false;
                    }
                }
                "LIFE_ENTRY" | "LIFE_EXIT" => {
                    let person_id = tokens.next();
                    let p_str = p.to_string();
                    if person_id != Some(p_str.as_str()) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

/// Resolves string place and person parameters to IDs against the world's display context.
///
/// # Errors
/// [`Error`] if a person or place name cannot be resolved.
pub fn resolve_filters(args: &TailArgs, display: Option<&Display>) -> Result<Filters, Error> {
    let follow = match &args.follow {
        None => None,
        Some(p) => {
            let id = if let Some(d) = display {
                d.names
                    .iter()
                    .find(|(_, n)| *n == p)
                    .map(|(id, _)| *id)
                    .or_else(|| p.parse().ok())
            } else {
                p.parse().ok()
            };
            Some(id.ok_or_else(|| {
                Error(format!(
                    "--follow {p}: no such person in the cast, nor an id"
                ))
            })?)
        }
    };
    let place = match &args.place {
        None => None,
        Some(p) => Some(p.parse::<u32>().map_err(|_| {
            Error(format!(
                "--place {p}: no such place in the manifest, nor an id"
            ))
        })?),
    };
    Ok(Filters {
        from_seq: args.from_seq,
        to_seq: args.to_seq,
        follow,
        kinds: args.kinds.clone(),
        place,
        pattern: args.pattern.clone(),
        min_tellability: args.min_tellability,
        digest: args.digest,
    })
}

/// The channel-C annotations of a run, by the channel-A seq they close at (none before a sift).
pub type Annotations = BTreeMap<u64, Vec<String>>;

/// No channel C yet (before T-11's sift): no annotation.
///
/// # Errors
/// Never.
pub fn no_annotations(_: &Path) -> Result<Annotations, Error> {
    Ok(Annotations::new())
}

/// Where `tail` stands.
struct Cursor {
    next: u64,
    last_seq: Option<u64>,
    header_printed: bool,
}

/// The status line: the newest sealed segment on disk (its row only), the cursor and the lag.
fn status(c: &Cursor, dir: &Path, canonical: bool) -> Result<String, Error> {
    let newest = sealed_segments(dir)
        .map_err(|e| Error(e.to_string()))?
        .into_iter()
        .next_back();
    let last = match newest {
        Some((year, path)) => {
            let bytes = std::fs::read(path.join(segment::SEGMENT_FILE))
                .map_err(|e| Error(format!("{}: {e}", path.display())))?;
            let row = segment::read_row(&bytes).map_err(|e| Error(e.to_string()))?;
            Some((year, row.last_tick, row.last_seq))
        }
        None => None,
    };
    let sealed = match last {
        Some((y, t, s)) => format!("sealed through y{y:06} (tick {t}, seq {s})"),
        None => "nothing sealed yet".to_owned(),
    };
    let lag = last.map_or(0, |(_, _, s)| (s + 1).saturating_sub(c.next));
    let line = format!("STATUS {sealed} | cursor {} | lag {lag} records", c.next);
    Ok(if canonical { format!("# {line}") } else { line })
}

/// The display context of the run's world: its calendar and the cast's names.
///
/// # Errors
/// [`Error`] if the world cannot be set up.
pub fn display_of(repo: &Path, run: &RunToml) -> Result<Display, Error> {
    let world = crate::world::setup(repo, &run.world, Some(run.seed), false)?;
    Ok(Display {
        calendar: world.calendar,
        names: world.names,
    })
}

/// Runs `tail` and writes to `out`. Returns the exit code (0, or 2 on a tool error).
///
/// # Errors
/// [`Error`] if the run directory, a segment or the world cannot be read.
#[allow(
    clippy::too_many_lines,
    reason = "the read loop in one place: segments in order, filters, annotations, status, follow"
)]
pub fn tail(
    args: &TailArgs,
    annotations: &dyn Fn(&Path) -> Result<Annotations, Error>,
    out: &mut dyn std::io::Write,
) -> Result<(), Error> {
    if args.channel_c {
        return tail_c(args, out);
    }
    // With --wait a tail may start before the run has written its run.toml (C9: bounded wait).
    if args.wait {
        let path = args.run_dir.join(crate::stream::RUN_TOML);
        if !path.is_file() {
            let timeout = args.timeout.unwrap_or(60);
            eprintln!("waiting for {} (timeout {timeout} s)...", path.display());
            let max_iters = timeout.saturating_mul(20);
            let mut iters = 0u64;
            while !path.is_file() {
                if iters >= max_iters {
                    return Err(Error(format!(
                        "timed out after {timeout} s waiting for {}",
                        path.display()
                    )));
                }
                std::thread::sleep(Duration::from_millis(50));
                iters += 1;
            }
        }
    }
    let run = RunToml::read(&args.run_dir)?;
    let display = display_of(&args.repo, &run)?;
    let filters = resolve_filters(args, Some(&display))?;
    let dir = channel_dir(&args.run_dir.join("stream"), TIMELINE, Channel::A);
    let last_year = run.years - 1;
    let mut cursor = Cursor {
        next: args.from_seq,
        last_seq: None,
        header_printed: false,
    };
    let io = |e: std::io::Error| Error(format!("tail: {e}"));
    let mut printed_through: Option<i64> = None;
    // REV-SPR-013 MIN-9: under --wait, when no new segment has come and the run's writer is gone
    // (its lock is free: the run ended or was killed), one more pass picks up a segment sealed in
    // between, and the tail ends, saying so.
    let mut writer_gone = false;
    loop {
        let notes = annotations(&args.run_dir)?;
        let seen = printed_through;
        for (year, path) in sealed_segments(&dir).map_err(|e| Error(e.to_string()))? {
            if printed_through.is_some_and(|p| year <= p) {
                continue;
            }
            let seg = segment::decode(
                &read_segment(&path).map_err(|e| Error(e.to_string()))?,
                run.minutes_per_year,
            )
            .map_err(|e| Error(format!("{}: {e}", path.display())))?;
            let expected = cursor.last_seq.map_or(0, |s| s + 1);
            if seg.row.first_seq != expected {
                return Err(Error(format!(
                    "{}: starts at seq {}, not {expected} (V-STR-05: no gap, no duplicate)",
                    path.display(),
                    seg.row.first_seq
                )));
            }
            if args.canonical && !cursor.header_printed {
                writeln!(out, "{}", header(&seg.dictionaries)).map_err(io)?;
                cursor.header_printed = true;
            }
            for r in &seg.records {
                let seq = r.header.seq;
                let in_range = seq >= args.from_seq && args.to_seq.is_none_or(|m| seq <= m);
                let keep = filters.matches_record(r);
                if keep {
                    let line = if args.canonical {
                        canonical::render(r).map_err(|e| Error(e.to_string()))?
                    } else {
                        display_line(r, &display)
                    };
                    writeln!(out, "{line}").map_err(io)?;
                    for note in notes.get(&seq).into_iter().flatten() {
                        if let Some(ref pat) = args.pattern
                            && (!note.starts_with("match ") || !note.contains(&format!("/{pat} ")))
                        {
                            continue;
                        }
                        if args.canonical {
                            writeln!(out, "#^ {note}").map_err(io)?;
                        } else {
                            writeln!(out, "{:>7} ^ {note}", "").map_err(io)?;
                        }
                    }
                }
                if in_range {
                    cursor.next = seq + 1;
                }
            }
            cursor.last_seq = Some(seg.row.last_seq);
            printed_through = Some(year);
            if args.wait {
                writeln!(out, "{}", status(&cursor, &dir, args.canonical)?).map_err(io)?;
                out.flush().map_err(io)?;
            }
            if args.to_seq.is_some_and(|m| seg.row.last_seq >= m) {
                break;
            }
        }
        let done = !args.wait
            || printed_through.is_some_and(|y| y >= last_year)
            || args
                .to_seq
                .is_some_and(|m| cursor.last_seq.is_some_and(|s| s >= m));
        if done {
            break;
        }
        if writer_gone {
            let through =
                printed_through.map_or_else(|| "no segment".to_owned(), |y| format!("y{y:06}"));
            let prefix = if args.canonical { "# " } else { "" };
            writeln!(
                out,
                "{prefix}STATUS writer gone: the run's writer stopped before its horizon (sealed \
                 through {through}, cursor {}); resume it with rh-cli run --resume",
                cursor.next
            )
            .map_err(io)?;
            break;
        }
        if printed_through == seen && crate::stream::writer_gone(&args.run_dir) {
            writer_gone = true;
            continue;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if args.canonical && !cursor.header_printed {
        writeln!(out, "{}", header_of_registry(&run)).map_err(io)?;
    }
    if !args.wait {
        writeln!(out, "{}", status(&cursor, &dir, args.canonical)?).map_err(io)?;
    }
    Ok(())
}

fn header(d: &Dictionaries) -> String {
    FileHeader {
        timeline_id: TIMELINE,
        channel: Channel::A,
        registry_version: d.registry_version.clone(),
        format: FORMAT_A.to_owned(),
    }
    .line()
}

fn header_of_registry(run: &RunToml) -> String {
    FileHeader {
        timeline_id: TIMELINE,
        channel: Channel::A,
        registry_version: run.registry_version.clone(),
        format: FORMAT_A.to_owned(),
    }
    .line()
}

/// `tail --channel C` (SPR-026 T-04): the sealed channel-C segments' records, in seq order, as
/// canonical lines after the channel's file header or as display lines, then the status line. An
/// empty sim-year (no match) holds no line.
fn tail_c(args: &TailArgs, out: &mut dyn std::io::Write) -> Result<(), Error> {
    if args.wait
        || args.follow.is_some()
        || !args.kinds.is_empty()
        || args.place.is_some()
        || args.min_tellability.is_some()
        || args.digest
    {
        return Err(Error(
            "tail --channel C takes --from-seq, --to-seq, --pattern and --canonical only"
                .to_owned(),
        ));
    }
    let run = RunToml::read(&args.run_dir)?;
    let io = |e: std::io::Error| Error(format!("tail: {e}"));
    let display = (!args.canonical)
        .then(|| display_of(&args.repo, &run))
        .transpose()?;
    let filters = resolve_filters(args, display.as_ref())?;
    if args.canonical {
        let h = FileHeader {
            timeline_id: TIMELINE,
            channel: Channel::C,
            registry_version: run.registry_version.clone(),
            format: rh_sifter::sift::FORMAT_C.to_owned(),
        };
        writeln!(out, "{}", h.line()).map_err(io)?;
    }
    let segs = rh_sifter::sift::read_channel_c(&args.run_dir, TIMELINE)
        .map_err(|e| Error(e.to_string()))?;
    let mut next = args.from_seq;
    for seg in &segs {
        for r in &seg.records {
            let seq = r.header.seq;
            if !filters.matches_record(r) {
                continue;
            }
            let line = match &display {
                None => canonical::render(r).map_err(|e| Error(e.to_string()))?,
                Some(d) => display_line(r, d),
            };
            writeln!(out, "{line}").map_err(io)?;
            next = seq + 1;
        }
    }
    let through = segs.last().map_or_else(
        || "nothing sealed yet".to_owned(),
        |s| format!("sealed through y{:06}", s.row.sim_year),
    );
    let prefix = if args.canonical { "# " } else { "" };
    writeln!(out, "{prefix}STATUS channel C {through} | cursor {next}").map_err(io)?;
    Ok(())
}

/// The canonical lines of the sealed segments of channel A (or C) with `from <= seq <= to`, in
/// seq order: a live viewer's sealed read (`wire_v1.md` §6.3; SPR-026 T-05). Channel A reads only
/// the segments whose rows overlap the range.
///
/// # Errors
/// [`Error`] if a segment does not read.
pub fn sealed_lines(
    run_dir: &Path,
    channel_c: bool,
    minutes_per_year: i64,
    from: u64,
    to: u64,
) -> Result<Vec<String>, Error> {
    let err = |e: &dyn std::fmt::Display| Error(e.to_string());
    let mut out = Vec::new();
    let mut take = |records: &[Record]| -> Result<(), Error> {
        for r in records {
            if (from..=to).contains(&r.header.seq) {
                out.push(canonical::render(r).map_err(|e| err(&e))?);
            }
        }
        Ok(())
    };
    if channel_c {
        for seg in rh_sifter::sift::read_channel_c(run_dir, TIMELINE).map_err(|e| err(&e))? {
            take(&seg.records)?;
        }
    } else {
        let dir = channel_dir(&run_dir.join("stream"), TIMELINE, Channel::A);
        for path in sealed_segments(&dir).map_err(|e| err(&e))?.values() {
            let row = rh_stream::seal::read_segment_row(path).map_err(|e| err(&e))?;
            if row.last_seq < from || row.first_seq > to {
                continue;
            }
            let seg = segment::decode(&read_segment(path).map_err(|e| err(&e))?, minutes_per_year)
                .map_err(|e| Error(format!("{}: {e}", path.display())))?;
            take(&seg.records)?;
        }
    }
    Ok(out)
}
