"""ODD-R v2 linter, brief generator and code-skeleton generator (ADR-0033).

System specs are data (`specs/systems/*.odd.toml`). This tool checks every spec on its own and against
  - the Registry (system exists, fidelity levels, parameter set equal both ways, era-state variables),
  - every other spec (single writer per column; read set resolves to an owner that declares the column),
  - the plan (requirement IDs exist in docs/03, test IDs in docs/09, phases in 05.1 §4.2, ADR files exist),
  - the plane rules (a plane-A system never reads plane-B or plane-C columns),
  - the coupling rules (RT-2026-10-16): a read that enters a rate needs a bounded, sourced in-coupling with a
    neutral value, and every rate or mode read needs a synthetic forcing in [isolation] (docs/09 V-SF-22/23).
    Missing coupling detail is a warning on a draft spec and an error on a frozen one,
  - the event vocabulary (SPR-011 ruling 11): every [[emits]] kind is a Registry verb whose emitted_by is the
    spec's system, and every verb the Registry gives to a system with a spec is in that spec's [[emits]];
  - the event envelope (SPR-020; specs/chronicle/envelope_v0.md §2, §4, §7): an [[emits]] entry's `roles` are Registry
    [event_roles] ids, each once (witness may repeat on an event, not in the list); its `motives` are [motive_edges] ids;
    each of its `snapshots` names a Registry attribute, the role (`of`) it is read of (or "event"), a `partner` role for
    a two-argument attribute, and the owned or read column (`from`) it is filled from, whose type matches the
    attribute's; a `cause_column` on an [[owns]] state column names an owned `event_ref` column of the same entity
    (SAGA-15; an Option<EventId>, none by default, so no id is reserved: REV-SPR-020 R17); an event_ref column
    declares no `default` (R26).
    A missing `roles` is an error on a frozen spec and a warning on a draft; a wrong value is always an error
    (REV-SPR-020 R7). `of = "witness"` is refused (a witness is many); a repeated motive or snapshot key is an error;
    an `algorithm.<name>` source declares its `type`, checked like a column's; a foreign enum source's codes are
    compared with the attribute's values. A `cause_column` needs the state column's declared `default`.
    A warning on a draft spec, an error on a frozen one,
  - parameter defaults (SPR-011 ruling 8): a [[params]] `default` equals the Registry's `defaults` entry, both ways,
  - lifecycle commands (PR #32; REV-P1a §7): each [[applies]] of a spec names a command, its payload fields and its
    senders (`from`); every sender's [[commands]] entry for it carries a `payload` with the same field names in the same
    order, `from` lists exactly the specs that send it, and a command sent to a spec that declares [[applies]] must be
    one of them. A warning on a draft spec, an error on a frozen one,
  - absent owners (EVO-001; REV-P1a NEW-4; REV-SPR-017 round 3; 05.1 §2 rule 2): an identity read of another system's
    column always requires its owner (registration refuses a world without it, by name) and declares no `neutral`. A
    `rate` or `mode` read of another system's column carries a machine-readable `neutral`, the value a keyed lookup
    returns when the owner is absent and the read is dropped (the type's zero is never a fallback). A neutral is typed
    by the column's type, from the owner's [[owns]] `type` or the Registry [era_state] entry for an owner with no spec:
    a TOML integer in the type's range, true/false, a member of the enum's codes, or a number representable in
    q8_8/q16_16 (an exact decimal that is a multiple of the step) and inside its range. A rate read's neutral is also
    named in its in-coupling's `neutral` prose as `<column> = <value>`. A missing or untypeable neutral is a warning on
    a draft spec and an error on a frozen one; a mistyped neutral is always an error (a deliberate departure from
    warning-on-draft: a wrong value is never right). The lint prints, per spec, the owners its identity reads require
    (NOTE lines), and warns, as advice only, when a substrate (`[purpose] role = "substrate"`) reads as identity a
    column of a system that is not a substrate, since that makes it require a system a smaller world may lack
    (registration enforces the rule itself: a world lacking a required owner is refused by name; R12, R16),
  - enum codes (EVO-001): an enum column of [[owns]] carries its codes, `values = [...]` (code = position) or
    `registry = "<table>"` (the Registry table's `ids` list, else its keys in byte-wise ascending order, as verb_id),
  - lifecycle commands, typed (EVO-001): each [[applies]] entry (one per command kind) has a payload `{ name: Type, ... }`
    in a closed vocabulary (PersonId, EventRef, SimTime, the id types, u8..u64, i64, bool, q8_8, q16_16,
    enum(<entity.column>) naming an enum column with codes; a field named *_event is an EventRef; no name repeats);
    `drained_by = "<Pulse>/<Phase>"` names the one binding of [[schedule]] that takes the command (an escape: with
    `sprint = "SPR-NNN"` naming a sprint that exists (a document under CODEX/05_PROJECT, or a row of a PRJ plan
    until the document exists; any other id is an error, R18) an undeclared binding is a counted warning until that
    sprint's document is CLOSED, then an error on a frozen spec); `key` is "living" (the dead-person drop applies,
    05.1 §2 rule 2) or "any", stated; `key_field` names the PersonId payload field the command is keyed by, or
    "person" for Command::person itself when the payload does not repeat it (R15); `outcome = true` marks a command
    that carries a sampled, dated outcome, and every sender's [[commands]] entry for it then names the sender's own
    `timer` kind and the [[rng]] `purpose` it is re-sampled under (05.1 §2 rule 2: the sender's timer submits it),
    and the sender binds the draining binding's pulse and phase, where its timer fires (a sender naming a timer
    for a command that carries no outcome is warned),
  - observed event kinds (EVO-002; 05.1 §2 rule 2, the instant log's part (c)): each [[observes]] entry names a known
    kind (a Registry verb: the instant log records known kinds only), its emitter (`owner`, the verb's `emitted_by`,
    which declares it in [[emits]] and is not the declaring system) and `why`; the emitter is one of the declaring
    system's reads (a `same_instant` [[reads]] entry of one of its columns, so the scheduling edge orders the
    emitter's bindings before the observer's in every phase both bind, and the identity rule requires the emitter in
    every world that registers the observer); the observer binds a phase at or after one the emitter binds (the log
    is cleared at the instant's end, so an event merged in a later phase is never read); a kind is observed once and
    no other field is allowed. Every rule is an error on every status (the schema is new: no draft predates it),
and it generates what people and code need from the same source:
  --brief <id>   one-page summary for the director
  --rust  <id>   a Rust skeleton (system id, schedule, RNG purposes, parameter names) so code cannot drift

Run: python -I specs/systems/lint_odd.py            (lints everything; exit 1 on any error)
     python -I specs/systems/lint_odd.py --brief S5.daily_life
     python -I specs/systems/lint_odd.py --extra-specs DIR --overlay FILE
        DIR: more *.odd.toml specs, linted with the repository's (an id may not repeat), e.g. a world's own
        systems (SPR-008: worlds/smoke/systems); FILE: a Registry overlay whose [systems] and [verbs] entries are
        added to the Registry's (a name may not repeat), e.g. worlds/smoke/registry_overlay.toml

A spec set outside the Registry (SPR-008 ruling 2: the smoke world's dummy systems never enter the real
Registry) is linted, and its skeletons generated, with
     python -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml
     python -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml --rust X1.smoke_weather
--specs DIR replaces the repository's specs (where --extra-specs adds to them). The overlay's [systems] and
[verbs] are added to the Registry's for this run only; with --specs, each overlay system id is X-prefixed
(`X<digits>.<name>`) and every spec in DIR must be one of the overlay's systems, so a dummy spec can never
stand in for a real one.

The next version of a frozen spec (SPR-015; specs/systems/next/README.md) is linted with
     python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml
--next DIR: every spec in DIR supersedes the repository spec of the same id for this run (an id with no spec to
supersede is an error), and `lint_next` checks that the superseding spec preserves the frozen slice: the same id
and crate, a higher spec_version, status draft; every frozen [[owns]] column with the same entity, type, plane and
fixed_point; every frozen [algorithm.*] table equal; the frozen [[rng]] purposes as a prefix in order with the same
key, resample_on_fork and draws; every frozen [[invariants]] id with the same statement and test; every frozen
[[applies]] command with the same payload; every frozen schedule binding, [[emits]] kind and parameter, frozen
defaults unchanged; `from` on a frozen [[applies]] may only grow; a frozen [[emits]] `when`, the frozen
[initialization] state and the frozen schedule steps are kept inside the new text. A next version may change a
frozen statement only through a `[[amends]]` record (REV-SPR-015 R1): `of` names the path (invariants.<id>.statement,
algorithm.<name>.steps.<Sn>, emits.<KIND>.when, initialization.state, schedule.<Pulse>/<Phase>.steps.<name>),
`frozen` quotes the frozen text byte for byte, `to` is the new text and `reason` says why; the lint checks both texts
and prints every amendment (AMEND lines). In --next mode the superseding spec is held to frozen-level strictness on
[[applies]] and [[emits]] (REV-SPR-015 R6). Frozen texts are compared for equality, never as substrings: a frozen
`when`, `[initialization]` text, schedule step, binding `when`, column `meaning` and purpose `use` are kept or amended
(amendable paths: schedule.<Pulse>/<Phase>.when, owns.<entity.column>.meaning, rng.<purpose>.use), an amended algorithm
step bumps that table's `version`, and every spec, in both modes, names each owned column, parameter, purpose,
invariant, emitted kind, applied command, binding and step once (REV-SPR-015 R15, R17). With --next, an overlay may
replace the Registry [systems] entry of a superseded system (never another), keeping fidelity_levels and every existing
parameter's bounds and default. Deny by default (PR #79's review R3, 2026-10-17): every other field of the
frozen slice is compared too, as a generic path, and changes only through a record: submodels.<name>.<field>,
commands.<command>.<field>, couplings.<system>/<direction>.<field>, params.<name>.<field> (source and the rest),
fidelity_ladder.<level>.<field>, purpose.<field> and purpose.patterns.<test>.<field>, isolation.<field> and
isolation.forcing.<input>.<field>, design.<field>, scales.<field>, verification.prototype, system.name, system.adrs
and system.requirements, and (N4) every other leaf: reads.<column>.<field>, applies.<command>.<field>, emits.<kind>.<field>,
owns.<column>.<field>, initialization.<field>, input_data.<field>; a list-valued field is quoted as JSON. Entries may be
added (a read, an emitted kind, a submodel, a command, a coupling, a pattern, a forcing input); a frozen entry's fields
may not change; an enum column's values may only be appended. The exemptions (the versions, the status, the fidelity,
the lists that may only grow) are listed in specs/systems/next/README.md.
"""
import copy
import hashlib
import json
import re
import sys
import tomllib
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPECS = Path(__file__).resolve().parent
REGISTRY = ROOT / "specs" / "manifest" / "registry_v0.toml"
PHASES = ["Inputs", "Environment", "Production", "Exchange", "Vital", "Activity", "Contagion", "Polity",
          "Military", "Agenda", "Culture", "Bookkeeping"]                  # 05.1 §4.2 (5a Activity, ADR-0031)
PULSES = ["Year", "Season", "Month", "Day", "Hour"]
REQUIRED = ["system", "purpose", "scales", "schedule", "rng", "design", "initialization", "input_data",
            "params", "submodels", "invariants", "verification"]
DESIGN_KEYS = ["basic_principles", "emergence", "adaptation", "objectives", "learning", "prediction", "sensing",
               "interaction", "stochasticity", "collectives", "observation", "era_state", "scale_behaviour"]
TYPES = {"i64", "u64", "i32", "u32", "u16", "u8", "bool", "enum", "q8_8", "q16_16", "f32_field", "event_ref"}
# event_ref: an Option<EventId>, none by default; the type of a cause column (envelope_v0.md §7, REV-SPR-020 R17). Its
# hashed and keyframe encoding is opt(u64), so a present cause may be EventId 0 (the first event) and none is distinct.
SNAKE = re.compile(r"^[a-z][a-z0-9_]*$")
READ_USES = ("identity", "mode", "rate")
IN_COUPLING_FIELDS = ("form", "neutral", "output_bounds", "data_range", "beyond_range", "source")
# A snapshot's source column type per Registry attribute type (envelope_v0.md §4 item 5).
SNAPSHOT_SOURCE_TYPES = {"q8_8": {"q8_8"}, "bool": {"bool"}, "int": {"u8", "u16", "u32", "i32", "i64"}, "enum": {"enum"}}
SNAPSHOT_KEYS = {"attr", "of", "partner", "from", "type"}
INTEGER_TYPES = {"i64", "u64", "i32", "u32", "u16", "u8"}
NUMBER_TYPES = {"q8_8", "q16_16", "f32_field"}
INT_RANGE = {"u8": (0, 2**8 - 1), "u16": (0, 2**16 - 1), "u32": (0, 2**32 - 1), "u64": (0, 2**64 - 1),
             "i32": (-2**31, 2**31 - 1), "i64": (-2**63, 2**63 - 1)}
FIXED_RANGE = {"q8_8": (Decimal(-128), Decimal(128)), "q16_16": (Decimal(-32768), Decimal(32768))}   # [lo, hi)
REGISTRY_ID = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")
PAYLOAD_TYPES = {"PersonId", "EventRef", "SimTime", "TitleId", "OfficeId", "ClaimId", "HouseId", "VowId", "ArtifactId",
                 "HouseholdId", "PlaceId", "u8", "u16", "u32", "u64", "i64", "bool", "q8_8", "q16_16"}
PAYLOAD_ENUM = re.compile(r"^enum\(([a-z][a-z0-9_]*\.[a-z][a-z0-9_]*)\)$")
PAYLOAD_FIELD = re.compile(r"^([a-z][a-z0-9_]*)\s*:\s*(\S+)$")
BINDING = re.compile(r"^(" + "|".join(PULSES) + r")/(" + "|".join(PHASES) + r")$")
KEY_RULES = ("living", "any")
SPRINT_ID = re.compile(r"^SPR-\d{3}$")
TIMER_KIND = re.compile(r"^[A-Z][A-Z0-9_]*$")
FIXED_STEP = {"q8_8": 256, "q16_16": 65536}
SERIES = ("constant", "step", "sine", "ramp", "historical")


OVERLAY_ID = re.compile(r"^X\d+\.[a-z][a-z0-9_]*$")


def replacement_problems(overlay_path, specs):
    """With --specs (a spec set outside the Registry, SPR-008 ruling 2): every overlay system id is
    X-prefixed, and every spec is one of the overlay's systems. ``load`` has already refused an overlay
    entry that repeats a Registry name."""
    overlay = tomllib.loads(Path(overlay_path).read_text(encoding="utf-8"), parse_float=Decimal)
    ids = set(overlay.get("systems", {}))
    problems = [f"{overlay_path}: overlay system '{sid}' must be X-prefixed (X<digits>.<snake_name>)"
                for sid in sorted(ids) if not OVERLAY_ID.match(sid)]
    problems += [f"{f.name}: system '{sid}' is not in the overlay {overlay_path}; a spec linted with an "
                 "overlay must be one of its systems" for sid, (f, _) in specs.items() if sid not in ids]
    return problems


def spec_files(d):
    """The spec files of directory ``d`` (``_template`` and other ``_``-prefixed files are not specs)."""
    return sorted(p for p in Path(d).glob("*.odd.toml") if not p.name.startswith("_"))


