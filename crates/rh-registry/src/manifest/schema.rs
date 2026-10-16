//! JSON Schema emitted from the manifest types, and its diff against the reference schema
//! `specs/manifest/manifest_v0.schema.json` (Pydantic's, SPR-004 T-04).
//!
//! The comparison is semantic. Before comparing, both schemas are normalised:
//! - annotations are dropped: `title` (Pydantic derives one from every name), `description`
//!   (doc comments vs docstrings), `format` (schemars adds `int64`/`double`), `$schema` and
//!   `$comment`;
//! - numbers compare by value (`150` equals `150.0`);
//! - `required` compares as a set.
//!
//! Everything else must be equal: definition names, properties, types, bounds, patterns,
//! lengths, `enum`/`const`, defaults, `additionalProperties`/`patternProperties`, nullability
//! and the discriminated unions. [`EXPLAINED`] lists the differences that remain by design; it is
//! empty, and any entry added to it must say why.

use serde_json::{Map, Value};

use super::types::Manifest;

/// Keys that are annotations, not constraints.
pub const ANNOTATIONS: [&str; 5] = ["title", "description", "format", "$schema", "$comment"];

/// Differences that remain by design, as `(path, reason)`. None today.
pub const EXPLAINED: &[(&str, &str)] = &[];

/// The manifest's JSON Schema, generated from the Rust types.
#[must_use]
pub fn manifest_schema() -> Value {
    schemars::schema_for!(Manifest).to_value()
}

/// Drops annotations, turns every number into a float and sorts `required`.
#[must_use]
pub fn normalize(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut out = Map::new();
            for (k, x) in m {
                if ANNOTATIONS.contains(&k.as_str()) {
                    continue;
                }
                let x = if k == "required" {
                    let mut items: Vec<Value> = x.as_array().cloned().unwrap_or_default();
                    items.sort_by_key(ToString::to_string);
                    Value::Array(items)
                } else {
                    normalize(x)
                };
                out.insert(k.clone(), x);
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        Value::Number(n) => n
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| v.clone(), Value::Number),
        other => other.clone(),
    }
}

/// Every place where two normalised schemas differ, as `path: reference vs generated`.
#[must_use]
pub fn differences(reference: &Value, generated: &Value) -> Vec<String> {
    let mut out = Vec::new();
    diff(&normalize(reference), &normalize(generated), "", &mut out);
    out.retain(|d| {
        !EXPLAINED
            .iter()
            .any(|(path, _)| d.starts_with(&format!("{path}:")))
    });
    out
}

fn short(v: Option<&Value>) -> String {
    let s = v.map_or_else(|| "(absent)".to_owned(), ToString::to_string);
    if s.chars().count() > 80 {
        format!("{}...", s.chars().take(77).collect::<String>())
    } else {
        s
    }
}

fn diff(reference: &Value, generated: &Value, path: &str, out: &mut Vec<String>) {
    match (reference, generated) {
        (Value::Object(left), Value::Object(right)) => {
            let mut keys: Vec<&String> = left.keys().chain(right.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match (left.get(k), right.get(k)) {
                    (Some(l), Some(r)) => diff(l, r, &p, out),
                    (l, r) => out.push(format!("{p}: {} vs {}", short(l), short(r))),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (i, (l, r)) in left.iter().zip(right).enumerate() {
                diff(l, r, &format!("{path}[{i}]"), out);
            }
        }
        _ if reference == generated => {}
        _ => out.push(format!(
            "{path}: {} vs {}",
            short(Some(reference)),
            short(Some(generated))
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../specs/manifest/manifest_v0.schema.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// T-04 acceptance: no unexplained difference from the reference schema.
    #[test]
    fn generated_schema_matches_the_reference() {
        let d = differences(&reference(), &manifest_schema());
        assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    }

    #[test]
    fn a_changed_bound_is_a_difference() {
        let mut r = reference();
        r["$defs"]["Horizon"]["properties"]["years"]["maximum"] = serde_json::json!(6000);
        let d = differences(&r, &manifest_schema());
        assert_eq!(
            d,
            vec!["$defs.Horizon.properties.years.maximum: 6000.0 vs 5000.0".to_owned()]
        );
    }

    #[test]
    fn annotations_and_number_forms_are_ignored() {
        let a = serde_json::json!({"title": "X", "type": "number", "default": 150, "required": ["b", "a"]});
        let b = serde_json::json!({"description": "y", "format": "double", "type": "number", "default": 150.0, "required": ["a", "b"]});
        assert!(differences(&a, &b).is_empty());
    }
}
