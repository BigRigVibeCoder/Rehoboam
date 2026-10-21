# SPEC-{{ID}}: {{Title}}

- **Sprint / Wave:** `{{SPR-NNN}}`
- **Author:** Architect Coordinator (`claude-architect`)
- **Status:** `ACTIVE` *(Moves to `.spec/archive/` with `VER-{{ID}}_WaveAudit.md` upon wave completion)*
- **Created:** `{{YYYY-MM-DD}}`

---

## 1. Observable Outcomes (Behavioral State Transitions)
- [ ] When `{{Input / Tick Condition}}` occurs, system transitions deterministically to `{{Expected State / Output}}`.
- [ ] Edge/Error condition `{{Condition}}` returns `{{Expected Deterministic Error / Event}}`.

## 2. Explicit Non-Goals (Out of Scope — Enforced by Gemini Verifier)
- **DO NOT** implement `{{Forbidden Feature 1}}`.
- **DO NOT** modify any files outside the decomposed task `Allowed Write Paths`.
- **DO NOT** add new third-party dependencies to `Cargo.toml` or Python environments.

## 3. Frozen Interface Contract (`CON-{{ID}}` — Committed to `main` Before Worker Dispatch)
> *"The contract is truth. The code is an attempt to match it."* (GOV-007 §3.1)

- **Contract / Trait File:** `crates/{{crate}}/src/{{file}}.rs` (Commit SHA: `{{sha}}`)
- **Reference Spec Script:** `specs/{{domain}}/{{script}}.py`
- **Input / Output & Error Schema:**
  | Field / Method | Input Type & Constraints | Output Type | Deterministic Error Behavior (GOV-004) |
  |:--|:--|:--|:--|
  | `{{fn_name}}` | `{{input_type}}` | `Result<{{ok_type}}, {{err_type}}>` | Returns `{{err_variant}}` on invalid input; zero `.unwrap()` |
- **Frozen Types / Signatures:**
  ```rust
  // Paste frozen struct/trait/function signatures here
  ```

## 4. Mandatory Governance Compliance Table (`GOV-001..008`)
| Governance Standard | Mandatory Requirement for All Leaf Tasks in This Wave |
|:--|:--|
| **GOV-002 (Testing)** | 1:1 test coverage required: every modified `.rs`/`.py` source file must include unit/spec test assertions. No `#[ignore]` or `.skip`. |
| **GOV-003 (Determinism)** | `BTreeMap` / `BTreeSet` / sorted `Vec` only in `crates/rh-*`. Zero `HashMap`, `SystemTime::now()`, or `thread_rng()`. |
| **GOV-004 (Errors)** | Explicit typed `Result<T, E>` error handling. Zero runtime `.unwrap()` or `panic!()` on fallible inputs. |
| **GOV-005 (Commits)** | Forensic commit trailers (`Why:`, `What:`, `Files:`, `Agent:`, `Refs:`) enforced via `scion_api_submit.py`. |
| **GOV-007 (Discrepancy)** | If this contract is ambiguous, Worker MUST run `cerberus_orchestrator.py report-upstream --type EVO` rather than guessing. |
| **GOV-008 (Hygiene)** | Zero secrets, `.env` files, `*.pyc`, `*.log`, or `target/` artifacts permitted in diff. |

## 5. Deterministic Verification Criteria
Every worker must pass these exact commands with exit code `0` before opening a PR:
```bash
{{verification_command}}
```

## 6. Decomposed Leaf Work Orders (Forgejo Issues)
- [ ] **Issue #{{N1}}:** Implement `{{Sub-unit 1}}` | Allowed Write Paths: `crates/{{crate}}/src/*` | Worker: `worker-or-01`
- [ ] **Issue #{{N2}}:** Implement `{{Sub-unit 2}}` | Allowed Write Paths: `specs/{{domain}}/*, tests/*` | Worker: `worker-or-02`