def load(extra_dirs=(), overlay=None, specs_dir=SPECS, next_dir=None):
    """Return (registry, specs, requirement ids, test ids, ADR ids, frozen), where ``frozen`` maps each id that a
    ``--next`` spec superseded to the repository spec it replaced (empty without ``--next``)."""
    reg = tomllib.loads(REGISTRY.read_text(encoding="utf-8"), parse_float=Decimal)
    specs = {}
    files = spec_files(specs_dir)
    for d in extra_dirs:
        files += spec_files(d)
    for f in files:
        s = tomllib.loads(f.read_text(encoding="utf-8"), parse_float=Decimal)   # exact decimals (SPR-011 ruling 9)
        sid = s.get("system", {}).get("id", f.stem)
        if sid in specs:
            raise SystemExit(f"{f}: system {sid} already has a spec ({specs[sid][0]})")
        specs[sid] = (f, s)
    frozen = {}
    if next_dir is not None:                      # next versions supersede for this run (SPR-015)
        for f in spec_files(next_dir):
            s = tomllib.loads(f.read_text(encoding="utf-8"), parse_float=Decimal)   # exact decimals (SPR-011 ruling 9)
            sid = s.get("system", {}).get("id", f.stem)
            if sid not in specs:
                raise SystemExit(f"{f}: a next-version spec must supersede a spec in {specs_dir}; {sid} has none")
            if sid in frozen:
                raise SystemExit(f"{f}: system {sid} already has a next-version spec ({frozen[sid][0]})")
            frozen[sid] = specs[sid]
            specs[sid] = (f, s)
    if overlay is not None:                       # a world's own systems and verbs (SPR-011 ruling 11)
        over = tomllib.loads(Path(overlay).read_text(encoding="utf-8"), parse_float=Decimal)
        unknown = sorted(set(over) - {"systems", "verbs"})
        if unknown:
            raise SystemExit(f"overlay {overlay}: only [systems] and [verbs] may be overlaid, found {unknown}")
        for sec in ("systems", "verbs"):
            clash = sorted(set(over.get(sec, {})) & set(reg.get(sec, {})))
            # With --next, the [systems] entry of a superseded system may be replaced (its F1 defaults), never another's.
            replaced = [c for c in clash if sec == "systems" and c in frozen]
            if sorted(set(clash) - set(replaced)):
                raise SystemExit(f"overlay {overlay}: [{sec}] entries already in the Registry: {sorted(set(clash) - set(replaced))}")
            for sid in replaced:
                for problem in replacement_keeps(sid, reg["systems"][sid], over["systems"][sid]):
                    raise SystemExit(f"overlay {overlay}: [systems.\"{sid}\"] {problem}")
            reg.setdefault(sec, {}).update(over.get(sec, {}))
    plan03 = (ROOT / "docs" / "03_requirements.md").read_text(encoding="utf-8")
    plan09 = (ROOT / "docs" / "09_verification_and_validation.md").read_text(encoding="utf-8")
    req_ids = set(re.findall(r"^\| ([A-Z][A-Z0-9]*-\d+) \|", plan03, re.M))
    test_ids = set(re.findall(r"^\| (V-[A-Z0-9]+-\d+[a-z]?) \|", plan09, re.M))
    adr_ids = {p.name[:8] for p in (ROOT / "docs" / "06_adr").glob("ADR-*.md")}
    return reg, specs, req_ids, test_ids, adr_ids, frozen


def replacement_keeps(sid, old, new):
    """Problems with a --next overlay's replacement of a Registry [systems] entry: fidelity_levels, every existing
    parameter's bounds and every existing default must be kept (a replacement adds, it never moves a contract)."""
    problems = []
    if new.get("fidelity_levels") != old.get("fidelity_levels"):
        problems.append(f"replacement changes fidelity_levels {old.get('fidelity_levels')} -> {new.get('fidelity_levels')}")
    for name, bounds in old.get("params", {}).items():
        if new.get("params", {}).get(name) != bounds:
            problems.append(f"replacement changes or drops parameter {name} bounds {bounds} -> {new.get('params', {}).get(name)}")
    for name, value in old.get("defaults", {}).items():
        if new.get("defaults", {}).get(name) != value:
            problems.append(f"replacement changes or drops default {name} = {value} -> {new.get('defaults', {}).get(name)}")
    return problems


def semver(text):
    """The (major, minor, patch) of ``text``, or None."""
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", str(text))
    return tuple(int(x) for x in m.groups()) if m else None


AMEND_PATH = re.compile(r"^(params\.(?P<param>[a-z0-9_]+)\.meaning|invariants\.(?P<inv>[A-Za-z0-9_-]+)\.statement|algorithm\.(?P<alg>[a-z0-9_]+)\.steps\.(?P<step>[A-Za-z0-9]+)|"
                        r"emits\.(?P<kind>[A-Z_]+)\.when|initialization\.state|schedule\.(?P<pulse>[A-Za-z]+)/(?P<phase>[A-Za-z]+)\.steps\.(?P<sname>[a-z_]+)|"
                        r"schedule\.(?P<wpulse>[A-Za-z]+)/(?P<wphase>[A-Za-z]+)\.when|owns\.(?P<col>[a-z_]+\.[a-z_]+)\.meaning|rng\.(?P<rpurpose>[a-z_]+)\.use|"
                        r"emits\.(?P<skind>[A-Z_]+)\.snapshots|owns\.(?P<dcol>[a-z_]+\.[a-z_]+)\.default|owns\.(?P<ccol>[a-z_]+\.[a-z_]+)\.cause_column)$")
# The last three are the envelope's (SPR-020; REV-SPR-020 R15): a next version keeps a frozen kind's snapshots and a
# frozen column's default and cause_column equal unless a record amends them; roles and motives may only grow.


def snapshots_text(entry):
    """The text of an [[emits]] entry's snapshots, for an `[[amends]] of = "emits.<KIND>.snapshots"` record: each
    snapshot as `attr of ROLE [with PARTNER] from SOURCE [as TYPE]`, joined by `; `, in declaration order; `none`."""
    rows = []
    for x in entry.get("snapshots", []):
        if not isinstance(x, dict):
            return "malformed"
        text = f"{x.get('attr')} of {x.get('of')}"
        if x.get("partner"):
            text += f" with {x.get('partner')}"
        text += f" from {x.get('from')}"
        if x.get("type"):
            text += f" as {x.get('type')}"
        rows.append(text)
    return "; ".join(rows) if rows else "none"


def owns_field_text(column, key):
    """The text of an [[owns]] column's `default` or `cause_column`: the value as written, or `none`."""
    return "none" if key not in column else str(column[key])


def text_at(spec, m):
    """The text a `[[amends]] of` path names in ``spec``, or None when the path has no text there."""
    if m.group("inv"):
        return next((i.get("statement") for i in spec.get("invariants", []) if i.get("id") == m.group("inv")), None)
    if m.group("alg"):
        steps = spec.get("algorithm", {}).get(m.group("alg"), {}).get("steps", [])
        return next((st for st in steps if str(st).startswith(m.group("step") + " ")), None)
    if m.group("kind"):
        return next((e.get("when") for e in spec.get("emits", []) if e.get("kind") == m.group("kind")), None)
    if m.group("skind"):
        return next((snapshots_text(e) for e in spec.get("emits", []) if e.get("kind") == m.group("skind")), None)
    if m.group("dcol") or m.group("ccol"):
        col, key = (m.group("dcol"), "default") if m.group("dcol") else (m.group("ccol"), "cause_column")
        return next((owns_field_text(o, key) for o in spec.get("owns", []) if o.get("column") == col), None)
    if m.group("pulse"):
        for x in spec.get("schedule", []):
            if (x.get("pulse"), x.get("phase")) == (m.group("pulse"), m.group("phase")):
                return next((st for st in x.get("steps", []) if st.split(":")[0].strip() == m.group("sname")), None)
        return None
    if m.group("wpulse"):
        return next((x.get("when") for x in spec.get("schedule", [])
                     if (x.get("pulse"), x.get("phase")) == (m.group("wpulse"), m.group("wphase"))), None)
    if m.group("param"):
        return next((x.get("meaning") for x in spec.get("params", []) if x.get("name") == m.group("param")), None)
    if m.group("col"):
        return next((o.get("meaning") for o in spec.get("owns", []) if o.get("column") == m.group("col")), None)
    if m.group("rpurpose"):
        return next((g.get("use") for g in spec.get("rng", []) if g.get("purpose") == m.group("rpurpose")), None)
    return spec.get("initialization", {}).get("state")


GENERIC_PATH = re.compile(r"^(?P<sec>system|purpose|owns|emits|commands|applies|reads|observes|scales|fidelity_ladder|schedule|rng|design|initialization|input_data|params|submodels|invariants|couplings|isolation|verification)\.(?P<rest>.+)$")
# keyed sections: the field that names an entry (a repeated key carries its ordinal: key, key[1], key[2], in file order)
KEYED = {"owns": "column", "emits": "kind", "commands": "command", "applies": "command", "reads": "column", "observes": "kind",
         "fidelity_ladder": "level",
         "rng": "purpose", "params": "name", "submodels": "name", "invariants": "id"}
PAIR_KEYED = {"couplings": ("system", "direction"), "schedule": ("pulse", "phase")}  # key = "<a>/<b>"
SUBKEYED = {"patterns": "test", "forcing": "input"}  # purpose.patterns.<test>, isolation.forcing.<input>
# Deny by default (PR #79's review R3, N4): every frozen leaf is compared unless it is listed here. The list is the
# exemption list specs/systems/next/README.md states (the Architect rules on it).
EXEMPT = {
    ("system", "spec_version"), ("system", "status"), ("system", "fidelity"),   # a next version moves these by definition
    ("verification", "tests"), ("emits", "roles"), ("emits", "motives"), ("applies", "from"),   # may only grow (own checks)
    ("owns", "values"),   # an enum's codes may be appended, never renamed or moved (the prefix rule below)
}
# fields with a check of their own (a specialised diagnostic, or an amendable path of their own); the generic walk
# skips them so that one change gives one diagnostic
SPECIALISED = {
    ("system", "id"), ("system", "crate"), ("owns", "meaning"), ("owns", "default"), ("owns", "cause_column"),
    ("owns", "entity"), ("owns", "type"), ("owns", "plane"), ("owns", "fixed_point"), ("emits", "when"), ("emits", "snapshots"),
    ("rng", "use"), ("rng", "key"), ("rng", "resample_on_fork"), ("rng", "draws"), ("invariants", "statement"),
    ("invariants", "test"), ("schedule", "steps"), ("schedule", "when"), ("initialization", "state"), ("params", "meaning"),
    ("params", "default"), ("applies", "payload"),
}
TABLES = ("system", "purpose", "scales", "design", "initialization", "input_data", "isolation", "verification")


def leaf_text(value):
    """A frozen field as the text an [[amends]] record quotes: a string as is, anything else as JSON."""
    return value if isinstance(value, str) else json.dumps(value, ensure_ascii=False, default=str)


def generic_text_at(spec, path):
    """The text a generic amendment path names in ``spec`` (deny by default, R3, N4), or None when there is none."""
    m = GENERIC_PATH.match(path)
    if m is None:
        return None
    sec, rest = m.group("sec"), m.group("rest")
    if sec in KEYED or sec in PAIR_KEYED:
        key, _, field = rest.rpartition(".")
        entry = keyed_entry(spec.get(sec, []), sec, key)
        return None if entry is None or field not in entry else leaf_text(entry[field])
    for sub in SUBKEYED:
        if rest.startswith(sub + "."):
            key, _, field = rest[len(sub) + 1:].rpartition(".")
            entry = keyed_entry(spec.get(sec, {}).get(sub, []), sub, key)
            return None if entry is None or field not in entry else leaf_text(entry[field])
    table = spec.get(sec, {})
    return leaf_text(table[rest]) if rest in table else None


def entry_key(x, sec):
    """The key of a keyed entry: its naming field, or "<a>/<b>" for a coupling or a schedule binding."""
    if sec in PAIR_KEYED:
        a, b = PAIR_KEYED[sec]
        return f"{x.get(a)}/{x.get(b)}"
    return str(x.get(KEYED.get(sec) or SUBKEYED[sec]))


def keyed_paths(entries, sec):
    """(path key, entry) for each entry of a keyed section; a repeated key carries its ordinal: key, key[1], key[2]
    (the entries after the first, in file order: an amendment names the n-th repeat by its ordinal)."""
    seen = {}
    out = []
    for x in entries:
        k = entry_key(x, sec)
        n = seen.get(k, 0)
        seen[k] = n + 1
        out.append((k if n == 0 else f"{k}[{n}]", x))
    return out


def keyed_entry(entries, sec, key):
    """The entry a path key names, or None."""
    return next((x for k, x in keyed_paths(entries, sec) if k == key), None)


def key_fields(sec):
    """The fields that name an entry of ``sec`` (never leaves)."""
    if sec in PAIR_KEYED:
        return set(PAIR_KEYED[sec])
    return {KEYED.get(sec) or SUBKEYED[sec]}


def frozen_leaves(fs):
    """Every generic path of a frozen spec with its text: every leaf the deny-by-default walk protects (R3, N4):
    each field of each entry of each keyed section, each field of each table, the sub-keyed patterns and forcing
    entries; minus EXEMPT (listed in next/README.md) and SPECIALISED (a check of their own)."""
    out = {}
    for sec in list(KEYED) + list(PAIR_KEYED):
        for key, x in keyed_paths(fs.get(sec, []), sec):
            for field, value in x.items():
                if field in key_fields(sec) or (sec, field) in EXEMPT or (sec, field) in SPECIALISED:
                    continue
                out[f"{sec}.{key}.{field}"] = leaf_text(value)
    for sec in TABLES:
        for field, value in fs.get(sec, {}).items():
            if (sec, field) in EXEMPT or (sec, field) in SPECIALISED:
                continue
            if field in SUBKEYED:
                for key, x in keyed_paths(value, field):
                    for f2, v2 in x.items():
                        if f2 != SUBKEYED[field]:
                            out[f"{sec}.{field}.{key}.{f2}"] = leaf_text(v2)
            else:
                out[f"{sec}.{field}"] = leaf_text(value)
    return out


def values_prefix_problems(fs, ns, p):
    """An enum column's codes are its values' indexes: a next version may append a value, never rename or move one."""
    out = []
    nexts = {o.get("column"): o for o in ns.get("owns", [])}
    for o in fs.get("owns", []):
        was, now = o.get("values"), nexts.get(o.get("column"), {}).get("values")
        if was is None:
            continue
        if now is None or list(now[:len(was)]) != list(was):
            out.append(f"{p}: frozen owns.{o.get('column')}.values changed ({was} -> {now}): an enum's codes may be appended, never renamed or moved")
    return out


def resolve_text(spec, of):
    """The text an [[amends]] path names: the named paths first, then the generic ones; None for neither."""
    m = AMEND_PATH.match(of)
    if m is not None:
        return text_at(spec, m)
    return generic_text_at(spec, of)


