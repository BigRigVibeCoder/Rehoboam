//! The canonical text form v0 and `segment.canonical_digest` (`specs/stream/canonical_v0.md`).
//!
//! Every record has one lossless, ASCII, one-line spelling: [`render`] prints it, [`parse`] reads
//! it and refuses every other spelling, so `parse(render(r)) = r` and `render(parse(l)) = l`. Both
//! check the DDL's domains (enum codes) and the list orders and record invariants of §3, exactly
//! as the reference `specs/stream/canonical_v0.py` does; `tests/canonical.rs` reproduces every
//! line and digest of its golden file. [`digest`] is the XXH3-128 of the lines, each with its LF
//! (§4); [`digest_file`] checks a whole canonical file (§1) first. [`match_id`] is channel C's
//! content-addressed `MatchId` (§6).
//!
//! No float anywhere: every value is an integer, a hash, a blob or a string.
//!
//! REF: `specs/stream/canonical_v0.md` §1–§6; `specs/stream/canonical_v0.py`.

use core::fmt;

use rh_snapshot::hash::{xxh3, xxh3_128};

use crate::record::{
    Binding, Body, Cause, Channel, ClauseEvent, Event, Header, Input, Keyframe, Kind, LifeEntry,
    LifeExit, Match, Record, Role, Snapshot, StepDigest,
};

/// The canonical form's version line (`canonical_v0.md` §1).
pub const FORMAT: &str = "rehoboam/stream/canonical/v0";
/// `MatchId`'s domain tag (§6).
pub const D_MATCH: &[u8] = b"rehoboam/sift/match/v0";
/// The token of SQL NULL.
const NULL: &str = "~";
/// The token of the empty string.
const EMPTY: &str = "%";
/// "-9223372036854775808" and "18446744073709551615" are 20 characters.
const INT_MAX_CHARS: usize = 20;

/// A token, line, file or value that is not in the canonical form or is outside its domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalError(pub String);

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CanonicalError {}

type Result<T> = core::result::Result<T, CanonicalError>;

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(CanonicalError(message.into()))
}

// ------------------------------------------------------------------------------------- tokens

/// Whether `b` is written as itself in a string token (§2): `A-Z a-z 0-9 . _ - / : + @`.
const fn safe(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/' | b':' | b'+' | b'@')
}

/// The string token of `s`: its UTF-8 bytes, unsafe ones as `%XX`; the empty string is `%`.
#[must_use]
pub fn encode_str(s: &str) -> String {
    if s.is_empty() {
        return EMPTY.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if safe(b) {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(hex_digit(b >> 4, true));
            out.push(hex_digit(b & 0xF, true));
        }
    }
    out
}

fn hex_digit(n: u8, upper: bool) -> char {
    let c = if n < 10 {
        b'0' + n
    } else if upper {
        b'A' + n - 10
    } else {
        b'a' + n - 10
    };
    char::from(c)
}

/// The string a token encodes (the inverse of [`encode_str`]), refusing every other spelling.
///
/// # Errors
/// [`CanonicalError`]: an empty token, an unescaped unsafe byte, a lowercase, truncated or
/// safe-byte escape, or bytes that are not UTF-8.
pub fn decode_str(tok: &str) -> Result<String> {
    if tok == EMPTY {
        return Ok(String::new());
    }
    if tok.is_empty() {
        return err("string: empty token (the empty string is '%')");
    }
    let bytes = tok.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            let esc = bytes.get(i + 1..i + 3).unwrap_or_default();
            let value = match esc {
                [h, l] => upper_hex(*h).zip(upper_hex(*l)).map(|(h, l)| h << 4 | l),
                _ => None,
            };
            let Some(value) = value else {
                return err(format!("string: bad escape in {tok:?}"));
            };
            if safe(value) {
                return err(format!("string: an escape of a safe byte in {tok:?}"));
            }
            out.push(value);
            i += 3;
        } else if safe(b) {
            out.push(b);
            i += 1;
        } else {
            return err(format!("string: an unescaped byte {b:#04x} in {tok:?}"));
        }
    }
    String::from_utf8(out).map_err(|e| CanonicalError(format!("string: {tok:?} is not UTF-8: {e}")))
}

const fn upper_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

