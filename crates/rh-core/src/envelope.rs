//! `StepCtx::emit`'s declaration check (SPR-020 T-03; `specs/chronicle/envelope_v0.md` §1, §2,
//! §4): each system's `[[emits]]` declarations, compiled at registration against the Registry it
//! runs under, and the check of one draft against its kind's declaration.
//!
//! - **Roles** (§1): every role row is one of the kind's roles; no role but `witness` appears
//!   twice, no `(role, entity)` repeats, and a cast holds at most [`rh_events::MAX_CAST`] rows. The
//!   cast is sorted to its canonical order, `(role, entity_kind, entity_id)`.
//! - **Snapshots** (§4): every snapshot is a declared one, of the cast member its declaration
//!   names (or of the event), with a value in its attribute's domain; rows are sorted to
//!   `(attr_id, entity, partner)` (the name order is the `attr_id` order) and none repeats. Every
//!   declared snapshot whose roles are filled and whose source is present is on the draft (R10). A
//!   snapshot whose source is a read of an owner that is not registered is discarded and counted
//!   (§4 rule 4): the neutral value of a read is never a snapshot.
//! - **Motives** (§2): every edge's motive is one of the kind's.
//!
//! The engine never fills a snapshot; the emitter sets the pre-act value it read (§4 rule 2).

use std::collections::{BTreeMap, BTreeSet};

use rh_events::{EventDraft, MAX_CAST, RoleId, Snapshot};
use rh_ids::EntityRef;

use crate::access::AccessSpec;
use crate::ownership::{AttributeDomain, EmitDecl, SpecTable};
use crate::system::SimError;

/// One kind's compiled declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindEnvelope {
    /// The role codes the kind may fill.
    roles: BTreeSet<u8>,
    /// The motive names its edges may carry.
    motives: BTreeSet<String>,
    /// Its declared snapshots.
    snapshots: Vec<CompiledSnapshot>,
}

/// One declared snapshot, compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledSnapshot {
    attr: String,
    /// The role code it is read of; `None`: the event itself.
    of: Option<u8>,
    partner: Option<u8>,
    /// The attribute's domain; `None` when the host passed no Registry.
    domain: Option<AttributeDomain>,
    /// The owner of its source when that owner is not registered: the snapshot is absent.
    absent_owner: Option<String>,
}

/// A system's compiled `[[emits]]` declarations, by kind.
pub(crate) type Envelope = BTreeMap<String, KindEnvelope>;