def amendments(fs, ns, p, errs):
    """The next version's `[[amends]]` records (REV-SPR-015 R1): each names a path, quotes the frozen text byte for
    byte, gives the new text (which must be what the next version now says there) and a reason. Returns
    path -> (frozen text, new text) for the valid ones."""
    out = {}
    for a in ns.get("amends", []):
        of = str(a.get("of", ""))
        if AMEND_PATH.match(of) is None and GENERIC_PATH.match(of) is None:
            errs.append(f"{p}: [[amends]] of = {of!r} is not a path this lint knows (invariants.<id>.statement, "
                        "algorithm.<name>.steps.<Sn>, emits.<KIND>.when, initialization.state, schedule.<Pulse>/<Phase>.steps.<name>, "
                        "schedule.<Pulse>/<Phase>.when, owns.<entity.column>.meaning, rng.<purpose>.use, params.<name>.meaning, "
                        "emits.<KIND>.snapshots, owns.<entity.column>.default, owns.<entity.column>.cause_column, or a generic "
                        "path of the frozen slice: submodels.<name>.<field>, commands.<command>.<field>, "
                        "couplings.<system>/<direction>.<field>, params.<name>.<field>, fidelity_ladder.<level>.<field>, "
                        "purpose.<field>, purpose.patterns.<test>.<field>, isolation.<field>, isolation.forcing.<input>.<field>, "
                        "design.<field>, scales.<field>, verification.prototype, system.name|adrs|requirements)")
            continue
        if not a.get("reason"):
            errs.append(f"{p}: [[amends]] {of} needs a reason")
        was, now = resolve_text(fs, of), resolve_text(ns, of)
        if was is None:
            errs.append(f"{p}: [[amends]] {of}: the frozen spec has no text there")
        elif a.get("frozen") != was:
            errs.append(f"{p}: [[amends]] {of}: `frozen` is not the frozen text byte for byte")
        if now is None:
            errs.append(f"{p}: [[amends]] {of}: the next version has no text there")
        elif a.get("to") != now:
            errs.append(f"{p}: [[amends]] {of}: `to` is not what the next version says there")
        if a.get("frozen") == a.get("to"):
            errs.append(f"{p}: [[amends]] {of}: `to` equals `frozen`; nothing is amended")
        if of in out:
            errs.append(f"{p}: [[amends]] {of} is recorded twice")
        out[of] = (a.get("frozen"), a.get("to"))
    return out


def duplicates(items):
    """The values that occur more than once in ``items``, in first-seen order."""
    seen, out = set(), []
    for x in items:
        if x in seen and x not in out:
            out.append(x)
        seen.add(x)
    return out


def lint_duplicate_keys(p, s, errs):
    """A spec names each owned column, parameter, RNG purpose, invariant, emitted kind, applied command and schedule
    step once (REV-SPR-015 R15): a second entry would shadow the first in every reader, including this lint."""
    checks = [
        ("owned column", [o.get("column") for o in s.get("owns", [])]),
        ("parameter", [x.get("name") for x in s.get("params", [])]),
        ("RNG purpose", [g.get("purpose") for g in s.get("rng", [])]),
        ("invariant id", [i.get("id") for i in s.get("invariants", [])]),
        ("emitted kind", [e.get("kind") for e in s.get("emits", [])]),
        ("applied command", [a.get("command") for a in s.get("applies", [])]),
        ("observed kind", [o.get("kind") for o in s.get("observes", []) if isinstance(o, dict)]),
        ("schedule binding", [(x.get("pulse"), x.get("phase")) for x in s.get("schedule", [])]),
    ]
    for x in s.get("schedule", []):
        checks.append((f"step name of {(x.get('pulse'), x.get('phase'))}", [st.split(":")[0].strip() for st in x.get("steps", [])]))
    for what, keys in checks:
        for d in duplicates(keys):
            errs.append(f"{p}: {what} {d!r} is declared twice (a duplicate shadows the first entry)")


LOCK_NAME = "amendments.lock.toml"
LOCK_FLAG = "--write-amendments-lock"
OVERRIDE = re.compile(r"\b(?:replac\w*|overrid\w*|supersed\w*)\b[^.;]{0,60}\b(S\d{1,2})\b"
                      r"|\b(S\d{1,2})\b[^.;]{0,60}\b(?:is replaced|is overridden|is superseded|no longer applies|does not apply|is void)\b")


def step_label(step):
    """The id of a step text: its first token (`S9`, `F4`, `apply_death:`)."""
    return str(step).split(None, 1)[0].rstrip(":") if str(step).split() else ""


def frozen_default_probe(fs, ns):
    """The self-check's planted change of a frozen default (REV-SPR-015 R6): a copy of the next version ``ns`` with the
    default of the first parameter whose FROZEN entry in ``fs`` declares one moved by 1, or None when no frozen
    parameter has a default. A default the next version adds (a draft's new Registry default, carried by the
    --next overlay and mirrored in [[params]]: R3 of the P1a re-check, 2026-10-18) is not frozen, so the frozen-default
    rule does not reach it and planting there would prove nothing."""
    frozen = {x.get("name") for x in fs.get("params", []) if "default" in x}
    bad = copy.deepcopy(ns)
    for x in bad.get("params", []):
        if x.get("name") in frozen and "default" in x:
            x["default"] = x["default"] + 1
            return bad
    return None


def amendment_digests(ns):
    """path -> sha256 of the new text, for every [[amends]] record of a next version (REV-SPR-015 round 3 MIN-1)."""
    return {str(a.get("of", "")): "sha256:" + hashlib.sha256(str(a.get("to", "")).encode()).hexdigest()
            for a in ns.get("amends", [])}


def compare_lock(sid, want, have, p, errs):
    """The next version's amendments (``want``) must be exactly the locked set (``have``): an amendment added,
    withdrawn or changed since the lock is an error naming its path, so the count alone can never hide one."""
    for of in sorted(set(want) - set(have)):
        errs.append(f"{p}: amendment {of} is not in {LOCK_NAME} (an amendment was added since the lock; {LOCK_FLAG} after review)")
    for of in sorted(set(have) - set(want)):
        errs.append(f"{p}: {LOCK_NAME} records an amendment of {of} that the next version of {sid} no longer makes (withdrawn since the lock)")
    for of in sorted(set(want) & set(have)):
        if want[of] != have[of]:
            errs.append(f"{p}: amendment {of} changed its text since {LOCK_NAME}")


def load_lock(path):
    """The committed lock, {system id: {path: digest}}, or None when there is none."""
    if not path.exists():
        return None
    with path.open("rb") as f:
        return {sid: dict(v) for sid, v in tomllib.load(f).get("amendments", {}).items()}


def write_lock(path, digests, cmd):
    """Write the lock from the tool's own digests (never typed; REV-SPR-015 round 3 MIN-1)."""
    lines = [f"# {LOCK_NAME}: the amendments of every next version in this directory, path -> sha256 of the new text.",
             f"# Generated by `{cmd}`; never typed. The next-mode lint fails on any amendment added, withdrawn or",
             "# changed since this lock, naming its path (REV-SPR-015 round 3 MIN-1), so a review that approved the",
             "# amendments approved exactly these. Regenerate only after the Architect has read the new AMEND lines.", ""]
    for sid in sorted(digests):
        lines.append(f'[amendments."{sid}"]')
        lines += [f'"{of}" = "{d}"' for of, d in sorted(digests[sid].items())]
        lines.append("")
    path.write_text("\n".join(lines))


def lint_next(frozen, specs, errs):
    """A next-version spec preserves the frozen slice it supersedes, or amends it through `[[amends]]` records
    (SPR-015; REV-SPR-015 R1, R6, R15; specs/systems/next/README.md). Returns the amendment notes to print."""
    notes = []
    for sid, (ff, fs) in frozen.items():
        nf, ns = specs[sid]
        p = f"{nf.name} (next of {ff.name})"
        fh, nh = fs.get("system", {}), ns.get("system", {})
        if fh.get("status") != "frozen":
            errs.append(f"{p}: only a frozen spec takes a next version; {ff.name} is {fh.get('status')}")
        if nh.get("status") != "draft":
            errs.append(f"{p}: a next version is a draft until it supersedes the file, not {nh.get('status')}")
        if nh.get("crate") != fh.get("crate"):
            errs.append(f"{p}: crate {nh.get('crate')} differs from the frozen {fh.get('crate')}")
        fv, nv = semver(fh.get("spec_version")), semver(nh.get("spec_version"))
        if fv is None or nv is None or nv <= fv:
            errs.append(f"{p}: spec_version {nh.get('spec_version')} must be a semver above the frozen {fh.get('spec_version')}")
        amended = amendments(fs, ns, p, errs)
        notes += [f"{nf.name}: amends {of} ({next((a.get('reason') for a in ns.get('amends', []) if a.get('of') == of), '')})" for of in amended]
        notes.append(f"{nf.name}: amendments (next of {sid}): {len(amended)} recorded")
        owns = {o.get("column"): o for o in ns.get("owns", [])}
        for o in fs.get("owns", []):
            col = o.get("column")
            n = owns.get(col)
            if n is None:
                errs.append(f"{p}: frozen column {col} is missing from [[owns]]")
                continue
            for k in ("entity", "type", "plane", "fixed_point"):
                if n.get(k) != o.get(k):
                    errs.append(f"{p}: frozen column {col} changed {k}: {o.get(k)!r} -> {n.get(k)!r}")
            if n.get("meaning") != o.get("meaning") and f"owns.{col}.meaning" not in amended:
                errs.append(f"{p}: frozen column {col} changed its meaning without an [[amends]] record")
            for key in ("default", "cause_column"):   # SPR-020; REV-SPR-020 R15: equal unless amended
                if owns_field_text(n, key) != owns_field_text(o, key) and f"owns.{col}.{key}" not in amended:
                    errs.append(f"{p}: frozen column {col} changed its {key} without an [[amends]] record "
                                f"({owns_field_text(o, key)} -> {owns_field_text(n, key)}; envelope_v0.md §7)")
        for name, table in fs.get("algorithm", {}).items():
            got = ns.get("algorithm", {}).get(name)
            if got is None:
                errs.append(f"{p}: frozen [algorithm.{name}] is missing")
                continue
            expect = copy.deepcopy(table)
            step_amended = False
            for of, (was, now) in amended.items():
                m = AMEND_PATH.match(of)
                if m is not None and m.group("alg") == name:
                    expect["steps"] = [now if st == was else st for st in expect.get("steps", [])]
                    step_amended = True
            if step_amended:
                # REV-SPR-015 R17 (4): an amended step is a new edition of the algorithm, so its version moves
                if got.get("version") == table.get("version"):
                    errs.append(f"{p}: [algorithm.{name}] has an amended step but keeps version {table.get('version')!r}; an amendment bumps the table's version")
                expect["version"] = got.get("version")
            if got != expect:
                keys = sorted(k for k in set(expect) | set(got) if expect.get(k) != got.get(k))
                errs.append(f"{p}: frozen [algorithm.{name}] changed (keys {keys}); a changed algorithm is a new table, "
                            "or an amended step is recorded in [[amends]]")
        # REV-SPR-015 round 3 MIN-1: a new table may not override a frozen step, by reusing its id or by prose
        frozen_labels = {step_label(st): name for name, table in fs.get("algorithm", {}).items() for st in table.get("steps", [])}
        for name, table in ns.get("algorithm", {}).items():
            if name in fs.get("algorithm", {}):
                continue
            for st in table.get("steps", []):
                label = step_label(st)
                if label in frozen_labels:
                    errs.append(f"{p}: new [algorithm.{name}] step {label} reuses a frozen step id of [algorithm.{frozen_labels[label]}]: "
                                "a frozen step changes only through an [[amends]] record on its own table")
                m = OVERRIDE.search(str(st))
                if m and (m.group(1) or m.group(2)) in frozen_labels:
                    errs.append(f"{p}: new [algorithm.{name}] step {label} names frozen step {m.group(1) or m.group(2)} as replaced "
                                f"({m.group(0)!r}): a frozen step changes only through an [[amends]] record on its own table")
        frozen_rng, next_rng = fs.get("rng", []), ns.get("rng", [])
        for i, g in enumerate(frozen_rng):
            n = next_rng[i] if i < len(next_rng) else {}
            if n.get("purpose") != g.get("purpose"):
                errs.append(f"{p}: frozen RNG purpose {g.get('purpose')} must stay at position {i} (found {n.get('purpose')}): the generated RngPurpose discriminants are positional")
                continue
            for k in ("key", "resample_on_fork", "draws"):
                if n.get(k) != g.get(k):
                    errs.append(f"{p}: frozen RNG purpose {g.get('purpose')} changed {k}")
            if n.get("use") != g.get("use") and f"rng.{g.get('purpose')}.use" not in amended:
                errs.append(f"{p}: frozen RNG purpose {g.get('purpose')} changed its use text without an [[amends]] record")
        invs = {i.get("id"): i for i in ns.get("invariants", [])}
        for i in fs.get("invariants", []):
            n = invs.get(i.get("id"))
            of = f"invariants.{i.get('id')}.statement"
            if n is None:
                errs.append(f"{p}: frozen invariant {i.get('id')} is missing")
            elif n.get("test") != i.get("test"):
                errs.append(f"{p}: frozen invariant {i.get('id')} changed its test")
            elif n.get("statement") != i.get("statement") and of not in amended:
                errs.append(f"{p}: frozen invariant {i.get('id')} changed its statement without an [[amends]] record")
        applies = {a.get("command"): a for a in ns.get("applies", [])}
        for a in fs.get("applies", []):
            n = applies.get(a.get("command"))
            if n is None or n.get("payload") != a.get("payload"):
                errs.append(f"{p}: frozen [[applies]] {a.get('command')} is missing or changed its payload")
            elif not set(a.get("from", [])) <= set(n.get("from", [])):
                errs.append(f"{p}: frozen [[applies]] {a.get('command')} lost a sender from `from`: {sorted(set(a.get('from', [])) - set(n.get('from', [])))}")
        bindings = {(x.get("pulse"), x.get("phase")): x for x in ns.get("schedule", [])}
        for x in fs.get("schedule", []):
            key = (x.get("pulse"), x.get("phase"))
            if key not in bindings:
                errs.append(f"{p}: frozen schedule binding {key} is missing")
                continue
            next_steps = {st.split(":")[0].strip(): st for st in bindings[key].get("steps", [])}
            for st in x.get("steps", []):
                name = st.split(":")[0].strip()
                of = f"schedule.{key[0]}/{key[1]}.steps.{name}"
                if name not in next_steps:
                    errs.append(f"{p}: frozen schedule step '{name}' of {key} is missing")
                elif next_steps[name] != st and of not in amended:
                    errs.append(f"{p}: frozen schedule step '{name}' of {key} changed without an [[amends]] record")
            if bindings[key].get("when") != x.get("when") and f"schedule.{key[0]}/{key[1]}.when" not in amended:
                errs.append(f"{p}: frozen schedule binding {key} changed its `when` without an [[amends]] record")
        whens = {e.get("kind"): e.get("when", "") for e in ns.get("emits", [])}
        next_emits = {e.get("kind"): e for e in ns.get("emits", [])}
        for e in fs.get("emits", []):
            k = e.get("kind")
            if k not in whens:
                errs.append(f"{p}: frozen [[emits]] {k} is missing")
                continue
            if e.get("when", "") != whens[k] and f"emits.{k}.when" not in amended:
                errs.append(f"{p}: frozen [[emits]] {k} changed its `when` without an [[amends]] record (REV-SPR-015 R15: equal text or an amendment; a prefix or suffix is a change)")
            # the envelope declarations (SPR-020; REV-SPR-020 R15): roles and motives may only grow; snapshots stay equal
            # unless an [[amends]] record changes them (a next version that captures a new attribute records why)
            for key in ("roles", "motives"):
                lost = [x for x in e.get(key, []) if x not in next_emits[k].get(key, [])]
                if lost:
                    errs.append(f"{p}: frozen [[emits]] {k} narrowed its {key}: {lost} dropped (a next version may only add; envelope_v0.md §2)")
            if snapshots_text(e) != snapshots_text(next_emits[k]) and f"emits.{k}.snapshots" not in amended:
                errs.append(f"{p}: frozen [[emits]] {k} changed its snapshots without an [[amends]] record "
                            f"({snapshots_text(e)} -> {snapshots_text(next_emits[k])}; envelope_v0.md §4)")
        was, now = fs.get("initialization", {}).get("state", ""), ns.get("initialization", {}).get("state", "")
        if was != now and "initialization.state" not in amended:
            errs.append(f"{p}: frozen [initialization] state changed without an [[amends]] record (equal text or an amendment)")
        params = {x.get("name"): x for x in ns.get("params", [])}
        for x in fs.get("params", []):
            n = params.get(x.get("name"))
            if n is None:
                errs.append(f"{p}: frozen parameter {x.get('name')} is missing")
            elif "default" in x and n.get("default") != x.get("default"):
                errs.append(f"{p}: frozen default of {x.get('name')} changed: {x.get('default')} -> {n.get('default')}")
            if n is not None and n.get("meaning") != x.get("meaning") and f"params.{x.get('name')}.meaning" not in amended:
                errs.append(f"{p}: frozen parameter {x.get('name')} changed its meaning without an [[amends]] record")
        # deny by default (PR #79's review R3, N4): every other frozen leaf, equal or amended; enum codes append only
        errs.extend(values_prefix_problems(fs, ns, p))
        for of, was in frozen_leaves(fs).items():
            now = generic_text_at(ns, of)
            if now is None:
                errs.append(f"{p}: frozen {of} is missing from the next version (a frozen entry keeps its fields)")
            elif now != was and of not in amended:
                errs.append(f"{p}: frozen {of} changed without an [[amends]] record (deny by default, R3 of PR #79's review)")
        # REV-SPR-015 round 3 NIT-5: the frozen verification tests and couplings are part of the slice
        next_tests = set(ns.get("verification", {}).get("tests", []))
        for t in fs.get("verification", {}).get("tests", []):
            if t not in next_tests:
                errs.append(f"{p}: frozen test {t} dropped from [verification] (frozen tests stay; new ones may be added)")
        next_couplings = {(c.get("system"), c.get("direction")) for c in ns.get("couplings", [])}
        for pair in sorted({(c.get("system"), c.get("direction")) for c in fs.get("couplings", [])} - next_couplings, key=str):
            errs.append(f"{p}: frozen coupling {pair} dropped from [[couplings]]")
    return notes