const fn lower_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// A canonical decimal integer token in `[lo, hi]` (§2).
fn parse_int(name: &str, tok: &str, lo: i128, hi: i128) -> Result<i128> {
    let digits = tok.strip_prefix('-').unwrap_or(tok);
    let canonical = tok.len() <= INT_MAX_CHARS
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" && tok == "0" || !digits.starts_with('0'));
    if !canonical {
        return err(format!("{name}: {tok:?} is not a canonical integer"));
    }
    let v: i128 = tok
        .parse()
        .map_err(|_| CanonicalError(format!("{name}: {tok:?} is not a canonical integer")))?;
    if v < lo || v > hi {
        return err(format!("{name}: {v} is outside [{lo}, {hi}]"));
    }
    Ok(v)
}

macro_rules! int_parser {
    ($fn:ident, $t:ty, $name:literal) => {
        fn $fn(tok: &str) -> Result<$t> {
            let v = parse_int($name, tok, i128::from(<$t>::MIN), i128::from(<$t>::MAX))?;
            <$t>::try_from(v).map_err(|_| CanonicalError(format!("{}: {v} out of range", $name)))
        }
    };
}

int_parser!(p_i64, i64, "i64");
int_parser!(p_i32, i32, "i32");
int_parser!(p_i16, i16, "i16");
int_parser!(p_u64, u64, "u64");
int_parser!(p_u32, u32, "u32");
int_parser!(p_u16, u16, "u16");
int_parser!(p_u8, u8, "u8");

fn p_opt<T>(tok: &str, inner: fn(&str) -> Result<T>) -> Result<Option<T>> {
    if tok == NULL {
        Ok(None)
    } else {
        inner(tok).map(Some)
    }
}

fn r_opt<T: fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| NULL.to_owned(), |x| x.to_string())
}

fn p_hash64(name: &str, tok: &str) -> Result<u64> {
    if tok.len() != 16 || !tok.bytes().all(|b| lower_hex(b).is_some()) {
        return err(format!("{name}: {tok:?} is not 16 lowercase hex digits"));
    }
    u64::from_str_radix(tok, 16).map_err(|e| CanonicalError(format!("{name}: {e}")))
}

fn r_hash64(v: u64) -> String {
    format!("{v:016x}")
}

fn p_blob16(name: &str, tok: &str) -> Result<[u8; 16]> {
    let b = tok.as_bytes();
    if b.len() != 32 {
        return err(format!("{name}: {tok:?} is not 32 lowercase hex digits"));
    }
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        let (Some(h), Some(l)) = (lower_hex(b[2 * i]), lower_hex(b[2 * i + 1])) else {
            return err(format!("{name}: {tok:?} is not 32 lowercase hex digits"));
        };
        *o = h << 4 | l;
    }
    Ok(out)
}

/// 16 bytes as 32 lowercase hex digits, in storage order.
#[must_use]
pub fn hex16(v: &[u8; 16]) -> String {
    v.iter()
        .flat_map(|b| [hex_digit(b >> 4, false), hex_digit(b & 0xF, false)])
        .collect()
}

fn p_ident(name: &str, tok: &str) -> Result<String> {
    let b = tok.as_bytes();
    let ok = b
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_');
    if ok {
        Ok(tok.to_owned())
    } else {
        err(format!("{name}: {tok:?} is not an identifier"))
    }
}

fn p_clause(tok: &str) -> Result<String> {
    let ok =
        tok.len() > 1 && tok.starts_with('e') && tok.bytes().skip(1).all(|c| c.is_ascii_digit());
    if ok {
        Ok(tok.to_owned())
    } else {
        err(format!("clause: {tok:?} is not e[0-9]+"))
    }
}

/// The items of a list token `[a,b,...]`; `[]` is empty.
fn list_items<'a>(name: &str, tok: &'a str) -> Result<Vec<&'a str>> {
    let Some(inner) = tok.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else {
        return err(format!("{name}: {tok:?} is not a bracketed list"));
    };
    Ok(if inner.is_empty() {
        Vec::new()
    } else {
        inner.split(',').collect()
    })
}

fn tuple_parts<'a>(name: &str, tok: &'a str, n: usize) -> Result<Vec<&'a str>> {
    let parts: Vec<&str> = tok.split('/').collect();
    if parts.len() == n {
        Ok(parts)
    } else {
        err(format!(
            "{name}: {tok:?} has {} parts, not {n}",
            parts.len()
        ))
    }
}