/// A snapshot discarded because its source's owner is not registered (§4 rule 4): counted per
/// `(system, snapshot, absent owner)`, where the snapshot is `KIND.attr(of)`.
pub type DroppedKey = (&'static str, String, String);

/// Compiles the declarations of `system`'s spec against the Registry table; `registered` is the
/// set of registered system ids. Role names the Registry lacks are skipped here (`lint_odd.py`
/// refuses them in the spec).
pub(crate) fn compile(
    system: &str,
    access: &AccessSpec,
    specs: &SpecTable,
    registered: &BTreeSet<&str>,
) -> Envelope {
    let Some(entry) = specs.specs.get(system) else {
        return Envelope::new();
    };
    entry
        .envelope
        .iter()
        .map(|(kind, decl)| (kind.clone(), compile_kind(decl, access, specs, registered)))
        .collect()
}

fn compile_kind(
    decl: &EmitDecl,
    access: &AccessSpec,
    specs: &SpecTable,
    registered: &BTreeSet<&str>,
) -> KindEnvelope {
    let code = |name: &str| specs.registry.role_code(name);
    let snapshots = decl
        .snapshots
        .iter()
        .map(|s| {
            // A snapshot read from a foreign column whose declared owner is not registered: the
            // read is dropped, so the snapshot is absent (§4 rule 4; `05.1` §2 rule 2).
            let absent_owner = access
                .read_of(&s.from)
                .map(|r| r.owner)
                .filter(|o| !registered.contains(o))
                .map(str::to_owned);
            CompiledSnapshot {
                attr: s.attr.clone(),
                of: (s.of != "event").then(|| code(&s.of)).flatten(),
                partner: s.partner.as_deref().and_then(code),
                domain: specs.registry.attributes.get(&s.attr).cloned(),
                absent_owner,
            }
        })
        .collect();
    KindEnvelope {
        roles: decl.roles.iter().filter_map(|r| code(r)).collect(),
        motives: decl.motives.iter().cloned().collect(),
        snapshots,
    }
}

/// The entity playing `role` in the (sorted) cast, if the role is filled once.
fn cast_member(draft: &EventDraft, role: u8) -> Option<EntityRef> {
    draft
        .roles
        .iter()
        .find(|r| r.role.0 == role)
        .map(|r| r.entity)
}

/// Checks `draft` (of a declared kind) against `kind`, sorting its cast and snapshots in place
/// and discarding the snapshots of absent sources, which it returns for the dropped counter.
///
/// # Errors
/// The [`SimError`] of the first rule the draft breaks (module docs).
pub(crate) fn check(
    system: &'static str,
    kind: &KindEnvelope,
    draft: &mut EventDraft,
) -> Result<Vec<(String, String)>, SimError> {
    let k = draft.kind;
    check_cast(system, kind, draft)?;
    let mut dropped = Vec::new();
    let mut kept: Vec<Snapshot> = Vec::with_capacity(draft.snapshots.len());
    for snap in std::mem::take(&mut draft.snapshots) {
        let decl = declared(system, kind, draft, &snap)?;
        if let Some(owner) = &decl.absent_owner {
            dropped.push((format!("{k}.{}", snap.attr), owner.clone()));
            continue;
        }
        if decl
            .domain
            .as_ref()
            .is_some_and(|d| !d.contains(snap.value))
        {
            return Err(SimError::SnapshotOutOfDomain {
                system,
                kind: k,
                attr: snap.attr,
                value: snap.value,
            });
        }
        kept.push(snap);
    }
    kept.sort_unstable_by(|a, b| {
        (a.attr.as_bytes(), a.entity, a.partner).cmp(&(b.attr.as_bytes(), b.entity, b.partner))
    });
    if let Some(w) = kept
        .windows(2)
        .find(|w| (w[0].attr, w[0].entity, w[0].partner) == (w[1].attr, w[1].entity, w[1].partner))
    {
        return Err(SimError::DuplicateSnapshot {
            system,
            kind: k,
            attr: w[1].attr,
        });
    }
    draft.snapshots = kept;
    check_missing(system, kind, draft)?;
    if let Some(m) = draft
        .causes
        .iter()
        .filter_map(|c| c.motive)
        .find(|m| !kind.motives.contains(*m))
    {
        return Err(SimError::UndeclaredMotive {
            system,
            kind: k,
            motive: m,
        });
    }
    Ok(dropped)
}

/// The cast rules of §1: at most 256 rows, every role declared, sorted, no repeat.
fn check_cast(
    system: &'static str,
    kind: &KindEnvelope,
    draft: &mut EventDraft,
) -> Result<(), SimError> {
    let k = draft.kind;
    if draft.roles.len() > MAX_CAST {
        return Err(SimError::CastTooLarge {
            system,
            kind: k,
            rows: draft.roles.len(),
        });
    }
    if let Some(r) = draft.roles.iter().find(|r| !kind.roles.contains(&r.role.0)) {
        return Err(SimError::UndeclaredRole {
            system,
            kind: k,
            role: r.role.0,
        });
    }
    draft.roles.sort_unstable();
    if let Some(w) = draft
        .roles
        .windows(2)
        .find(|w| w[0] == w[1] || (w[0].role == w[1].role && w[0].role != RoleId::WITNESS))
    {
        return Err(SimError::DuplicateRole {
            system,
            kind: k,
            role: w[1].role.0,
            entity: w[1].entity,
        });
    }
    Ok(())
}

/// R10: every declared snapshot whose roles are filled and whose source is present is on the
/// draft.
fn check_missing(
    system: &'static str,
    kind: &KindEnvelope,
    draft: &EventDraft,
) -> Result<(), SimError> {
    let k = draft.kind;
    for decl in kind.snapshots.iter().filter(|d| d.absent_owner.is_none()) {
        let Some(entity) = (match decl.of {
            None => Some(EntityRef::WORLD),
            Some(of) => cast_member(draft, of),
        }) else {
            continue;
        };
        let partner = match decl.partner {
            None => None,
            Some(p) => match cast_member(draft, p) {
                Some(e) => Some(e),
                None => continue,
            },
        };
        let present = draft
            .snapshots
            .iter()
            .any(|s| s.attr == decl.attr && s.entity == entity && s.partner == partner);
        if !present {
            return Err(SimError::MissingSnapshot {
                system,
                kind: k,
                attr: decl.attr.clone(),
            });
        }
    }
    Ok(())
}

/// The declaration a draft snapshot answers to: one of the kind's snapshots of its attribute,
/// whose `of` (and `partner`) roles the snapshot's entities play in the cast.
fn declared<'k>(
    system: &'static str,
    kind: &'k KindEnvelope,
    draft: &EventDraft,
    snap: &Snapshot,
) -> Result<&'k CompiledSnapshot, SimError> {
    let mut of_attr = kind
        .snapshots
        .iter()
        .filter(|d| d.attr == snap.attr)
        .peekable();
    if of_attr.peek().is_none() {
        return Err(SimError::UndeclaredSnapshot {
            system,
            kind: draft.kind,
            attr: snap.attr,
        });
    }
    let plays = |role: Option<u8>, entity: EntityRef| match role {
        None => entity == EntityRef::WORLD,
        Some(r) => draft
            .roles
            .iter()
            .any(|row| row.role.0 == r && row.entity == entity),
    };
    of_attr
        .find(|d| {
            plays(d.of, snap.entity)
                && match (d.partner, snap.partner) {
                    (None, None) => true,
                    (Some(p), Some(e)) => plays(Some(p), e),
                    _ => false,
                }
        })
        .ok_or(SimError::SnapshotNotOfTheCast {
            system,
            kind: draft.kind,
            attr: snap.attr,
            entity: snap.entity,
        })
}
