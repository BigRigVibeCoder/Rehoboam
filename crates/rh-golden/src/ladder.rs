//! The hash ladder: a run's plane-A hashes at every rung, in a portable text file (SPR-008
//! ruling 4).
//!
//! # Format `rehoboam-ladder v1` (normative)
//!
//! UTF-8, `\n` line ends, no trailing spaces, ending with a newline:
//!
//! ```text
//! rehoboam-ladder v1
//! world smoke
//! seed 1
//! years 1000
//! rung yearly
//! hash rehoboam/rh-snapshot/hash/v1
//! rung 1 525599 0000-12 4f1c...e2 77a0...31   <- index, tick, date, state_hash, months
//! root person.alive 9a0b...11          <- every table's Merkle root, in table-name byte order
//! root rh-core/agenda.meta 03d1...7c
//! ...
//! rung 2 1051199 0001-12 ...
//! ```
//!
//! - **Header.** The world, the seed, the horizon in years, the rung kind (`yearly` or `monthly`)
//!   and the hash layout (`rh_snapshot::hash::LAYOUT`). Nothing else: no build id, no wall time,
//!   no thread count, no host name. So ladders written on two machines, by two builds or at two
//!   thread counts compare byte for byte.
//! - **Journal.** `journal <inputs> <head>` follows `hash`: the input journal's row count and
//!   its head chain hash (32 hex digits), the identity of the inputs the history was run from.
//!   Two runs of one horizon from two journals cannot share a header (VER-SPR-008 DEF-1).
//! - **Until.** A run stopped before its horizon (`--until`, the bisect's resumed runs) adds
//!   `until <year>` after the journal line.
//! - **Injection.** A run perturbed with the V-DET-11 test hook (`--inject`) adds the line
//!   `inject <month>:<table>:<row>` after `hash`, so an injected ladder can never pass as a
//!   clean one: it differs from every baseline byte for byte, and [`compare`] reports it
//!   (VER-SPR-008 MIN-2). A clean run has no such line.
//! - **Leak.** A run with the planted thread-identity leak (`--leak-thread`, the DEF-2 control)
//!   adds `leak thread` after the other header lines, for the same reason.
//! - **Rungs.** One per year end (`yearly`) or month end (`monthly`), from the first. A rung is
//!   its 1-based index, the tick of the instant it closes (the last minute of the year or the
//!   month), the date of that instant as `YYYY-MM` (the year counts from 0), and the plane-A
//!   `state_hash` (`rh_snapshot::state::state_hash`: the world tables and the engine-held tables,
//!   the undrained events excepted), then `months`: XXH3-64 (`rh_snapshot::hash::xxh3`) of the
//!   little-endian `u64` state hashes at the end of every month the rung covers, in order (one
//!   month for a monthly rung, twelve for a yearly one). So a yearly rung also differs when two
//!   runs diverged during the year and converged again by its end: a one-row change that an
//!   integer division absorbs a month later is still caught, and the bisect then finds its month.
//!   Hashes are 16 lowercase hex digits.
//! - **Roots.** Under each rung, every plane-A table's Merkle root
//!   (`rh_snapshot::hash::TableDigest::root`), in strictly increasing byte order of the names,
//!   the order the plane hash takes them in.
//!
//! **Bytes.** Lines end in `\n` only, and the file ends with one: [`Ladder::parse`] refuses a
//! carriage return anywhere and a missing final newline, so two ladders that parse equal are equal
//! byte for byte (ruling 4; VER-SPR-008-TEST GAP-4).
//!
//! **Cut.** [`Ladder::cut_yearly`] makes the yearly ladder of a monthly one: each year's last
//! monthly rung, with `months` recomputed over the year's twelve month hashes. The nightly tier
//! compares the cut of its monthly run with the committed yearly baseline (GAP-6).
//!
//! A run that fails stops writing rungs; the ladder then ends with a `failed <tick> <message>`
//! line, so a crash is a difference too.
//!
//! **Version line.** The format's version is the `v1` of the first line. A change to the text is
//! a new version, with new baselines (a C3 change).
//!
//! REF: `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md` rulings 4 and 8; `05.1` §6 item 6.

use std::fmt;
use std::fmt::Write as _;

/// The first line of every ladder file.
pub const MAGIC: &str = "rehoboam-ladder v1";