fn render_list<T>(items: &[T], f: impl Fn(&T) -> String) -> String {
    let inner: Vec<String> = items.iter().map(f).collect();
    format!("[{}]", inner.join(","))
}

// ------------------------------------------------------------------------------------- domains

fn check_code(name: &str, v: u8, allowed: &[u8]) -> Result<u8> {
    if allowed.contains(&v) {
        Ok(v)
    } else {
        err(format!("{name}: code {v} is not one of {allowed:?}"))
    }
}

const PHASES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const ZERO: [u8; 1] = [0];
const SECRECY: [u8; 4] = [0, 1, 2, 3];
const ORIGIN: [u8; 4] = [0, 1, 3, 4];
/// 5 = artifact, reserved for S6 (`envelope_v0.md` §1).
const ENTITY_KIND: [u8; 6] = [0, 1, 2, 3, 4, 5];
const ROLE: [u8; 4] = [0, 1, 2, 3];
const EDGE_KIND: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// `player`, `oracle`, `director`: in the DDL's domain, no writer until the input-event format
/// exists (`envelope_v0.md` §5; REV-SPR-020 R22 NIT-5).
const UNWRITTEN_KINDS: [u8; 3] = [6, 7, 8];
/// `contingent`, `triggered`, `player`, `oracle`, `director`: evidence is 1.
const DETERMINISTIC_KINDS: [u8; 5] = [0, 1, 6, 7, 8];
/// The one edge kind that may carry a motive and a weight.
const MOTIVATED_BY: u8 = 3;
/// The one role that may repeat on an event.
const WITNESS: u8 = 2;
/// `q8_8` raw bits of 1.
const EVIDENCE_ONE: i16 = 256;
/// The ordinal is a u8.
const MAX_CAST: usize = 256;
const CAUSE_TOLD: [u8; 2] = [0, 1];
const LIFE_ORIGIN: [u8; 3] = [0, 1, 2];
const SEX: [u8; 2] = [0, 1];
const EXIT_KIND: [u8; 2] = [0, 1];
const DEATH_CAUSE: [u8; 6] = [0, 1, 2, 3, 4, 5];
const SELECTION: [u8; 3] = [0, 1, 2];
const ON_COMPLETE: [u8; 3] = [0, 1, 2];
const STATUS: [u8; 1] = [1];

fn code(name: &str, tok: &str, allowed: &[u8]) -> Result<u8> {
    check_code(name, p_u8(tok)?, allowed)
}

/// The DDL domains, list orders and record invariants of a record (§2, §3).
fn check(rec: &Record) -> Result<()> {
    match &rec.body {
        Body::Input(i) => {
            check_code("class", i.class, &ZERO)?;
        }
        Body::Event(e) => check_event(e)?,
        Body::LifeEntry(e) => {
            check_code("life origin", e.origin, &LIFE_ORIGIN)?;
            check_code("sex", e.sex, &SEX)?;
        }
        Body::LifeExit(x) => {
            check_code("exit_kind", x.exit_kind, &EXIT_KIND)?;
            check_code("death_cause", x.death_cause, &DEATH_CAUSE)?;
            if x.exit_kind == 0 && x.death_cause == 0 {
                return err(
                    "LIFE_EXIT: a death (exit_kind 0) needs a death_cause other than 0 (none)",
                );
            }
            if x.exit_kind == 1 && (x.death_cause != 0 || x.event_id.is_some()) {
                return err(
                    "LIFE_EXIT: an emigration (exit_kind 1) has death_cause 0 and no event_id",
                );
            }
        }
        Body::StepDigest(_) | Body::Keyframe(_) => {}
        Body::Match(m) => check_match(m, rec.header.tick)?,
    }
    Ok(())
}

