"""Generate CODEX/00_INDEX/MANIFEST.yaml: the agents' map of every document (GOV-001 §3).

Sources:
  - CODEX/**/*.md with YAML frontmatter (required there; tags checked against GOV-001 §4)
  - docs/**/*.md, research/*.md, research/papers/README.md (frontmatter if present, else inferred
    from the filename, H1, "Status:" line and first paragraph)
  - specs/systems/*.odd.toml (system specs, ADR-0033)

Run:  python -I bin/gen_manifest.py          write the manifest (a local file, not committed: GOV-001 §3)
      python -I bin/gen_manifest.py --check  build it in memory and exit 1 on any error (bad or missing
                                             frontmatter, unknown tags, duplicate ids); used by bin/check_all.sh

DECISION (2026-10-16): the manifest is no longer committed. A committed generated index conflicted
with every concurrent PR that touched a document, forcing a rebase each time. Agents generate it at
session start; CI validates the inputs it is built from.
Standard library only.
"""
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "CODEX" / "00_INDEX" / "MANIFEST.yaml"
TAGS = {"governance", "project-management", "workflow", "testing", "coding", "observability", "documentation",
        "determinism", "simulation", "stream", "sifting", "validation", "research", "infrastructure", "agents",
        "sprint", "backlog", "defect", "evolution", "verification", "roadmap"}


def frontmatter(text):
    if not text.startswith("---\n"):
        return None, text
    end = text.find("\n---\n", 4)
    if end < 0:
        return None, text
    meta = {}
    for line in text[4:end].splitlines():
        m = re.match(r"^([a-z_]+):\s*(.*?)\s*(#.*)?$", line)
        if not m:
            continue
        k, v = m.group(1), m.group(2)
        if v.startswith("[") and v.endswith("]"):
            v = [x.strip().strip('"') for x in v[1:-1].split(",") if x.strip()]
        else:
            v = v.strip('"')
        meta[k] = v
    return meta, text[end + 5:]


def first_summary(body):
    m = re.search(r"\*\*BLUF:\*\*\s*(.+)", body)
    if m:
        s = m.group(1)
    else:
        s = ""
        for para in re.split(r"\n\s*\n", body):
            p = para.strip()
            if p and not p.startswith(("#", "|", "```", "<!--", "---", "**Status", "*Status", "*v", "*Updated", "*Research", "> **ODD")):
                s = p.replace("\n", " ")
                break
    s = re.sub(r"[*`]", "", s)
    return (s[:220] + "…") if len(s) > 220 else s


def h1(body):
    m = re.search(r"^# (.+)$", body, re.M)
    return m.group(1).strip() if m else ""


def infer_id(path):
    rel = path.relative_to(ROOT).as_posix()
    n = path.stem
    if m := re.match(r"(ADR-\d{4})", n):
        return m.group(1)
    if rel.startswith("docs/05_architecture/"):
        return "ARCH-" + n.split("_")[0]
    if rel.startswith("docs/templates/"):
        return "TPL-" + n
    if rel.startswith("docs/") and re.match(r"\d\d_", n):
        return "DOC-" + n[:2]
    if rel.startswith("docs/"):
        return "DOC-" + n.upper().replace("_", "-")
    if rel.startswith("research/papers/"):
        return "RES-PAPERS-INDEX"
    if rel.startswith("research/"):
        return "RES-" + (n.split("_")[0] if re.match(r"R\d\d", n) else n.upper().replace("_", "-"))
    return n


def entries():
    errs, out = [], []
    md = sorted((ROOT / "CODEX").rglob("*.md")) + sorted((ROOT / "docs").rglob("*.md")) + \
        sorted((ROOT / "research").glob("*.md")) + [ROOT / "research" / "papers" / "README.md"]
    for p in md:
        if "_templates" in p.parts:
            continue
        text = p.read_text(encoding="utf-8")
        meta, body = frontmatter(text)
        rel = p.relative_to(ROOT).as_posix()
        area = rel.split("/")[1] if rel.startswith("CODEX/") else rel.split("/")[0]
        if rel.startswith("CODEX/") and p.name != "README.md" and meta is None:
            errs.append(f"{rel}: CODEX documents need YAML frontmatter (GOV-001 §2)")
            continue
        if meta:
            bad = [t for t in meta.get("tags", []) if t not in TAGS]
            if bad:
                errs.append(f"{rel}: unknown tags {bad} (GOV-001 §4)")
            e = {"id": meta.get("id", infer_id(p)), "title": meta.get("title", h1(body)), "type": meta.get("type", ""),
                 "status": meta.get("status", ""), "tags": meta.get("tags", [])}
        else:
            st = re.search(r"\*\*Status:\*\*\s*([^·\n*]+)", text) or re.search(r"\*Status:\s*([^.(*\n]+)", text)
            e = {"id": infer_id(p) if p.name != "README.md" else "README-" + rel.replace("/", "-")[:-10].upper().strip("-"),
                 "title": h1(body), "type": "inferred", "status": st.group(1).strip() if st else "", "tags": []}
        e.update({"path": rel, "area": area, "summary": first_summary(body)})
        out.append(e)
    for p in sorted((ROOT / "specs" / "systems").glob("*.odd.toml")):
        if p.name.startswith("_"):
            continue
        s = tomllib.loads(p.read_text(encoding="utf-8"))
        h = s.get("system", {})
        out.append({"id": "SYS-" + h.get("id", p.stem), "title": h.get("name", ""), "type": "system-spec",
                    "status": h.get("status", ""), "tags": ["simulation"], "path": p.relative_to(ROOT).as_posix(),
                    "area": "specs", "summary": s.get("purpose", {}).get("statement", "")})
    ids = [e["id"] for e in out]
    for d in sorted({i for i in ids if ids.count(i) > 1}):
        errs.append(f"duplicate id {d}")
    return out, errs


def q(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


def render(es):
    lines = ["# GENERATED by bin/gen_manifest.py — do not edit by hand (GOV-001 §3).",
             "# Agents: filter by id, type, status, tags or area, then read only what matches.",
             "project: Rehoboam", f"documents: {len(es)}", "entries:"]
    for e in sorted(es, key=lambda x: (x["area"], x["id"])):
        lines += [f"  - id: {q(e['id'])}", f"    path: {q(e['path'])}", f"    title: {q(e['title'])}",
                  f"    type: {q(e['type'])}", f"    status: {q(e['status'])}", f"    area: {q(e['area'])}",
                  f"    tags: [{', '.join(e['tags'])}]", f"    summary: {q(e['summary'])}"]
    return "\n".join(lines) + "\n"


def main() -> int:
    es, errs = entries()
    for e in errs:
        print("ERROR", e)
    text = render(es)
    if "--check" in sys.argv:
        print(f"MANIFEST: {'FAIL' if errs else 'builds cleanly'} ({len(es)} documents, {len(errs)} errors)")
        return 1 if errs else 0
    OUT.write_text(text, encoding="utf-8")
    print(f"MANIFEST: wrote {len(es)} documents to {OUT.relative_to(ROOT)}")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())