/// How often a ladder takes a rung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RungKind {
    /// At the end of every sim-year.
    Yearly,
    /// At the end of every sim-month.
    Monthly,
}

impl RungKind {
    /// The name the header and the command line use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Yearly => "yearly",
            Self::Monthly => "monthly",
        }
    }

    /// The kind of a name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "yearly" => Some(Self::Yearly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }
}

/// A ladder's header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// The world.
    pub world: String,
    /// The world seed.
    pub seed: u64,
    /// The horizon, in sim-years.
    pub years: i64,
    /// The rung kind.
    pub rung: RungKind,
    /// The hash layout.
    pub hash: String,
    /// The input journal's row count and head chain hash (hex), DEF-1.
    pub journal: Option<(u32, String)>,
    /// The year the run stopped at, when before its horizon.
    pub until: Option<i64>,
    /// The V-DET-11 perturbation `month:table:row`, if the run was perturbed.
    pub inject: Option<String>,
    /// Whether the run carried the planted thread-identity leak (DEF-2 control).
    pub leak: bool,
    /// A manifest world's history tuple, `tuple engine_build=… registry_version=… manifest_hash=…
    /// seed=…` without the `tuple ` prefix (SPR-012 T-08; DET-1). `None` for smoke.
    pub tuple: Option<String>,
    /// A manifest world's built and not-built systems, `systems built … not_built …` without the
    /// `systems ` prefix (SPR-011 ruling 3). `None` for smoke.
    pub systems: Option<String>,
}

/// One rung: the state at the end of a year or a month.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rung {
    /// Its 1-based index.
    pub index: u64,
    /// The tick of the instant it closes.
    pub tick: i64,
    /// That instant's date, `YYYY-MM`.
    pub date: String,
    /// The plane-A state hash.
    pub hash: u64,
    /// XXH3-64 of the month-end state hashes the rung covers (module docs).
    pub months: u64,
    /// Every table's root, in name byte order.
    pub roots: Vec<(String, u64)>,
}

/// A ladder: a header, its rungs, and the failure that ended it early, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ladder {
    /// The header.
    pub header: Header,
    /// The rungs, in order.
    pub rungs: Vec<Rung>,
    /// `(tick, message)` of the failure that stopped the run, if it failed.
    pub failed: Option<(i64, String)>,
}

/// A ladder file that does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The 1-based line.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ladder line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

impl Ladder {
    /// The ladder as its file text (the format above).
    #[must_use]
    pub fn render(&self) -> String {
        let h = &self.header;
        let mut out = format!(
            "{MAGIC}\nworld {}\nseed {}\nyears {}\nrung {}\nhash {}\n",
            h.world,
            h.seed,
            h.years,
            h.rung.name(),
            h.hash
        );
        if let Some((inputs, head)) = &h.journal {
            let _ = writeln!(out, "journal {inputs} {head}");
        }
        if let Some(until) = h.until {
            let _ = writeln!(out, "until {until}");
        }
        if let Some(inject) = &h.inject {
            let _ = writeln!(out, "inject {inject}");
        }
        if h.leak {
            out.push_str("leak thread\n");
        }
        if let Some(tuple) = &h.tuple {
            let _ = writeln!(out, "tuple {tuple}");
        }
        if let Some(systems) = &h.systems {
            let _ = writeln!(out, "systems {systems}");
        }
        for r in &self.rungs {
            out.push_str(&render_rung(r));
        }
        if let Some((tick, message)) = &self.failed {
            let one_line = message.replace(['\n', '\r'], " ");
            let _ = writeln!(out, "failed {tick} {}", one_line.trim_end());
        }
        out
    }