/// The envelope's domains (`envelope_v0.md` §1, §4, §5): what `EventLog::append` refuses, the
/// parser refuses (§3).
fn check_event(e: &Event) -> Result<()> {
    check_code("phase", e.phase, &PHASES)?;
    check_code("secrecy", e.secrecy, &SECRECY)?;
    check_code("origin", e.origin, &ORIGIN)?;
    for r in &e.roles {
        check_code("entity_kind", r.entity_kind, &ENTITY_KIND)?;
        check_code("role", r.role, &ROLE)?;
    }
    if let Some(w) = e
        .roles
        .windows(2)
        .find(|w| w[0].order_key() >= w[1].order_key())
    {
        return err(format!(
            "roles (by role code, then entity) not strictly ascending: {:?} then {:?}",
            w[0].order_key(),
            w[1].order_key()
        ));
    }
    if e.roles.len() > MAX_CAST {
        return err(format!(
            "a cast of {} rows; the ordinal is a u8 (at most {MAX_CAST})",
            e.roles.len()
        ));
    }
    if let Some(w) = e
        .roles
        .windows(2)
        .find(|w| w[0].role == w[1].role && w[0].role != WITNESS)
    {
        return err(format!(
            "role {} filled twice; only witness ({WITNESS}) may repeat",
            w[0].role
        ));
    }
    for x in &e.snapshots {
        check_code("entity_kind", x.entity_kind, &ENTITY_KIND)?;
        if let Some(k) = x.partner_kind {
            check_code("entity_kind", k, &ENTITY_KIND)?;
        }
        if x.partner_kind.is_some() != x.partner_id.is_some() {
            return err(format!(
                "snapshot {:?}: partner_kind and partner_id are both NULL or both set",
                x.key()
            ));
        }
    }
    if let Some(w) = e.snapshots.windows(2).find(|w| w[0].key() >= w[1].key()) {
        return err(format!(
            "snapshot keys not strictly ascending: {:?} then {:?}",
            w[0].key(),
            w[1].key()
        ));
    }
    for c in &e.causes {
        check_code("edge kind", c.kind, &EDGE_KIND)?;
        if !(0..=EVIDENCE_ONE).contains(&c.evidence) {
            return err(format!("evidence: {} is outside [0, 256]", c.evidence));
        }
        if c.cause_id >= e.event_id {
            return err(format!(
                "cause {}: not earlier than event {} (ForwardCause)",
                c.cause_id, e.event_id
            ));
        }
        if UNWRITTEN_KINDS.contains(&c.kind) {
            return err(format!(
                "cause {}: kind {} (player, oracle, director) has no writer in P1a",
                c.cause_id, c.kind
            ));
        }
        if c.motive.is_some() && c.kind != MOTIVATED_BY {
            return err(format!(
                "cause {}: a motive refines motivated_by (kind {MOTIVATED_BY}) only",
                c.cause_id
            ));
        }
        if c.kind != MOTIVATED_BY && c.weight != 0 {
            return err(format!(
                "cause {}: weight is 0 on every kind but motivated_by",
                c.cause_id
            ));
        }
        if DETERMINISTIC_KINDS.contains(&c.kind) && c.evidence != EVIDENCE_ONE {
            return err(format!(
                "cause {}: evidence is 1 ({EVIDENCE_ONE}) on a deterministic kind",
                c.cause_id
            ));
        }
    }
    if let Some(w) = e.causes.windows(2).find(|w| w[0].cause_id >= w[1].cause_id) {
        return err(format!(
            "causes: cause ids not strictly ascending: {} then {}",
            w[0].cause_id, w[1].cause_id
        ));
    }
    Ok(())
}

fn check_match(m: &Match, tick: i64) -> Result<()> {
    check_code("selection", m.selection, &SELECTION)?;
    check_code("on_complete", m.on_complete, &ON_COMPLETE)?;
    check_code("status", m.status, &STATUS)?;
    for b in &m.bindings {
        p_ident("role variable", &b.role)?;
        check_code("entity_kind", b.entity_kind, &ENTITY_KIND)?;
    }
    if let Some(w) = m
        .bindings
        .windows(2)
        .find(|w| w[0].role.as_bytes() >= w[1].role.as_bytes())
    {
        return err(format!(
            "bindings: roles not strictly ascending: {:?} then {:?}",
            w[0].role, w[1].role
        ));
    }
    for e in &m.events {
        p_clause(&e.clause)?;
    }
    let names: std::collections::BTreeSet<&str> =
        m.events.iter().map(|e| e.clause.as_str()).collect();
    if names.len() != m.events.len() {
        return err("events: clause names repeat");
    }
    if let Some(w) = m.events.windows(2).find(|w| w[0].event_id >= w[1].event_id) {
        return err(format!(
            "events: event ids not strictly ascending: {} then {}",
            w[0].event_id, w[1].event_id
        ));
    }
    if m.events.is_empty() {
        return err("MATCH: a match binds at least one event");
    }
    if !m.events.iter().any(|e| e.event_id == m.closed_by_event_id) {
        return err(format!(
            "MATCH: closed_by_event_id {} is not among the events",
            m.closed_by_event_id
        ));
    }
    if m.opened_tick > tick {
        return err(format!(
            "MATCH: opened_tick {} is after the closing tick {tick}",
            m.opened_tick
        ));
    }
    Ok(())
}

