"""
Lint for Rehoboam sifting patterns (DSL v0). It enforces:
  * act classes and roles exist in the Registry;
  * every state read in `where` is anchored to a bound event (`...@eN`), and that event is bound earlier or at the same clause;
  * `after`, `between` and `require_edge` reference bound events;
  * clause count >= 1 and window sanity.
Run: python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ANCHOR = re.compile(r"@(?P<ev>e\d+)\b")
CALL = re.compile(r"\b[a-z_]+\(")
EDGE_KINDS = {"contingent", "triggered", "relayed_knowledge", "motivated_by", "enabled", "probabilistic", "player", "director"}


def lint(patterns: dict, reg: dict) -> list[str]:
    acts = set(reg["act_classes"]["ids"])
    roles = set(reg["roles"]["ids"])
    errs: list[str] = []
    for p in patterns.get("pattern", []):
        pid = p.get("id", "<no id>")
        for var, role in p.get("roles", {}).items():
            if role not in roles:
                errs.append(f"{pid}: role {var}='{role}' not in Registry")
        bound: list[str] = []
        clauses = p.get("clause", [])
        if not clauses:
            errs.append(f"{pid}: no clauses")
        for c in clauses:
            ev = c.get("bind")
            if not ev or not re.fullmatch(r"e\d+", ev):
                errs.append(f"{pid}: clause has invalid/missing bind")
                continue
            for a in c.get("act", []):
                if a not in acts:
                    errs.append(f"{pid}.{ev}: act class '{a}' not in Registry")
            if "after" in c and c["after"] not in bound:
                errs.append(f"{pid}.{ev}: 'after' references unbound event {c['after']}")
            if c.get("within_days", 1) <= 0:
                errs.append(f"{pid}.{ev}: within_days must be > 0")
            visible = set(bound) | {ev}
            for w in c.get("where", []):
                anchors = ANCHOR.findall(w)
                if CALL.search(w) and not anchors:
                    errs.append(f"{pid}.{ev}: UNANCHORED state read: '{w}' (use attr(...)@eN)")
                for a in anchors:
                    if a not in visible:
                        errs.append(f"{pid}.{ev}: anchor @{a} not bound at this point")
            edge = c.get("require_edge")
            if edge:
                if edge.get("kind") not in EDGE_KINDS:
                    errs.append(f"{pid}.{ev}: unknown edge kind {edge.get('kind')}")
                for k in ("from", "into"):
                    if k in edge and edge[k] not in visible:
                        errs.append(f"{pid}.{ev}: require_edge.{k} references unbound {edge[k]}")
            bound.append(ev)
        for u in p.get("unless", []):
            for a in u.get("act", []):
                if a not in acts:
                    errs.append(f"{pid}.unless: act class '{a}' not in Registry")
            for e in u.get("between", []):
                if e not in bound:
                    errs.append(f"{pid}.unless: between references unbound {e}")
    return errs


def main(argv: list[str]) -> int:
    pats = tomllib.loads(Path(argv[1]).read_text(encoding="utf-8"))
    reg = tomllib.loads(Path(argv[2]).read_text(encoding="utf-8"))
    errs = lint(pats, reg)
    # Self-check: the draft's rule must be rejected, because it reads current psychology rather than state at the event.
    draft = {"pattern": [{"id": "draft_tyrant_cleansing_order", "roles": {"S": "authority", "H": "successor"},
                          "clause": [{"bind": "e1", "act": ["coerce"], "initiator": "S", "target": "H",
                                      "where": ["psychology(S, 'paranoia') > 80"]}]}]}
    draft_errs = lint(draft, reg)
    assert any("UNANCHORED" in e for e in draft_errs), "linter failed to reject the draft's unanchored rule"
    for e in errs:
        print("ERROR", e)
    n = len(pats.get("pattern", []))
    print(f"{'FAIL' if errs else 'PASS'}: {n} patterns linted; draft-style unanchored rule correctly rejected: {draft_errs[0]}")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
