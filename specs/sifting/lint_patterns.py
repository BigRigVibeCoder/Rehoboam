"""
Lint for Rehoboam sifting patterns (DSL v0). Enforces:
  * a closed key set: unknown keys are errors, so nothing is silently ignored;
  * act classes, roles and verbs exist in the Registry; a clause binds a cast member by its Registry [event_roles]
    name (`actor`, `target`, `witness`, `instrument`), to one of the pattern's role variables (SPR-020, envelope_v0.md §1);
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


def lint(patterns: dict, reg: dict) -> list[str]:
    acts = set(reg["act_classes"]["ids"])
    roles = set(reg["roles"]["ids"])
    cast_keys = set(reg["event_roles"]["ids"])           # the clause keys that bind a cast member (envelope_v0.md §1)
    clause_keys, unless_keys = CLAUSE_KEYS | cast_keys, UNLESS_KEYS | cast_keys
    verbs = set(reg.get("verbs", {}).keys())
    attrs = reg.get("attributes", {})
    errs: list[str] = []
    pats = patterns.get("pattern", [])
    emitted = {p.get("emit", {}).get("match_event") for p in pats} - {None}
    for p in pats:
        pid = p.get("id", "<no id>")
        _unknown(set(p), PATTERN_KEYS, pid, errs)
        _unknown(set(p.get("emit", {})), EMIT_KEYS, f"{pid}.emit", errs)
        for var, role in p.get("roles", {}).items():
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", var):   # the canonical form's identifier (specs/stream/canonical_v0.md §2)
                errs.append(f"{pid}: role variable '{var}' is not an identifier [A-Za-z_][A-Za-z0-9_]*")
            if role not in roles:
                errs.append(f"{pid}: role {var}='{role}' not in Registry")
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
    errs = lint(pats, reg)
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
    d_errs, t_errs, s_errs, m_errs = lint(draft, reg), lint(typo, reg), lint(implicit, reg), lint(three, reg)
    o_errs = lint(old_key, reg)
    assert any("unknown key 'initiator'" in e for e in o_errs), o_errs      # the cast keys are [event_roles] ids (SPR-020)
    assert any("target = 'B' is not one of the pattern's role variables" in e for e in o_errs), o_errs
    assert any("not declared" in e or "UNANCHORED" in e for e in d_errs), d_errs
    assert any("unknown key 'edge'" in e for e in t_errs), t_errs
    assert any("selection" in e for e in s_errs) and any("firing_budget" in e for e in s_errs), s_errs
    assert any(">2 clauses" in e for e in m_errs) and any("anchor @e2 not bound" in e for e in m_errs), m_errs
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