// ------------------------------------------------------------------------------------- records

/// The canonical line of `rec`, without its LF.
///
/// # Errors
/// [`CanonicalError`] when a value is outside its DDL domain or a list is out of order.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per record kind, its fields in canonical_v0.md §3's order"
)]
pub fn render(rec: &Record) -> Result<String> {
    check(rec)?;
    let h = &rec.header;
    let mut t: Vec<String> = vec![
        rec.kind().name().to_owned(),
        h.timeline_id.to_string(),
        h.seq.to_string(),
        h.tick.to_string(),
    ];
    match &rec.body {
        Body::Input(i) => t.extend([
            i.journal_seq.to_string(),
            i.class.to_string(),
            encode_str(&i.input_kind),
            encode_str(&i.scope),
            r_opt(i.scope_id),
            encode_str(&i.producer),
            hex16(&i.payload_hash),
            hex16(&i.row_digest),
        ]),
        Body::Event(e) => t.extend([
            e.event_id.to_string(),
            e.phase.to_string(),
            e.verb_id.to_string(),
            encode_str(&e.verb),
            e.act_classes.to_string(),
            r_opt(e.place_id),
            e.magnitude.to_string(),
            e.tellability.to_string(),
            e.secrecy.to_string(),
            e.origin.to_string(),
            render_list(&e.payload, ToString::to_string),
            render_list(&e.roles, |r| {
                format!("{}/{}/{}", r.entity_kind, r.entity_id, r.role)
            }),
            render_list(&e.snapshots, |x| {
                format!(
                    "{}/{}/{}/{}/{}/{}",
                    x.attr_id,
                    x.entity_kind,
                    x.entity_id,
                    r_opt(x.partner_kind),
                    r_opt(x.partner_id),
                    x.value
                )
            }),
            render_list(&e.causes, |c| {
                format!(
                    "{}/{}/{}/{}/{}/{}",
                    c.cause_id,
                    c.kind,
                    r_opt(c.motive),
                    c.weight,
                    c.evidence,
                    u8::from(c.cause_told)
                )
            }),
        ]),
        Body::LifeEntry(e) => t.extend([
            e.person_id.to_string(),
            e.origin.to_string(),
            e.sex.to_string(),
            e.mother_id.to_string(),
            e.father_id.to_string(),
            e.birth_tick.to_string(),
        ]),
        Body::LifeExit(x) => t.extend([
            x.person_id.to_string(),
            x.exit_kind.to_string(),
            x.death_cause.to_string(),
            r_opt(x.event_id),
        ]),
        Body::StepDigest(d) => t.extend([
            d.living.to_string(),
            d.births.to_string(),
            d.deaths.to_string(),
            r_hash64(d.state_hash),
            r_hash64(d.observation_hash),
            r_hash64(d.narrative_hash),
        ]),
        Body::Keyframe(k) => t.extend([
            encode_str(&k.keyframe_format),
            encode_str(&k.keyframe_path),
            r_hash64(k.state_hash),
            k.journal_position.to_string(),
        ]),
        Body::Match(m) => t.extend([
            r_hash64(m.match_id),
            encode_str(&m.pack),
            encode_str(&m.pattern_id),
            m.pattern_version.to_string(),
            m.selection.to_string(),
            m.on_complete.to_string(),
            m.status.to_string(),
            m.opened_tick.to_string(),
            m.closed_by_event_id.to_string(),
            m.closed_by_seq.to_string(),
            render_list(&m.bindings, |b| {
                format!("{}/{}/{}", b.role, b.entity_kind, b.entity_id)
            }),
            render_list(&m.events, |e| format!("{}/{}", e.clause, e.event_id)),
        ]),
    }
    Ok(t.join(" "))
}

/// The number of fields after the kind name, per kind (§3).
const fn arity(kind: Kind) -> usize {
    3 + match kind {
        Kind::Input => 8,
        Kind::Event => 14,
        Kind::LifeEntry | Kind::StepDigest => 6,
        Kind::LifeExit | Kind::Keyframe => 4,
        Kind::Match => 12,
    }
}