    /// Parses a ladder file.
    ///
    /// # Errors
    /// [`ParseError`] at the first line that does not follow the format.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        check_bytes(text)?;
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
        let mut next = |what: &str| {
            lines.next().ok_or_else(|| ParseError {
                line: 0,
                message: format!("the file ends before {what}"),
            })
        };
        let (n, magic) = next("the magic line")?;
        if magic != MAGIC {
            return Err(err(n, format!("expected `{MAGIC}`")));
        }
        let world = field(next("world")?, "world")?.to_owned();
        let seed = parse_num(next("seed")?, "seed")?;
        let years = parse_num(next("years")?, "years")?;
        let rung_line = next("rung")?;
        let rung = RungKind::from_name(field(rung_line, "rung")?)
            .ok_or_else(|| err(rung_line.0, "rung is yearly or monthly".to_owned()))?;
        let hash = field(next("hash")?, "hash")?.to_owned();
        let mut body = 6;
        let mut optional = |prefix: &str| {
            let value = text
                .lines()
                .nth(body)
                .and_then(|l| l.strip_prefix(prefix))
                .map(|v| (body + 1, v.to_owned()));
            body += usize::from(value.is_some());
            value
        };
        let journal_line = optional("journal ");
        let until_line = optional("until ");
        let inject_line = optional("inject ");
        let leak = optional("leak thread").is_some_and(|(_, rest)| rest.is_empty());
        let tuple = optional("tuple ").map(|(_, v)| v);
        let systems = optional("systems ").map(|(_, v)| v);
        let journal = match journal_line {
            None => None,
            Some((n, j)) => {
                let (inputs, head) = j
                    .split_once(' ')
                    .filter(|(_, h)| h.len() == 32 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| {
                        err(n, "expected `journal <inputs> <32 hex digits>`".to_owned())
                    })?;
                Some((num(n, inputs)?, head.to_owned()))
            }
        };
        let until = until_line.map(|(n, u)| num(n, &u)).transpose()?;
        let header = Header {
            world,
            seed,
            years,
            rung,
            hash,
            journal,
            until,
            inject: inject_line.map(|(_, v)| v),
            leak,
            tuple,
            systems,
        };
        let mut rungs: Vec<Rung> = Vec::new();
        let mut failed = None;
        for (n, line) in text.lines().enumerate().skip(body).map(|(i, l)| (i + 1, l)) {
            let words: Vec<&str> = line.split(' ').collect();
            match words.as_slice() {
                ["rung", index, tick, date, hash, months] if failed.is_none() => {
                    rungs.push(Rung {
                        index: num(n, index)?,
                        tick: num(n, tick)?,
                        date: (*date).to_owned(),
                        hash: hex(n, hash)?,
                        months: hex(n, months)?,
                        roots: Vec::new(),
                    });
                }
                ["root", table, root] if failed.is_none() => {
                    let rung = rungs
                        .last_mut()
                        .ok_or_else(|| err(n, "a root before any rung".to_owned()))?;
                    rung.roots.push(((*table).to_owned(), hex(n, root)?));
                }
                ["failed", tick, ..] if failed.is_none() => {
                    let message = line.splitn(3, ' ').nth(2).unwrap_or_default().to_owned();
                    failed = Some((num(n, tick)?, message));
                }
                _ => return Err(err(n, format!("unexpected line `{line}`"))),
            }
        }
        Ok(Self {
            header,
            rungs,
            failed,
        })
    }
}

/// One rung's lines (`rung` and its `root`s), as [`Ladder::render`] writes them.
#[must_use]
pub fn render_rung(r: &Rung) -> String {
    let mut out = format!(
        "rung {} {} {} {:016x} {:016x}\n",
        r.index, r.tick, r.date, r.hash, r.months
    );
    for (table, root) in &r.roots {
        let _ = writeln!(out, "root {table} {root:016x}");
    }
    out
}

fn err(line: usize, message: String) -> ParseError {
    ParseError { line, message }
}

fn field<'a>((n, line): (usize, &'a str), key: &str) -> Result<&'a str, ParseError> {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(' '))
        .filter(|v| !v.is_empty() && !v.contains(' '))
        .ok_or_else(|| err(n, format!("expected `{key} <value>`")))
}

fn parse_num<T: std::str::FromStr>(line: (usize, &str), key: &str) -> Result<T, ParseError> {
    num(line.0, field(line, key)?)
}

fn num<T: std::str::FromStr>(line: usize, s: &str) -> Result<T, ParseError> {
    s.parse()
        .map_err(|_| err(line, format!("`{s}` is not a number")))
}

fn hex(line: usize, s: &str) -> Result<u64, ParseError> {
    if s.len() != 16
        || !s
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(err(line, format!("`{s}` is not 16 lowercase hex digits")));
    }
    u64::from_str_radix(s, 16).map_err(|_| err(line, format!("`{s}` is not hex")))
}

