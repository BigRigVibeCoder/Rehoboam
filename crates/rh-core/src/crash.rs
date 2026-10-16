//! Crash artifacts: what a fatal error leaves behind so that it replays exactly (SPR-006 T-05;
//! GOV-004 §4; ruling 6).
//!
//! When an instant fails (a violated invariant or any other [`crate::system::SimError`]), the
//! engine builds a [`CrashArtifact`] with:
//! - the history tuple: build, Registry version, manifest hash, seed (with the branch salt and the
//!   timeline, which a replay also needs);
//! - the input-journal position: the number of material inputs applied before the failure
//!   (replay refuses a non-zero one until SPR-007 supplies the journal; DEF-2);
//! - the last keyframe (`none` until SPR-007) and the failing instant and phase;
//! - the failing invariant or error;
//! - the last diagnostic records (at most [`DIAGNOSTIC_RECORDS`]).
//!
//! `rh-core` has no I/O: the host writes [`CrashArtifact::to_json`] to
//! `artifacts/crash/<timeline>_<tick>.json` ([`CrashArtifact::file_name`]; the directory is not
//! committed) and reads it back with [`CrashArtifact::from_json`]. Replay
//! ([`crate::engine::Engine::replay`]) rebuilds the engine from the tuple, runs to the instant and
//! must hit the same violation.
//!
//! The JSON is written and read here, with no third-party dependency: a flat object of strings,
//! integers, `null` and one array of strings, in a fixed field order. The reader accepts exactly
//! that shape and refuses anything else.
//!
//! Diagnostic records carry sim time only: plane A never reads the wall clock (GOV-004 law 3);
//! the host may add wall time when it writes the file.
//!
//! REF: `CODEX/10_GOVERNANCE/GOV-004` §2, §4; ADR-0006;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-05, ruling 6.

use core::fmt;
use std::fmt::Write as _;

/// The most diagnostic records an artifact carries (GOV-004 §4).
pub const DIAGNOSTIC_RECORDS: usize = 1_000;

/// The directory crash artifacts go to, relative to the repository root (not committed).
pub const CRASH_DIR: &str = "artifacts/crash";

/// The parts of the history tuple the engine cannot know by itself; the host supplies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The engine build (a commit hash or version).
    pub build: String,
    /// The Registry version.
    pub registry_version: String,
    /// The manifest hash.
    pub manifest_hash: String,
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            build: "unknown".to_owned(),
            registry_version: "unknown".to_owned(),
            manifest_hash: "unknown".to_owned(),
        }
    }
}

/// A crash artifact (GOV-004 §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrashArtifact {
    /// The timeline (`TimelineId`).
    pub timeline: u32,
    /// The engine build.
    pub build: String,
    /// The Registry version.
    pub registry_version: String,
    /// The manifest hash.
    pub manifest_hash: String,
    /// The world seed.
    pub seed: u64,
    /// The branch salt of the timeline.
    pub branch_salt: u64,
    /// The input-journal position: the number of material inputs applied before the failure.
    pub journal_position: u64,
    /// The last keyframe, `None` until SPR-007.
    pub last_keyframe: Option<String>,
    /// The failing instant, in `i64` minutes.
    pub tick: i64,
    /// The failing phase.
    pub phase: String,
    /// The system the failure belongs to (`rh-core` for the core's own).
    pub system: String,
    /// The failing invariant, when the failure is a violated invariant.
    pub invariant: Option<String>,
    /// The error, as text.
    pub error: String,
    /// The last diagnostic records, oldest first.
    pub diagnostics: Vec<String>,
}