/// The record of one canonical line (given without its LF).
///
/// # Errors
/// [`CanonicalError`] for any line that is not exactly a record's canonical spelling.
pub fn parse(line: &str) -> Result<Record> {
    if line.is_empty() || !line.is_ascii() || line.contains(['\n', '\r']) {
        return err(format!(
            "line is empty, not ASCII or holds a line break: {line:?}"
        ));
    }
    let tokens: Vec<&str> = line.split(' ').collect();
    let Some(kind) = Kind::from_name(tokens[0]) else {
        return err(format!("unknown kind {:?}", tokens[0]));
    };
    if tokens.len() - 1 != arity(kind) {
        return err(format!(
            "{}: {} fields, expected {}",
            kind.name(),
            tokens.len() - 1,
            arity(kind)
        ));
    }
    let f = &tokens[1..];
    let header = Header {
        timeline_id: p_u32(f[0])?,
        seq: p_u64(f[1])?,
        tick: p_i64(f[2])?,
    };
    let f = &f[3..];
    let body = match kind {
        Kind::Input => Body::Input(Input {
            journal_seq: p_u64(f[0])?,
            class: code("class", f[1], &ZERO)?,
            input_kind: decode_str(f[2])?,
            scope: decode_str(f[3])?,
            scope_id: p_opt(f[4], p_u64)?,
            producer: decode_str(f[5])?,
            payload_hash: p_blob16("payload_hash", f[6])?,
            row_digest: p_blob16("row_digest", f[7])?,
        }),
        Kind::Event => Body::Event(parse_event(f)?),
        Kind::LifeEntry => Body::LifeEntry(LifeEntry {
            person_id: p_u64(f[0])?,
            origin: code("life origin", f[1], &LIFE_ORIGIN)?,
            sex: code("sex", f[2], &SEX)?,
            mother_id: p_u64(f[3])?,
            father_id: p_u64(f[4])?,
            birth_tick: p_i64(f[5])?,
        }),
        Kind::LifeExit => Body::LifeExit(LifeExit {
            person_id: p_u64(f[0])?,
            exit_kind: code("exit_kind", f[1], &EXIT_KIND)?,
            death_cause: code("death_cause", f[2], &DEATH_CAUSE)?,
            event_id: p_opt(f[3], p_u64)?,
        }),
        Kind::StepDigest => Body::StepDigest(StepDigest {
            living: p_u64(f[0])?,
            births: p_u32(f[1])?,
            deaths: p_u32(f[2])?,
            state_hash: p_hash64("state_hash", f[3])?,
            observation_hash: p_hash64("observation_hash", f[4])?,
            narrative_hash: p_hash64("narrative_hash", f[5])?,
        }),
        Kind::Keyframe => Body::Keyframe(Keyframe {
            keyframe_format: decode_str(f[0])?,
            keyframe_path: decode_str(f[1])?,
            state_hash: p_hash64("state_hash", f[2])?,
            journal_position: p_u64(f[3])?,
        }),
        Kind::Match => Body::Match(parse_match(f)?),
    };
    let rec = Record { header, body };
    check(&rec)?;
    Ok(rec)
}