def payload_fields(text):
    """Field names of a payload written `{ name: Type, ... }` (an owner's [[applies]]) or `{ name, ... }` (a sender's
    [[commands]])."""
    inner = str(text).strip()
    if not (inner.startswith("{") and inner.endswith("}")):
        return None
    return [f.split(":")[0].strip() for f in inner[1:-1].split(",") if f.strip()]


def payload_types(text):
    """The `(name, type)` pairs of a typed payload `{ name: Type, ... }` (EVO-001), or None if any field lacks the
    form; the types are not checked here."""
    inner = str(text).strip()
    if not (inner.startswith("{") and inner.endswith("}")):
        return None
    out = []
    for f in inner[1:-1].split(","):
        if not f.strip():
            continue
        m = PAYLOAD_FIELD.match(f.strip())
        if m is None:
            return None
        out.append((m.group(1), m.group(2)))
    return out


def payload_type_problem(t, enum_names):
    """Why ``t`` is not in the closed payload vocabulary (EVO-001), or None."""
    if t in PAYLOAD_TYPES:
        return None
    m = PAYLOAD_ENUM.match(t)
    if m is None:
        return f"type '{t}' is not in the payload vocabulary {sorted(PAYLOAD_TYPES)} or enum(<entity.column>)"
    if m.group(1) not in enum_names:
        return f"type '{t}' names no enum column with codes in any spec"
    return None


def enum_codes(col_entry, reg):
    """The code names of an enum column of [[owns]] (EVO-001): `values` by position, or the Registry table named by
    `registry` (its `ids` list, else its keys in byte-wise ascending order). None, with a reason, if neither is usable."""
    values, table = col_entry.get("values"), col_entry.get("registry")
    if (values is None) == (table is None):
        return None, "needs exactly one of values = [...] or registry = \"<table>\" (its codes; EVO-001)"
    if values is not None:
        if not (isinstance(values, list) and values and all(isinstance(v, str) and SNAKE.match(v) for v in values)
                and len(set(values)) == len(values)):
            return None, "values must be a non-empty list of distinct snake_case names"
        return list(values), None
    t = reg.get(table)
    if not isinstance(t, dict):
        return None, f"registry = \"{table}\" names no Registry table"
    ids = t.get("ids")
    if isinstance(ids, list):
        names = [i for i in ids if isinstance(i, str)]
    else:
        names = sorted(k for k in t if isinstance(k, str))
    if not names or not all(REGISTRY_ID.match(n) for n in names):
        return None, f"registry = \"{table}\": the table has no usable ids"
    return names, None


def names_value(prose, column, value):
    """Whether the coupling's free-text `neutral` states ``<column> = <value>`` (the bare name or entity.column; EVO-001,
    REV-EVO-001 NIT-8), so that a number elsewhere in the prose cannot stand in for it."""
    forms = {str(value)}
    if isinstance(value, bool):
        forms |= {"true", "false"} & {str(value).lower()}
    elif isinstance(value, (int, Decimal, float)):
        d = Decimal(str(value)).normalize()
        forms |= {format(d, "f"), str(value)}
    bare = column.split(".", 1)[-1]
    name = r"(?:" + re.escape(column) + "|" + re.escape(bare) + ")"
    return any(re.search(r"(?<![\w.])" + name + r"\s*=\s*" + re.escape(f) + r"(?![\w.])", str(prose)) for f in forms)


def closed_sprints():
    """The sprint ids whose document in CODEX/05_PROJECT is CLOSED (the `sprint` escape of [[applies]] ends there)."""
    out = set()
    for f in (ROOT / "CODEX" / "05_PROJECT").glob("SPR-*.md"):
        head = f.read_text(encoding="utf-8")[:2000]
        if re.search(r"^status:\s*CLOSED\b", head, re.M):
            out.add(f.name[:7])
    return out


def known_sprints():
    """The sprint ids a `sprint` escape may name (REV-EVO-001 R18): a sprint document under CODEX/05_PROJECT, or, until
    the document exists, a sprint row of a PRJ plan there (`| SPR-NNN |`). Any other id is refused."""
    out = set()
    for f in (ROOT / "CODEX" / "05_PROJECT").glob("SPR-*.md"):
        out.add(f.name[:7])
    for f in (ROOT / "CODEX" / "05_PROJECT").glob("PRJ-*.md"):
        out.update(re.findall(r"^\|\s*(SPR-\d{3})\s*\|", f.read_text(encoding="utf-8"), re.M))
    return out


def lint_applies(specs, errs, warns, enum_names=None, next_ids=()):
    """[[applies]] against the senders' [[commands]] (PR #32; REV-P1a §7), and each entry's typed payload, drained_by
    and key (EVO-001). A next-version spec (``next_ids``) is held to frozen-level strictness (REV-SPR-015 R6)."""
    enum_names = enum_names or {}

    def frozen_or_next(sid):
        return specs[sid][1].get("system", {}).get("status") == "frozen" or sid in next_ids
    for tid, (tf, ts) in specs.items():
        strict = errs if frozen_or_next(tid) else warns
        applies = {}
        for a in ts.get("applies", []):
            if a.get("command") in applies:
                errs.append(f"{tf.name}: [[applies]] {a.get('command')} is declared twice; one draining binding per command kind (EVO-001)")
            applies.setdefault(a.get("command"), a)
        bindings = {f"{sc.get('pulse')}/{sc.get('phase')}" for sc in ts.get("schedule", [])}
        for cmd, a in sorted(applies.items()):
            typed = payload_types(a.get("payload"))
            names = []
            if typed is None:
                strict.append(f"{tf.name}: [[applies]] {cmd} payload must be typed, \"{{ name: Type, ... }}\" (EVO-001)")
            else:
                for name, t in typed:
                    if (why := payload_type_problem(t, enum_names)) is not None:
                        strict.append(f"{tf.name}: [[applies]] {cmd} field {name}: {why}")
                    if name.endswith("_event") and t != "EventRef":
                        errs.append(f"{tf.name}: [[applies]] {cmd} field {name} must be an EventRef (the merge rewrites event slots; EVO-001)")
                    if name in names:
                        errs.append(f"{tf.name}: [[applies]] {cmd} field {name} is repeated")
                    names.append(name)
            drained = a.get("drained_by")
            if not isinstance(drained, str) or not BINDING.match(drained):
                strict.append(f"{tf.name}: [[applies]] {cmd} needs drained_by = \"<Pulse>/<Phase>\", the one binding that takes it (EVO-001)")
            elif drained not in bindings:
                sprint = a.get("sprint")
                if sprint is not None and not (isinstance(sprint, str) and SPRINT_ID.match(sprint)):
                    errs.append(f"{tf.name}: [[applies]] {cmd} sprint must be \"SPR-NNN\" (EVO-001)")
                    sprint = None
                elif sprint is not None and sprint not in KNOWN_SPRINTS:
                    errs.append(f"{tf.name}: [[applies]] {cmd} sprint {sprint} names no sprint: no document under CODEX/05_PROJECT and no PRJ row (EVO-001 R18)")
                    sprint = None
                if sprint is None or sprint in CLOSED_SPRINTS:
                    strict.append(f"{tf.name}: [[applies]] {cmd} is drained_by {drained}, which [[schedule]] does not declare"
                                  + (f" ({sprint} is CLOSED: declare the binding)" if sprint else ""))
                else:
                    warns.append(f"{tf.name}: ESCAPE [[applies]] {cmd} is drained_by {drained}, which [[schedule]] does not declare until {sprint}")
            if a.get("key") is None:
                strict.append(f"{tf.name}: [[applies]] {cmd} needs key = \"living\" | \"any\" (EVO-001)")
            elif a.get("key") not in KEY_RULES:
                errs.append(f"{tf.name}: [[applies]] {cmd} key must be \"living\" | \"any\" (EVO-001)")
            kf = a.get("key_field")
            if not isinstance(kf, str):
                strict.append(f"{tf.name}: [[applies]] {cmd} needs key_field = \"<a PersonId payload field>\" | \"person\", the person it is keyed by (EVO-001)")
            elif typed is not None and (kf, "PersonId") not in typed and not (kf == "person" and "person" not in names):
                errs.append(f"{tf.name}: [[applies]] {cmd} key_field {kf} is not a PersonId field of the payload (nor \"person\", Command::person itself)")
            if not isinstance(a.get("outcome", False), bool):
                errs.append(f"{tf.name}: [[applies]] {cmd} outcome must be true or false")
            else:
                for sid, (sf, ss) in specs.items():
                    for c in ss.get("commands", []):
                        if c.get("target") != tid or c.get("command") != cmd:
                            continue
                        sp = errs if ss.get("system", {}).get("status") == "frozen" else warns
                        timer, purpose = c.get("timer"), c.get("purpose")
                        if not a.get("outcome"):
                            if timer is not None or purpose is not None:
                                warns.append(f"{sf.name}: [[commands]] {cmd} to {tid} names a timer, but the owner's [[applies]] says it carries no outcome (outcome = false): an undated request waits for the draining binding (EVO-001)")
                            continue
                        if not (isinstance(timer, str) and TIMER_KIND.match(timer)):
                            sp.append(f"{sf.name}: [[commands]] {cmd} to {tid} carries an outcome, so it needs timer = \"<KIND>\", the sender's own timer that submits it (EVO-001)")
                        purposes = {g.get("purpose") for g in ss.get("rng", [])}
                        if purpose not in purposes:
                            sp.append(f"{sf.name}: [[commands]] {cmd} to {tid} carries an outcome, so it needs purpose = one of its [[rng]] purposes, under which its timer is re-sampled (EVO-001)")
                        if isinstance(drained, str) and BINDING.match(drained) and drained not in {f"{sc.get('pulse')}/{sc.get('phase')}" for sc in ss.get("schedule", [])}:
                            sp.append(f"{sf.name}: [[commands]] {cmd} to {tid} carries an outcome submitted by {sid}'s timer in the timer pass of {drained}, which {sid} does not bind in [[schedule]] (a timer fires at the level of its owner's first binding in the phase; the engine would raise SimError::Timer at the first fire; EVO-001 R9)")
        senders = {}                              # command -> [(sender id, its [[commands]] entry)]
        for sid, (sf, ss) in specs.items():
            for c in ss.get("commands", []):
                if c.get("target") == tid:
                    senders.setdefault(c.get("command"), []).append((sid, c))
        if not applies:
            continue
        for cmd in sorted(set(senders) - set(applies)):
            strict.append(f"{tf.name}: {', '.join(s for s, _ in senders[cmd])} send {cmd}, which [[applies]] does not declare")
        for cmd, a in sorted(applies.items()):
            want = payload_fields(a.get("payload"))
            if not want:
                errs.append(f"{tf.name}: [[applies]] {cmd} needs payload = \"{{ field: Type, ... }}\"")
                continue
            declared, actual = set(a.get("from", [])), {s for s, _ in senders.get(cmd, [])}
            for s in sorted(declared - actual):
                (strict if s in specs else warns).append(f"{tf.name}: [[applies]] {cmd} lists {s} in from, but {s} does not send it")
            for s in sorted(actual - declared):
                strict.append(f"{tf.name}: {s} sends {cmd}, but [[applies]] {cmd} does not list it in from")
            for s, c in senders.get(cmd, []):
                got = payload_fields(c.get("payload")) if "payload" in c else None
                sp = errs if frozen_or_next(s) else warns
                if got is None:
                    sp.append(f"{specs[s][0].name}: [[commands]] {cmd} to {tid} needs payload = \"{{ {', '.join(want)} }}\" ({tid}'s [[applies]])")
                elif got != want:
                    (sp if sp is errs else strict).append(
                        f"{specs[s][0].name}: [[commands]] {cmd} to {tid} has payload fields {got}, but [[applies]] declares {want}")


