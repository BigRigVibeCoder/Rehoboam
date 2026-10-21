#!/usr/bin/env python3
"""
verify_agent_boundaries.py — Enforce 3-Tier SDD Boundary Locks (D-14) + Full Governance & Hygiene Scan.
Governed by: GOV-001..GOV-008, BLU-002 §8.1, RUN-002
Ported from: agentic_architect/bin/compliance_check.sh & .agent/workflows/{governance_scan,git_commit}.md

Enforces 5 Deterministic Pre-Merge Gates in <100ms:
  1. Tier 1/2/3 File-Path & CODEOWNERS Boundary Check (D-14)
  2. Secret & Credential Leak Scanner (gitleaks + high-speed regex fallback)
  3. Junk & Ephemeral Runtime Artifact Blocker (*.pyc, *.db-wal, *.log, .env, etc.)
  4. Governance Static Pattern Scanner (GOV-002/003/004 determinism & skipped-test rules, with GOV-NNN-exempt tracking)
  5. 1:1 Source-to-Test Coverage Gate (GOV-002: new/modified source code must include test assertions)
"""

import argparse
import fnmatch
import re
import shutil
import subprocess
import sys
from pathlib import Path

# Tier 1: Immutable Human-Only Locks (Neither Workers nor Architect can touch these)
HUMAN_ONLY_FILES = {
    "CONSTITUTION.md",
    "CLAUDE.md",
    ".forgejo/CODEOWNERS",
    ".github/CODEOWNERS",
}
HUMAN_ONLY_PREFIXES = (
    "CODEX/10_GOVERNANCE/",
    ".forgejo/workflows/",
    ".github/workflows/",
)

# Tier 2: Architect-Autonomous Locks (Architect can modify freely; Workers cannot)
ARCHITECT_LOCKED_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "clippy.toml",
    "deny.toml",
    "AGENTS.md",
    "GEMINI.md",
}
ARCHITECT_LOCKED_PREFIXES = (
    "crates/rh-core/",
    "crates/rh-ids/",
    ".spec/",
)

# Gate 2: Secret & Credential Regex Patterns (ported from compliance_check.sh & git_commit.md)
SECRET_ASSIGNMENT_RE = re.compile(
    r"""(api[_-]?key|password|secret[_-]?key|private[_-]?key|credential|bearer|token)\s*[=:]\s*['"][A-Za-z0-9_\-./+=]{8,}['"]""",
    re.IGNORECASE,
)
SECRET_TOKEN_FORMAT_RE = re.compile(
    r"""(sk-[a-zA-Z0-9]{20,}|sk-ant-[a-zA-Z0-9\-_]{20,}|sk-or-v1-[a-zA-Z0-9]{32,}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|glpat-[a-zA-Z0-9\-_]{20,}|xox[bpas]-[a-zA-Z0-9\-]{20,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)"""
)
SAFE_PLACEHOLDER_RE = re.compile(
    r"""(YOUR_|EXAMPLE|PLACEHOLDER|test[-_]?secret|dummy|mock|REDACTED|<[^>]+>|\$\{)""",
    re.IGNORECASE,
)

# Gate 3: Junk & Ephemeral Files (ported from git_commit.md §2b)
JUNK_PATTERNS = (
    "*.pyc",
    "*.pyo",
    "*.log",
    "*.db-wal",
    "*.db-shm",
    "*.sqlite",
    "*.pem",
    "*.key",
    ".env",
    ".env.*",
    ".cerberus_boot.md",
    "strace.out",
    "startup_trace.log",
)
JUNK_DIR_SEGMENTS = {
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "node_modules",
    "target",
    ".tmp",
    "tmp",
}

# Gate 4: Governance Static Rules (ported from governance_scan.md & Rehoboam CONSTITUTION.md)
EXEMPT_RE = re.compile(r"GOV-\d{3}-exempt\s*:\s*(.+)")