fn parse_event(f: &[&str]) -> Result<Event> {
    let roles = list_items("roles", f[11])?
        .into_iter()
        .map(|it| {
            let p = tuple_parts("role", it, 3)?;
            Ok(Role {
                entity_kind: code("entity_kind", p[0], &ENTITY_KIND)?,
                entity_id: p_u64(p[1])?,
                role: code("role", p[2], &ROLE)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let snapshots = list_items("snapshots", f[12])?
        .into_iter()
        .map(|it| {
            let p = tuple_parts("snapshot", it, 6)?;
            Ok(Snapshot {
                attr_id: p_u16(p[0])?,
                entity_kind: code("entity_kind", p[1], &ENTITY_KIND)?,
                entity_id: p_u64(p[2])?,
                partner_kind: p_opt(p[3], p_u8)?,
                partner_id: p_opt(p[4], p_u64)?,
                value: p_i32(p[5])?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let causes = list_items("causes", f[13])?
        .into_iter()
        .map(|it| {
            let p = tuple_parts("cause", it, 6)?;
            Ok(Cause {
                cause_id: p_u64(p[0])?,
                kind: code("edge kind", p[1], &EDGE_KIND)?,
                motive: p_opt(p[2], p_u16)?,
                weight: p_i16(p[3])?,
                evidence: p_i16(p[4])?,
                cause_told: code("cause_told", p[5], &CAUSE_TOLD)? == 1,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Event {
        event_id: p_u64(f[0])?,
        phase: code("phase", f[1], &PHASES)?,
        verb_id: p_u16(f[2])?,
        verb: decode_str(f[3])?,
        act_classes: p_u64(f[4])?,
        place_id: p_opt(f[5], p_u32)?,
        magnitude: p_i32(f[6])?,
        tellability: p_u8(f[7])?,
        secrecy: code("secrecy", f[8], &SECRECY)?,
        origin: code("origin", f[9], &ORIGIN)?,
        payload: list_items("payload", f[10])?
            .into_iter()
            .map(p_i64)
            .collect::<Result<Vec<_>>>()?,
        roles,
        snapshots,
        causes,
    })
}

fn parse_match(f: &[&str]) -> Result<Match> {
    let bindings = list_items("bindings", f[10])?
        .into_iter()
        .map(|it| {
            let p = tuple_parts("binding", it, 3)?;
            Ok(Binding {
                role: p_ident("role variable", p[0])?,
                entity_kind: code("entity_kind", p[1], &ENTITY_KIND)?,
                entity_id: p_u64(p[2])?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let events = list_items("events", f[11])?
        .into_iter()
        .map(|it| {
            let p = tuple_parts("event", it, 2)?;
            Ok(ClauseEvent {
                clause: p_clause(p[0])?,
                event_id: p_u64(p[1])?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Match {
        match_id: p_hash64("match_id", f[0])?,
        pack: decode_str(f[1])?,
        pattern_id: decode_str(f[2])?,
        pattern_version: p_u16(f[3])?,
        selection: code("selection", f[4], &SELECTION)?,
        on_complete: code("on_complete", f[5], &ON_COMPLETE)?,
        status: code("status", f[6], &STATUS)?,
        opened_tick: p_i64(f[7])?,
        closed_by_event_id: p_u64(f[8])?,
        closed_by_seq: p_u64(f[9])?,
        bindings,
        events,
    })
}

// ------------------------------------------------------------------------------------- digests

/// `canonical_digest` (§4): XXH3-128 over the lines, each followed by LF, in the order given; the
/// 16 bytes are XXH3's canonical form.
#[must_use]
pub fn digest<S: AsRef<str>>(lines: &[S]) -> [u8; 16] {
    let mut buf = Vec::new();
    for l in lines {
        buf.extend_from_slice(l.as_ref().as_bytes());
        buf.push(b'\n');
    }
    xxh3_128(&buf)
}

/// The canonical digest of records: each rendered, in the order given.
///
/// # Errors
/// [`CanonicalError`] if a record cannot be rendered.
pub fn digest_records(records: &[Record]) -> Result<[u8; 16]> {
    let lines = records.iter().map(render).collect::<Result<Vec<_>>>()?;
    Ok(digest(&lines))
}

/// Channel C's `MatchId` (§6): XXH3-64 over the pattern, the event ids in clause order and the
/// bindings in bytewise role order.
#[must_use]
pub fn match_id(
    pack: &str,
    pattern_id: &str,
    version: u16,
    event_ids: &[u64],
    bindings: &[Binding],
) -> u64 {
    let mut b = D_MATCH.to_vec();
    let put_str = |b: &mut Vec<u8>, s: &str| {
        b.extend_from_slice(&len32(s.len()).to_le_bytes());
        b.extend_from_slice(s.as_bytes());
    };
    put_str(&mut b, pack);
    put_str(&mut b, pattern_id);
    b.extend_from_slice(&version.to_le_bytes());
    b.extend_from_slice(&len32(event_ids.len()).to_le_bytes());
    for e in event_ids {
        b.extend_from_slice(&e.to_le_bytes());
    }
    let mut ordered: Vec<&Binding> = bindings.iter().collect();
    ordered.sort_by(|x, y| x.role.as_bytes().cmp(y.role.as_bytes()));
    b.extend_from_slice(&len32(ordered.len()).to_le_bytes());
    for x in ordered {
        put_str(&mut b, &x.role);
        b.push(x.entity_kind);
        b.extend_from_slice(&x.entity_id.to_le_bytes());
    }
    xxh3(&b)
}

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

// ------------------------------------------------------------------------------------- files

/// A canonical file's header line (§1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileHeader {
    /// The timeline.
    pub timeline_id: u32,
    /// The channel.
    pub channel: Channel,
    /// The Registry version the dictionaries came from.
    pub registry_version: String,
    /// The segment format.
    pub format: String,
}

impl FileHeader {
    /// The header line.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "#{FORMAT} timeline={} channel={} registry_version={} format={}",
            self.timeline_id,
            self.channel.letter(),
            encode_str(&self.registry_version),
            encode_str(&self.format)
        )
    }

    fn parse(line: &str) -> Result<Self> {
        let bad = || CanonicalError(format!("not a canonical header line: {line:?}"));
        let rest = line
            .strip_prefix('#')
            .and_then(|l| l.strip_prefix(FORMAT))
            .and_then(|l| l.strip_prefix(' '))
            .ok_or_else(bad)?;
        let parts: Vec<&str> = rest.split(' ').collect();
        let [tl, ch, rv, fm] = parts.as_slice() else {
            return Err(bad());
        };
        let value = |tok: &str, key: &str| tok.strip_prefix(key).ok_or_else(bad).map(str::to_owned);
        let tl = value(tl, "timeline=")?;
        let timeline_id = p_u32(&tl).map_err(|_| bad())?;
        let channel = match value(ch, "channel=")?.as_str() {
            "A" => Channel::A,
            "C" => Channel::C,
            _ => return Err(bad()),
        };
        let registry_version = value(rv, "registry_version=")?;
        let format = value(fm, "format=")?;
        if registry_version.is_empty() || format.is_empty() {
            return Err(bad());
        }
        Ok(Self {
            timeline_id,
            channel,
            registry_version: decode_str(&registry_version).map_err(|_| bad())?,
            format: decode_str(&format).map_err(|_| bad())?,
        })
    }
}

/// A checked canonical file: its header, its records in order and their digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalFile {
    /// The header.
    pub header: FileHeader,
    /// The records, in line order.
    pub records: Vec<Record>,
    /// The digest of their lines.
    pub digest: [u8; 16],
}

/// Checks a whole canonical file as §1 says (ASCII, LF-terminated, no CR, the header first, every
/// record line canonical and of the header's channel and timeline, contiguous `seq`, ticks never
/// decreasing) and digests its record lines; comment lines (`#`) are skipped.
///
/// # Errors
/// [`CanonicalError`] naming the first problem and its line.
pub fn digest_file(data: &[u8]) -> Result<CanonicalFile> {
    if data.last() != Some(&b'\n') {
        return err("the file is empty or does not end with LF");
    }
    if !data.is_ascii() {
        return err("the file holds a byte outside ASCII");
    }
    if data.contains(&b'\r') {
        return err("the file holds a carriage return");
    }
    let text =
        core::str::from_utf8(&data[..data.len() - 1]).map_err(|e| CanonicalError(e.to_string()))?;
    let lines: Vec<&str> = text.split('\n').collect();
    let header = FileHeader::parse(lines[0]).map_err(|e| CanonicalError(format!("1: {e}")))?;
    let mut records = Vec::new();
    let mut raw = Vec::new();
    let mut last: Option<(usize, Header)> = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        let n = i + 1;
        if line.starts_with('#') {
            continue;
        }
        let rec = parse(line).map_err(|e| CanonicalError(format!("{n}: {e}")))?;
        if rec.kind().channel() != header.channel {
            return err(format!(
                "{n}: a {} record in a channel-{} file",
                rec.kind().name(),
                header.channel.letter()
            ));
        }
        if rec.header.timeline_id != header.timeline_id {
            return err(format!(
                "{n}: timeline {} is not {}",
                rec.header.timeline_id, header.timeline_id
            ));
        }
        if let Some((m, prev)) = last {
            if rec.header.seq != prev.seq.wrapping_add(1) || prev.seq == u64::MAX {
                return err(format!(
                    "{n}: seq {} does not follow {} (line {m})",
                    rec.header.seq, prev.seq
                ));
            }
            if rec.header.tick < prev.tick {
                return err(format!(
                    "{n}: tick {} is before {} (line {m})",
                    rec.header.tick, prev.tick
                ));
            }
        }
        last = Some((n, rec.header));
        raw.push(*line);
        records.push(rec);
    }
    Ok(CanonicalFile {
        header,
        records,
        digest: digest(&raw),
    })
}