def neutral_problem(value, col_type, names=None):
    """Why ``value`` is not a well-typed `neutral` for a column of ``col_type`` (EVO-001), or None. ``names`` are the
    enum's codes when known (an enum column without codes is reported at its owner)."""
    shown = str(value)
    if col_type in INTEGER_TYPES:
        if not isinstance(value, int) or isinstance(value, bool):
            return f"neutral {shown} must be a TOML integer for a {col_type} column"
        lo, hi = INT_RANGE[col_type]
        if not lo <= value <= hi:
            return f"neutral {shown} is outside {col_type}'s range [{lo}, {hi}]"
    elif col_type == "bool":
        if not isinstance(value, bool):
            return f"neutral {shown} must be true or false for a bool column"
    elif col_type == "enum":
        if not isinstance(value, str) or not REGISTRY_ID.match(value):
            return f"neutral {shown} must be a value name for a enum column"
        if names is not None and value not in names:
            return f"neutral {shown} is not one of the enum's codes {names}"
    elif col_type in NUMBER_TYPES:
        if isinstance(value, bool) or not isinstance(value, (int, Decimal, float)):
            return f"neutral {shown} must be a TOML number for a {col_type} column"
        if col_type in FIXED_RANGE:
            lo, hi = FIXED_RANGE[col_type]
            d = Decimal(str(value))
            if d * FIXED_STEP[col_type] != int(d * FIXED_STEP[col_type]):
                return f"neutral {shown} is not representable in {col_type} (a multiple of 1/{FIXED_STEP[col_type]})"
            if not lo <= d < hi:
                return f"neutral {shown} is outside {col_type}'s range [{lo}, {hi})"
    else:
        return f"column type '{col_type}' has no neutral form"
    return None


def column_type(col, owner, owners_type, reg):
    """The declared type of ``col`` (``entity.name``): the owner spec's [[owns]], else the Registry [era_state]
    entry of the same name and owner (M6's exec_constraint before M6 has a spec). None if neither."""
    if col in owners_type:
        return owners_type[col]
    name = col.split(".", 1)[-1]
    era = reg.get("era_state", {}).get(name)
    if era is not None and era.get("owner") == owner:
        return era.get("type")
    return None


CLOSED_SPRINTS = set()
KNOWN_SPRINTS = set()
ALL_OWNED_TYPES = {}   # column -> type over every spec of the run (for a snapshot read from a foreign column)


def lint(reg, specs, req_ids, test_ids, adr_ids, notes=None, next_ids=()):
    """``next_ids``: systems a --next spec supersedes for this run (SPR-015). A draft spec that still reads a column
    from the owner it had before the next version claimed it warns (it moves when the next version supersedes the
    file); a frozen reader errs as always."""
    errs, warns = [], []
    notes = notes if notes is not None else []
    roles = {sid: s.get("purpose", {}).get("role") for sid, (_, s) in specs.items()}
    owners_type = {o.get("column"): o.get("type") for _, (_, s) in specs.items() for o in s.get("owns", [])}
    ALL_OWNED_TYPES.clear()
    ALL_OWNED_TYPES.update(owners_type)
    enum_names = {}                               # enum column -> its code names (EVO-001)
    for _, (_, s) in specs.items():
        for o in s.get("owns", []):
            if o.get("type") == "enum":
                names, _why = enum_codes(o, reg)
                if names is not None:
                    enum_names[o.get("column")] = names
    owners = {}                                   # column -> (system, plane)
    for sid, (f, s) in specs.items():
        for o in s.get("owns", []):
            col = o.get("column")
            if col in owners:
                errs.append(f"{sid}: column '{col}' is also owned by {owners[col][0]} (single writer, 05.1 §2)")
            owners[col] = (sid, o.get("plane"))
    for sid, (f, s) in specs.items():
        p = f"{f.name}"
        if s.get("odd_r") != 2:
            errs.append(f"{p}: odd_r must be 2")
        lint_duplicate_keys(p, s, errs)
        for sec in REQUIRED:
            if sec not in s:
                errs.append(f"{p}: missing section [{sec}] (write 'n/a: <reason>' fields rather than omitting)")
        sysdef = reg.get("systems", {}).get(sid)
        if sysdef is None:
            errs.append(f"{p}: system '{sid}' is not in the Registry")
            sysdef = {}
        hdr = s.get("system", {})
        if sysdef and hdr.get("fidelity") not in sysdef.get("fidelity_levels", []):
            errs.append(f"{p}: fidelity {hdr.get('fidelity')} not among Registry levels {sysdef.get('fidelity_levels')}")
        for a in hdr.get("adrs", []):
            if a not in adr_ids:
                errs.append(f"{p}: {a} does not exist in docs/06_adr")
        for r in hdr.get("requirements", []):
            if r not in req_ids:
                errs.append(f"{p}: requirement {r} not found in docs/03")
        # purpose and patterns
        for pat in s.get("purpose", {}).get("patterns", []):
            if pat.get("test") not in test_ids:
                errs.append(f"{p}: pattern test {pat.get('test')} not found in docs/09")
            if pat.get("kind") not in ("identity", "emergent"):
                errs.append(f"{p}: pattern {pat.get('test')} kind must be identity or emergent")
        # owned columns
        planes = set()
        for o in s.get("owns", []):
            if o.get("type") not in TYPES:
                errs.append(f"{p}: owned column {o.get('column')} has unknown type '{o.get('type')}'")
            if o.get("plane") not in ("A", "B", "C"):
                errs.append(f"{p}: owned column {o.get('column')} needs plane A, B or C")
            planes.add(o.get("plane"))
            if o.get("type") in ("q8_8", "q16_16") and not o.get("fixed_point", False):
                errs.append(f"{p}: {o.get('column')} is fixed-point by type; set fixed_point = true")
            if o.get("type") == "enum":
                _names, why = enum_codes(o, reg)
                if why is not None:
                    (errs if hdr.get("status") == "frozen" else warns).append(f"{p}: enum column {o.get('column')} {why}")
        # reads resolve to owners; plane rule
        for r in s.get("reads", []):
            col, owner = r.get("column"), r.get("owner")
            if col in owners:
                if owners[col][0] != owner:
                    moved = owners[col][0] in next_ids and hdr.get("status") != "frozen"
                    (warns if moved else errs).append(
                        f"{p}: reads {col} from {owner}, but {owners[col][0]} owns it"
                        + (" (in its next version: point the read at it when that version supersedes the file)" if moved else ""))
                if "A" in planes and owners[col][1] in ("B", "C"):
                    errs.append(f"{p}: a plane-A system reads plane-{owners[col][1]} column {col} (ADR-0024)")
            elif owner in specs:
                errs.append(f"{p}: reads {col}, which {owner}'s spec does not declare in [[owns]]")
            elif owner not in reg.get("systems", {}) and owner not in ("S1.culture", "S2.technology", "S3.migration"):
                errs.append(f"{p}: reads {col} from unknown system {owner}")
            else:
                warns.append(f"{p}: reads {col} from {owner}, which has no spec yet")
            if r.get("lag") not in ("same_instant", "start_of_day", "previous_pulse", "lagged"):
                errs.append(f"{p}: read {col} needs lag same_instant | start_of_day | previous_pulse | lagged")
            if r.get("use") not in READ_USES:
                errs.append(f"{p}: read {col} needs use = identity | mode | rate (RT-2026-10-16)")
        # couplings and isolation (RT-2026-10-16): detail is required once the spec is frozen
        strict = errs if hdr.get("status") == "frozen" else warns
        couplings = s.get("couplings", [])
        for c in couplings:
            if c.get("direction") not in ("in", "out"):
                errs.append(f"{p}: coupling to {c.get('system')} needs direction = in | out")
        in_by_col = {c.get("column"): c for c in couplings if c.get("direction") == "in"}
        for r in s.get("reads", []):
            if r.get("use") != "rate":
                continue
            col = r.get("column")
            c = in_by_col.get(col)
            if c is None:
                strict.append(f"{p}: rate read {col} needs an in-coupling with {', '.join(IN_COUPLING_FIELDS)}")
                continue
            missing = [k for k in IN_COUPLING_FIELDS if not c.get(k)]
            if missing:
                strict.append(f"{p}: in-coupling on {col} is missing {missing}")
            ob = c.get("output_bounds")
            if ob is not None and (not isinstance(ob, list) or len(ob) != 2):
                errs.append(f"{p}: in-coupling on {col}: output_bounds must be [min, max]")
        # absent owners (EVO-001; 05.1 §2 rule 2; REV-EVO-001 R12): an identity read requires its owner and has no
        # neutral; a rate or mode read carries the neutral a keyed lookup returns once the read is dropped
        required_owners = sorted({r.get("owner") for r in s.get("reads", []) if r.get("owner") != sid and r.get("use") == "identity"})
        if required_owners:
            notes.append(f"{p}: requires {', '.join(required_owners)} (identity reads; a world that registers {sid} registers them)")
        for r in s.get("reads", []):
            col, owner, use = r.get("column"), r.get("owner"), r.get("use")
            if owner == sid:
                continue
            if use == "identity" and s.get("purpose", {}).get("role") == "substrate" and roles.get(owner) != "substrate":
                warns.append(f"{p}: a substrate reads {col} from {owner} as identity, which requires {owner}"
                             + (" (not a substrate)" if owner in roles else " (no spec, so its role is unknown)")
                             + "; advisory: a substrate that must run alone reads it as rate or mode with a neutral, and registration refuses a world that lacks the owner (EVO-001 R12, R16)")
            if "required" in r:
                errs.append(f"{p}: read {col} from {owner}: `required` is not a field; an identity read always requires its owner (EVO-001 R12)")
            if use == "identity":
                if "neutral" in r:
                    errs.append(f"{p}: read {col} from {owner} is an identity read, which requires its owner and declares no neutral (EVO-001)")
                continue
            if "neutral" not in r:
                strict.append(f"{p}: read {col} from {owner} needs a neutral value, typed by the column "
                              f"(a {use} read, dropped when its owner is absent; EVO-001; 05.1 §2 rule 2)")
                continue
            value = r.get("neutral")
            ct = column_type(col, owner, owners_type, reg)
            if ct is None:
                strict.append(f"{p}: neutral of {col} cannot be typed: {owner} has no spec declaring it and the Registry "
                              "[era_state] has no entry for it (EVO-001)")
            elif (why := neutral_problem(value, ct, enum_names.get(col))) is not None:
                errs.append(f"{p}: read {col} from {owner}: {why} (EVO-001)")
            c = in_by_col.get(col)
            if use == "rate" and c is not None and c.get("neutral") and not names_value(c.get("neutral"), col, value):
                strict.append(f"{p}: in-coupling on {col}: its neutral prose does not state {col.split('.', 1)[-1]} = {value} (EVO-001)")
        iso = s.get("isolation")
        if iso is None:
            strict.append(f"{p}: missing [isolation] (synthetic forcing for this system alone; docs/09 V-SF-22)")
        else:
            forced = {fo.get("input") for fo in iso.get("forcing", [])}
            for fo in iso.get("forcing", []):
                if not str(fo.get("series", "")).split(":")[0] in SERIES:
                    errs.append(f"{p}: forcing of {fo.get('input')}: series must be one of {SERIES}")
            for r in s.get("reads", []):
                if r.get("use") in ("mode", "rate") and r.get("column") not in forced:
                    strict.append(f"{p}: [isolation] has no forcing for {r.get('use')} read {r.get('column')}")
            for k in ("horizon", "expect"):
                if not iso.get(k):
                    strict.append(f"{p}: [isolation] needs {k}")
        # schedule
        for sc in s.get("schedule", []):
            if sc.get("pulse") not in PULSES:
                errs.append(f"{p}: unknown pulse {sc.get('pulse')}")
            if sc.get("phase") not in PHASES:
                errs.append(f"{p}: unknown phase {sc.get('phase')} (05.1 §4.2)")
            if not sc.get("steps"):
                errs.append(f"{p}: schedule entry without steps")
        step_names = {st.split(":")[0].strip() for sc in s.get("schedule", []) for st in sc.get("steps", [])}
        sub_names = {sm.get("name") for sm in s.get("submodels", [])}
        for n in sub_names - step_names:
            errs.append(f"{p}: submodel '{n}' is not named in any schedule step (ODD: same names in §3 and §7)")
        # rng
        seen = set()
        for g in s.get("rng", []):
            pu = g.get("purpose", "")
            if not SNAKE.match(pu):
                errs.append(f"{p}: RNG purpose '{pu}' must be snake_case")
            if pu in seen:
                errs.append(f"{p}: duplicate RNG purpose '{pu}'")
            seen.add(pu)
            if not g.get("key"):
                errs.append(f"{p}: RNG purpose '{pu}' needs a key (ADR-0006 keyed counter-based RNG)")
        # design concepts
        d = s.get("design", {})
        for k in DESIGN_KEYS:
            if k not in d:
                errs.append(f"{p}: design concept '{k}' missing (write 'n/a: <reason>')")
        for v in d.get("era_state", []):
            if v not in reg.get("era_state", {}):
                errs.append(f"{p}: era-state variable '{v}' is not in the Registry [era_state]")
        # parameters: equal sets both ways
        spec_params = {x.get("name") for x in s.get("params", [])}
        reg_params = set(sysdef.get("params", {}))
        for n in sorted(spec_params - reg_params):
            errs.append(f"{p}: parameter '{n}' is not declared in the Registry for {sid}")
        for n in sorted(reg_params - spec_params):
            errs.append(f"{p}: Registry parameter '{n}' is undocumented in the spec")
        for x in s.get("params", []):
            if not x.get("source"):
                errs.append(f"{p}: parameter '{x.get('name')}' needs a source (a reference or 'design value')")
        # parameter type (SPR-016 MR2; REV-SPR-016-FOLD MIN-4): a [[params]] entry is q16_16 unless it declares
        # type = "count", an integer (u64 in the skeleton); a count's Registry bounds and default are integers
        for x in s.get("params", []):
            t = x.get("type")
            if t is None:
                continue
            if t != "count":
                errs.append(f"{p}: parameter '{x.get('name')}' has type '{t}'; the only declared type is 'count' "
                            "(an integer); without `type` a parameter is q16_16")
                continue
            values = list(sysdef.get("params", {}).get(x.get("name"), [])) + ([x["default"]] if "default" in x else [])
            if not all(isinstance(v, int) and not isinstance(v, bool) for v in values):
                errs.append(f"{p}: count parameter '{x.get('name')}' needs integer Registry bounds and default (SPR-016 MR2)")
        # defaults (SPR-011 ruling 8): the spec mirrors the Registry's defaults exactly, both ways
        reg_defaults = sysdef.get("defaults", {})
        for x in s.get("params", []):
            n, d = x.get("name"), x.get("default")
            if d is not None and reg_defaults.get(n) != d:
                errs.append(f"{p}: parameter '{n}' default {d} differs from the Registry's ({reg_defaults.get(n)})")
        spec_defaults = {x.get("name") for x in s.get("params", []) if "default" in x}
        for n in sorted(set(reg_defaults) - spec_defaults):
            errs.append(f"{p}: Registry default for '{n}' ({reg_defaults[n]}) is not mirrored as `default` in [[params]]")
        # event vocabulary (SPR-011 ruling 11): [[emits]] against the Registry's [verbs], both ways; a next-version
        # spec is held to frozen-level strictness here (REV-SPR-015 R6)
        verbs = reg.get("verbs", {})
        emitted = [e.get("kind") for e in s.get("emits", [])]
        strict_emits = errs if sid in next_ids else strict
        for k in emitted:
            if k not in verbs:
                strict_emits.append(f"{p}: emits {k}, which is not a Registry verb ([verbs]; SPR-011 ruling 11)")
            elif verbs[k].get("emitted_by") != sid:
                strict_emits.append(f"{p}: emits {k}, which the Registry gives to {verbs[k].get('emitted_by')}")
        for k in sorted(v for v, d in verbs.items() if d.get("emitted_by") == sid and v not in emitted):
            strict_emits.append(f"{p}: the Registry gives verb {k} to {sid}, but [[emits]] does not declare it")
        lint_envelope(p, s, reg, enum_names, errs, warns, hdr.get("status") == "frozen")   # SPR-020: roles, motives, snapshots, cause_column
        lint_observes(p, sid, s, reg, specs, errs)   # EVO-002: the instant log's observed kinds
        # submodels earn their place
        for sm in s.get("submodels", []):
            ep = sm.get("earns_its_place_by", [])
            if not ep:
                errs.append(f"{p}: submodel '{sm.get('name')}' must name the tests or bands that justify it")
            for t in ep:
                if t not in test_ids:
                    errs.append(f"{p}: submodel '{sm.get('name')}' cites {t}, not found in docs/09")
        # invariants and verification
        for inv in s.get("invariants", []):
            if inv.get("test") not in test_ids:
                errs.append(f"{p}: invariant {inv.get('id')} cites test {inv.get('test')}, not found in docs/09")
        for t in s.get("verification", {}).get("tests", []):
            if t not in test_ids:
                errs.append(f"{p}: verification test {t} not found in docs/09")
        proto = s.get("verification", {}).get("prototype")
        if proto and not (ROOT / proto).exists():
            errs.append(f"{p}: prototype {proto} does not exist")
        for c in s.get("couplings", []):
            if c.get("system") not in reg.get("systems", {}) and c.get("system") not in ("S1.culture", "S2.technology", "S3.migration"):
                errs.append(f"{p}: coupling to unknown system {c.get('system')}")
    lint_applies(specs, errs, warns, enum_names, next_ids)
    return errs, warns