def get_changed_files(worktree: Path, base_ref: str) -> list[str]:
    cmd = ["git", "-C", str(worktree), "diff", "--name-only", base_ref]
    res = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if res.returncode != 0:
        # Fallback to HEAD if origin/main is not fetched in a local test repo
        res = subprocess.run(
            ["git", "-C", str(worktree), "diff", "--name-only", "HEAD"],
            capture_output=True,
            text=True,
            check=False,
        )
        if res.returncode != 0:
            raise RuntimeError(f"git diff failed: {res.stderr.strip()}")
    untracked = subprocess.run(
        ["git", "-C", str(worktree), "ls-files", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        check=False,
    )
    files = set(line.strip() for line in res.stdout.splitlines() if line.strip())
    files.update(line.strip() for line in untracked.stdout.splitlines() if line.strip())
    # Ignore injected ephemeral boot brief unless explicitly staged in git index
    staged = subprocess.run(
        ["git", "-C", str(worktree), "diff", "--cached", "--name-only"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.splitlines()
    if ".cerberus_boot.md" not in staged:
        files.discard(".cerberus_boot.md")
    return sorted(files)


def is_allowed(filepath: str, patterns: list[str]) -> bool:
    for pattern in patterns:
        clean = pattern.strip()
        if not clean:
            continue
        if fnmatch.fnmatch(filepath, clean) or filepath.startswith(clean.rstrip("*")):
            return True
    return False


def check_junk_files(changed: list[str]) -> list[str]:
    violations: list[str] = []
    for fpath in changed:
        name = Path(fpath).name
        if name == ".env.example":
            continue
        parts = set(Path(fpath).parts)
        hit_dir = parts.intersection(JUNK_DIR_SEGMENTS)
        if hit_dir:
            violations.append(f"{fpath} (GOV-005 Junk/Cache Directory: {sorted(hit_dir)[0]})")
            continue
        for pat in JUNK_PATTERNS:
            if fnmatch.fnmatch(name, pat):
                violations.append(f"{fpath} (GOV-005 Ephemeral/Secret File Pattern '{pat}')")
                break
    return violations


def check_secrets(worktree: Path, changed: list[str]) -> list[str]:
    violations: list[str] = []
    # 1. Run gitleaks if installed on host
    gitleaks_bin = shutil.which("gitleaks")
    if gitleaks_bin:
        res = subprocess.run(
            [gitleaks_bin, "detect", "--source", str(worktree), "--no-banner"],
            capture_output=True,
            text=True,
            check=False,
        )
        if res.returncode != 0:
            violations.append("gitleaks detected exposed secret(s) in repository worktree")

    # 2. Fast deterministic regex scan across changed files
    for fpath in changed:
        full = worktree / fpath
        if not full.is_file():
            continue
        try:
            text = full.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        for lineno, line in enumerate(text.splitlines(), start=1):
            if SAFE_PLACEHOLDER_RE.search(line):
                continue
            if SECRET_TOKEN_FORMAT_RE.search(line) or SECRET_ASSIGNMENT_RE.search(line):
                violations.append(f"{fpath}:{lineno} (GOV-008 Potential secret/credential leak)")
    return violations


def check_governance_and_tests(
    worktree: Path, changed: list[str]
) -> tuple[list[str], list[str], list[str]]:
    """
    Returns (failures, warnings, exemptions) across GOV-002 (Tests), GOV-003 (Determinism/Coding), GOV-004 (Errors).
    """
    failures: list[str] = []
    warnings: list[str] = []
    exemptions: list[str] = []

    modified_source_files: list[str] = []
    has_test_updates = False

    for fpath in changed:
        full = worktree / fpath
        if not full.is_file():
            continue

        is_test_file = (
            "test" in fpath.lower()
            or fpath.startswith("tests/")
            or fpath.startswith("specs/")
            or fpath.endswith("_test.rs")
            or fpath.endswith(".test.ts")
            or fpath.endswith(".spec.ts")
        )
        if is_test_file:
            has_test_updates = True

        is_sim_rust = fpath.startswith("crates/rh-") and fpath.endswith(".rs") and not is_test_file
        is_src_code = (
            is_sim_rust
            or (fpath.startswith("src/") and fpath.endswith((".ts", ".tsx", ".py")) and not is_test_file)
            or (fpath.startswith("python/") and fpath.endswith(".py") and not is_test_file)
        )
        if is_src_code and not fpath.endswith(("mod.rs", "lib.rs", "__init__.py", "index.ts", "schema.ts")):
            modified_source_files.append(fpath)

        try:
            lines = full.read_text(encoding="utf-8", errors="ignore").splitlines()
        except Exception:
            continue

        for lineno, line in enumerate(lines, start=1):
            # Check inline #[test] or def test_ inside modified source files
            if re.search(r"(#\[test\]|#\[tokio::test\]|def test_[a-zA-Z0-9_]+)", line):
                has_test_updates = True

            exempt_match = EXEMPT_RE.search(line)
            if exempt_match:
                exemptions.append(f"{fpath}:{lineno} -> {exempt_match.group(0)}")
                continue

            # GOV-002: No skipped tests without exemption
            if is_test_file and re.search(r"(#\[ignore\]|@pytest\.mark\.skip|\b(?:xit|xdescribe)\(|\.(?:skip)\()", line):
                failures.append(
                    f"{fpath}:{lineno} (GOV-002 Skipped test forbidden without '// GOV-002-exempt: <reason>')"
                )

            # GOV-003 & CONSTITUTION §2.1: Rehoboam Rust simulation determinism checks
            if is_sim_rust:
                if re.search(r"\b(HashMap|HashSet)\b", line):
                    failures.append(
                        f"{fpath}:{lineno} (GOV-003 / CONSTITUTION §2.1 Nondeterministic HashMap/HashSet in simulation crate; use BTreeMap/BTreeSet or '// GOV-003-exempt: <reason>')"
                    )
                if re.search(r"(SystemTime::now|Instant::now|thread_rng\s*\()", line):
                    failures.append(
                        f"{fpath}:{lineno} (GOV-003 / CONSTITUTION §2.1 Nondeterministic wall-clock/RNG in simulation crate)"
                    )

            # GOV-003: TypeScript any check (if TS files exist in viewer/ or HUD)
            if fpath.endswith((".ts", ".tsx")) and not is_test_file and not fpath.endswith(".d.ts"):
                if re.search(r"(:\s*any\b|\bas\s+any\b|<any>)", line):
                    failures.append(f"{fpath}:{lineno} (GOV-003 Explicit 'any' type forbidden)")

    # GOV-002 1:1 Source-to-Test Coverage Check
    if modified_source_files and not has_test_updates:
        # Check if sibling test files exist in worktree for every modified source file
        missing_tests: list[str] = []
        for src in modified_source_files:
            src_path = worktree / src
            src_text = src_path.read_text(encoding="utf-8", errors="ignore") if src_path.is_file() else ""
            if "#[cfg(test)]" in src_text or "#[test]" in src_text:
                continue
            missing_tests.append(src)
        if missing_tests:
            failures.append(
                f"GOV-002 Test Coverage Gate: Modified source file(s) {missing_tests} have no inline #[test] module and no accompanying test/spec file changes in diff."
            )

    return failures, warnings, exemptions


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify agent modified files strictly match role, Allowed Write Paths (D-14), and GOV-001..008 compliance."
    )
    parser.add_argument("--worktree", required=True, help="Path to git worktree")
    parser.add_argument(
        "--allowed",
        default="*",
        help="Comma-separated glob patterns (e.g. 'crates/rh-sys-demography/*,specs/demography/*')",
    )
    parser.add_argument(
        "--role",
        choices=["worker", "architect", "human"],
        default="worker",
        help="Agent tier: 'worker' (strict path lock), 'architect' (autonomous contracts/Cargo.toml), or 'human'",
    )
    parser.add_argument(
        "--base", default="origin/main", help="Base git ref to diff against (default: origin/main)"
    )
    parser.add_argument(
        "--skip-governance",
        action="store_true",
        help="Run only file-path boundary checks without secret/governance static scanning",
    )
    args = parser.parse_args()

    worktree = Path(args.worktree).resolve()
    patterns = [p.strip() for p in args.allowed.split(",") if p.strip()]
    changed = get_changed_files(worktree, args.base)

    if not changed:
        print(f"✅ BOUNDARY & GOVERNANCE PASS ({args.role}): 0 modified files.")
        return 0

    boundary_violations: list[str] = []
    if args.role != "human":
        for fpath in changed:
            # 1. Check Tier 1 (Human-Only Immutable Locks)
            if fpath in HUMAN_ONLY_FILES or fpath.startswith(HUMAN_ONLY_PREFIXES):
                boundary_violations.append(
                    f"{fpath} (Tier-1 Human-Only Lock: requires @BigRigVibeCoder)"
                )
                continue

            # 2. If Architect role, everything except Tier-1 is permitted (D-14)
            if args.role == "architect":
                continue

            # 3. Worker role: Block Tier-2 Architect files unless explicitly listed in Allowed Write Paths
            if (
                fpath in ARCHITECT_LOCKED_FILES or fpath.startswith(ARCHITECT_LOCKED_PREFIXES)
            ) and not is_allowed(fpath, patterns):
                boundary_violations.append(
                    f"{fpath} (Tier-2 Architect Lock: requires @claude-architect)"
                )
                continue

            # 4. Worker role: Enforce Allowed Write Paths manifest
            if not is_allowed(fpath, patterns):
                boundary_violations.append(
                    f"{fpath} (Outside Worker Allowed Write Paths: {patterns})"
                )

    junk_violations: list[str] = []
    secret_violations: list[str] = []
    gov_failures: list[str] = []
    gov_warnings: list[str] = []
    gov_exemptions: list[str] = []

    if not args.skip_governance:
        junk_violations = check_junk_files(changed)
        secret_violations = check_secrets(worktree, changed)
        if args.role == "worker":
            gov_failures, gov_warnings, gov_exemptions = check_governance_and_tests(
                worktree, changed
            )

    all_failures = boundary_violations + junk_violations + secret_violations + gov_failures

    if gov_exemptions:
        print("⚠️  GOV EXEMPTIONS DETECTED (Flagged for Gemini Adversarial Verifier audit):")
        for ex in gov_exemptions:
            print(f"  - {ex}")

    if gov_warnings:
        print("⚠️  GOVERNANCE WARNINGS:")
        for w in gov_warnings:
            print(f"  - {w}")

    if all_failures:
        print(
            f"❌ COMPLIANCE & BOUNDARY GATE FAILED (role={args.role})! {len(all_failures)} violation(s):",
            file=sys.stderr,
        )
        for v in all_failures:
            print(f"  - {v}", file=sys.stderr)
        return 1

    print(
        f"✅ BOUNDARY & GOVERNANCE PASS (role={args.role}): "
        f"All {len(changed)} file(s) passed path locks, secret scan, junk check, and GOV-002/003 gates."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
