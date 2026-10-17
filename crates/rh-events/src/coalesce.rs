//! One edge per cause (`envelope_v0.md` §5; R9, total by R21, the motive by REV-SPR-029 R20).
//!
//! A draft may carry several contributors to one cause; the merge coalesces them into one edge by
//! a fixed rule, so that two implementations agree:
//!
//! - **kind:** the first, in [`EdgeKind::PRECEDENCE`], among the contributors' kinds;
//! - **weight:** when the kept kind is `motivated_by`, the exact `i64` sum of the contributors'
//!   weights saturated once to `i16` (independent of their order); 0 otherwise;
//! - **motive:** when the kept kind is `motivated_by`, it takes the sign of the summed weight: the
//!   motivated contributor with the largest positive weight when the sum is positive, the one with
//!   the most negative weight when it is negative, none when it is 0. A tie goes to the lower
//!   `[motive_edges]` index, and an unnamed contributor counts as the highest index (so it yields
//!   none when chosen). None on every other kind;
//! - **evidence:** the largest among the contributors of the kept kind.
//!
//! Edges come out sorted by cause id. Every contributor is checked first by the rules
//! [`crate::EventLog::append`] applies to an edge, so a contributor that breaks one is refused
//! rather than absorbed.

use std::collections::BTreeMap;

use rh_ids::EventId;

use crate::envelope::{Edge, EdgeKind};
use crate::{EventError, check_edge};

/// Coalesces `edges` (of an event of `kind`) to one per cause, sorted by cause id.
///
/// `motive_index` gives a motive name's index in the Registry's `[motive_edges]` (ties go to the
/// lower index); a name it does not know ranks with the unnamed contributors, last.
///
/// # Errors
/// The [`EventError`] of a contributor that breaks an edge rule of [`crate::EventLog::append`]
/// other than the forward-cause and repeated-cause rules (the first needs the new id; the second
/// is what coalescing removes).
pub fn coalesce(
    kind: &'static str,
    edges: Vec<Edge>,
    motive_index: impl Fn(&str) -> Option<usize>,
) -> Result<Vec<Edge>, EventError> {
    for e in &edges {
        check_edge(kind, e)?;
    }
    let mut groups: BTreeMap<EventId, Vec<Edge>> = BTreeMap::new();
    for e in edges {
        groups.entry(e.cause).or_default().push(e);
    }
    Ok(groups
        .into_iter()
        .map(|(cause, group)| coalesce_one(cause, &group, &motive_index))
        .collect())
}

fn coalesce_one(
    cause: EventId,
    group: &[Edge],
    motive_index: &impl Fn(&str) -> Option<usize>,
) -> Edge {
    let kept = group
        .iter()
        .map(|e| e.kind)
        .min_by_key(|k| k.precedence())
        .unwrap_or(EdgeKind::Triggered);
    let evidence = group
        .iter()
        .filter(|e| e.kind == kept)
        .map(|e| e.evidence)
        .max()
        .unwrap_or(0);
    if kept != EdgeKind::MotivatedBy {
        return Edge {
            cause,
            kind: kept,
            motive: None,
            weight: 0,
            evidence,
        };
    }
    let sum: i64 = group.iter().map(|e| i64::from(e.weight)).sum();
    let weight = i16::try_from(sum).unwrap_or(if sum < 0 { i16::MIN } else { i16::MAX });
    let rank = |e: &Edge| e.motive.and_then(motive_index).unwrap_or(usize::MAX);
    let motivated = group.iter().filter(|e| e.kind == EdgeKind::MotivatedBy);
    // The chosen contributor: the strongest in the sum's direction, ties to the lower index.
    let chosen = match sum.signum() {
        1 => motivated
            .filter(|e| e.weight > 0)
            .min_by_key(|e| (core::cmp::Reverse(e.weight), rank(e))),
        -1 => motivated
            .filter(|e| e.weight < 0)
            .min_by_key(|e| (e.weight, rank(e))),
        _ => None,
    };
    Edge {
        cause,
        kind: kept,
        motive: chosen.and_then(|e| e.motive),
        weight,
        evidence,
    }
}