def default_problem(value, ct, enum_codes_of_column):
    """Why ``value`` is not a well-typed default for a column of type ``ct`` (envelope_v0.md §7), or None."""
    return neutral_problem(value, ct, enum_codes_of_column)


OBSERVES_KEYS = ("kind", "owner", "why")


def lint_observes(p, sid, s, reg, specs, errs):
    """[[observes]] (EVO-002; 05.1 §2 rule 2, the instant log's part (c)): the event kinds a system reads in the instant
    log, by (kind, target) or by kind, and cites in the merge-time form. Each entry names a known kind (a Registry verb),
    its emitter (`owner`: the verb's `emitted_by`, another system, which declares the kind in [[emits]]) and why; the
    emitter is one of the declaring system's reads at `same_instant` (the scheduling edge then orders the emitter's
    bindings before the observer's in every phase both bind, 05.1 §2 rule 4, and the identity rule requires the
    emitter in every world that registers the observer); the observer binds a phase at or after one the emitter binds
    (the log is cleared at the instant's end); a kind is observed once; no other field. Errors on every status."""
    verbs = reg.get("verbs", {})
    near = {r.get("owner") for r in s.get("reads", []) if r.get("lag") == "same_instant"}
    mine = [PHASES.index(x.get("phase")) for x in s.get("schedule", []) if x.get("phase") in PHASES]
    for o in s.get("observes", []):
        if not isinstance(o, dict):
            errs.append(f"{p}: an [[observes]] entry must be a table with kind, owner and why (EVO-002)")
            continue
        kind, owner = o.get("kind"), o.get("owner")
        where = f"{p}: [[observes]] {kind}"
        extra = sorted(set(o) - set(OBSERVES_KEYS))
        if extra:
            errs.append(f"{where}: unknown field(s) {extra}; an entry has {', '.join(OBSERVES_KEYS)} (EVO-002)")
        if not o.get("why"):
            errs.append(f"{where} needs a why: what the system reads the kind for (EVO-002)")
        if kind not in verbs:
            errs.append(f"{where} is not a Registry verb ([verbs]): the instant log records known kinds only (EVO-002)")
            continue
        emitter = verbs[kind].get("emitted_by")
        if owner != emitter:
            errs.append(f"{where}: owner {owner} is not its emitter; the Registry gives {kind} to {emitter} (EVO-002)")
            continue
        if owner == sid:
            errs.append(f"{where}: a system does not observe its own kind; a step names its own drafts by DraftRef "
                        "(05.1 §2 rule 4; EVO-002)")
            continue
        if owner in specs and kind not in [e.get("kind") for e in specs[owner][1].get("emits", [])]:
            errs.append(f"{where}: {owner}'s [[emits]] does not declare {kind} (EVO-002)")
        if owner not in near:
            errs.append(f"{where}: {sid} reads none of {owner}'s columns at same_instant; an observed kind's emitter is one "
                        "of the declaring system's reads, so that the scheduling edge orders its bindings first in every phase "
                        "both bind and registration requires it (05.1 §2 rule 4; EVO-002)")
        if owner in specs:
            theirs = [PHASES.index(x.get("phase")) for x in specs[owner][1].get("schedule", []) if x.get("phase") in PHASES]
            if mine and theirs and max(mine) < min(theirs):
                errs.append(f"{where}: {sid} binds no phase at or after one {owner} binds, so it could never read {kind} "
                            "before the instant log is cleared at the instant's end (EVO-002)")


def lint_envelope(p, s, reg, enum_names, errs, warns, frozen=False):
    """The event envelope's declarations (SPR-020; envelope_v0.md §2, §4, §7): [[emits]] `roles`, `motives` and
    `snapshots` against the Registry [event_roles], [motive_edges] and [attributes] and the spec's own [[owns]] and
    [[reads]]; `cause_column` on an [[owns]] state column. A wrong value is an error on any spec; a missing `roles` is a
    warning until SPR-020 closes."""
    role_ids = reg.get("event_roles", {}).get("ids", [])
    motive_ids = reg.get("motive_edges", {}).get("ids", [])
    attrs = reg.get("attributes", {})
    own = {o.get("column"): o for o in s.get("owns", [])}
    readable = set(own) | {r.get("column") for r in s.get("reads", [])}
    algorithms = set(s.get("algorithm", {}))
    all_types = dict(ALL_OWNED_TYPES)
    for o in s.get("owns", []):
        if o.get("type") == "event_ref" and "default" in o:
            errs.append(f"{p}: {o.get('column')} is an event_ref, whose default is none by definition: drop `default` (REV-SPR-020 R26)")
        cc = o.get("cause_column")
        if cc is None:
            continue
        col, entity = o.get("column"), o.get("entity")
        target = own.get(cc)
        if cc == col or target is None:
            errs.append(f"{p}: {col} cause_column {cc} must name another column this spec owns (SAGA-15; envelope_v0.md §7)")
        elif target.get("type") != "event_ref" or target.get("entity") != entity:
            errs.append(f"{p}: {col} cause_column {cc} must be an event_ref (an Option<EventId>, none by default; REV-SPR-020 R17) of the same entity {entity}")
        if "default" not in o:
            errs.append(f"{p}: {col} has a cause_column, so it needs a declared default (the state rh-core compares against; envelope_v0.md §7, REV-SPR-020 R2)")
        else:
            why = default_problem(o.get("default"), o.get("type"), enum_names.get(col))
            if why is not None:
                errs.append(f"{p}: {col} default: {why}")
    for e in s.get("emits", []):
        where = f"{p}: [[emits]] {e.get('kind')}"
        roles = e.get("roles")
        if roles is None:
            (errs if frozen else warns).append(f"{where} declares no roles: the [event_roles] it fills (SPR-020; an error on a frozen spec)")
            roles = []
        elif not (isinstance(roles, list) and all(isinstance(r, str) for r in roles)):
            errs.append(f"{where}: roles must be a list of [event_roles] ids")
            roles = []
        for r in roles:
            if r not in role_ids:
                errs.append(f"{where}: role '{r}' is not in the Registry [event_roles] {role_ids}")
        dup = sorted({r for r in roles if roles.count(r) > 1})
        if dup:
            errs.append(f"{where}: role(s) {dup} listed twice (a role is listed once; only witness may repeat on an event)")
        motives = e.get("motives", [])
        if not (isinstance(motives, list) and all(isinstance(m, str) for m in motives)):
            errs.append(f"{where}: motives must be a list of [motive_edges] ids")
            motives = []
        for m in motives:
            if m not in motive_ids:
                errs.append(f"{where}: motive '{m}' is not in the Registry [motive_edges] {motive_ids}")
        dup_m = sorted({m for m in motives if motives.count(m) > 1})
        if dup_m:
            errs.append(f"{where}: motive(s) {dup_m} listed twice")
        seen_snapshots = set()
        for sn in e.get("snapshots", []):
            if not isinstance(sn, dict):
                errs.append(f"{where}: a snapshot is a table {{ attr, of, [partner], from }}")
                continue
            unknown = sorted(set(sn) - SNAPSHOT_KEYS)
            if unknown:
                errs.append(f"{where}: snapshot has unknown key(s) {unknown}")
            name, of, partner, src = sn.get("attr"), sn.get("of"), sn.get("partner"), sn.get("from")
            a = attrs.get(name)
            if a is None:
                errs.append(f"{where}: snapshot attr '{name}' is not in the Registry [attributes]")
                continue
            args = a.get("args", [])
            sw = f"{where}: snapshot {name}"
            key = (name, of, partner)
            if key in seen_snapshots:
                errs.append(f"{sw} of {of} is declared twice")
            seen_snapshots.add(key)
            if of == "witness" or partner == "witness":
                errs.append(f"{sw}: a snapshot of witness is undefined (a witness is many; envelope_v0.md §4 item 3)")
            if args[:1] == ["event"]:
                if of != "event":
                    errs.append(f"{sw} is an attribute of the event; of must be \"event\"")
            elif of not in roles:
                errs.append(f"{sw}: of = {of!r} is not one of the kind's roles {roles} (the entity it is read of)")
            if len(args) >= 2 and args[1] != "event":
                if partner not in roles:
                    errs.append(f"{sw} takes {args[1]} as a second argument: partner must be one of the kind's roles {roles}")
            elif partner is not None:
                errs.append(f"{sw} has one argument; partner does not apply")
            if any(x == "event" for x in args[1:]) or (args[:1] == ["event"] and len(args) > 1):
                errs.append(f"{sw} names another event as an argument, which has no encoding before the knowledge tables (P1b)")
            if src is None:
                errs.append(f"{sw} needs from = \"<entity.column>\" (an owned or read column) or \"algorithm.<name>\"")
            elif str(src).startswith("algorithm."):
                if str(src)[len("algorithm."):] not in algorithms:
                    errs.append(f"{sw}: from = {src} names no [algorithm.*] table of this spec")
                at = sn.get("type")
                if at is None:
                    errs.append(f"{sw}: from = {src} computes a value, so the snapshot declares its type (a column type; REV-SPR-020 R7)")
                elif at not in SNAPSHOT_SOURCE_TYPES.get(a.get("type"), set()):
                    errs.append(f"{sw} is a {a.get('type')} attribute; from = {src} yields a {at} (envelope_v0.md §4 item 5)")
            elif src not in readable:
                errs.append(f"{sw}: from = {src} is neither owned by this spec nor one of its [[reads]]")
            else:
                ct = own[src].get("type") if src in own else None
                if ct is None:
                    ct = column_type(src, next((r.get("owner") for r in s.get("reads", []) if r.get("column") == src), None), all_types, reg)
                if ct is not None and ct not in SNAPSHOT_SOURCE_TYPES.get(a.get("type"), set()):
                    errs.append(f"{sw} is a {a.get('type')} attribute; from = {src} is a {ct} column (envelope_v0.md §4 item 5)")
                if a.get("type") == "enum":
                    names = enum_codes(own[src], reg)[0] if src in own else enum_names.get(src)
                    if names is not None and names != list(a.get("values", [])):
                        errs.append(f"{sw}: the enum column {src} codes {names} differ from the attribute's values {a.get('values')}")


def brief(sid, specs, reg):
    f, s = specs[sid]
    h, pu, d = s["system"], s["purpose"], s["design"]
    out = [f"# {h['id']} — {h['name']} (fidelity {h['fidelity']}, spec {h['spec_version']}, {h['status']})", "",
           f"**What it does.** {pu['statement']}", "",
           "**What it owns:** " + (", ".join(o["column"] for o in s.get("owns", [])) or "nothing (pure function)"),
           "**What it reads:** " + (", ".join(f"{r['column']} ({r['owner']})" for r in s.get("reads", [])) or "nothing"),
           "**What it emits:** " + (", ".join(e["kind"] for e in s.get("emits", [])) or "no events"),
           "**When it runs:** " + "; ".join(f"{x['pulse']} pulse, {x['phase']} phase" for x in s["schedule"]),
           "**Randomness:** " + ", ".join(f"{g['purpose']} keyed by ({', '.join(g['key'])})" for g in s["rng"]),
           "**Era-state it reads:** " + (", ".join(d.get("era_state", [])) or "none"), "",
           "**Settings (Registry bounds):**"]
    regp = reg["systems"][sid].get("params", {})
    for x in s["params"]:
        lo, hi = regp.get(x["name"], ["?", "?"])
        dflt = f" default {x['default']}" if "default" in x else ""
        out.append(f"- `{x['name']}` [{lo}, {hi}]{dflt}: {x['meaning']} — source: {x['source']}")
    out += ["", "**How we know it works:** " + ", ".join(s["verification"]["tests"]),
            "**Not reproduced yet:** " + ("; ".join(pu.get("not_yet", [])) or "nothing listed")]
    return "\n".join(out)


def envelope_constants(s, reg):
    """The skeleton's envelope constants (SPR-020 T-01; envelope_v0.md §2): one ``ROLE_*`` per [event_roles] id the
    spec's [[emits]] fill, with its code (the index in the Registry's list, read at generation time), one ``ATTR_*`` per
    snapshot attribute and one ``MOTIVE_*`` per motive, so that no system crate types a name; and ``ROLES``, the role
    constants by name, which registration compares with the Registry it runs under (as it compares SCHEDULE)."""
    order = reg.get("event_roles", {}).get("ids", [])
    emits = s.get("emits", [])
    roles = sorted({r for e in emits for r in e.get("roles", []) if r in order}, key=order.index)
    attrs = sorted({x["attr"] for e in emits for x in e.get("snapshots", [])})
    motives = sorted({m for e in emits for m in e.get("motives", [])})
    up = lambda n: re.sub(r"[^A-Za-z0-9]", "_", n).upper()
    lines = [f"pub const ROLE_{up(r)}: rh_core::RoleId = rh_core::RoleId({order.index(r)});" for r in roles]
    lines += ["/// The role constants by Registry name: registration compares each code with the Registry's.",
              "pub const ROLES: &[(&str, rh_core::RoleId)] = &["]
    lines += [f'    ("{r}", ROLE_{up(r)}),' for r in roles]
    lines += ["];"]
    lines += [f'pub const ATTR_{up(a)}: &str = "{a}";' for a in attrs]
    lines += [f'pub const MOTIVE_{up(m)}: &str = "{m}";' for m in motives]
    return lines


