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
  - neutral values (EVO-001; REV-P1a NEW-4; REV-SPR-017 round 3): every [[reads]] entry whose owner is another system
    carries a machine-readable `neutral`, the value the step view returns when a world does not register the owner
    (05.1 §2 rule 2, the absent-owner rule) and the value the Registry effect language's loader uses. It is typed by the
    column's type: the owner's [[owns]] `type`, or the Registry [era_state] entry for an owner with no spec (a TOML
    integer for an integer type, a boolean for bool, a value name for enum, a number for q8_8, q16_16 and f32_field).
    A missing neutral, or one that cannot be typed, is a warning on a draft spec and an error on a frozen one; a
    mistyped one is always an error,
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
SERIES = ("constant", "step", "sine", "ramp", "historical")


OVERLAY_ID = re.compile(r"^X\d+\.[a-z][a-z0-9_]*$")


def replacement_problems(overlay_path, specs):
    """With --specs (a spec set outside the Registry, SPR-008 ruling 2): every overlay system id is
    X-prefixed, and every spec is one of the overlay's systems. ``load`` has already refused an overlay
    entry that repeats a Registry name."""
    overlay = tomllib.loads(Path(overlay_path).read_text(encoding="utf-8"))
    ids = set(overlay.get("systems", {}))
    problems = [f"{overlay_path}: overlay system '{sid}' must be X-prefixed (X<digits>.<snake_name>)"
                for sid in sorted(ids) if not OVERLAY_ID.match(sid)]
    problems += [f"{f.name}: system '{sid}' is not in the overlay {overlay_path}; a spec linted with an "
                 "overlay must be one of its systems" for sid, (f, _) in specs.items() if sid not in ids]
    return problems


def load(extra_dirs=(), overlay=None, specs_dir=SPECS):
    reg = tomllib.loads(REGISTRY.read_text(encoding="utf-8"))
    if overlay is not None:                       # a world's own systems and verbs (SPR-011 ruling 11)
        over = tomllib.loads(Path(overlay).read_text(encoding="utf-8"))
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
        s = tomllib.loads(f.read_text(encoding="utf-8"))
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
    """Field names of a payload written `{ name: Type, name: Type }` (types may hold `|` and `or`, never commas)."""
    inner = str(text).strip()
    if not (inner.startswith("{") and inner.endswith("}")):
        return None
    return [f.split(":")[0].strip() for f in inner[1:-1].split(",") if f.strip()]


def lint_applies(specs, errs, warns):
    """[[applies]] against the senders' [[commands]] (PR #32; REV-P1a §7)."""
    for tid, (tf, ts) in specs.items():
        strict = errs if ts.get("system", {}).get("status") == "frozen" else warns
        applies = {a.get("command"): a for a in ts.get("applies", [])}
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


def neutral_problem(value, col_type):
    """Why ``value`` is not a well-typed `neutral` for a column of ``col_type`` (EVO-001), or None."""
    if col_type in INTEGER_TYPES:
        ok, want = isinstance(value, int) and not isinstance(value, bool), "a TOML integer"
    elif col_type == "bool":
        ok, want = isinstance(value, bool), "true or false"
    elif col_type == "enum":
        ok, want = isinstance(value, str) and bool(SNAKE.match(value)), "a snake_case value name"
    elif col_type in NUMBER_TYPES:
        ok, want = isinstance(value, (int, float)) and not isinstance(value, bool), "a TOML number"
    else:
        return f"column type '{col_type}' has no neutral form"
    return None if ok else f"neutral {value!r} must be {want} for a {col_type} column"


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


