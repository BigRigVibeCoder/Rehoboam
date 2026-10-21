# Project Constitution: Rehoboam (Zero-Friction SDD Swarm Governance v2.0)

> **Binding Authority:** This constitution governs all AI coding agents (`claude-architect`, `gemini-verifier`, `worker-or-01..100`) operating on `BigRigVibeCoder/Rehoboam`. Only the Human Director (`@BigRigVibeCoder`) may modify this `CONSTITUTION.md` or CI workflow definitions.
> **Ported Standards:** Integrates `GOV-001..008`, `AGT-001..003`, and `VER-001` from `agentic_architect` into a high-commit-rate Specification-Driven Development (SDD) swarm.

---

## 1. Zero-Friction Autonomy Model (Decisions D-12..D-15)
- **Sandboxed-Autonomous Execution:** Inside disposable MicroVMs/sandboxes, agents execute commands without mid-run human permission prompts.
- **Architect-Autonomous Contracts:** The Lead Architect Agent (`claude-architect`) is authorized to commit frozen specifications (`.spec/*`, `specs/*`), shared crate traits (`crates/rh-core/*`, `crates/rh-ids/*`), and `Cargo.toml` dependencies directly to `main` to unblock the swarm without waiting for human approval.
- **Full Auto-Merge & GitHub Auto-Mirror:** Any Pull Request that passes (1) the 5-Gate Boundary & Governance Check (`verify_agent_boundaries.py`), (2) the deterministic `Verification Command` (`exit 0`), and (3) the Gemini Adversarial Verifier audit (`VERDICT: PASS`) is **automatically squash-merged into Forgejo `main` and mirrored to GitHub `main` with zero human taps**.

---

## 2. Discrepancy Resolution Protocol & Two-Strike Escalation (`GOV-007 §5` + `D-13`)
When a PR fails verification or is rejected by the Gemini Adversarial Verifier, the failure MUST be classified into strictly one of two discrepancy types (`cerberus_orchestrator.py triage-verdict`):

| Verdict / Discrepancy Type | Meaning | Automated Routing Action |
|:--|:--|:--|
| **`VERDICT: PASS`** | Code matches frozen contract, passes tests, zero scope/governance violations. | Auto-squash-merged to Forgejo `main` (<1s branch prune) and auto-mirrored to GitHub `main`. |
| **`VERDICT: REJECT_WORKER_ERROR`** *(Developer Error)* | The `.spec/active/SPEC-XXX.md` contract is clear, but the Worker wrote buggy code, missed a test assertion, or violated a determinism rule. | **Fast Self-Repair (Turn < 15):** Worker receives 1 immediate self-repair retry with the exact error trace. If it fails again → **Strike 1** (routes to `claude-architect`). |
| **`VERDICT: REJECT_SPEC_AMBIGUITY`** *(Contract Ambiguity)* | The frozen contract or `.spec/active/SPEC-XXX.md` is contradictory, incomplete, or impossible to satisfy without modifying a Tier-2 Architect-locked file. | **Immediate Architect Escalation (Strike 1):** Bypasses worker retry completely! Releases worker lock and dispatches `claude-architect` to patch `.spec/` or shared traits on `main`. |
| **Strike 2 (`cb:strikes >= 2`)** | Task fails again after `claude-architect` Strike-1 intervention. | Marks issue `status:critical-fail`, releases file locks, and pages the Human Director via `ntfy`. |

---

## 3. Infrastructure & Runtime Reality Adaptation Table (`GOV-008`)
> *"Architecture assumes. Infrastructure decides."* All agents MUST code to the actual runtime reality below:

| Idealized Assumption | Actual `Rehoboam` + `Cerberus` Runtime Reality (Enforced by `verify_agent_boundaries.py`) |
|:--|:--|
| Full repository clone on disk | **Blobless Sparse Checkout (`--filter=blob:none`):** Only your `Allowed Write Paths` are materialized locally. Read `.cerberus_boot.md` first and use `cerberus-cat`, `cerberus-search`, `cerberus-blame` to inspect non-materialized files over HTTP in <10ms. |
| Standard collections (`HashMap` / `HashSet`) | **FORBIDDEN in `crates/rh-*` simulation logic:** Iteration order must be 100% deterministic across platforms. Use `BTreeMap`, `BTreeSet`, or `Vec` sorted by ascending `id`. |
| Wall-clock time / OS entropy | **FORBIDDEN in `crates/rh-*`:** Never call `SystemTime::now()`, `Instant::now()`, or `rand::thread_rng()` in simulation ticks. Use explicit tick indices and seeded deterministic PRNGs. |
| "Existing tests pass" is enough | **FORBIDDEN (`GOV-002` 1:1 Coverage Gate):** Every modified source file (`crates/*/src/*.rs`, `python/**/*.py`) MUST have inline `#[test]` assertions or an accompanying test/spec file update in the same PR. |
| Working around a broken contract | **FORBIDDEN (`AGT-002 §4.2`):** Workers may never edit locked traits (`crates/rh-core/*`) or work around contract gaps silently. File an upstream proposal via `cerberus_orchestrator.py report-upstream --type EVO\|DEF`. |
| Inline rule exemptions | **Audited (`GOV-NNN-exempt`):** Any line using `// GOV-003-exempt: <reason>` or `// GOV-002-exempt: <reason>` is automatically extracted and flagged for mandatory review by the Gemini Adversarial Verifier. |

---

## 4. Swarm Worker Invariants (`role = worker`)
1. **Zero-Turn Orientation:** Always read `.cerberus_boot.md` in your worktree root on Turn 1.
2. **Strict File Boundary Lock:** Modify **only** files matching `Allowed Write Paths` in your assigned Forgejo Issue.
3. **Forensic Commit Provenance (`GOV-005 §5.2`):** Submit via `scripts/scion_api_submit.py` so every commit includes structured `Why:`, `What:`, `Files:`, `Agent:`, and `Refs:` trailers.
4. **Circuit Breaker Discipline:** Hard cap of **15 bash turns** and **$1.50** in model spend per task.