def rust(sid, specs, reg):
    f, s = specs[sid]
    camel = lambda n: "".join(w.capitalize() for w in n.split("_"))
    shown = f.resolve().relative_to(ROOT).as_posix() if f.resolve().is_relative_to(ROOT) else f.name
    lines = [f"// GENERATED from {shown} by lint_odd.py --rust. Do not edit; edit the spec.",
             f'pub const SYSTEM_ID: &str = "{sid}";',
             f'pub const SPEC_VERSION: &str = "{s["system"]["spec_version"]}";',
             "pub const SCHEDULE: &[(Pulse, Phase)] = &["]
    lines += [f"    (Pulse::{x['pulse']}, Phase::{x['phase']})," for x in s["schedule"]]
    # DECISION (SPR-014 T-01; REV-EVO-001 NIT-10): one (command, Pulse, Phase) per [[applies]] entry, its drained_by, so
    # registration can compare the code's draining bindings with the spec as it compares SCHEDULE.
    lines += ["];", "pub const DRAINS: &[(&str, Pulse, Phase)] = &["]
    for a in s.get("applies", []):
        pulse, _, phase = str(a.get("drained_by", "")).partition("/")
        lines.append(f'    ("{a["command"]}", Pulse::{pulse}, Phase::{phase}),')
    # DECISION: the enum carries the spec's purpose names, so code never maps variants to names by
    # hand (registration checks the names against [[rng]], SPR-005 T-06). An empty enum takes no
    # #[repr]: Rust refuses a representation for a zero-variant enum. REF: SPR-005 ruling 2.
    lines += ["];"] + (["#[repr(u32)]"] if s["rng"] else [])
    lines += ["#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]", "pub enum RngPurpose {"]
    lines += [f"    {camel(g['purpose'])} = {i + 1},  // key: ({', '.join(g['key'])})" for i, g in enumerate(s["rng"])]
    lines += ["}", "impl RngPurpose {", "    /// The purpose's name in the spec's `[[rng]]` table.",
              "    pub const fn name(self) -> &'static str {", "        match self {"]
    lines += [f'            Self::{camel(g["purpose"])} => "{g["purpose"]}",' for g in s["rng"]]
    lines += ["        }", "    }", "}", "pub struct Params {"]
    # a count parameter (type = "count", SPR-016 MR2) is a u64; every other parameter is q16_16 (Fixed)
    lines += [f"    pub {x['name']}: {'u64' if x.get('type') == 'count' else 'Fixed'},  "
              "// bounds checked against the Registry at load" for x in s["params"]]
    lines += ["}"]
    lines += envelope_constants(s, reg)
    return "\n".join(lines)


def parse_args(argv):
    """Return (specs dir, extra spec dirs, overlay path or None, (mode, id) or None, next dir or None) from
    ``argv``, or raise ValueError."""
    specs_dir, extra, overlay, gen, next_dir = SPECS, [], None, None, None
    i = 0
    while i < len(argv):
        if argv[i] in ("--specs", "--extra-specs", "--overlay", "--brief", "--rust", "--next") and i + 1 < len(argv):
            flag, value = argv[i], argv[i + 1]
            if flag == "--specs":
                specs_dir = Path(value)
            elif flag == "--extra-specs":
                extra.append(value)
            elif flag == "--overlay":
                overlay = value
            elif flag == "--next":
                next_dir = value
            else:
                gen = (flag, value)
            i += 2
        else:
            raise ValueError(f"unknown argument {argv[i]!r}; see the module docstring")
    return specs_dir, extra, overlay, gen, next_dir