/// Where two ladders first differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Difference {
    /// The headers differ (another world, seed, horizon, rung kind or hash layout): the ladders
    /// are not comparable.
    Header {
        /// The first differing header field.
        field: &'static str,
    },
    /// The first rung that differs, by its index, with what differs.
    Rung {
        /// The rung's own index (1-based year or month), as the ladders record it.
        index: u64,
        /// Its tick and date on the first ladder, if it has the rung.
        a: Option<(i64, String)>,
        /// Its tick and date on the second ladder, if it has the rung.
        b: Option<(i64, String)>,
        /// The tables whose roots differ (or that only one side has), in name order. Empty when
        /// one side lacks the rung, or when only the month hashes differ (the runs diverged and
        /// converged again within the rung).
        tables: Vec<String>,
    },
    /// Every rung agrees, but one ladder records a failure the other does not (or another one).
    Failure,
    /// Every rung agrees, but one ladder was perturbed by the V-DET-11 hook and the other was
    /// not (or differently): an injected ladder never passes as a clean one (MIN-2).
    Inject {
        /// The first ladder's perturbation.
        a: Option<String>,
        /// The second ladder's.
        b: Option<String>,
    },
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header { field } => write!(f, "the headers differ in `{field}`"),
            Self::Rung {
                index,
                a,
                b,
                tables,
            } => {
                let at = |side: &Option<(i64, String)>| match side {
                    Some((tick, date)) => format!("{date} (tick {tick})"),
                    None => "absent".to_owned(),
                };
                write!(f, "first differing rung: {index}, at {} / {}", at(a), at(b))?;
                if !tables.is_empty() {
                    write!(f, "; tables: {}", tables.join(", "))?;
                }
                Ok(())
            }
            Self::Failure => f.write_str("every rung agrees, but the runs ended differently"),
            Self::Inject { a, b } => write!(
                f,
                "every rung agrees, but the perturbations differ: {} / {}",
                a.as_deref().unwrap_or("none"),
                b.as_deref().unwrap_or("none")
            ),
        }
    }
}

impl Ladder {
    /// The yearly ladder of a monthly one: for every year, its December rung (the year's last
    /// month), re-indexed by year, with `months` = XXH3-64 of the year's month hashes in order. A
    /// year with fewer than twelve monthly rungs (a run that failed or stopped in it) is left out;
    /// the failure line is kept.
    ///
    /// # Errors
    /// A message when the ladder is not monthly, or a rung's date is not `YYYY-MM`.
    pub fn cut_yearly(&self) -> Result<Self, String> {
        if self.header.rung != RungKind::Monthly {
            return Err("cut-yearly needs a monthly ladder".to_owned());
        }
        let mut rungs = Vec::new();
        let mut year: Vec<&Rung> = Vec::new();
        for r in &self.rungs {
            let (y, m) = r
                .date
                .split_once('-')
                .and_then(|(y, m)| Some((y.parse::<u64>().ok()?, m.parse::<u32>().ok()?)))
                .ok_or_else(|| format!("rung {}: date `{}` is not YYYY-MM", r.index, r.date))?;
            year.push(r);
            if m == 12 {
                if year.len() == 12 {
                    let hashes: Vec<u8> = year.iter().flat_map(|r| r.hash.to_le_bytes()).collect();
                    rungs.push(Rung {
                        index: y + 1,
                        months: rh_snapshot::hash::xxh3(&hashes),
                        ..r.clone()
                    });
                }
                year.clear();
            }
        }
        let mut header = self.header.clone();
        header.rung = RungKind::Yearly;
        Ok(Self {
            header,
            rungs,
            failed: self.failed.clone(),
        })
    }
}

/// GAP-4: lines end in `\n` only, and a non-empty file ends with one.
fn check_bytes(text: &str) -> Result<(), ParseError> {
    let line_of = |i: usize| text[..i].matches('\n').count() + 1;
    if let Some(i) = text.find('\r') {
        return Err(err(
            line_of(i),
            "a carriage return: ladder lines end in `\\n` only (GAP-4)".to_owned(),
        ));
    }
    if !text.is_empty() && !text.ends_with('\n') {
        return Err(err(
            line_of(text.len()),
            "the file does not end in a newline (GAP-4)".to_owned(),
        ));
    }
    Ok(())
}