/// A text that is not a crash artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a crash artifact: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl CrashArtifact {
    /// `<timeline>_<tick>.json`.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}_{}.json", self.timeline, self.tick)
    }

    /// The artifact as JSON, fields in a fixed order, one per line.
    #[must_use]
    pub fn to_json(&self) -> String {
        let opt = |v: &Option<String>| v.as_deref().map_or_else(|| "null".to_owned(), quote);
        let mut out = String::from("{\n");
        let fields: [(&str, String); 13] = [
            ("timeline", self.timeline.to_string()),
            ("build", quote(&self.build)),
            ("registry_version", quote(&self.registry_version)),
            ("manifest_hash", quote(&self.manifest_hash)),
            ("seed", self.seed.to_string()),
            ("branch_salt", self.branch_salt.to_string()),
            ("journal_position", self.journal_position.to_string()),
            ("last_keyframe", opt(&self.last_keyframe)),
            ("tick", self.tick.to_string()),
            ("phase", quote(&self.phase)),
            ("system", quote(&self.system)),
            ("invariant", opt(&self.invariant)),
            ("error", quote(&self.error)),
        ];
        for (name, value) in fields {
            // Writing to a String cannot fail.
            let _ = writeln!(out, "  \"{name}\": {value},");
        }
        out.push_str("  \"diagnostics\": [");
        for (i, d) in self.diagnostics.iter().enumerate() {
            out.push_str(if i == 0 { "\n    " } else { ",\n    " });
            out.push_str(&quote(d));
        }
        out.push_str(if self.diagnostics.is_empty() {
            "]\n}\n"
        } else {
            "\n  ]\n}\n"
        });
        out
    }

    /// Reads an artifact written by [`CrashArtifact::to_json`].
    ///
    /// # Errors
    /// [`ParseError`] for anything but that shape: every field present once, with its type.
    pub fn from_json(text: &str) -> Result<Self, ParseError> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let fields = p.object()?;
        let take = |name: &str| -> Result<Value, ParseError> {
            let pos = fields
                .iter()
                .position(|(n, _)| n == name)
                .ok_or_else(|| ParseError(format!("missing field {name}")))?;
            Ok(fields[pos].1.clone())
        };
        let a = Self {
            timeline: u32::try_from(take("timeline")?.int()?)
                .map_err(|_| ParseError("timeline out of range".to_owned()))?,
            build: take("build")?.string()?,
            registry_version: take("registry_version")?.string()?,
            manifest_hash: take("manifest_hash")?.string()?,
            seed: take("seed")?.uint()?,
            branch_salt: take("branch_salt")?.uint()?,
            journal_position: take("journal_position")?.uint()?,
            last_keyframe: take("last_keyframe")?.opt_string()?,
            tick: i64::try_from(take("tick")?.int()?)
                .map_err(|_| ParseError("tick out of range".to_owned()))?,
            phase: take("phase")?.string()?,
            system: take("system")?.string()?,
            invariant: take("invariant")?.opt_string()?,
            error: take("error")?.string()?,
            diagnostics: take("diagnostics")?.strings()?,
        };
        if fields.len() != 14 {
            return Err(ParseError(format!(
                "{} fields where 14 are expected",
                fields.len()
            )));
        }
        Ok(a)
    }
}

/// A JSON string literal.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if u32::from(c) < 0x20 => {
                // Writing to a String cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[derive(Clone, Debug)]
enum Value {
    Str(String),
    Int(i128),
    Null,
    Strs(Vec<String>),
}