def main() -> int:
    try:
        write_lock_flag = LOCK_FLAG in sys.argv[1:]
        specs_dir, extra, overlay, gen, next_dir = parse_args([a for a in sys.argv[1:] if a != LOCK_FLAG])
    except ValueError as e:
        print(e)
        return 2
    reg, specs, req_ids, test_ids, adr_ids, frozen = load(extra, overlay, specs_dir, next_dir)
    CLOSED_SPRINTS.update(closed_sprints())
    KNOWN_SPRINTS.update(known_sprints())
    overlay_errs = []
    if overlay is not None and specs_dir != SPECS:
        overlay_errs = replacement_problems(overlay, specs)
    if gen is not None:
        sid = gen[1]
        if sid not in specs:
            print(f"no spec for {sid}; have {sorted(specs)}")
            return 1
        if overlay_errs:
            print("\n".join(overlay_errs))
            return 1
        print(brief(sid, specs, reg) if gen[0] == "--brief" else rust(sid, specs, reg))
        return 0
    lint_notes = []
    errs, warns = lint(reg, specs, req_ids, test_ids, adr_ids, lint_notes, next_ids=set(frozen))
    for n in lint_notes:
        print("NOTE ", n)
    notes = lint_next(frozen, specs, errs)
    errs = overlay_errs + errs
    # REV-SPR-015 round 3 MIN-1: the amendments are locked by path and text digest, not counted
    digests = {sid: amendment_digests(specs[sid][1]) for sid in frozen}
    if next_dir is not None and frozen:
        lock_path = Path(next_dir) / LOCK_NAME
        if write_lock_flag:
            if errs:
                print("\n".join("ERROR " + e for e in errs))
                print(f"FAIL: {LOCK_NAME} not written: the next version does not lint")
                return 1
            cmd = "python -I specs/systems/lint_odd.py " + " ".join(sys.argv[1:])
            write_lock(lock_path, digests, cmd)
            print(f"WROTE {lock_path}: {sum(len(d) for d in digests.values())} amendments locked")
        else:
            lock = load_lock(lock_path)
            if lock is None:
                errs.append(f"{lock_path}: no amendments lock; a next version is linted against the lock its review approved ({LOCK_FLAG} writes it)")
            else:
                for sid in frozen:
                    compare_lock(sid, digests[sid], lock.get(sid, {}), specs[sid][0].name, errs)
                    notes.append(f"{specs[sid][0].name}: amendments lock: {len(digests[sid])} paths match {lock_path}")
    for note in notes:
        print("AMEND" if " amends " in note else " ", note)
    if errs:
        # REV-SPR-015 round 3 NIT-4: a spec that does not lint gets its diagnostics, not a self-check that may trip on it
        for w in warns:
            print("WARN ", w)
        for e in errs:
            print("ERROR", e)
        print(f"FAIL: {len(specs)} system specs linted ({len(warns)} warnings); the self-checks run on a clean set only")
        return 1
    # self-check (SPR-015; REV-SPR-015 R1, R6): a next version that changes a frozen algorithm step, a frozen column's
    # type, a frozen default, a frozen invariant's statement, a frozen emits `when` or the frozen [initialization]
    # without an [[amends]] record, records an amendment whose frozen text is wrong, drops a sender from a frozen
    # [[applies]] `from`, or puts a new RNG purpose before a frozen one, must be rejected; the clean one must not be
    for sid, (nf, ns) in ((sid, specs[sid]) for sid in frozen):
        clean = []
        lint_next({sid: frozen[sid]}, {sid: (nf, ns)}, clean)
        probes = []
        bad = copy.deepcopy(ns)
        for name in bad.get("algorithm", {}):
            if name in frozen[sid][1].get("algorithm", {}):
                bad["algorithm"][name]["steps"] = ["S0 changed"]
                probes.append((bad, f"frozen [algorithm.{name}] changed"))
                break
        bad = copy.deepcopy(ns)
        bad["invariants"][0]["statement"] = "planted"
        probes.append((bad, "changed its statement without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad["emits"][0]["when"] = "planted"
        probes.append((bad, "changed its `when` without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad["initialization"]["state"] = "planted"
        probes.append((bad, "frozen [initialization] state changed without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad["applies"][0]["from"] = []
        probes.append((bad, "lost a sender from `from`"))
        if ns.get("amends"):
            bad = copy.deepcopy(ns)
            bad["amends"][0]["frozen"] = "planted"
            probes.append((bad, "`frozen` is not the frozen text byte for byte"))
            bad = copy.deepcopy(ns)
            bad["amends"][0]["to"] = "planted"
            probes.append((bad, "`to` is not what the next version says there"))
        # REV-SPR-015 R15: shadowing duplicates, prefix negations, a second same-name step, a dropped step, a changed
        # frozen meaning, a frozen binding's `when`, and an amended algorithm step under an unchanged version
        fz = frozen[sid][1]
        bad = copy.deepcopy(ns)
        bad["invariants"].insert(0, dict(fz["invariants"][0], statement="planted shadow"))
        probes.append((bad, "invariant id"))
        bad = copy.deepcopy(ns)
        bad["emits"].insert(0, dict(fz["emits"][0], when="planted shadow"))
        probes.append((bad, "emitted kind"))
        bad = copy.deepcopy(ns)
        bad["applies"].insert(0, dict(fz["applies"][0], **{"from": []}))
        probes.append((bad, "applied command"))
        bad = copy.deepcopy(ns)
        for e in bad["emits"]:
            if e.get("kind") == fz["emits"][0].get("kind"):
                e["when"] = "NEVER (the text below is void): " + e["when"]
        probes.append((bad, "changed its `when` without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad["schedule"][0]["steps"].append(bad["schedule"][0]["steps"][0].split(":")[0] + ": planted contradiction")
        probes.append((bad, "step name of"))
        bad = copy.deepcopy(ns)
        bad["schedule"][0]["steps"] = bad["schedule"][0]["steps"][1:]
        probes.append((bad, "is missing"))
        bad = copy.deepcopy(ns)
        bad["schedule"][0]["when"] = "never (planted)"
        probes.append((bad, "changed its `when` without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        for o in bad["owns"]:
            if o.get("column") == fz["owns"][-1].get("column"):
                o["meaning"] = "planted"
        probes.append((bad, "changed its meaning without an [[amends]] record"))
        for a in ns.get("amends", []):
            m = AMEND_PATH.match(str(a.get("of", "")))
            if m and m.group("alg"):
                bad = copy.deepcopy(ns)
                bad["algorithm"][m.group("alg")]["version"] = fz["algorithm"][m.group("alg")]["version"]
                probes.append((bad, "an amendment bumps the table's version"))
                break
        bad = copy.deepcopy(ns)
        bad["owns"][0]["type"] = "u8" if bad["owns"][0].get("type") != "u8" else "u16"
        probes.append((bad, "changed type"))
        # SPR-020; REV-SPR-020 R15: a narrowed cast, a narrowed motive list, changed snapshots, a changed default and a
        # changed cause_column of the frozen slice must be rejected (the first two may only grow; the rest need a record)
        cast_kind = next((e for e in fz.get("emits", []) if e.get("roles")), None)
        if cast_kind is not None:
            bad = copy.deepcopy(ns)
            for e in bad["emits"]:
                if e.get("kind") == cast_kind.get("kind"):
                    e["roles"] = list(cast_kind["roles"])[:-1]
            probes.append((bad, "narrowed its roles"))
        motive_kind = next((e for e in fz.get("emits", []) if e.get("motives")), None)
        if motive_kind is not None:
            bad = copy.deepcopy(ns)
            for e in bad["emits"]:
                if e.get("kind") == motive_kind.get("kind"):
                    e["motives"] = list(motive_kind["motives"])[:-1]
            probes.append((bad, "narrowed its motives"))
        bad = copy.deepcopy(ns)
        for e in bad["emits"]:
            if e.get("kind") == fz["emits"][0].get("kind"):
                e["snapshots"] = [{"attr": "planted", "of": "actor", "from": "planted.column"}]
        probes.append((bad, "changed its snapshots without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        for o in bad["owns"]:
            if o.get("column") == fz["owns"][0].get("column"):
                o["default"] = "planted"
        probes.append((bad, "changed its default without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        for o in bad["owns"]:
            if o.get("column") == fz["owns"][0].get("column"):
                o["cause_column"] = "planted.cause"
        probes.append((bad, "changed its cause_column without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad["rng"].insert(0, {"purpose": "planted_first", "key": ["person"]})
        probes.append((bad, "must stay at position 0"))
        bad = frozen_default_probe(fz, ns)
        if bad is not None:
            probes.append((bad, "frozen default of"))
        # REV-SPR-015 round 3 NIT-4 and NIT-5: a purpose's use text, an unknown amend path, a dropped frozen test or
        # coupling, a frozen parameter's meaning
        bad = copy.deepcopy(ns)
        bad["rng"][0]["use"] = "planted"
        probes.append((bad, "changed its use text without an [[amends]] record"))
        bad = copy.deepcopy(ns)
        bad.setdefault("amends", []).append({"of": "planted.path", "frozen": "x", "to": "y", "reason": "planted"})
        probes.append((bad, "is not a path this lint knows"))
        if fz.get("verification", {}).get("tests"):
            bad = copy.deepcopy(ns)
            bad["verification"]["tests"] = [t for t in bad["verification"]["tests"] if t != fz["verification"]["tests"][0]]
            probes.append((bad, "dropped from [verification]"))
        if fz.get("couplings"):
            bad = copy.deepcopy(ns)
            drop = (fz["couplings"][0].get("system"), fz["couplings"][0].get("direction"))
            bad["couplings"] = [c for c in bad["couplings"] if (c.get("system"), c.get("direction")) != drop]
            probes.append((bad, "dropped from [[couplings]]"))
        if fz.get("params"):
            bad = copy.deepcopy(ns)
            for x in bad["params"]:
                if x.get("name") == fz["params"][0].get("name"):
                    x["meaning"] = "planted"
            probes.append((bad, "changed its meaning without an [[amends]] record"))
        first_frozen_alg = next(iter(fz.get("algorithm", {})), None)
        if first_frozen_alg:
            fsteps = fz["algorithm"][first_frozen_alg]["steps"]
            bad = copy.deepcopy(ns)
            bad["algorithm"]["planted_table"] = {"version": "planted", "steps": [fsteps[-1].split(None, 1)[0] + " planted override"]}
            probes.append((bad, "reuses a frozen step id"))
            bad = copy.deepcopy(ns)
            bad["algorithm"]["planted_table"] = {"version": "planted", "steps": ["Z1 Planted. This step replaces " + step_label(fsteps[-1]) + " entirely."]}
            probes.append((bad, "as replaced"))
        # deny by default (R3): a planted change on each generic path class must be rejected
        fzs = frozen[sid][1]
        generic_plants = []
        if fzs.get("submodels"):
            generic_plants.append(("submodels", 0, "summary"))
        if fzs.get("commands"):
            generic_plants.append(("commands", 0, "why"))
        if fzs.get("couplings"):
            generic_plants.append(("couplings", 0, "via"))
        if fzs.get("params"):
            generic_plants.append(("params", 0, "source"))
        if fzs.get("fidelity_ladder"):
            generic_plants.append(("fidelity_ladder", 0, "representation"))
        for sec, i, field in generic_plants:
            bad = copy.deepcopy(ns)
            target_key = keyed_paths(fzs[sec], sec)[i][0]
            for k, x in keyed_paths(bad.get(sec, []), sec):
                if k == target_key:
                    x[field] = "planted " + str(x.get(field, ""))
            probes.append((bad, f"frozen {sec}."))
        for sec, field in (("purpose", "statement"), ("isolation", "expect"), ("design", "basic_principles"), ("scales", "extent"), ("verification", "prototype"), ("system", "requirements"), ("initialization", "from_manifest")):
            if field in fzs.get(sec, {}):
                bad = copy.deepcopy(ns)
                bad[sec][field] = ["planted"] if isinstance(fzs[sec][field], list) else "planted " + str(bad[sec].get(field, ""))
                probes.append((bad, f"frozen {sec}.{field}"))
        # N4: an enum code renamed, an [[applies]] drained_by changed, a [[reads]] lag changed
        enum_col = next((o.get("column") for o in fzs.get("owns", []) if o.get("values")), None)
        if enum_col:
            bad = copy.deepcopy(ns)
            for o in bad["owns"]:
                if o.get("column") == enum_col:
                    o["values"] = ["planted", *o["values"][1:]]
            probes.append((bad, f"frozen owns.{enum_col}.values changed"))
        if fzs.get("applies"):
            bad = copy.deepcopy(ns)
            target = keyed_paths(fzs["applies"], "applies")[0][0]
            for k, x in keyed_paths(bad.get("applies", []), "applies"):
                if k == target:
                    x["drained_by"] = "Year/Polity"
            probes.append((bad, f"frozen applies.{target}.drained_by"))
        if fzs.get("reads"):
            bad = copy.deepcopy(ns)
            target = keyed_paths(fzs["reads"], "reads")[0][0]
            for k, x in keyed_paths(bad.get("reads", []), "reads"):
                if k == target:
                    x["lag"] = "planted"
            probes.append((bad, f"frozen reads.{target}.lag"))
        for probe, needle in probes:
            pe = []
            lint_duplicate_keys(nf.name, probe, pe)
            lint_next({sid: frozen[sid]}, {sid: (nf, probe)}, pe)
            # a planted change at a path the next version already amends is caught by the record's `to` check instead
            assert any(needle in e or "`to` is not what the next version says there" in e for e in pe), (sid, needle, pe)
        # the amendments lock (REV-SPR-015 round 3 MIN-1): the reviewer's probe keeps the COUNT of amendments (a valid
        # record amends a frozen arithmetic step while one meaning amendment is withdrawn) and lint_next accepts it; the
        # lock must reject it naming both paths; and a changed text or a dropped path alone must be rejected too
        want = amendment_digests(ns)
        lock_probes = 0
        meaning_of = next((of for of in want if of.startswith("owns.")), None)
        if first_frozen_alg and meaning_of:
            fsteps = fz["algorithm"][first_frozen_alg]["steps"]
            target = next(st for st in fsteps if step_label(st) not in {step_label(x) for x in ns["algorithm"][first_frozen_alg]["steps"] if x not in fsteps})
            bad = copy.deepcopy(ns)
            bad["algorithm"][first_frozen_alg]["steps"] = [step_label(target) + " planted arithmetic" if st == target else st for st in bad["algorithm"][first_frozen_alg]["steps"]]
            bad["amends"] = [a for a in bad["amends"] if a.get("of") != meaning_of]
            bad["amends"].append({"of": f"algorithm.{first_frozen_alg}.steps.{step_label(target)}", "frozen": target, "to": step_label(target) + " planted arithmetic", "reason": "planted"})
            col = meaning_of[len("owns."):-len(".meaning")]
            for o in bad["owns"]:
                if o.get("column") == col:
                    o["meaning"] = next(fo["meaning"] for fo in fz["owns"] if fo.get("column") == col)
            pe = []
            lint_next({sid: frozen[sid]}, {sid: (nf, bad)}, pe)
            # beyond whatever the spec under test already fails on, the probe adds nothing lint_next can see
            assert not [e for e in pe if e not in clean], ("the count-preserving probe should pass lint_next; the lock is what rejects it", pe)
            got = amendment_digests(bad)
            assert len(got) == len(want), (len(got), len(want))
            pe = []
            compare_lock(sid, got, want, nf.name, pe)
            assert any("is not in" in e and f"steps.{step_label(target)}" in e for e in pe), pe
            assert any("no longer makes" in e and meaning_of in e for e in pe), pe
            lock_probes += 1
        # REV-SPR-020 R26 (NIT-7): a next version with no amendment has nothing to plant in the lock
        for probe_lock, needle in ((({**want, next(iter(want)): "sha256:planted"}, "changed its text since"),
                                    ({k: v for k, v in list(want.items())[1:]}, "is not in")) if want else ()):
            pe = []
            compare_lock(sid, want, probe_lock, nf.name, pe)
            assert any(needle in e for e in pe), (needle, pe)
            lock_probes += 1
        recorded = sum(1 for n in notes if n.startswith(f"{nf.name}: amends "))
        locked = any(n.startswith(f"{nf.name}: amendments lock:") for n in notes)
        print(f"  self-check (next of {sid}): {len(probes) + lock_probes} planted changes rejected ({lock_probes} of them by the amendments lock), "
              f"{recorded} amendments recorded{' and locked' if locked else ''}, the clean next version {'kept' if not clean else 'REJECTED'}")
    for w in warns:
        print("WARN ", w)
    for e in errs:
        print("ERROR", e)
    # self-check: a broken spec (wrong fidelity, a column M2 already owns, an unregistered parameter, a bad RNG
    # purpose, missing sections) must be rejected
    bad = (Path("bad.odd.toml"), {"odd_r": 2, "system": {"id": "S5.daily_life", "fidelity": "F9", "status": "frozen"},
           "owns": [{"column": "person.alive", "type": "bool", "plane": "A"}],
           "reads": [{"column": "region.conditions", "owner": "M6.polity", "lag": "start_of_day", "use": "rate"}],
           "params": [{"name": "nonsense", "source": ""}], "rng": [{"purpose": "Bad Name"}]})
    probe = {"M2.demography": specs["M2.demography"], "S5.daily_life": bad} if "M2.demography" in specs else {"S5.daily_life": bad}
    b_errs, _ = lint(reg, probe, req_ids, test_ids, adr_ids)
    caught = [e for e in b_errs if "bad.odd.toml" in e or "also owned" in e]
    for needle in ("not among Registry levels", "snake_case", "not declared in the Registry", "missing section",
                   "needs an in-coupling", "missing [isolation]"):
        assert any(needle in e for e in caught), (needle, caught)
    if "M2.demography" in specs:
        assert any("also owned" in e for e in caught), caught
    # self-check (EVO-001): the absent-owner rules on [[reads]], enum codes on [[owns]], typed [[applies]] entries and
    # the outcome timer on [[commands]]. frozen -> error, draft -> warning for the declarations; a wrong value is always
    # an error.
    if "M2.demography" in specs:
        base = {"odd_r": 2, "system": {"id": "S5.daily_life", "fidelity": "F1"}, "purpose": {"role": "substrate"}, "owns": [], "params": [], "rng": [{"purpose": "fix", "key": ["person"]}],
                "schedule": [{"pulse": "Day", "phase": "Activity", "steps": ["x: y"]}]}
        rd = lambda col, owner, use, **kw: dict({"column": col, "owner": owner, "lag": "lagged", "use": use}, **kw)
        ap = lambda **kw: dict({"command": "X", "payload": "{ a: PersonId }", "from": [], "drained_by": "Day/Activity", "key": "living", "key_field": "a"}, **kw)
        M2 = "M2.demography"
        probes = [   # (status, reads, applies, commands, needle, want_err)
            ("draft", [rd("person.alive", M2, "identity", neutral=False)], [], [], "is an identity read, which requires its owner and declares no neutral", True),
            ("draft", [rd("person.alive", M2, "identity", required=True)], [], [], "`required` is not a field", True),
            ("frozen", [rd("polity.exec_constraint", "M6.polity", "rate")], [], [], "needs a neutral value", True),
            ("draft", [rd("person.death_cause", M2, "mode")], [], [], "needs a neutral value", False),
            ("draft", [rd("person.death_cause", M2, "mode", neutral=0)], [], [], "must be a value name for a enum column", True),
            ("draft", [rd("person.death_tick", M2, "mode", neutral="0")], [], [], "must be a TOML integer for a i64 column", True),
            ("draft", [rd("person.mother", M2, "mode", neutral=-5)], [], [], "is outside u64's range", True),
            ("draft", [rd("person.sex", M2, "mode", neutral="not_a_sex")], [], [], "is not one of the enum's codes", True),
            ("draft", [rd("person.death_cause", M2, "mode", neutral="none")], [], [], "", False),
            ("draft", [rd("polity.exec_constraint", "M6.polity", "mode", neutral=Decimal("1e9"))], [], [], "is outside q8_8's range", True),
            ("draft", [rd("polity.exec_constraint", "M6.polity", "mode", neutral=Decimal("0.001"))], [], [], "is not representable in q8_8", True),
            ("draft", [rd("polity.exec_constraint", "M6.polity", "mode", neutral=Decimal("127.999"))], [], [], "is not representable in q8_8", True),
            ("draft", [rd("polity.exec_constraint", "M6.polity", "mode", neutral="zero")], [], [], "must be a TOML number for a q8_8 column", True),
            ("frozen", [rd("region.conditions", "M6.polity", "mode", neutral=0)], [], [], "cannot be typed", True),
            ("draft", [rd("region.conditions", "M6.polity", "mode", neutral=0)], [], [], "cannot be typed", False),
            ("frozen", [], [ap(payload="{ a, b }")], [], "payload must be typed", True),
            ("frozen", [], [ap(payload="{ a: PersonId, b: Potato }")], [], "field b: type 'Potato' is not in the payload vocabulary", True),
            ("frozen", [], [ap(payload="{ a: PersonId, c: enum(person.death_cause), e: EventRef }")], [], "", False),
            ("frozen", [], [ap(payload="{ a: PersonId, e: enum(person.nope) }")], [], "names no enum column with codes", True),
            ("draft", [], [ap(payload="{ a: PersonId, cause_event: u64 }")], [], "field cause_event must be an EventRef", True),
            ("draft", [], [ap(payload="{ a: PersonId, a: u64 }")], [], "field a is repeated", True),
            ("frozen", [], [dict(ap(), drained_by=None)], [], "needs drained_by", True),
            ("frozen", [], [ap(drained_by="Day/Agenda")], [], "is drained_by Day/Agenda, which [[schedule]] does not declare", True),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="SPR-015")], [], "ESCAPE [[applies]] X is drained_by Day/Agenda, which [[schedule]] does not declare until SPR-015", False),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="SPR-999")], [], "sprint SPR-999 names no sprint: no document under CODEX/05_PROJECT and no PRJ row", True),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="SPR-006")], [], "(SPR-006 is CLOSED: declare the binding)", True),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="P1.13 (SPR-014)")], [], "sprint must be \"SPR-NNN\"", True),
            ("frozen", [], [dict(ap(), key=None)], [], "needs key = ", True),
            ("draft", [], [ap(key="maybe")], [], "key must be", True),
            ("frozen", [], [dict(ap(), key_field=None)], [], "needs key_field", True),
            ("draft", [], [ap(payload="{ a: PersonId, n: u64 }", key_field="n")], [], "key_field n is not a PersonId field", True),
            ("frozen", [], [ap(payload="{ row: i64 }", key_field="person")], [], "", False),
            ("draft", [], [ap(payload="{ person: u64 }", key_field="person")], [], "key_field person is not a PersonId field", True),
            ("frozen", [], [ap(), ap(drained_by="Day/Activity")], [], "is declared twice", True),
            ("draft", [], [ap(outcome="yes")], [], "outcome must be true or false", True),
            ("frozen", [], [ap(outcome=True)], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }"}], "carries an outcome, so it needs timer", True),
            ("frozen", [], [ap(outcome=True)], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "nope"}], "needs purpose = one of its [[rng]] purposes", True),
            ("frozen", [], [ap(outcome=True, **{"from": ["S5.daily_life"]})], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "fix"}], "", False),
            ("frozen", [], [ap(outcome=True, drained_by="Day/Agenda", sprint="SPR-015", **{"from": ["S5.daily_life"]})], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "fix"}], "in the timer pass of Day/Agenda, which S5.daily_life does not bind", True),
            ("frozen", [], [ap(**{"from": ["S5.daily_life"]})], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "fix"}], "names a timer, but the owner's [[applies]] says it carries no outcome", False),
            ("frozen", [rd("title.holder", "M7.dynasty", "identity")], [], [], "a substrate reads title.holder from M7.dynasty as identity", False),
            ("draft", [rd("title.holder", "M7.dynasty", "identity")], [], [], "a substrate reads title.holder from M7.dynasty as identity", False),
            ("frozen", [rd("person.alive", M2, "identity")], [], [], "", False),
        ]
        for status, reads, applies, commands, needle, want_err in probes:
            probe_spec = dict(base, system=dict(base["system"], status=status), reads=reads, applies=applies, commands=commands)
            CLOSED_SPRINTS.add("SPR-006")
            KNOWN_SPRINTS.update({"SPR-006", "SPR-014", "SPR-015"})
            pe, pw = lint(reg, {M2: specs[M2], "M7.dynasty": specs["M7.dynasty"], "S5.daily_life": (Path("probe.odd.toml"), probe_spec)}, req_ids, test_ids, adr_ids) if "M7.dynasty" in specs else lint(reg, {M2: specs[M2], "S5.daily_life": (Path("probe.odd.toml"), probe_spec)}, req_ids, test_ids, adr_ids)
            pe = [e for e in pe if e.startswith("probe.odd.toml")]
            pw = [e for e in pw if e.startswith("probe.odd.toml")]
            mine = lambda xs: [e for e in xs if "neutral" in e or "required" in e or "[[applies]]" in e or "[[commands]]" in e or "substrate" in e]
            if needle:
                assert any(needle in e for e in (pe if want_err else pw)), (status, needle, pe, pw)
                assert not any(needle in e for e in (pw if want_err else pe)), (status, needle, pe, pw)
            else:
                assert not mine(pe) and not mine(pw), (status, reads, applies, commands, mine(pe), mine(pw))
        # an enum column without codes is reported at its owner; a rate read's neutral must be stated by its coupling as
        # `<column> = <value>` (a number elsewhere in the prose does not count: NIT-8)
        cpl = lambda prose: [{"system": "M6.polity", "direction": "in", "column": "polity.exec_constraint", "form": "f", "neutral": prose,
                              "output_bounds": ["0", "1"], "data_range": "d", "beyond_range": "b", "source": "s"}]
        probe_spec = dict(base, system=dict(base["system"], status="frozen"),
                          owns=[{"column": "mood.state", "entity": "mood", "type": "enum", "plane": "A"}],
                          reads=[rd("polity.exec_constraint", "M6.polity", "rate", neutral=1)], couplings=cpl("m_ec = 1 + 0 x 1, so 1"))
        pe, _ = lint(reg, {M2: specs[M2], "S5.daily_life": (Path("probe.odd.toml"), probe_spec)}, req_ids, test_ids, adr_ids)
        for needle in ("enum column mood.state needs exactly one of values", "its neutral prose does not state exec_constraint = 1"):
            assert any(needle in e for e in pe), (needle, pe)
        probe_spec = dict(probe_spec, owns=[], couplings=cpl("exec_constraint = 1, an assembly"))
        pe, _ = lint(reg, {M2: specs[M2], "S5.daily_life": (Path("probe.odd.toml"), probe_spec)}, req_ids, test_ids, adr_ids)
        assert not any("neutral prose" in e for e in pe), pe
    print(f"  self-check (broken spec rejected): {len(caught)} errors, e.g. {caught[0]}")
    print(f"{'PASS' if not errs else 'FAIL'}: {len(specs)} system specs linted ({len(warns)} warnings)")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())
