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
parameter's bounds and default.
"""
import copy
import hashlib
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


def spec_files(d):
    """The spec files of directory ``d`` (``_template`` and other ``_``-prefixed files are not specs)."""
    return sorted(p for p in Path(d).glob("*.odd.toml") if not p.name.startswith("_"))


def load(extra_dirs=(), overlay=None, specs_dir=SPECS, next_dir=None):
    """Return (registry, specs, requirement ids, test ids, ADR ids, frozen), where ``frozen`` maps each id that a
    ``--next`` spec superseded to the repository spec it replaced (empty without ``--next``)."""
    reg = tomllib.loads(REGISTRY.read_text(encoding="utf-8"))
    specs = {}
    files = spec_files(specs_dir)
    for d in extra_dirs:
        files += spec_files(d)
    for f in files:
        s = tomllib.loads(f.read_text(encoding="utf-8"))
        sid = s.get("system", {}).get("id", f.stem)
        if sid in specs:
            raise SystemExit(f"{f}: system {sid} already has a spec ({specs[sid][0]})")
        specs[sid] = (f, s)
    frozen = {}
    if next_dir is not None:                      # next versions supersede for this run (SPR-015)
        for f in spec_files(next_dir):
            s = tomllib.loads(f.read_text(encoding="utf-8"))
            sid = s.get("system", {}).get("id", f.stem)
            if sid not in specs:
                raise SystemExit(f"{f}: a next-version spec must supersede a spec in {specs_dir}; {sid} has none")
            if sid in frozen:
                raise SystemExit(f"{f}: system {sid} already has a next-version spec ({frozen[sid][0]})")
            frozen[sid] = specs[sid]
            specs[sid] = (f, s)
    if overlay is not None:                       # a world's own systems and verbs (SPR-011 ruling 11)
        over = tomllib.loads(Path(overlay).read_text(encoding="utf-8"))
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
                        r"schedule\.(?P<wpulse>[A-Za-z]+)/(?P<wphase>[A-Za-z]+)\.when|owns\.(?P<col>[a-z_]+\.[a-z_]+)\.meaning|rng\.(?P<rpurpose>[a-z_]+)\.use)$")


def text_at(spec, m):
    """The text a `[[amends]] of` path names in ``spec``, or None when the path has no text there."""
    if m.group("inv"):
        return next((i.get("statement") for i in spec.get("invariants", []) if i.get("id") == m.group("inv")), None)
    if m.group("alg"):
        steps = spec.get("algorithm", {}).get(m.group("alg"), {}).get("steps", [])
        return next((st for st in steps if str(st).startswith(m.group("step") + " ")), None)
    if m.group("kind"):
        return next((e.get("when") for e in spec.get("emits", []) if e.get("kind") == m.group("kind")), None)
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


def amendments(fs, ns, p, errs):
    """The next version's `[[amends]]` records (REV-SPR-015 R1): each names a path, quotes the frozen text byte for
    byte, gives the new text (which must be what the next version now says there) and a reason. Returns
    path -> (frozen text, new text) for the valid ones."""
    out = {}
    for a in ns.get("amends", []):
        of = str(a.get("of", ""))
        m = AMEND_PATH.match(of)
        if m is None:
            errs.append(f"{p}: [[amends]] of = {of!r} is not a path this lint knows (invariants.<id>.statement, "
                        "algorithm.<name>.steps.<Sn>, emits.<KIND>.when, initialization.state, schedule.<Pulse>/<Phase>.steps.<name>, "
                        "schedule.<Pulse>/<Phase>.when, owns.<entity.column>.meaning, rng.<purpose>.use, params.<name>.meaning)")
            continue
        if not a.get("reason"):
            errs.append(f"{p}: [[amends]] {of} needs a reason")
        was, now = text_at(fs, m), text_at(ns, m)
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
        for name, table in fs.get("algorithm", {}).items():
            got = ns.get("algorithm", {}).get(name)
            if got is None:
                errs.append(f"{p}: frozen [algorithm.{name}] is missing")
                continue
            expect = copy.deepcopy(table)
            step_amended = False
            for of, (was, now) in amended.items():
                m = AMEND_PATH.match(of)
                if m.group("alg") == name:
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
        for e in fs.get("emits", []):
            k = e.get("kind")
            if k not in whens:
                errs.append(f"{p}: frozen [[emits]] {k} is missing")
            elif e.get("when", "") != whens[k] and f"emits.{k}.when" not in amended:
                errs.append(f"{p}: frozen [[emits]] {k} changed its `when` without an [[amends]] record (REV-SPR-015 R15: equal text or an amendment; a prefix or suffix is a change)")
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
    """Field names of a payload written `{ name: Type, name: Type }` (types may hold `|` and `or`, never commas)."""
    inner = str(text).strip()
    if not (inner.startswith("{") and inner.endswith("}")):
        return None
    return [f.split(":")[0].strip() for f in inner[1:-1].split(",") if f.strip()]


def lint_applies(specs, errs, warns, next_ids=()):
    """[[applies]] against the senders' [[commands]] (PR #32; REV-P1a §7). A next-version spec (``next_ids``) is held
    to frozen-level strictness (REV-SPR-015 R6)."""
    def frozen_or_next(sid):
        return specs[sid][1].get("system", {}).get("status") == "frozen" or sid in next_ids
    for tid, (tf, ts) in specs.items():
        strict = errs if frozen_or_next(tid) else warns
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
                sp = errs if frozen_or_next(s) else warns
                if got is None:
                    sp.append(f"{specs[s][0].name}: [[commands]] {cmd} to {tid} needs payload = \"{{ {', '.join(want)} }}\" ({tid}'s [[applies]])")
                elif got != want:
                    (sp if sp is errs else strict).append(
                        f"{specs[s][0].name}: [[commands]] {cmd} to {tid} has payload fields {got}, but [[applies]] declares {want}")


def lint(reg, specs, req_ids, test_ids, adr_ids, next_ids=()):
    """``next_ids``: systems a --next spec supersedes for this run. A draft spec that still reads a column from the
    owner it had before the next version claimed it warns (it moves when the next version supersedes the file); a
    frozen reader errs as always."""
    errs, warns = [], []
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
    lint_applies(specs, errs, warns, next_ids)
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
    errs, warns = lint(reg, specs, req_ids, test_ids, adr_ids, next_ids=set(frozen))
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
        bad = copy.deepcopy(ns)
        bad["rng"].insert(0, {"purpose": "planted_first", "key": ["person"]})
        probes.append((bad, "must stay at position 0"))
        bad = copy.deepcopy(ns)
        for x in bad.get("params", []):
            if "default" in x:
                x["default"] = x["default"] + 1
                probes.append((bad, "frozen default of"))
                break
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
        for probe_lock, needle in (({**want, next(iter(want)): "sha256:planted"}, "changed its text since"),
                                   ({k: v for k, v in list(want.items())[1:]}, "is not in")):
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