def lint(reg, specs, req_ids, test_ids, adr_ids):
    errs, warns = [], []
    owners_type = {o.get("column"): o.get("type") for _, (_, s) in specs.items() for o in s.get("owns", [])}
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
        # neutral values (EVO-001): a foreign read carries the value the step view returns when the owner is absent
        for r in s.get("reads", []):
            col, owner = r.get("column"), r.get("owner")
            if owner == sid:
                continue
            if "neutral" not in r:
                strict.append(f"{p}: read {col} from {owner} needs a neutral value, typed by the column (EVO-001; 05.1 §2 rule 2)")
                continue
            ct = column_type(col, owner, owners_type, reg)
            if ct is None:
                strict.append(f"{p}: neutral of {col} cannot be typed: {owner} has no spec declaring it and the Registry "
                              "[era_state] has no entry for it (EVO-001)")
            elif (why := neutral_problem(r.get("neutral"), ct)) is not None:
                errs.append(f"{p}: read {col} from {owner}: {why} (EVO-001)")
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
    lint_applies(specs, errs, warns)
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
    errs, warns = lint(reg, specs, req_ids, test_ids, adr_ids)
    errs = overlay_errs + errs
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
                   "needs an in-coupling", "missing [isolation]", "needs a neutral value"):
        assert any(needle in e for e in caught), (needle, caught)
    if "M2.demography" in specs:
        assert any("also owned" in e for e in caught), caught
    # self-check (EVO-001): a frozen spec with a foreign read and no neutral, or one that cannot be typed, is an error
    # and a draft one warns; a mistyped neutral is always an error; the Registry [era_state] types an owner without a spec
    if "M2.demography" in specs:
        base = {"odd_r": 2, "system": {"id": "S5.daily_life", "fidelity": "F1"}, "owns": [], "params": [], "rng": []}
        probes = [
            ("frozen", [{"column": "person.alive", "owner": "M2.demography", "lag": "lagged", "use": "identity"}], "needs a neutral value", True),
            ("draft", [{"column": "person.alive", "owner": "M2.demography", "lag": "lagged", "use": "identity"}], "needs a neutral value", False),
            ("frozen", [{"column": "person.alive", "owner": "M2.demography", "lag": "lagged", "use": "identity", "neutral": 0}], "must be true or false for a bool column", True),
            ("draft", [{"column": "person.death_tick", "owner": "M2.demography", "lag": "lagged", "use": "identity", "neutral": "0"}], "must be a TOML integer for a i64 column", True),
            ("frozen", [{"column": "person.sex", "owner": "M2.demography", "lag": "lagged", "use": "identity", "neutral": "Male"}], "must be a snake_case value name for a enum column", True),
            ("frozen", [{"column": "region.conditions", "owner": "M6.polity", "lag": "lagged", "use": "identity", "neutral": 0}], "cannot be typed", True),
            ("draft", [{"column": "region.conditions", "owner": "M6.polity", "lag": "lagged", "use": "identity", "neutral": 0}], "cannot be typed", False),
            ("frozen", [{"column": "polity.exec_constraint", "owner": "M6.polity", "lag": "lagged", "use": "identity", "neutral": "zero"}], "must be a TOML number for a q8_8 column", True),
        ]
        for status, reads, needle, want_err in probes:
            probe_spec = dict(base, system=dict(base["system"], status=status), reads=reads)
            pe, pw = lint(reg, {"M2.demography": specs["M2.demography"], "S5.daily_life": (Path("probe.odd.toml"), probe_spec)},
                          req_ids, test_ids, adr_ids)
            assert any(needle in e for e in (pe if want_err else pw)), (status, needle, pe, pw)
            assert not any(needle in e for e in (pw if want_err else pe)), (status, needle, pe, pw)
        ok_reads = [{"column": "person.alive", "owner": "M2.demography", "lag": "lagged", "use": "identity", "neutral": False},
                    {"column": "polity.exec_constraint", "owner": "M6.polity", "lag": "lagged", "use": "identity", "neutral": 0.0}]
        pe, pw = lint(reg, {"M2.demography": specs["M2.demography"],
                            "S5.daily_life": (Path("probe.odd.toml"), dict(base, system=dict(base["system"], status="frozen"), reads=ok_reads))},
                      req_ids, test_ids, adr_ids)
        assert not any("neutral" in e for e in pe + pw), (pe, pw)
    # self-check (SPR-011 ruling 11): a frozen spec emitting an unknown verb or another system's verb is an error;
    # a draft one only warns
    m2 = specs.get("M2.demography")
    if m2 is not None:
        for status, want_err in (("frozen", True), ("draft", False)):
            probe_spec = dict(m2[1], system=dict(m2[1]["system"], status=status),
                              emits=m2[1].get("emits", []) + [{"kind": "NOT_A_VERB"}, {"kind": "SUCCEED"}])
            pe, pw = lint(reg, {"M2.demography": (m2[0], probe_spec)}, req_ids, test_ids, adr_ids)
            for needle in ("emits NOT_A_VERB, which is not a Registry verb", "emits SUCCEED, which the Registry gives to M7.dynasty"):
                assert any(needle in e for e in (pe if want_err else pw)), (status, needle, pe, pw)
                assert not any(needle in e for e in (pw if want_err else pe)), (status, needle, pe, pw)
    print(f"  self-check (broken spec rejected): {len(caught)} errors, e.g. {caught[0]}")
    print(f"{'PASS' if not errs else 'FAIL'}: {len(specs)} system specs linted ({len(warns)} warnings)")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())
