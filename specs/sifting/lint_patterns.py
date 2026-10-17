"""
Lint for Rehoboam sifting patterns (DSL v0). Enforces:
  * a closed key set: unknown keys are errors, so nothing is silently ignored;
  * act classes, roles and verbs exist in the Registry; a clause binds a cast member by its Registry [event_roles]
    name (`actor`, `target`, `witness`, `instrument`), to one of the pattern's role variables (SPR-020, envelope_v0.md §1);
    a clause with `witness = "W"` binds W to each witness row in turn (one partial per witness);
  * a role variable's domain (`A = "authority"`, `S = "successor of A"`) has a Registry [role_domains] entry whose bool
    attribute it compiles to as an anchored read (holds_title(A)@e1 == true) at the variable's ANCHOR: the event of the
    first clause, in pattern order, whose cast binds the variable through actor, target or instrument (a witness is many
    and a match_event has no cast); a domain on a variable no such clause binds is refused (REV-SPR-020 R16); a relational
    domain names a partner variable of the pattern; a domain whose owner is "none" has no snapshot yet and warns
    (envelope_v0.md §1, R1); the [role_domains] table itself has the shape envelope_v0.md §8 fixes (every [roles] id but
    `any`, a bool attribute of the right arity, an owner that is a system, `life_course` or `none`, a column: R22 NIT-3);
  * every state read is a Registry-declared attribute, ANCHORED to a bound event (`attr(...)@eN`), with the right arity;
  * enum comparisons use declared symbolic values;
  * `after`, `between` and `require_edge` reference bound events; `require_edge.from_state` is anchored or names an attribute;
  * consumed `match_event`s are emitted by some pattern in the library;
  * declared match semantics (selection, on_complete, dedupe) use known values and bound role variables;
  * every pattern declares a positive firing budget; a completion hazard names a Registry system hazard;
  * window sanity.
Run: python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

PATTERN_KEYS = {"id", "version", "tellability", "description", "roles", "clause", "unless", "emit",
                "selection", "on_complete", "dedupe", "firing_budget", "completion_hazard", "threatened_role",
                "death_tolerant", "surface_from"}
SELECTIONS = {"latest", "earliest", "all"}
ON_COMPLETE = {"clear_binding", "consume", "keep"}
DEDUPE_KEYS = {"key", "bucket_days"}
BUDGET_KEYS = {"per_century_per_100k"}
HAZARD_KEYS = {"system", "hazard"}
# A clause's cast keys are the Registry [event_roles] ids (actor, target, ...), read from the Registry at lint time.
CLAUSE_KEYS = {"bind", "act", "verb", "match_event", "where", "after", "within_days", "require_edge"}
UNLESS_KEYS = {"bind", "act", "verb", "between", "where"}
EDGE_KEYS = {"from", "into", "kind", "from_state"}
EMIT_KEYS = {"match_event"}
EDGE_KINDS = {"contingent", "triggered", "relayed_knowledge", "motivated_by", "enabled", "probabilistic", "player", "oracle", "director"}
CALL = re.compile(r"\b(?P<name>[a-z_]+)\((?P<args>[^()]*)\)(?P<anchor>@e\d+)?")
ENUM_CMP = re.compile(r"\b(?P<name>[a-z_]+)\([^()]*\)@e\d+\s*[!=]=\s*'(?P<val>[a-z_]+)'")


def _unknown(keys: set[str], allowed: set[str], where: str, errs: list[str]) -> None:
    for k in sorted(keys - allowed):
        errs.append(f"{where}: unknown key '{k}'")


def _check_reads(expr: str, visible: set[str], attrs: dict, where: str, errs: list[str], require_anchor: bool = True) -> None:
    calls = list(CALL.finditer(expr))
    if not calls and require_anchor:
        errs.append(f"{where}: no attribute read in '{expr}'")
    for m in calls:
        name, anchor = m["name"], m["anchor"]
        if name not in attrs:
            errs.append(f"{where}: attribute '{name}' not declared in Registry [attributes]")
            continue
        argc = len([a for a in m["args"].split(",") if a.strip()])
        if argc != len(attrs[name].get("args", [])):
            errs.append(f"{where}: '{name}' expects {len(attrs[name]['args'])} args, got {argc}")
        if require_anchor and not anchor:
            errs.append(f"{where}: UNANCHORED state read: '{m.group(0)}' (use attr(...)@eN)")
        if anchor and anchor[1:] not in visible:
            errs.append(f"{where}: anchor {anchor} not bound at this point")
    for m in ENUM_CMP.finditer(expr):
        a = attrs.get(m["name"], {})
        if a.get("type") != "enum":
            errs.append(f"{where}: symbolic comparison on non-enum attribute '{m['name']}'")
        elif m["val"] not in a.get("values", []):
            errs.append(f"{where}: '{m['val']}' is not a value of enum '{m['name']}' {a.get('values')}")


DOMAIN_KEYS = {"attribute", "relational", "owner", "column"}
SOURCE_COLUMN = re.compile(r"^([a-z][a-z0-9_]*\.[a-z][a-z0-9_]*|algorithm\.[a-z][a-z0-9_]*|life_course)$")


PINNED_EVENT_ROLES = ["actor", "target", "witness", "instrument"]
ROLE_ID = re.compile(r"^[a-z][a-z0-9_]*$")


def _event_roles_shape(reg: dict) -> list[str]:
    """The Registry [event_roles] list as envelope_v0.md §8 fixes it (SPR-020 T-01; R22 NIT-3, as rh-manifest's
    Registry::problems refuses it): not empty, each id [a-z][a-z0-9_]*, the first four actor, target, witness,
    instrument in that order (the role codes schema.sql, channel A and the life_ends twin read as literals)."""
    ids = reg["event_roles"].get("ids", []) if isinstance(reg["event_roles"], dict) else []
    errs = [f"Registry [event_roles]: id {i!r} is not [a-z][a-z0-9_]*" for i in ids if not ROLE_ID.match(str(i))]
    if not ids:
        errs.append("Registry [event_roles]: the list is empty (envelope_v0.md §8)")
    elif ids[:4] != PINNED_EVENT_ROLES:
        errs.append(f"Registry [event_roles]: the first four ids are not actor, target, witness, instrument: {ids[:4]}")
    return errs


def _domain_shape(reg: dict) -> list[str]:
    """The Registry [role_domains] table as envelope_v0.md §8 fixes it (REV-SPR-020 R16, R22 NIT-3): one entry per
    [roles] id but `any`, no other key; a bool [attributes] entry whose arity matches `relational` (one argument, or
    actor and partner); an `owner` that is a [systems] id, `life_course` or `none`; a `column` (entity.column,
    algorithm.<name> or life_course) read of that owner, `none` with owner none."""
    errs: list[str] = []
    roles = set(reg["roles"]["ids"]) - {"any"}
    domains = reg["role_domains"]
    attrs = reg.get("attributes", {})
    systems = set(reg.get("systems", {}))
    for role in sorted(roles - set(domains)):
        errs.append(f"Registry [role_domains]: no entry for the [roles] id '{role}'")
    for key in sorted(set(domains) - roles):
        errs.append(f"Registry [role_domains]: '{key}' is not a [roles] id (or is 'any', which has no domain)")
    for key, d in sorted(domains.items()):
        if not isinstance(d, dict) or set(d) != DOMAIN_KEYS:
            errs.append(f"Registry [role_domains].{key}: the keys are exactly {sorted(DOMAIN_KEYS)}")
            continue
        a = attrs.get(d["attribute"])
        if a is None or a.get("type") != "bool":
            errs.append(f"Registry [role_domains].{key}: attribute {d['attribute']!r} is not a bool [attributes] entry")
        elif len(a.get("args", [])) != (2 if d["relational"] else 1):
            errs.append(f"Registry [role_domains].{key}: attribute {d['attribute']!r} takes {len(a.get('args', []))} argument(s); "
                        f"relational = {str(d['relational']).lower()} needs {2 if d['relational'] else 1}")
        if d["owner"] not in systems | {"life_course", "none"}:
            errs.append(f"Registry [role_domains].{key}: owner {d['owner']!r} is not a [systems] id, 'life_course' or 'none'")
        if d["owner"] == "none":
            if d["column"] != "none":
                errs.append(f"Registry [role_domains].{key}: owner none has no column; write column = \"none\"")
        elif not SOURCE_COLUMN.match(str(d["column"])):
            errs.append(f"Registry [role_domains].{key}: column {d['column']!r} is not entity.column, algorithm.<name> or life_course")
    return errs


def lint(patterns: dict, reg: dict, warns: list[str] | None = None) -> list[str]:
    warns = warns if warns is not None else []
    acts = set(reg["act_classes"]["ids"])
    roles = set(reg["roles"]["ids"])
    for section in ("event_roles", "role_domains"):      # SPR-020: both are required (envelope_v0.md §8)
        if section not in reg:
            return [f"Registry: missing [{section}] (SPR-020, envelope_v0.md §8)"]
    roles_shape = _event_roles_shape(reg)
    if roles_shape:
        return roles_shape
    cast_keys = set(reg["event_roles"]["ids"])           # the clause keys that bind a cast member (envelope_v0.md §1)
    clause_keys, unless_keys = CLAUSE_KEYS | cast_keys, UNLESS_KEYS | cast_keys
    anchor_keys = cast_keys - {"witness"}                # the keys whose binding anchors a domain read (R16)
    domains = reg["role_domains"]
    shape = _domain_shape(reg)
    if shape:
        return shape
    verbs = set(reg.get("verbs", {}).keys())
    attrs = reg.get("attributes", {})
    errs: list[str] = []
    pats = patterns.get("pattern", [])
    emitted = {p.get("emit", {}).get("match_event") for p in pats} - {None}
    for p in pats:
        pid = p.get("id", "<no id>")
        _unknown(set(p), PATTERN_KEYS, pid, errs)
        _unknown(set(p.get("emit", {})), EMIT_KEYS, f"{pid}.emit", errs)
        anchors: dict[str, str] = {}                     # role variable -> the clause that anchors its domain read (R16)
        for c in p.get("clause", []):
            if "match_event" in c:
                continue
            for k in sorted(anchor_keys & set(c)):
                anchors.setdefault(str(c[k]), str(c.get("bind")))
        for var, spec_role in p.get("roles", {}).items():
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", var):   # the canonical form's identifier (specs/stream/canonical_v0.md §2)
                errs.append(f"{pid}: role variable '{var}' is not an identifier [A-Za-z_][A-Za-z0-9_]*")
            role, _, partner = str(spec_role).partition(" of ")     # `S = "successor of A"`: a relational domain (envelope_v0.md §1)
            if role not in roles:
                errs.append(f"{pid}: role {var}='{role}' not in Registry")
                continue
            if role == "any":
                if partner:
                    errs.append(f"{pid}: role {var}: 'any' takes no partner")
                continue
            d = domains.get(role)
            if d is None:
                errs.append(f"{pid}: role {var}='{role}' has no [role_domains] entry (its anchored attribute; SPR-020)")
                continue
            a = attrs.get(d.get("attribute"))
            if a is None or a.get("type") != "bool":
                errs.append(f"{pid}: role {var}='{role}': domain attribute {d.get('attribute')!r} is not a bool [attributes] entry")
            if d.get("relational") and partner not in p.get("roles", {}):
                errs.append(f"{pid}: role {var}='{role}' is relational: write \"{role} of <variable>\" naming a role variable of the pattern")
            if not d.get("relational") and partner:
                errs.append(f"{pid}: role {var}='{role}' is not relational; drop 'of {partner}'")
            anchor = anchors.get(var)
            if anchor is None:
                errs.append(f"{pid}: role {var}='{role}' has a domain, but no clause binds {var} through actor, target or instrument "
                            "(a witness is many; a match_event has no cast): its anchored read has no event (REV-SPR-020 R16)")
                continue
            if d.get("owner") == "none":
                warns.append(f"{pid}: role {var}='{role}' compiles to {d.get('attribute')}({var}{', ' + partner if partner else ''})@{anchor}, "
                             "which no system captures yet (owner = none): the clause cannot bind until its owner declares the snapshot")
        for key, allowed in (("selection", SELECTIONS), ("on_complete", ON_COMPLETE)):
            if key not in p:
                errs.append(f"{pid}: missing {key} (match semantics are declared, never implicit; 05.4 §2.3)")
            elif p[key] not in allowed:
                errs.append(f"{pid}: {key} '{p[key]}' not in {sorted(allowed)}")
        if len(p.get("clause", [])) > 2 and (p.get("selection"), p.get("on_complete")) != ("all", "keep"):
            errs.append(f"{pid}: >2 clauses needs selection='all', on_complete='keep' until 05.4 §2.3 defines "
                        f"per-stage latest/earliest/clear_binding/consume (B3 fork-and-accept is the only N-stage rule)")
        for var in p.get("death_tolerant", []):
            if var not in p.get("roles", {}):
                errs.append(f"{pid}.death_tolerant: '{var}' is not a role variable")
        if "dedupe" in p:
            d = p["dedupe"]
            _unknown(set(d), DEDUPE_KEYS, f"{pid}.dedupe", errs)
            for var in d.get("key", []):
                if var not in p.get("roles", {}):
                    errs.append(f"{pid}.dedupe: key '{var}' is not a role variable")
            if not d.get("key") or d.get("bucket_days", 0) <= 0:
                errs.append(f"{pid}.dedupe: needs a non-empty key and bucket_days > 0")
        budget = p.get("firing_budget")
        if budget is None:
            errs.append(f"{pid}: missing firing_budget (required, 05.4 §2.3)")
        else:
            _unknown(set(budget), BUDGET_KEYS, f"{pid}.firing_budget", errs)
            if not budget.get("per_century_per_100k", 0) > 0:
                errs.append(f"{pid}.firing_budget: per_century_per_100k must be > 0")
        hz = p.get("completion_hazard")
        if hz is not None:
            _unknown(set(hz), HAZARD_KEYS, f"{pid}.completion_hazard", errs)
            sysdef = reg.get("systems", {}).get(hz.get("system"), {})
            if hz.get("hazard") not in sysdef.get("hazards", []):
                errs.append(f"{pid}.completion_hazard: '{hz.get('system')}.{hz.get('hazard')}' is not a Registry system hazard")
            tr = p.get("threatened_role")
            if tr is None:
                errs.append(f"{pid}: completion_hazard requires threatened_role (the role for whom completion is a negative outcome; 05.4 §2.3)")
            elif tr not in p.get("roles", {}):
                errs.append(f"{pid}.threatened_role: '{tr}' is not one of the pattern's roles")
        elif p.get("threatened_role") is not None:
            errs.append(f"{pid}: threatened_role without completion_hazard has no effect")
        bound: list[str] = []
        clauses = p.get("clause", [])
        if not clauses:
            errs.append(f"{pid}: no clauses")
        # VER-SPR-013-TEST GAP-4 (rh-sifter's MIN-1): a declared role that no clause binds through its
        # cast is refused; a match_event clause imports the match's bindings, so it may bind any.
        if not any("match_event" in c for c in clauses):
            cast_bound = {str(c[k]) for c in clauses for k in cast_keys & set(c)}
            for var in sorted(set(p.get("roles", {})) - cast_bound):
                errs.append(f"{pid}: role {var} is declared but no clause binds it")
        for c in clauses:
            ev = c.get("bind")
            where = f"{pid}.{ev}"
            _unknown(set(c), clause_keys, where, errs)
            for k in sorted(cast_keys & set(c)):
                if c[k] not in p.get("roles", {}):
                    errs.append(f"{where}: {k} = {c[k]!r} is not one of the pattern's role variables")
            if not ev or not re.fullmatch(r"e[0-9]+", ev):   # ASCII digits only, as the canonical form (canonical_v0.md §2)
                errs.append(f"{pid}: clause has invalid/missing bind")
                continue
            selectors = [k for k in ("act", "verb", "match_event") if k in c]
            if len(selectors) != 1:
                errs.append(f"{where}: exactly one of act/verb/match_event required (got {selectors})")
            if "match_event" in c and cast_keys & set(c):   # a match has bindings, not a cast (REV-SPR-020 R26)
                errs.append(f"{where}: a match_event clause has no cast; it imports the match's bindings, so drop {sorted(cast_keys & set(c))}")
            for a in c.get("act", []):
                if a not in acts:
                    errs.append(f"{where}: act class '{a}' not in Registry")
            if "verb" in c and c["verb"] not in verbs:
                errs.append(f"{where}: verb '{c['verb']}' not in Registry [verbs]")
            if "match_event" in c and c["match_event"] not in emitted:
                errs.append(f"{where}: consumes match_event '{c['match_event']}' that no pattern emits")
            if "after" in c and c["after"] not in bound:
                errs.append(f"{where}: 'after' references unbound event {c['after']}")
            if not bound and ("after" in c or "within_days" in c):
                errs.append(f"{where}: the first clause cannot take 'after' or 'within_days'")
            if c.get("within_days", 1) <= 0:
                errs.append(f"{where}: within_days must be > 0")
            visible = set(bound) | {ev}
            for w in c.get("where", []):
                _check_reads(w, visible, attrs, where, errs)
            edge = c.get("require_edge")
            if edge:
                _unknown(set(edge), EDGE_KEYS, f"{where}.require_edge", errs)
                if edge.get("kind") not in EDGE_KINDS:
                    errs.append(f"{where}: unknown edge kind {edge.get('kind')}")
                for k in ("from", "into"):
                    if k in edge and edge[k] not in visible:
                        errs.append(f"{where}: require_edge.{k} references unbound {edge[k]}")
                if "from_state" in edge:   # names the motivating state; must be a declared attribute (anchored to `into`)
                    _check_reads(edge["from_state"], visible, attrs, f"{where}.require_edge", errs, require_anchor=False)
            bound.append(ev)
        sf = p.get("surface_from")
        if sf is not None and (sf not in bound or sf == bound[-1]):
            errs.append(f"{pid}.surface_from: '{sf}' must name a bound clause other than the last")
        for u in p.get("unless", []):
            _unknown(set(u), unless_keys, f"{pid}.unless", errs)
            for k in sorted(cast_keys & set(u)):
                if u[k] not in p.get("roles", {}):
                    errs.append(f"{pid}.unless: {k} = {u[k]!r} is not one of the pattern's role variables")
            for a in u.get("act", []):
                if a not in acts:
                    errs.append(f"{pid}.unless: act class '{a}' not in Registry")
            btw = u.get("between", [bound[0], bound[-1]] if bound else [])   # default: whole kernel (B3 compiler default)
            if len(btw) != 2 or any(e not in bound for e in btw) or bound.index(btw[0]) >= bound.index(btw[1]):
                errs.append(f"{pid}.unless: between must name two bound clauses in kernel order, got {btw}")
                continue
            ub = u.get("bind")
            if ub is not None and not re.fullmatch(r"u[0-9]+", ub):
                errs.append(f"{pid}.unless: bind must look like 'uN', got {ub}")
            # an unless event arrives while btw[0] is bound and btw[1] is not: it may read only those anchors
            visible_u = set(bound[: bound.index(btw[0]) + 1]) | ({ub} if ub else set())
            for w in u.get("where", []):
                _check_reads(w, visible_u, attrs, f"{pid}.unless", errs)
    return errs


def main(argv: list[str]) -> int:
    pats = tomllib.loads(Path(argv[1]).read_text(encoding="utf-8"))
    reg = tomllib.loads(Path(argv[2]).read_text(encoding="utf-8"))
    warns: list[str] = []
    errs = lint(pats, reg, warns)
    if errs and all(e.startswith("Registry") for e in errs):   # the Registry itself is refused; no self-check runs on it (R26)
        for e in errs:
            print("ERROR", e)
        print("FAIL: the Registry is refused; no pattern was linted.")
        return 1
    # Self-checks: the draft's rule and a misspelled key must both be rejected.
    draft = {"pattern": [{"id": "draft_tyrant_cleansing_order", "roles": {"S": "authority", "H": "successor"},
                          "selection": "latest", "on_complete": "clear_binding",
                          "firing_budget": {"per_century_per_100k": 1},
                          "clause": [{"bind": "e1", "act": ["coerce"], "actor": "S", "target": "H",
                                      "where": ["psychology(S, 'paranoia') > 80"]}]}]}
    typo = {"pattern": [{"id": "typo", "roles": {"A": "authority"}, "selection": "all", "on_complete": "keep",
                         "firing_budget": {"per_century_per_100k": 1},
                         "clause": [{"bind": "e1", "act": ["harm"], "edge": {"to": "e1", "kind": "motivated_by"}}]}]}
    implicit = {"pattern": [{"id": "implicit", "roles": {"A": "any"}, "selection": "first",
                             "clause": [{"bind": "e1", "act": ["harm"], "actor": "A"}]}]}
    three = {"pattern": [{"id": "three", "roles": {"A": "any", "B": "any"}, "firing_budget": {"per_century_per_100k": 1},
                          "selection": "latest", "on_complete": "clear_binding",
                          "clause": [{"bind": "e1", "act": ["aid"], "actor": "A"},
                                     {"bind": "e2", "act": ["aid"], "actor": "B", "target": "A"},
                                     {"bind": "e3", "act": ["harm"], "actor": "B", "target": "A"}],
                          "unless": [{"bind": "u1", "act": ["flee"], "actor": "A", "between": ["e1", "e3"],
                                      "where": ["trust(A, B)@e2 >= 0.5"]}]}]}
    old_key = {"pattern": [{"id": "old_key", "roles": {"A": "any"}, "selection": "latest", "on_complete": "clear_binding",
                            "firing_budget": {"per_century_per_100k": 1},
                            "clause": [{"bind": "e1", "act": ["harm"], "initiator": "A", "target": "B"}]}]}
    domain = {"pattern": [{"id": "domain", "roles": {"A": "authority", "S": "successor", "H": "host", "K": "kin of A"},
                           "selection": "all", "on_complete": "keep", "firing_budget": {"per_century_per_100k": 1},
                           "clause": [{"bind": "e1", "act": ["harm"], "actor": "A", "target": "S"},
                                      {"bind": "e2", "act": ["aid"], "actor": "H", "target": "K", "after": "e1"}]}]}
    # REV-SPR-020 R16: a domain on a variable bound only as a witness, or only through a match_event, has no anchor
    unbound = {"pattern": [{"id": "unbound", "roles": {"A": "authority", "W": "kin of A", "M": "any"},
                            "selection": "latest", "on_complete": "clear_binding", "firing_budget": {"per_century_per_100k": 1},
                            "clause": [{"bind": "e1", "act": ["harm"], "actor": "A", "witness": "W", "target": "M"}]}]}
    # REV-SPR-020 R26: a cast key on a match_event clause is refused, so a match never anchors a domain
    cast_on_match = {"pattern": [{"id": "cast_on_match", "roles": {"A": "authority"}, "selection": "latest", "on_complete": "clear_binding",
                                  "firing_budget": {"per_century_per_100k": 1},
                                  "clause": [{"bind": "e1", "match_event": "successor_purged", "actor": "A"}]}]}
    # REV-SPR-020 R22 NIT-3: a [role_domains] entry of the wrong shape is refused with the Registry
    bad_reg = {**reg, "role_domains": {**reg["role_domains"],
                                       "authority": {"attribute": "is_successor", "relational": False, "owner": "M9.bogus", "column": "x"}}}
    d_errs, t_errs, s_errs, m_errs = lint(draft, reg), lint(typo, reg), lint(implicit, reg), lint(three, reg)
    o_errs = lint(old_key, reg)
    dom_warns: list[str] = []
    dom_errs = lint(domain, reg, dom_warns)
    u_errs, r_errs = lint(unbound, reg), lint(domain, bad_reg)
    c_errs = lint(cast_on_match, reg)
    spare = {"pattern": [{"id": "spare", "roles": {"A": "any", "B": "any"}, "selection": "latest",
                          "on_complete": "clear_binding", "firing_budget": {"per_century_per_100k": 1},
                          "clause": [{"bind": "e1", "verb": "DEATH", "target": "A"}]}]}
    assert any("role B is declared but no clause binds it" in e for e in lint(spare, reg))
    assert any("a match_event clause has no cast" in e for e in c_errs), c_errs
    assert any("A='authority' has a domain, but no clause binds A" in e for e in c_errs), c_errs
    assert any("W='kin' has a domain, but no clause binds W through actor, target or instrument" in e for e in u_errs), u_errs
    assert not any("A='authority' has a domain" in e for e in u_errs), u_errs
    assert any("takes 2 argument(s); relational = false needs 1" in e for e in r_errs), r_errs
    assert any("owner 'M9.bogus' is not a [systems] id" in e for e in r_errs), r_errs
    assert any("column 'x' is not entity.column" in e for e in r_errs), r_errs
    assert any("unknown key 'initiator'" in e for e in o_errs), o_errs      # the cast keys are [event_roles] ids (SPR-020)
    assert any("target = 'B' is not one of the pattern's role variables" in e for e in o_errs), o_errs
    assert any("S='successor' is relational" in e for e in dom_errs), dom_errs   # domains compile to anchored reads (R1)
    assert any("H='host' compiles to is_host(H)@e2" in w for w in dom_warns), dom_warns
    assert not any("K=" in e for e in dom_errs) and not any("A=" in e for e in dom_errs), dom_errs
    no_domains = {k: v for k, v in reg.items() if k != "role_domains"}
    assert lint(domain, no_domains) == ["Registry: missing [role_domains] (SPR-020, envelope_v0.md §8)"]
    assert any("not declared" in e or "UNANCHORED" in e for e in d_errs), d_errs
    assert any("unknown key 'edge'" in e for e in t_errs), t_errs
    assert any("selection" in e for e in s_errs) and any("firing_budget" in e for e in s_errs), s_errs
    assert any(">2 clauses" in e for e in m_errs) and any("anchor @e2 not bound" in e for e in m_errs), m_errs
    for w in warns:
        print("WARN ", w)
    for e in errs:
        print("ERROR", e)
    n = len(pats.get("pattern", []))
    print(f"{'FAIL' if errs else 'PASS'}: {n} patterns linted.")
    print(f"  self-check (draft rule rejected): {d_errs[0]}")
    print(f"  self-check (unknown key rejected): {t_errs[0]}")
    print(f"  self-check (undeclared semantics rejected): {'; '.join(s_errs)}")
    print(f"  self-check (multi-stage guard, unless anchors): {'; '.join(m_errs)}")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