impl Value {
    fn string(self) -> Result<String, ParseError> {
        match self {
            Self::Str(s) => Ok(s),
            other => Err(ParseError(format!("expected a string, found {other:?}"))),
        }
    }
    fn opt_string(self) -> Result<Option<String>, ParseError> {
        match self {
            Self::Null => Ok(None),
            other => other.string().map(Some),
        }
    }
    fn int(self) -> Result<i128, ParseError> {
        match self {
            Self::Int(v) => Ok(v),
            other => Err(ParseError(format!("expected an integer, found {other:?}"))),
        }
    }
    fn uint(self) -> Result<u64, ParseError> {
        u64::try_from(self.int()?).map_err(|_| ParseError("integer out of range".to_owned()))
    }
    fn strings(self) -> Result<Vec<String>, ParseError> {
        match self {
            Self::Strs(v) => Ok(v),
            other => Err(ParseError(format!(
                "expected an array of strings, found {other:?}"
            ))),
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err<T>(&self, what: &str) -> Result<T, ParseError> {
        Err(ParseError(format!("{what} at byte {}", self.i)))
    }

    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, b: u8) -> Result<(), ParseError> {
        self.ws();
        if self.s.get(self.i) == Some(&b) {
            self.i += 1;
            Ok(())
        } else {
            self.err(&format!("expected '{}'", char::from(b)))
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn object(&mut self) -> Result<Vec<(String, Value)>, ParseError> {
        self.eat(b'{')?;
        let mut fields: Vec<(String, Value)> = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
        } else {
            loop {
                let name = self.string()?;
                if fields.iter().any(|(n, _)| *n == name) {
                    return self.err(&format!("duplicate field {name}"));
                }
                self.eat(b':')?;
                let value = self.value()?;
                fields.push((name, value));
                match self.peek() {
                    Some(b',') => self.i += 1,
                    Some(b'}') => {
                        self.i += 1;
                        break;
                    }
                    _ => return self.err("expected ',' or '}'"),
                }
            }
        }
        if self.peek().is_some() {
            return self.err("trailing text");
        }
        Ok(fields)
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some(b'"') => self.string().map(Value::Str),
            Some(b'n') => {
                if self.s[self.i..].starts_with(b"null") {
                    self.i += 4;
                    Ok(Value::Null)
                } else {
                    self.err("expected null")
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(Value::Strs(items));
                }
                loop {
                    items.push(self.string()?);
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Strs(items));
                        }
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => {
                let start = self.i;
                if self.s[self.i] == b'-' {
                    self.i += 1;
                }
                while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                    self.i += 1;
                }
                let text = core::str::from_utf8(&self.s[start..self.i])
                    .map_err(|_| ParseError("bad number".to_owned()))?;
                text.parse::<i128>()
                    .map(Value::Int)
                    .map_err(|_| ParseError(format!("bad integer {text}")))
            }
            _ => self.err("expected a value"),
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.eat(b'"')?;
        let mut bytes = Vec::new();
        loop {
            let Some(&b) = self.s.get(self.i) else {
                return self.err("unterminated string");
            };
            self.i += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return self.err("unterminated escape");
                    };
                    self.i += 1;
                    match e {
                        b'"' => bytes.push(b'"'),
                        b'\\' => bytes.push(b'\\'),
                        b'/' => bytes.push(b'/'),
                        b'n' => bytes.push(b'\n'),
                        b't' => bytes.push(b'\t'),
                        b'r' => bytes.push(b'\r'),
                        b'u' => {
                            let hex = self
                                .s
                                .get(self.i..self.i + 4)
                                .and_then(|h| core::str::from_utf8(h).ok())
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .and_then(char::from_u32);
                            let Some(c) = hex else {
                                return self.err("bad \\u escape");
                            };
                            self.i += 4;
                            let mut buf = [0_u8; 4];
                            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return self.err("bad escape"),
                    }
                }
                b if b < 0x20 => return self.err("control character in string"),
                b => bytes.push(b),
            }
        }
        String::from_utf8(bytes).map_err(|_| ParseError("string is not UTF-8".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::{CrashArtifact, ParseError};

    fn sample() -> CrashArtifact {
        CrashArtifact {
            timeline: 0,
            build: "abc1234".to_owned(),
            registry_version: "0.1.0".to_owned(),
            manifest_hash: "deadbeef".to_owned(),
            seed: u64::MAX,
            branch_salt: 0,
            journal_position: 0,
            last_keyframe: None,
            tick: -1_440,
            phase: "Bookkeeping".to_owned(),
            system: "X.stub".to_owned(),
            invariant: Some("no-negatives".to_owned()),
            error: "invariant \"no-negatives\" violated:\n\trow 3 = -1 \\ \u{1}".to_owned(),
            diagnostics: vec!["t=0 a".to_owned(), "t=1 b ü".to_owned()],
        }
    }

    #[test]
    fn round_trips() {
        let a = sample();
        assert_eq!(CrashArtifact::from_json(&a.to_json()), Ok(a.clone()));
        let mut b = a;
        b.diagnostics.clear();
        b.invariant = None;
        b.last_keyframe = Some("kf/0001".to_owned());
        assert_eq!(CrashArtifact::from_json(&b.to_json()), Ok(b.clone()));
        assert_eq!(b.file_name(), "0_-1440.json");
    }

    #[test]
    fn refuses_other_shapes() {
        let good = sample().to_json();
        let bad = [
            good.replace("\"seed\": 18446744073709551615", "\"seed\": -1"),
            good.replace("\"tick\"", "\"tock\""),
            good.replace("\"phase\": \"Bookkeeping\"", "\"phase\": 3"),
            good.replace("\n}\n", ",\n  \"extra\": 1\n}\n"),
            good.replace("\n}\n", "\n} x"),
            good.replace(
                "\"build\": \"abc1234\",",
                "\"build\": \"abc1234\", \"build\": \"x\",",
            ),
        ];
        for text in bad {
            assert!(
                matches!(CrashArtifact::from_json(&text), Err(ParseError(_))),
                "accepted: {text}"
            );
        }
    }
}
