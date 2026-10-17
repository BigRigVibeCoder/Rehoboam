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
    spec's system, and every verb the Registry gives to a system with a spec is in that spec's [[emits]].
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
    (NOTE lines), and a substrate (`[purpose] role = "substrate"`, a system that must run alone: M2 on `solo`) may
    read as identity only another substrate's column, so it never requires a system a smaller world lacks,
  - enum codes (EVO-001): an enum column of [[owns]] carries its codes, `values = [...]` (code = position) or
    `registry = "<table>"` (the Registry table's `ids` list, else its keys in byte-wise ascending order, as verb_id),
  - lifecycle commands, typed (EVO-001): each [[applies]] entry (one per command kind) has a payload `{ name: Type, ... }`
    in a closed vocabulary (PersonId, EventRef, SimTime, the id types, u8..u64, i64, bool, q8_8, q16_16,
    enum(<entity.column>) naming an enum column with codes; a field named *_event is an EventRef; no name repeats);
    `drained_by = "<Pulse>/<Phase>"` names the one binding of [[schedule]] that takes the command (an escape: with
    `sprint = "SPR-NNN"` an undeclared binding is a counted warning until that sprint's document is CLOSED, then an
    error on a frozen spec); `key` is "living" (the dead-person drop applies, 05.1 §2 rule 2) or "any", stated;
    `key_field` names the PersonId payload field the command is keyed by; `outcome = true` marks a command that
    carries a sampled, dated outcome, and every sender's [[commands]] entry for it then names the sender's own
    `timer` kind and the [[rng]] `purpose` it is re-sampled under (05.1 §2 rule 2: the sender's timer submits it),
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
"""
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
TYPES = {"i64", "u64", "i32", "u32", "u16", "u8", "bool", "enum", "q8_8", "q16_16", "f32_field"}
SNAKE = re.compile(r"^[a-z][a-z0-9_]*$")
READ_USES = ("identity", "mode", "rate")
IN_COUPLING_FIELDS = ("form", "neutral", "output_bounds", "data_range", "beyond_range", "source")
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


def load(extra_dirs=(), overlay=None, specs_dir=SPECS):
    reg = tomllib.loads(REGISTRY.read_text(encoding="utf-8"), parse_float=Decimal)
    if overlay is not None:                       # a world's own systems and verbs (SPR-011 ruling 11)
        over = tomllib.loads(Path(overlay).read_text(encoding="utf-8"), parse_float=Decimal)
        unknown = sorted(set(over) - {"systems", "verbs"})
        if unknown:
            raise SystemExit(f"overlay {overlay}: only [systems] and [verbs] may be overlaid, found {unknown}")
        for sec in ("systems", "verbs"):
            clash = sorted(set(over.get(sec, {})) & set(reg.get(sec, {})))
            if clash:
                raise SystemExit(f"overlay {overlay}: [{sec}] entries already in the Registry: {clash}")
            reg.setdefault(sec, {}).update(over.get(sec, {}))
    specs = {}
    files = sorted(p for p in Path(specs_dir).glob("*.odd.toml") if not p.name.startswith("_"))   # _template is not a spec
    for d in extra_dirs:
        files += sorted(p for p in Path(d).glob("*.odd.toml") if not p.name.startswith("_"))
    for f in files:
        s = tomllib.loads(f.read_text(encoding="utf-8"), parse_float=Decimal)   # exact decimals (SPR-011 ruling 9)
        sid = s.get("system", {}).get("id", f.stem)
        if sid in specs:
            raise SystemExit(f"{f}: system {sid} already has a spec ({specs[sid][0]})")
        specs[sid] = (f, s)
    plan03 = (ROOT / "docs" / "03_requirements.md").read_text(encoding="utf-8")
    plan09 = (ROOT / "docs" / "09_verification_and_validation.md").read_text(encoding="utf-8")
    req_ids = set(re.findall(r"^\| ([A-Z][A-Z0-9]*-\d+) \|", plan03, re.M))
    test_ids = set(re.findall(r"^\| (V-[A-Z0-9]+-\d+[a-z]?) \|", plan09, re.M))
    adr_ids = {p.name[:8] for p in (ROOT / "docs" / "06_adr").glob("ADR-*.md")}
    return reg, specs, req_ids, test_ids, adr_ids


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


def lint_applies(specs, errs, warns, enum_names=None):
    """[[applies]] against the senders' [[commands]] (PR #32; REV-P1a §7), and each entry's typed payload, drained_by
    and key (EVO-001)."""
    enum_names = enum_names or {}
    for tid, (tf, ts) in specs.items():
        strict = errs if ts.get("system", {}).get("status") == "frozen" else warns
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
                strict.append(f"{tf.name}: [[applies]] {cmd} needs key_field = \"<a PersonId payload field>\", the person it is keyed by (EVO-001)")
            elif typed is not None and (kf, "PersonId") not in typed:
                errs.append(f"{tf.name}: [[applies]] {cmd} key_field {kf} is not a PersonId field of the payload")
            if not isinstance(a.get("outcome", False), bool):
                errs.append(f"{tf.name}: [[applies]] {cmd} outcome must be true or false")
            elif a.get("outcome"):
                for sid, (sf, ss) in specs.items():
                    for c in ss.get("commands", []):
                        if c.get("target") == tid and c.get("command") == cmd:
                            sp = errs if ss.get("system", {}).get("status") == "frozen" else warns
                            timer, purpose = c.get("timer"), c.get("purpose")
                            if not (isinstance(timer, str) and TIMER_KIND.match(timer)):
                                sp.append(f"{sf.name}: [[commands]] {cmd} to {tid} carries an outcome, so it needs timer = \"<KIND>\", the sender's own timer that submits it (EVO-001)")
                            purposes = {g.get("purpose") for g in ss.get("rng", [])}
                            if purpose not in purposes:
                                sp.append(f"{sf.name}: [[commands]] {cmd} to {tid} carries an outcome, so it needs purpose = one of its [[rng]] purposes, under which its timer is re-sampled (EVO-001)")
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
                sp = errs if specs[s][1].get("system", {}).get("status") == "frozen" else warns
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


def lint(reg, specs, req_ids, test_ids, adr_ids, notes=None):
    errs, warns = [], []
    notes = notes if notes is not None else []
    roles = {sid: s.get("purpose", {}).get("role") for sid, (_, s) in specs.items()}
    owners_type = {o.get("column"): o.get("type") for _, (_, s) in specs.items() for o in s.get("owns", [])}
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
                    errs.append(f"{p}: reads {col} from {owner}, but {owners[col][0]} owns it")
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
                strict.append(f"{p}: a substrate reads {col} from {owner} as identity, which requires {owner}"
                              + (" (not a substrate)" if owner in roles else " (no spec, so its role is unknown)")
                              + "; a substrate must run alone, so read it as rate or mode with a neutral (EVO-001 R12)")
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
        # defaults (SPR-011 ruling 8): the spec mirrors the Registry's defaults exactly, both ways
        reg_defaults = sysdef.get("defaults", {})
        for x in s.get("params", []):
            n, d = x.get("name"), x.get("default")
            if d is not None and reg_defaults.get(n) != d:
                errs.append(f"{p}: parameter '{n}' default {d} differs from the Registry's ({reg_defaults.get(n)})")
        spec_defaults = {x.get("name") for x in s.get("params", []) if "default" in x}
        for n in sorted(set(reg_defaults) - spec_defaults):
            errs.append(f"{p}: Registry default for '{n}' ({reg_defaults[n]}) is not mirrored as `default` in [[params]]")
        # event vocabulary (SPR-011 ruling 11): [[emits]] against the Registry's [verbs], both ways
        verbs = reg.get("verbs", {})
        emitted = [e.get("kind") for e in s.get("emits", [])]
        for k in emitted:
            if k not in verbs:
                strict.append(f"{p}: emits {k}, which is not a Registry verb ([verbs]; SPR-011 ruling 11)")
            elif verbs[k].get("emitted_by") != sid:
                strict.append(f"{p}: emits {k}, which the Registry gives to {verbs[k].get('emitted_by')}")
        for k in sorted(v for v, d in verbs.items() if d.get("emitted_by") == sid and v not in emitted):
            strict.append(f"{p}: the Registry gives verb {k} to {sid}, but [[emits]] does not declare it")
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
    lint_applies(specs, errs, warns, enum_names)
    return errs, warns


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


def rust(sid, specs):
    f, s = specs[sid]
    camel = lambda n: "".join(w.capitalize() for w in n.split("_"))
    shown = f.resolve().relative_to(ROOT).as_posix() if f.resolve().is_relative_to(ROOT) else f.name
    lines = [f"// GENERATED from {shown} by lint_odd.py --rust. Do not edit; edit the spec.",
             f'pub const SYSTEM_ID: &str = "{sid}";',
             f'pub const SPEC_VERSION: &str = "{s["system"]["spec_version"]}";',
             "pub const SCHEDULE: &[(Pulse, Phase)] = &["]
    lines += [f"    (Pulse::{x['pulse']}, Phase::{x['phase']})," for x in s["schedule"]]
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
    lines += [f"    pub {x['name']}: Fixed,  // bounds checked against the Registry at load" for x in s["params"]]
    lines += ["}"]
    return "\n".join(lines)


def parse_args(argv):
    """Return (specs dir, extra spec dirs, overlay path or None, (mode, id) or None) from ``argv``, or raise
    ValueError."""
    specs_dir, extra, overlay, gen = SPECS, [], None, None
    i = 0
    while i < len(argv):
        if argv[i] in ("--specs", "--extra-specs", "--overlay", "--brief", "--rust") and i + 1 < len(argv):
            flag, value = argv[i], argv[i + 1]
            if flag == "--specs":
                specs_dir = Path(value)
            elif flag == "--extra-specs":
                extra.append(value)
            elif flag == "--overlay":
                overlay = value
            else:
                gen = (flag, value)
            i += 2
        else:
            raise ValueError(f"unknown argument {argv[i]!r}; see the module docstring")
    return specs_dir, extra, overlay, gen


def main() -> int:
    try:
        specs_dir, extra, overlay, gen = parse_args(sys.argv[1:])
    except ValueError as e:
        print(e)
        return 2
    reg, specs, req_ids, test_ids, adr_ids = load(extra, overlay, specs_dir)
    CLOSED_SPRINTS.update(closed_sprints())
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
        print(brief(sid, specs, reg) if gen[0] == "--brief" else rust(sid, specs))
        return 0
    notes = []
    errs, warns = lint(reg, specs, req_ids, test_ids, adr_ids, notes)
    errs = overlay_errs + errs
    for n in notes:
        print("NOTE ", n)
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
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="SPR-999")], [], "ESCAPE [[applies]] X is drained_by Day/Agenda, which [[schedule]] does not declare until SPR-999", False),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="SPR-006")], [], "(SPR-006 is CLOSED: declare the binding)", True),
            ("frozen", [], [ap(drained_by="Day/Agenda", sprint="P1.13 (SPR-014)")], [], "sprint must be \"SPR-NNN\"", True),
            ("frozen", [], [dict(ap(), key=None)], [], "needs key = ", True),
            ("draft", [], [ap(key="maybe")], [], "key must be", True),
            ("frozen", [], [dict(ap(), key_field=None)], [], "needs key_field", True),
            ("draft", [], [ap(payload="{ a: PersonId, n: u64 }", key_field="n")], [], "key_field n is not a PersonId field", True),
            ("frozen", [], [ap(), ap(drained_by="Day/Activity")], [], "is declared twice", True),
            ("draft", [], [ap(outcome="yes")], [], "outcome must be true or false", True),
            ("frozen", [], [ap(outcome=True)], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }"}], "carries an outcome, so it needs timer", True),
            ("frozen", [], [ap(outcome=True)], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "nope"}], "needs purpose = one of its [[rng]] purposes", True),
            ("frozen", [], [ap(outcome=True, **{"from": ["S5.daily_life"]})], [{"target": "S5.daily_life", "command": "X", "payload": "{ a }", "timer": "X_DUE", "purpose": "fix"}], "", False),
            ("frozen", [rd("title.holder", "M7.dynasty", "identity")], [], [], "a substrate reads title.holder from M7.dynasty as identity", True),
            ("draft", [rd("title.holder", "M7.dynasty", "identity")], [], [], "a substrate reads title.holder from M7.dynasty as identity", False),
            ("frozen", [rd("person.alive", M2, "identity")], [], [], "", False),
        ]
        for status, reads, applies, commands, needle, want_err in probes:
            probe_spec = dict(base, system=dict(base["system"], status=status), reads=reads, applies=applies, commands=commands)
            CLOSED_SPRINTS.add("SPR-006")
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
