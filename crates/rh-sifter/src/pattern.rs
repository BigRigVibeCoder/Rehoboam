//! Pattern packs, DSL v0, as `rh-sifter` v0 compiles them (`05.4` §2; SPR-011 ruling 14).
//!
//! v0 matches **single-clause** patterns only: one `[[pattern.clause]]` naming a `verb` or an
//! `act` class (or several), and the Registry roles (`actor`, `target`, `witness`, `instrument`;
//! `initiator` is `actor`) that the pattern's role variables fill. The declared semantics
//! (`selection`, `on_complete`) are carried into every match; with one clause no partial match is
//! ever open, so they change nothing yet. Everything v0 cannot honour is refused at load, never
//! ignored: a second clause, `where` reads, `require_edge`, `unless`, `emit`, `dedupe`,
//! `death_tolerant`, `completion_hazard`. The lint (`specs/sifting/lint_patterns.py`) checks the
//! DSL itself (anchoring, budgets) before a pack is enabled (V-SIFT-05).
//!
//! A pack is identified by its file's digest (`pattern.pack_digest`: XXH3-128 of the bytes,
//! `channel_c_v0.sql` R8), never by a version number.
//!
//! REF: `docs/05_architecture/05.4_narrative_intelligence.md` §2.2–§2.3; `specs/stream/channel_c_v0.sql`
//! R6–R8; `specs/sifting/patterns.solitary_life.toml`.

use std::collections::BTreeMap;

use rh_snapshot::hash::xxh3_128;
use serde::Deserialize;

use crate::SiftError;

/// `selection` codes (`pattern.selection`).
pub const SELECTIONS: [&str; 3] = ["latest", "earliest", "all"];
/// `on_complete` codes (`pattern.on_complete`).
pub const ON_COMPLETE: [&str; 3] = ["clear_binding", "consume", "keep"];

/// One compiled pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    /// The pack (Registry `[pattern_packs]` id).
    pub pack: String,
    /// The pattern's `id`.
    pub id: String,
    /// The pattern's `version`.
    pub version: u16,
    /// `selection` code.
    pub selection: u8,
    /// `on_complete` code.
    pub on_complete: u8,
    /// The role variables, bytewise order.
    pub roles: Vec<String>,
    /// The clause's `bind` (`e1`).
    pub bind: String,
    /// The verbs the clause matches (empty: any verb of the act classes).
    pub verbs: Vec<String>,
    /// The act classes the clause matches (empty: any act class of the verbs).
    pub acts: Vec<String>,
    /// The clause's role constraints: (Registry role code, role variable), by code.
    pub clause_roles: Vec<(u8, String)>,
    /// `firing_budget.per_century_per_100k` as the exact rational `num / den`.
    pub budget: (u64, u64),
    /// The XXH3-128 of the pack file.
    pub pack_digest: [u8; 16],
}

/// A Registry role name to its `[event_roles]` index (`initiator` is the actor).
fn role_code(name: &str) -> Option<u8> {
    match name {
        "actor" | "initiator" => Some(0),
        "target" => Some(1),
        "witness" => Some(2),
        "instrument" => Some(3),
        _ => None,
    }
}

#[derive(Deserialize)]
struct PackFile {
    pattern: Vec<toml::Table>,
}

fn refuse(pack: &str, id: &str, what: &str) -> SiftError {
    SiftError::Pattern(format!(
        "{pack}/{id}: {what} (rh-sifter v0, SPR-011 ruling 14)"
    ))
}

/// The exact rational of a TOML number: an integer, or a float's shortest decimal spelling.
fn rational(v: &toml::Value) -> Option<(u64, u64)> {
    match v {
        toml::Value::Integer(i) => u64::try_from(*i).ok().map(|n| (n, 1)),
        toml::Value::Float(_) => {
            let text = v.to_string();
            let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
            if !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
                return None;
            }
            let den = 10u64.checked_pow(u32::try_from(frac.len()).ok()?)?;
            let num: u64 = format!("{int}{frac}").parse().ok()?;
            let g = gcd(num, den);
            Some((num / g, den / g))
        }
        _ => None,
    }
}

const fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    if a == 0 { 1 } else { a }
}

fn strings(v: Option<&toml::Value>) -> Option<Vec<String>> {
    match v {
        None => Some(Vec::new()),
        Some(toml::Value::String(s)) => Some(vec![s.clone()]),
        Some(toml::Value::Array(a)) => a.iter().map(|x| x.as_str().map(str::to_owned)).collect(),
        Some(_) => None,
    }
}