/// The first place `a` and `b` differ, or `None` if they are the same ladder.
#[must_use]
pub fn compare(a: &Ladder, b: &Ladder) -> Option<Difference> {
    let (ha, hb) = (&a.header, &b.header);
    let fields: [(&'static str, bool); 8] = [
        ("world", ha.world == hb.world),
        ("seed", ha.seed == hb.seed),
        ("years", ha.years == hb.years),
        ("rung", ha.rung == hb.rung),
        ("hash", ha.hash == hb.hash),
        ("journal", ha.journal == hb.journal),
        ("tuple", ha.tuple == hb.tuple),
        ("systems", ha.systems == hb.systems),
    ];
    if let Some((field, _)) = fields.iter().find(|(_, same)| !same) {
        return Some(Difference::Header { field });
    }
    let longest = a.rungs.len().max(b.rungs.len());
    for i in 0..longest {
        let (ra, rb) = (a.rungs.get(i), b.rungs.get(i));
        if ra == rb {
            continue;
        }
        let tables = match (ra, rb) {
            (Some(x), Some(y)) => differing_tables(&x.roots, &y.roots),
            _ => Vec::new(),
        };
        // The rung's own index: a resumed ladder starts after rung 1.
        let index = ra
            .or(rb)
            .map_or_else(|| u64::try_from(i).unwrap_or(u64::MAX) + 1, |r| r.index);
        return Some(Difference::Rung {
            index,
            a: ra.map(|r| (r.tick, r.date.clone())),
            b: rb.map(|r| (r.tick, r.date.clone())),
            tables,
        });
    }
    if a.failed != b.failed {
        return Some(Difference::Failure);
    }
    if ha.leak != hb.leak {
        return Some(Difference::Header { field: "leak" });
    }
    (ha.inject != hb.inject).then(|| Difference::Inject {
        a: ha.inject.clone(),
        b: hb.inject.clone(),
    })
}

/// The tables whose roots differ, or that only one side has, in name order.
#[must_use]
pub fn differing_tables(a: &[(String, u64)], b: &[(String, u64)]) -> Vec<String> {
    let mut names: Vec<&String> = a.iter().chain(b).map(|(n, _)| n).collect();
    names.sort();
    names.dedup();
    let root = |side: &[(String, u64)], name: &String| {
        side.iter().find(|(n, _)| n == name).map(|(_, r)| *r)
    };
    names
        .into_iter()
        .filter(|n| root(a, n) != root(b, n))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder() -> Ladder {
        Ladder {
            header: Header {
                world: "smoke".to_owned(),
                seed: 1,
                years: 2,
                rung: RungKind::Yearly,
                hash: "rehoboam/rh-snapshot/hash/v1".to_owned(),
                journal: Some((8, "0123456789abcdef0123456789abcdef".to_owned())),
                until: None,
                inject: None,
                leak: false,
                tuple: None,
                systems: None,
            },
            rungs: (1..=2)
                .map(|i| Rung {
                    index: i,
                    tick: i64::try_from(i).unwrap_or(0) * 100 - 1,
                    date: format!("{:04}-12", i - 1),
                    hash: 0xabc0 + i,
                    months: 0xd0 + i,
                    roots: vec![("a.x".to_owned(), i), ("b.y".to_owned(), 7)],
                })
                .collect(),
            failed: None,
        }
    }

    #[test]
    fn a_ladder_round_trips_through_its_text() {
        let l = ladder();
        let text = l.render();
        assert!(text.starts_with("rehoboam-ladder v1\nworld smoke\nseed 1\n"));
        assert!(text.contains(
            "rung 1 99 0000-12 000000000000abc1 00000000000000d1\nroot a.x 0000000000000001\n"
        ));
        assert_eq!(Ladder::parse(&text), Ok(l));
        let mut failed = ladder();
        failed.failed = Some((150, "invariant INV-X1-01 violated".to_owned()));
        assert_eq!(Ladder::parse(&failed.render()), Ok(failed));
        let mut injected = ladder();
        injected.header.inject = Some("12:harvest.stock:2".to_owned());
        let text = injected.render();
        assert!(text.contains(
            "hash rehoboam/rh-snapshot/hash/v1\njournal 8 0123456789abcdef0123456789abcdef\n\
                 inject 12:harvest.stock:2\nrung 1 "
        ));
        assert_eq!(Ladder::parse(&text), Ok(injected));
    }

    #[test]
    fn the_journal_and_until_lines_round_trip_and_the_journal_is_compared() {
        let mut stopped = ladder();
        stopped.header.until = Some(1);
        stopped.rungs.pop();
        assert_eq!(Ladder::parse(&stopped.render()), Ok(stopped));
        let mut other = ladder();
        other.header.journal = Some((9, "0123456789abcdef0123456789abcdef".to_owned()));
        assert_eq!(
            compare(&ladder(), &other),
            Some(Difference::Header { field: "journal" })
        );
    }

    #[test]
    fn an_injected_ladder_never_compares_equal_to_a_clean_one() {
        let clean = ladder();
        let mut injected = ladder();
        injected.header.inject = Some("12:harvest.stock:2".to_owned());
        assert!(matches!(
            compare(&clean, &injected),
            Some(Difference::Inject {
                a: None,
                b: Some(_)
            })
        ));
        assert_ne!(clean.render(), injected.render());
    }

    #[test]
    fn crlf_and_a_missing_final_newline_are_refused() {
        let text = ladder().render();
        assert!(Ladder::parse(&text).is_ok());
        let crlf = text.replace('\n', "\r\n");
        let e = Ladder::parse(&crlf).unwrap_err();
        assert_eq!(e.line, 1, "{e}");
        assert!(e.message.contains("carriage return"), "{e}");
        let unterminated = text.trim_end_matches('\n');
        let e = Ladder::parse(unterminated).unwrap_err();
        assert!(e.message.contains("does not end in a newline"), "{e}");
        assert_eq!(e.line, unterminated.lines().count(), "{e}");
    }

    #[test]
    fn a_leaked_ladder_says_so_and_never_compares_equal_to_a_clean_one() {
        let clean = ladder();
        let mut leaked = ladder();
        leaked.header.leak = true;
        let text = leaked.render();
        assert!(text.contains("\nleak thread\nrung 1 "), "{text}");
        assert_eq!(Ladder::parse(&text), Ok(leaked.clone()));
        assert_eq!(
            compare(&clean, &leaked),
            Some(Difference::Header { field: "leak" })
        );
    }

    #[test]
    fn compare_names_the_first_rung_and_its_tables() {
        let a = ladder();
        assert_eq!(compare(&a, &a), None);
        let mut b = ladder();
        b.rungs[1].roots[1].1 = 8;
        b.rungs[1].hash = 1;
        assert_eq!(
            compare(&a, &b),
            Some(Difference::Rung {
                index: 2,
                a: Some((199, "0001-12".to_owned())),
                b: Some((199, "0001-12".to_owned())),
                tables: vec!["b.y".to_owned()],
            })
        );
        // A ladder that starts later (a resumed run) is named by its rungs' own indices.
        let (mut late_a, mut late_b) = (ladder(), ladder());
        late_a.rungs.remove(0);
        late_b.rungs.remove(0);
        late_b.rungs[0].hash = 5;
        assert!(matches!(
            compare(&late_a, &late_b),
            Some(Difference::Rung { index: 2, .. })
        ));
        let mut short = ladder();
        short.rungs.pop();
        short.failed = Some((150, "boom".to_owned()));
        assert!(matches!(
            compare(&a, &short),
            Some(Difference::Rung {
                index: 2,
                b: None,
                ..
            })
        ));
        let mut other = ladder();
        other.header.seed = 2;
        assert_eq!(
            compare(&a, &other),
            Some(Difference::Header { field: "seed" })
        );
    }

    #[test]
    fn a_malformed_ladder_is_refused_at_its_line() {
        let text = ladder()
            .render()
            .replace("root b.y 0000000000000007", "root b.y 7");
        assert_eq!(Ladder::parse(&text).map_err(|e| e.line), Err(10));
        assert!(Ladder::parse("rehoboam-ladder v2\n").is_err());
        let bad_journal = ladder()
            .render()
            .replace("0123456789abcdef0123456789abcdef", "xyz");
        assert_eq!(Ladder::parse(&bad_journal).map_err(|e| e.line), Err(7));
        let upper = ladder().render().replace("abc1", "ABC1");
        assert!(Ladder::parse(&upper).is_err());
    }
}