/// Compiles the patterns of pack `pack` from its file's `text`.
///
/// # Errors
/// [`SiftError::Pattern`] for a file that is not a pack, a field v0 does not honour, or a value
/// outside its domain.
#[allow(
    clippy::too_many_lines,
    reason = "one pattern's fields in DSL order, each refused where v0 cannot honour it"
)]
pub fn compile_pack(pack: &str, text: &str) -> Result<Vec<Pattern>, SiftError> {
    let file: PackFile =
        toml::from_str(text).map_err(|e| SiftError::Pattern(format!("pack {pack}: {e}")))?;
    let digest = xxh3_128(text.as_bytes());
    let mut out = Vec::new();
    for p in &file.pattern {
        let id = p
            .get("id")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| SiftError::Pattern(format!("pack {pack}: a pattern without an id")))?
            .to_owned();
        for unsupported in [
            "unless",
            "emit",
            "dedupe",
            "death_tolerant",
            "completion_hazard",
            "threatened_role",
            "surface_from",
        ] {
            if p.contains_key(unsupported) {
                return Err(refuse(
                    pack,
                    &id,
                    &format!("`{unsupported}` is not supported"),
                ));
            }
        }
        let version = p
            .get("version")
            .and_then(toml::Value::as_integer)
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| refuse(pack, &id, "`version` is not a u16"))?;
        let code_of = |key: &str, table: &[&str], default: u8| -> Result<u8, SiftError> {
            match p.get(key) {
                None => Ok(default),
                Some(v) => v
                    .as_str()
                    .and_then(|s| table.iter().position(|x| *x == s))
                    .and_then(|i| u8::try_from(i).ok())
                    .ok_or_else(|| {
                        refuse(pack, &id, &format!("`{key}` = {v} is not one of {table:?}"))
                    }),
            }
        };
        let selection = code_of("selection", &SELECTIONS, 0)?;
        let on_complete = code_of("on_complete", &ON_COMPLETE, 0)?;
        let mut roles: Vec<String> = p
            .get("roles")
            .and_then(toml::Value::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default();
        roles.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        let budget = p
            .get("firing_budget")
            .and_then(|b| b.get("per_century_per_100k"))
            .and_then(rational)
            .ok_or_else(|| {
                refuse(
                    pack,
                    &id,
                    "`firing_budget.per_century_per_100k` is required",
                )
            })?;
        let clauses = p
            .get("clause")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| refuse(pack, &id, "a pattern needs one clause"))?;
        let [clause] = clauses.as_slice() else {
            return Err(refuse(pack, &id, "v0 matches single-clause patterns only"));
        };
        let clause = clause
            .as_table()
            .ok_or_else(|| refuse(pack, &id, "a clause is a table"))?;
        let mut clause_roles = Vec::new();
        let mut keys = BTreeMap::new();
        for (key, value) in clause {
            match key.as_str() {
                "bind" | "verb" | "act" => {}
                k => {
                    let code = role_code(k).ok_or_else(|| {
                        refuse(pack, &id, &format!("clause field `{k}` is not supported"))
                    })?;
                    let var = value
                        .as_str()
                        .filter(|v| roles.iter().any(|r| r == v))
                        .ok_or_else(|| {
                            refuse(pack, &id, &format!("`{k}` names no role variable"))
                        })?;
                    if keys.insert(code, var.to_owned()).is_some() {
                        return Err(refuse(pack, &id, &format!("role `{k}` given twice")));
                    }
                }
            }
        }
        clause_roles.extend(keys);
        let bind = clause
            .get("bind")
            .and_then(toml::Value::as_str)
            .filter(|b| {
                b.len() > 1 && b.starts_with('e') && b[1..].bytes().all(|c| c.is_ascii_digit())
            })
            .ok_or_else(|| refuse(pack, &id, "the clause's `bind` is not e<N>"))?
            .to_owned();
        let verbs = strings(clause.get("verb"))
            .ok_or_else(|| refuse(pack, &id, "`verb` is a string or a list"))?;
        let acts = strings(clause.get("act"))
            .ok_or_else(|| refuse(pack, &id, "`act` is a string or a list"))?;
        if verbs.is_empty() && acts.is_empty() {
            return Err(refuse(
                pack,
                &id,
                "the clause names no verb and no act class",
            ));
        }
        out.push(Pattern {
            pack: pack.to_owned(),
            id: id.clone(),
            version,
            selection,
            on_complete,
            roles,
            bind,
            verbs,
            acts,
            clause_roles,
            budget,
            pack_digest: digest,
        });
    }
    Ok(out)
}
