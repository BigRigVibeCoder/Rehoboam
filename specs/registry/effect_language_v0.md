# Registry effect language v0

*Status: Draft for the director's approval at the SPR-017 plan gate (C2). Version `0.1.0` of the language. Contract for WP P1.15 (`docs/11`), SPR-017 (`CODEX/05_PROJECT/PRJ-002_P1aPlan.md`) and its dependants SPR-021 (M7 succession) and SPR-024 (minds' verbs). Sources: `05.5` §2.1 and §3.2, `05.1` §2, §4.3 and §6, ADR-0006, ADR-0010 (amendment v1.1 item 4), ADR-0024, ADR-0025, ADR-0033, ADR-0034, ADR-0035, `docs/01` §5 principles 8 and 9, `crates/rh-core/src/fixed/`. Golden cases: `specs/registry/effect_language_golden_v0.toml`. Reference implementation: `specs/registry/effect_ref.py` (standard-library Python; `--check` reproduces every golden case).*

> **BLUF.** The effect language is the small, typed, total expression language in which the Registry states succession laws (who may inherit a title and in what order), verb effects (which lifecycle commands an act queues), and the vows' completion and conflict predicates. A program reads only the columns its declaring system has declared, iterates only over Registry relations with declared bounds, draws randomness only from the keyed RNG with a declared purpose, and changes the world only by queuing typed commands to the column's owner. Every well-typed program terminates and yields a value: there is no recursion, every loop is bounded, and division by zero, overflow and reads of missing entities have defined results (the saturating, round-half-to-even rules of `rh-core::fixed`). Floats do not exist in the language. This document is the grammar, the type system, the evaluation rules, the v0 prelude of relations and accessors, the seven succession laws of the Registry as programs, the error classes a program can be rejected with, and the versioning rule. The Rust evaluator of SPR-017 (`rh-registry`) must reproduce every golden case bit for bit; the Python reference is the independent oracle, as `bin/gen_fixed_vectors.py` is for `rh-core::fixed`.

## 0. Scope and non-goals

**In v0:**
- Expressions over fixed-point numbers, integers, booleans, Registry enums and entity references (§3).
- Bounded aggregates (`count`, `any`, `all`, `sum`, `min`, `max`) over declared relations, nested at most two deep (§5.4).
- Sort keys (tuples and bounded lists of tuples) for ordering candidates (§3.4).
- Effect programs: ordered, conditional, bounded emission of lifecycle commands (§6).
- Program kinds: succession laws (`candidates`, `eligible`, `order`), verb `effects`, vow `fulfilled` and `conflict`, and the reserved overlay `derive` (§7).
- The seven succession laws of `specs/manifest/registry_v0.toml` as programs (§8).

**Not in v0** (each is a `needs_code` gap or a later version, §10): user-defined functions, recursion, unbounded iteration, strings of any kind, floats, money (M4 is P1b), timers scheduled from a program (`schedule_outcome` of `05.5` §2.1 is v0.2 at the earliest), event emission from a program (the verb's own event is emitted by the host system), writes of any kind other than a queued command, relations beyond the prelude (§5.3; adding one is code in a contract crate).

## 1. Principles the language enforces

| Rule | Source | How the language holds it |
|---|---|---|
| Mechanics come only from the Registry; no free-string semantics | ADR-0010; `CLAUDE.md` | There is no string type. Every name is a Registry id (enum value, column, relation, command, RNG purpose, parameter) or a bound variable, resolved at compile time (§4.7) |
| The simulation decides; LLMs describe | ADR-0034 | A program is Registry data compiled at load; nothing at run time can replace it. The language has no input channel |
| Fixed point for thresholds; no floats in plane A | `05.1` §6 item 4; SPR-003 ruling | Numbers are `int`, `q8_8` or `q16_16` with the `rh-core::fixed` rules (§3.1, §4.3). A decimal literal is compiled to fixed point by exact half-to-even rounding (`05.5` §3.2); exponent or float syntax is rejected (`E_FLOAT_LITERAL`) |
| Determinism: no hash-ordered iteration; ties by a stated total order | ADR-0006; `05.1` §6 item 5 | Relations yield their elements in the canonical order of §5.2; aggregates fold in that order; sorting is by key, then entity id (§3.4, §7.2) |
| Keyed RNG only, with a declared purpose | `05.1` §6 item 2; ADR-0006 | `draw` and `chance` take a purpose the declaring system lists in its `[[rng]]`, an entity and a literal draw index; no other randomness exists (§4.6) |
| Single writer; effects are queued commands to the owner, never inline writes | `05.1` §2 rules 1–2, 5; `05.5` §2.1 | An effect program can only emit commands the declaring system's spec lists in `[[commands]]`, to the command's owner (§6); there is no assignment |
| Plane A never reads plane B or C | ADR-0024 | A read of a column whose plane is not `A` is rejected (`E_PLANE`); a read the declaring system does not declare in `[[reads]]` or own is rejected (`E_UNDECLARED_READ`) |
| No spontaneous reality | `docs/01` principle 8 | A program cannot create an entity, a column or an event; it can only queue commands that the owner applies through its own simulated path |
| Tragic, not slapstick | `docs/01` principle 9 | No rule here filters an outcome for its tone; the Registry's authors write the programs for the saga register |
| Totality | `05.5` §2.1; ADR-0010 v1.1 item 4 | §4.8 |

## 2. Grammar (EBNF)

Programs are TOML strings in the Registry (§7). A program is one of four syntactic forms, chosen by the program kind: an expression (`expr`), a relation expression (`relation`), an effect list (`effects`), or an expression of key type (`expr` whose type is `key`, §3.4).

```ebnf
(* ----- lexical ----- *)
Ident     = ( Letter | "_" ) { Letter | Digit | "_" } ;         (* not a Keyword *)
Keyword   = "and" | "or" | "not" | "if" | "then" | "else" | "when" | "for" | "in"
          | "where" | "true" | "false" | "none" | "div" | "mod" ;
Int       = Digit { Digit } ;
Decimal   = Digit { Digit } "." Digit { Digit } ;                (* no exponent, no bare "."; see E_FLOAT_LITERAL *)
Comment   = "#" { any character except newline } ;             (* ignored, like whitespace *)

(* ----- programs ----- *)
program    = expr ;                                 (* kinds eligible, order, fulfilled, conflict, derive *)
candidates = relation ;                             (* kind candidates *)
effects    = [ effect { ";" effect } ] ;            (* kind effects; the empty program emits nothing *)
effect     = [ "when" expr ":" ] action
           | "for" Ident "in" relation [ "where" expr ] ":" action ;
action     = Ident "(" [ named_arg { "," named_arg } ] ")" ;   (* Ident is a command name *)
named_arg  = Ident ":" expr ;

(* ----- expressions, lowest precedence first ----- *)
expr       = or_expr ;
or_expr    = and_expr { "or" and_expr } ;
and_expr   = not_expr { "and" not_expr } ;
not_expr   = "not" not_expr | cmp_expr ;
cmp_expr   = add_expr [ cmp_op add_expr ] ;         (* non-associative: a < b < c is E_SYNTAX *)
cmp_op     = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
add_expr   = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr   = unary { ( "*" | "/" | "div" | "mod" ) unary } ;
unary      = "-" unary | primary ;
primary    = Int | Decimal | "true" | "false" | "none"
           | conditional
           | aggregate
           | column_read
           | enum_lit
           | call
           | tuple
           | list_key
           | Ident                                  (* a bound variable *)
           | "(" expr ")" ;
conditional = "if" expr "then" expr "else" expr ;
column_read = Ident "." Ident "(" expr ")" ;        (* entity.column(entity_expr) *)
enum_lit    = Ident "." Ident ;                     (* enum_type.value; never followed by "(" *)
call        = Ident "(" [ expr { "," expr } ] ")" ; (* a builtin, an accessor, params.x and calendar.x
                                                       are written as Ident "." Ident without "(" (see §4.7) *)
aggregate   = agg "(" Ident "in" relation [ "where" expr ] [ ":" expr ] [ "else" expr ] ")" ;
agg         = "count" | "any" | "all" | "sum" | "min" | "max" ;
relation    = Ident "(" [ expr { "," expr } ] ")" ;  (* a prelude relation, §5.3 *)
tuple       = "(" expr "," expr { "," expr } ")" ;   (* two or more elements; key type only *)
list_key    = "[" expr "for" Ident "in" relation "]" ;  (* key type only *)
```

Disambiguation rules:
- `Ident "." Ident "("` is a column read; `Ident "." Ident` without `(` is an enum literal, a parameter (`params.<name>`) or a calendar constant (`calendar.<name>`), decided by the first identifier.
- `Ident "("` is an aggregate when `Ident` is one of the six aggregate names **and** the token after `(` is an identifier followed by `in`; otherwise it is a call (`min(a, b)` and `min(x in R: e else d)` are both valid).
- A parenthesised expression with one element is that expression; with two or more, a tuple.
- `-` binds tighter than `*`: `-a * b` is `(-a) * b`. Comparison is non-associative; `not` binds looser than comparison: `not a == b` is `not (a == b)`.
- Programs are UTF-8; only ASCII is meaningful. Whitespace and comments separate tokens and are otherwise ignored.

## 3. Type system

### 3.1 Scalar types

| Type | Storage | Values | Literal | `rh-core` type |
|---|---|---|---|---|
| `bool` | 1 bit | `false`, `true` | `true`, `false` | `bool` |
| `int` | `i64` | −2⁶³ … 2⁶³−1 | `Int` | `i64` (ticks, counts, ids as integers) |
| `q8_8` | `i16` bits, value bits/256 | [−128, 128 − 1/256] | `Decimal` or `Int` in a `q8_8` context | `rh_core::fixed::Q8_8` |
| `q16_16` | `i32` bits, value bits/65536 | [−32768, 32768 − 1/65536] | `Decimal` or `Int` in a `q16_16` context | `rh_core::fixed::Q16_16` |
| enum `E` | index | the ids of the Registry list `E`, in declaration order | `E.value` | the column's enum |
| entity `K` | `u64` id | the ids of entities of kind `K`, plus `none` (id 0) | `none` | `PersonId` and the other typed ids |

`05.5` §2.1 writes the fixed types `q8.8` and `q16.16`; they are the same types, spelled here as the column types of the system specs (`q8_8`, `q16_16`). There is no money type in v0 (`Money` arrives with M4 F0, P1b) and no float type at all.

**Enum types** are the Registry's id lists (`sex`, `death_cause`, `vocations`, `title_tiers`, `succession_laws`, `vow_kinds`, `opinion_modifiers`, …) and the per-column enums the system specs declare (`title.state`, `claim.status`, `artifact.state`, …). The environment the checker sees (§7.4) names each enum column's type. Enums have no order in the language: `ordinal(e)` gives the declaration index as an `int` when an order is wanted, which makes the dependency on list order explicit (reordering an id list a program takes the ordinal of is then a C3 change, §11).

**Entity kinds** are the `entity` values of `[[owns]]` across the system specs: `person`, `title`, `office`, `claim`, `house`, `polity`, `artifact`, `vow`, `region`, … The value `none` belongs to every kind and is the id 0, the convention of every `… or 0` column of the specs (`person.father`, `title.holder`). `exists(e)` is true iff `e` is not `none` and a row with that id exists in the snapshot.

### 3.2 Literals and their types

- An `Int` literal is polymorphic over `int`, `q8_8` and `q16_16`; a `Decimal` literal over `q8_8` and `q16_16`. The literal takes the type the other operand, the parameter or the result demands; where nothing demands one, an `Int` literal is `int` and a `Decimal` literal is `q16_16`. Resolution is by the rules of §4.1.
- A decimal literal becomes bits by the rule of `05.5` §3.2: the exact rational value of the text, times 2^FRAC_BITS, rounded half to even, with no intermediate float. `0.1` in `q8_8` is 26 bits (25.6 rounded), in `q16_16` 6554 bits (6553.6 rounded).
- A literal that does not fit its resolved type is rejected (`E_LITERAL_RANGE`), never saturated: `200` as `q8_8`, `40000` as `q16_16`, `9223372036854775808` as `int`.
- The only numeric literal forms are `Int` and `Decimal`. Anything else that starts with a digit (`1e3`, `1.`, `1.5e-2`, `0x10`, `1_000`, `1.5f`) is rejected as `E_FLOAT_LITERAL`, and so are the words `inf` and `nan` wherever a number is expected (they are unknown names, `E_UNKNOWN_NAME`).

### 3.3 Operators and builtins

Every operator and builtin is total on well-typed arguments. `num` stands for `int | q8_8 | q16_16`; `q` for `q8_8 | q16_16`; both operands of a binary numeric operator have the same type, and so does the result. There is no implicit conversion between `int`, `q8_8` and `q16_16` (the compile-fail rule of `rh-core::fixed`).

| Form | Types | Meaning |
|---|---|---|
| `a + b`, `a - b` | `num × num → num` | saturating add, subtract (§4.3) |
| `a * b` | `num × num → num` | saturating multiply; fixed point rounds half to even |
| `a / b` | `num × num → num` | quotient rounded half to even, saturating; `b` must be a non-zero constant (§4.4) |
| `a div b`, `a mod b` | `int × int → int` | Euclidean quotient and remainder (`mod` ≥ 0); `b` a non-zero constant |
| `-a` | `num → num` | saturating negation (`-MIN = MAX`) |
| `a == b`, `a != b` | same scalar type | equality; fixed point compares bits; entities compare ids; `none == none` |
| `a < b`, `<=`, `>`, `>=` | `num × num → bool` | numeric order (fixed point: order of bits) |
| `not a`, `a and b`, `a or b` | `bool` | logic; `and` and `or` do not short-circuit in effect (both sides are total; evaluation order is left to right, §4.2) |
| `if c then a else b` | `bool × T × T → T` | conditional; `T` any type, including `key` |
| `abs(a)` | `num → num` | saturating absolute value (`abs(MIN) = MAX`) |
| `min(a, b)`, `max(a, b)` | `num × num → num` | the smaller, the larger |
| `clamp(x, lo, hi)` | `num³ → num` | `min(max(x, lo), hi)`: defined for `lo > hi` too (it is then `hi`) |
| `safe_div(a, b, d)` | `num³ → num` | `d` if `b == 0`, else `a / b` with any `b` (the guard of `05.5` §2.1) |
| `widen(a)` | `q8_8 → q16_16` | exact |
| `narrow(a)` | `q16_16 → q8_8` | round half to even to 8 fractional bits, then saturate |
| `q8(n)`, `q16(n)` | `int → q8_8`, `int → q16_16` | the integer as fixed point, saturating |
| `floor(a)`, `round(a)` | `q → int` | floor; nearest, half to even (always fits `int`) |
| `ordinal(e)` | enum → `int` | declaration index, from 0 |
| `exists(e)` | entity → `bool` | §3.1 |
| `draw(purpose, e, k)` | → `int` | keyed uniform draw, §4.6 |
| `chance(purpose, e, k, p)` | `… × q16_16 → bool` | `draw_hits` of `rh-core::fixed::draw`, §4.6 |
| `up(x, p)`, `common(x, p)`, `up_agnatic(x, p)`, `common_agnatic(x, p)` | `person × person → int`, `→ person` | genealogical accessors, §5.3 |
| `related(a, b, n)` | `person × person × Int → bool` | `a` and `b` are distinct persons within `n` degrees of kinship (`n` an `Int` literal in 1..4): the predicate form of `kin`, §5.3 |
| `entity.column(e)` | entity `K` → the column's type | a declared read, §4.5 |
| `params.name` | → `int` or `q16_16` | a Registry parameter of the declaring system, §4.7 |
| `calendar.days_per_year` | → `int` | the manifest's calendar, §4.7 |
| aggregates | §5.4 | |

### 3.4 Keys

The `order` program of a succession law evaluates to a **key**. A key is a scalar, a tuple of two or more keys, or a bounded list of keys built by `[ key_expr for y in relation ]`. All keys an `order` program produces have the same static type (lists may differ in length). Keys compare by a total order:

- scalars: `int` and fixed point by value; `bool` with `false < true`; an enum by declaration index (so `ordinal` is implied inside a key); an entity by id with `none` first;
- tuples: element by element, lexicographically;
- lists: lexicographically, element by element; when one list is a prefix of the other, the shorter comes first (an ancestor before its line).

Candidates sort by key ascending, then by entity id ascending (§7.2). Key values never enter columns: a key exists only between an `order` program and the host's sort.

### 3.5 What has no type

Relations, programs and command names are not values: a relation appears only after `in` or as the `candidates` program; a command only as an action. Keys exist only as the result of an `order` program and inside its tuples and lists. The checker rejects any other use (`E_TYPE`).

## 4. Semantics

### 4.1 Type resolution

1. Column reads, variables, parameters, accessors and builtins with fixed signatures have concrete types.
2. In a binary operator, a comparison, `min`/`max`/`clamp`/`safe_div`, a conditional and the body-or-else of an aggregate, the operands unify: a concrete type is adopted by a polymorphic literal; two literals default (`Int` to `int`; any `Decimal` to `q16_16`); two different concrete types are `E_TYPE`.
3. A literal passed to a parameter of a known type (a command argument, `q8(n)`, `chance(..., p)`) adopts that type.
4. A literal with no context (a key element, the body of `sum` over nothing concrete) takes its default.
5. A relation's arguments, `kin`'s degree and `draw`'s index must be constants (§4.4); the degree and the index are `Int` literals.

### 4.2 Evaluation order

Evaluation is innermost first, left to right, in the order of the syntax tree. The order is observable only through the ordered emission of effects (§6) and through diagnostics; `draw` is keyed by its arguments, not by evaluation order, so no construct can change which draw a call returns. A relation yields its elements in its canonical order (§5.2); an aggregate folds over them in that order; a `for` effect emits in that order. The host evaluates each program over one snapshot of plane A (the lagged or same-instant view its `[[reads]]` declare; lags are the host's business, not the language's).

### 4.3 Arithmetic: the `rh-core::fixed` rules

Every result is computed exactly (as a rational), rounded once with one rule, then saturated. The rule and the edge cases are those of `crates/rh-core/src/fixed/` (SPR-010 and its Architect rulings), restated here as the contract; the golden file pins every row.

| Operation | Rule |
|---|---|
| `int` `+`, `-`, `*`, unary `-`, `abs` | `i64` saturating (`MAX + 1 = MAX`, `MIN - 1 = MIN`, `-MIN = MAX`, `abs(MIN) = MAX`) |
| `q` `+`, `-`, unary `-`, `abs` | saturating at the type's `MIN` and `MAX` |
| `q` `*` | the exact product of the values, rounded half to even to the type's fractional bits, then saturated (`Q8_8: 100 * 2 = MAX`) |
| `num` `/` | the exact quotient, rounded half to even to the type's precision (whole units for `int`), then saturated; `MIN / -1 = MAX` (the Architect's ruling on SPR-010 GAP-1); `int`: `5 / 2 = 2`, `7 / 2 = 4`, `-5 / 2 = -2` |
| `int` `div`, `mod` | Euclidean: `a = (a div b) * b + (a mod b)` with `0 <= a mod b < |b|`; `-7 div 2 = -4`, `-7 mod 2 = 1`, `7 div -2 = -3`; `MIN div -1 = MAX`; `MIN mod -1 = 0` |
| `safe_div(a, b, d)` | `d` when `b` is zero, else as `/` |
| `widen` | exact |
| `narrow`, `round` | round half to even, then saturate (`narrow`) or exact (`round`) |
| `q8(n)`, `q16(n)` | saturating (`q16(40000) = MAX`) |
| `floor` | toward −∞ |
| `sum` | left fold of saturating `+` from zero, in relation order |
| `count` | the number of elements passing `where`, at most the relation's bound |

Rounding happens before saturation, so a result is always the representable value nearest the exact one; half-to-even is the one rounding rule (`round.rs`, DECISION), and it is what Python's `round(Fraction)` does, which is why the reference needs no rounding code of its own.

### 4.4 Constants and guarded division

A **constant** is an `Int` or `Decimal` literal (under unary minus or not), `calendar.days_per_year`, or `params.<name>` whose Registry bounds `[lo, hi]` exclude zero (`lo > 0` or `hi < 0`). The divisor of `/`, `div` and `mod` must be a constant whose value is not zero, or the program is rejected (`E_DIV`); every other division is written `safe_div(a, b, d)` with an explicit default. This is `05.5` §2.1's "no division by a non-constant without a guard" made checkable. `kin`'s and `related`'s degree, `ancestors`'s and `descendants`'s generation count, `draw`'s and `chance`'s index `k` must be `Int` literals (`E_CONST`).

### 4.5 Reads

`entity.column(e)` reads column `entity.column` of the entity `e`, which must have the kind the column is keyed by: the table's own entity, except that the life-course log (`life_course.*`) is keyed by `life_course.person_id` and is read with a `person`. The checker accepts the read only if the declaring system owns the column or lists it in `[[reads]]` (`E_UNDECLARED_READ`), and only if the column's plane is `A` (`E_PLANE`). A relation (§5.3) reads the columns of its definition; using a relation requires those reads of the declaring system too. **Missing entity:** a read through `none`, or of an id with no row in the snapshot, yields the **zero of the column's type** (`0`, `0.0`, `false`, the enum's first value, `none`), and the evaluator counts it in its diagnostics (`missing_reads`); a program that must distinguish the cases tests `exists(e)` first. The count is not part of the value and does not enter hashes; invariant tests may require it to be zero.

### 4.6 Randomness

`draw(purpose, e, k)` is the keyed draw `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` of `05.1` §6 item 2, with `system_id` the declaring system's, `t = now`, `entity_id = e` (an entity reference of any kind; `none` is id 0) and `k` the literal index. Its value is the 64-bit draw shifted right by one bit, an `int` in [0, 2⁶³). `chance(purpose, e, k, p)` is `rh_core::fixed::draw_hits(u, p)` on the unshifted 64-bit draw `u`: true iff `u / 2⁶⁴ < p`, so `p <= 0` never hits and `p >= 1` always hits. `purpose` must be a purpose the declaring system lists in `[[rng]]` (`E_RNG_PURPOSE`). Because the key fixes the value, two calls with the same arguments return the same value, and the call-site rule of `05.1` §6 item 3 holds by construction: every draw has its own `(purpose, entity, k)`. The host supplies the draws; the golden cases list them explicitly, so the reference and the Rust evaluator see the same numbers without re-implementing ChaCha8.

### 4.7 Names

Every identifier resolves at compile time to exactly one of: a keyword; a bound variable of the program kind (§7.1) or of an enclosing aggregate, `for` or list comprehension; a builtin (§3.3); a prelude relation or accessor (§5.3); the prefix of a column read, an enum literal, `params` or `calendar`. An unresolved name is `E_UNKNOWN_NAME`; a variable that shadows another is `E_SYNTAX`. `params.<name>` is a parameter of the declaring system's Registry entry, typed `int` when both bounds are written as integers and `q16_16` otherwise, with the manifest's value compiled by the rule of `05.5` §3.2. `calendar.days_per_year` is the manifest's calendar (`int`).

### 4.8 Totality

Every well-typed program terminates and yields a value of its declared type. Proof sketch: the syntax has no recursion and no user-defined names, so evaluation is structural over a finite tree; the only iteration is over relations, each of which yields at most its declared bound (§5.2), and nesting is limited to depth 2 with a product of bounds of at most 65,536 (`E_BOUND`); every partial operation has a defined result (§4.3: saturation; §4.4: guarded division; §4.5: missing reads; §5.3: empty paths and absent ancestors; §5.4: `else` for empty `min` and `max`); draws are pure functions of their keys. The checker enforces every precondition at compile time, so the evaluator has no error path: its result type is the value, plus diagnostics (`missing_reads`, `truncated_relations`).

## 5. Relations and iteration

### 5.1 What a relation is

A relation is a named, bounded, ordered set of entities computed by the owning system's contract crate from declared columns (`05.5` §2.1: "only over declared relations with a declared upper bound"). Relations are code, not data: the prelude (§5.3) is the v0 set, each with its owner, its arguments, the columns it reads, its bound and its order. Adding a relation is a `needs_code` gap in a contract crate (`rh-contracts`), never an effect-language change.

### 5.2 Canonical order and bounds

A relation yields its elements in **ascending entity id** (`none` never appears), except the path relations `line` and `line_agnatic`, which yield in path order. A relation yields at most its bound; when the underlying set is larger, the elements beyond the bound in canonical order are dropped and the evaluator counts a `truncated_relations` diagnostic (an invariant test may require zero). This keeps every program bounded without a hidden failure.

### 5.3 The v0 prelude

**Genealogy comes from the life-course log** (M2: `life_course.person_id`, `life_course.mother`, `life_course.father`, `u64` with 0 for unknown; `life_course.birth_tick`, `i64`; `life_course.sex`, the enum of `person.sex` at entry; REV-P1a NEW-3 and NEW-7), not from the live ledger: the ledger's dead rows are compacted at Bookkeeping of the death instant (SPR-011 ruling 5), while the log keeps every person who ever lived, so a line through a dead intermediary (a grandson whose father predeceased the holder) stays complete. The log is append-only, so its users declare these reads `same_instant`. The genealogy relations read `person_id`, `mother` and `father`; the law programs read `birth_tick` and `sex` of the persons on a line. Siblings are ordered by `life_course.birth_tick`, ties (twins) by `life_course.person_id`, which every key below ends with. "Ancestor-or-self of `x` at height `h`" is the set `A_h(x)`: `A_0 = {x}`, `A_{h+1} = {father(y), mother(y) : y ∈ A_h} \ {none}`; the agnatic variant follows fathers only. Heights are bounded by `KIN_DEGREES_MAX = 4` for `kin`, `related`, `up` and `common`, and generations by `DESCENT_MAX = 6` for `ancestors`, `descendants` and the lines. Children are ordered by `life_course.person_id`, so every order below is deterministic without hashing.

| Name | Owner | Yields | Reads | Bound | Definition |
|---|---|---|---|---|---|
| `children(p)` | M2 | `person` | the three genealogy columns | 64 | every person whose mother or father is `p`, alive or dead; id order |
| `kin(p, n)` | M2 | `person` | genealogy | 256 | every person other than `p` reachable from `p` in at most `n` parent-or-child steps (`n` an `Int` literal in 1..4); alive or dead; id order |
| `related(a, b, n)` | M2 | `bool` | genealogy | | `b` is in `kin(a, n)` (false when either is `none`); the predicate form the review calls `kin(a, b, n)` |
| `ancestors(p, g)` | M2 | `person` | genealogy | 64 | the ancestors of `p` within `g` generations (`g` an `Int` literal in 1..6), by generation (parents first), then id |
| `descendants(p, g)` | M2 | `person` | genealogy | 256 | the descendants of `p` within `g` generations, alive or dead, by generation (children first), then id |
| `line(a, x)` | M2 | `person` | genealogy | 6 | the persons on the shortest parent-chain from `x` up to `a`, listed from `a`'s child down to `x` (`a` excluded, `x` included); among equally short chains, the one that takes the father at the first step where the chains differ; `line(x, x)` and a pair with no chain of length ≤ 6 yield the empty list |
| `line_agnatic(a, x)` | M2 | `person` | `person_id`, `father` | 6 | as `line`, following fathers only |
| `up(x, p)` | M2 | `int` | genealogy | | the least `h` in 0..4 such that some member of `A_h(p)` is an ancestor-or-self of `x` within 6 generations; `5` if there is none |
| `common(x, p)` | M2 | `person` | genealogy | | the member of `A_h(p)` found by `up`; when several qualify at that height, the first in father-first enumeration order (`L_0 = [p]`, `L_{h+1}` = for each `y` in `L_h`: `father(y)`, then `mother(y)`, each once, `none` skipped); `none` when `up` is 5 |
| `up_agnatic(x, p)`, `common_agnatic(x, p)` | M2 | `int`, `person` | `person_id`, `father` | | as `up` and `common` through fathers only, on both sides |
| `titles(p)` | M7 | `title` | `title.holder` | 16 | titles whose holder is `p`; id order |
| `vassals(t)` | M7 | `title` | `title.liege` | 64 | titles whose liege is `t`; id order |
| `offices(p)` | M7 | `office` | `office.holder` | 16 | offices whose holder is `p`; id order |
| `court(t)` | M7 | `person` | `title.liege`, `title.holder`, `office.title`, `office.holder` | 128 | the holders of `t`'s vassal titles and of `t`'s offices, `none` excluded, each once; id order |
| `claims_on(t)` | M7 | `claim` | `claim.title` | 64 | claims on `t`; id order |
| `claims_of(p)` | M7 | `claim` | `claim.claimant` | 16 | claims of `p`; id order |
| `vows_of(p)` | S7 | `vow` | `vow.vower` | 8 | vows of `p`, every status; id order |

`line` and `common` are defined cognatically with a stated father-first tie rule because the Registry's `primogeniture` is one law, not a family of laws (decision D4, §12); agnatic laws use the `_agnatic` forms, which never cross a mother.

### 5.4 Aggregates

`agg(y in R where c: e else d)`: `y` is bound to each element of `R` in canonical order; elements where `c` is false are skipped; `e` is the body, `d` the default for an empty fold.

| Aggregate | Body | Result | Empty | Notes |
|---|---|---|---|---|
| `count(y in R where c)` | none | `int` | 0 | no body, no `else` |
| `any(y in R where c)` | none | `bool` | `false` | `where` is the predicate |
| `all(y in R where c)` | none | `bool` | `true` | |
| `sum(y in R where c: e)` | `num` | the body's type | 0 | saturating left fold |
| `min(y in R where c: e else d)`, `max(…)` | `num` | the body's type | `d` | `else` is required |

`any` and `all` are also written with a body: `any(y in R: e)` means `any(y in R where e)`. Nesting: an aggregate, a `for` effect or a list comprehension inside another is allowed to depth 2; deeper nesting, or a product of the relations' bounds above 65,536, is `E_BOUND`.

## 6. Effects

An effect program (`verbs.<V>.effects`) emits an ordered list of lifecycle commands; it writes nothing itself (`05.5` §2.1: "effects are queued, never applied inline"; `05.1` §2 rule 5). The host (the system that emitted the verb) submits them through `StepCtx::submit` in the listed order, with the verb's event as every command's `cause`; `rh-core` queues each for its owner and the owner applies it when it next runs (`05.1` §2 rule 2).

- An action `Cmd(person: e, a: v, …)` names a command of the Registry `[commands]` table (§7.4) and gives every parameter once by name (`E_ARITY` otherwise). The `person` argument is the queue key (`rh_core::command::Command::person`); the remaining arguments are the command's parameters in the declared order.
- The checker accepts an action only if the declaring system's spec lists the command in `[[commands]]` with `target` equal to the command's owner (`E_UNDECLARED_COMMAND` when not listed; `E_COMMAND_OWNER` when the listed target is not the owner, which is the rejected "write to a non-owned column"), and only if the owner is a registered system of the world (`E_OWNER_NOT_REGISTERED`; SPR-006 G5: a sender whose owner is not registered is refused). A command to the declaring system's own columns is still a queued command, to itself, and must be listed the same way.
- `when c: A` emits `A` iff `c`; `for y in R where c: A` emits `A` once per element of `R` passing `c`, in canonical order.
- **Encoding into `Command::args`** (`Vec<i64>`): `int` as itself; `q8_8` and `q16_16` as their raw bits; `bool` as 0 or 1; an enum as its ordinal; an entity as its id reinterpreted as two's-complement `i64` (`none` is 0). The golden file renders a command as `Name(person=<id>, args=[<i64>, …])`, the exact queue form.

Neither `draw` nor a read is an effect: a program is pure except for the commands it lists. It cannot emit an event (the verb's event is the host's), schedule a timer, create an entity or touch a plane-B or plane-C value.

## 7. Program kinds, environments and the Registry schema

### 7.1 Kinds and their signatures

| Kind | Registry location | Bound variables | Result type |
|---|---|---|---|
| `candidates` | `[succession_laws.<law>]` | `holder: person` (the previous holder, dead or deposed, readable in the instant per SPR-011 ruling 5), `title: title`, `now: int` | relation of `person` |
| `eligible` | same | as above plus `x: person` (the candidate) | `bool` |
| `order` | same | as `eligible` | `key` |
| `effects` | `[verbs.<verb>]` | `actor: person`, `target: person` (`none` if the verb has no target), `instrument: artifact` (`none` if none), `now: int` | effects |
| `fulfilled` | `[vow_kinds.<kind>]` | `vower: person`, `target` of the kind's `target` entity kind, `now: int` | `bool` |
| `conflict` | same | as `fulfilled` | `q8_8`, expected in [0, 1] (S7 clamps) |
| `derive` | `[overlays.<overlay>]` | `self: person`, `now: int` | `q8_8` (reserved: the psychological core it reads is P3.2b; no v0 Registry entry declares one) |

`now` is the instant's tick (`int`, days since the epoch, `05.1` §4.1). The declaring system is: the owner of the law's consumer, `M7.dynasty`, for succession laws; the verb's `emitted_by` for effects; `S7.vows` for vow predicates; `T1.minds` for overlays.

### 7.2 The succession protocol

For a title `t` whose holder `h` died or was deposed at `now`, under law `L` (`title.law(t)`), the host computes the law's line:

1. `C` = the elements of `candidates` evaluated with `holder = h`, `title = t`.
2. `E` = `[x in C where eligible(x)]`, in canonical order.
3. The **line** is `E` sorted by `(order(x), id(x))` ascending, the id as the final tie-break (§3.4).

The first of the line is the legal heir; M7's arbitration (`M7.dynasty` submodel `arbitrate`) applies its own exclusions (the deposed ruler, persons in holy orders) and scoring after this step. The regent rule ("the nearest adult kin in the law's order") reads the same line. The law never reads claims, offices or opinions: those are M7's arbitration terms, not the law.

### 7.3 Registry schema (the C2 change SPR-017 lands with the loader)

The programs live next to the ids they belong to. `ids` stays the enumeration manifests reference; the per-id tables carry the programs; a lint (SPR-017) requires every id with a program table to be in `ids`, and every law in `ids` to have the three programs from the first world that registers `M7.dynasty`.

```toml
[effect_language]
version = "0.1.0"                 # this document's version; the Registry's registry_version covers it

[succession_laws]
ids = ["primogeniture", "ultimogeniture", "seniority", "partition", "elective", "tanistry", "sortition"]

[succession_laws.primogeniture]   # §8
candidates = "kin(holder, 4)"
eligible   = "…"
order      = "…"

[verbs]
MURDER_ATTEMPT = { act_classes = ["harm"], emitted_by = "T1.minds", effects = "when chance(act_outcome, actor, 0, params.attempt_success): ScheduleDeath(person: target, cause: death_cause.murder, culprit: actor)" }

[vow_kinds]
CLAIM_TITLE = { kind_weight = 0.8, target = "title", fulfilled = "title.holder(target) == vower", conflict = "0.0" }

[commands]                        # the signatures the checker and the queues share (new section)
ScheduleDeath = { owner = "M2.demography", person = "victim", params = [{ name = "cause", type = "enum:death_cause" }, { name = "culprit", type = "person" }] }
```

The `[commands]` section is new: today a command's owner is declared by the owner's `AccessSpec::applies` and named by each sender's `[[commands]] target` (SPR-006). The checker needs the parameter list, which nothing declares today, so the Registry gains it; SPR-017's lint checks `[commands]` against the specs' `[[commands]]` tables in both directions. The Rust loader's `IdList` form of `[succession_laws]` and the `Verb` type gain the optional program fields; the Python reference validator reads only `ids`, so no manifest changes.

### 7.4 The environment the checker sees

A program is checked against: the Registry enums and the `[commands]` signatures; every column of the system specs with its entity, type, enum type and plane (`[[owns]]`); the declaring system's `[[reads]]`, `[[commands]]`, `[[rng]]` and Registry parameters with their bounds; the set of registered systems; the manifest's calendar. The golden file carries one such environment in full (`[registry]`), so the checker's decisions are reproducible without the engine.

## 8. The succession laws of the Registry, as programs

The seven laws are the ids of `[succession_laws]`; M7's spec names all seven (`exit_hazards`: the law multiplier covers every id; `arbitrate`: "the legal heir by the title's law"). Each program below is checked and evaluated in the golden file on a fixture dynasty (`world.dynasty`) and its expected line is pinned. Every lineage law orders siblings by `life_course.birth_tick` with ties broken by `life_course.person_id`, and inherits **by representation**: a dead elder child's line comes before a living younger child (the golden cases `succession_primogeniture` and `succession_representation_grandson_before_living_younger_sons`). The sex rule of each law is stated in its comment and is a design decision (D4, §12): the Registry has one `primogeniture`, so it is male-preference cognatic; `seniority` and `tanistry` are agnatic, as their sources are; the golden file also carries `absolute_primogeniture` and `agnatic_primogeniture` as data-only extensions, to show that a variant is a Registry edit. Under male-preference cognatic order a dead daughter's son comes after every son's line and before a living younger daughter's line: the daughter's own key (female, her birth tick) places her line, and she is represented by it.

```toml
[succession_laws.primogeniture]
# Sex rule: male-preference cognatic (sons by age, then daughters by age, each represented by their line).
# Primogeniture by representation, lineal. Descendants first (sons by age, each son's line before the next
# son, then daughters likewise), then the lines of the nearest common ancestor (siblings and their lines, then
# uncles and aunts and theirs). An ancestor never inherits from a descendant (common(x, holder) != x).
candidates = "kin(holder, 4)"
eligible   = "person.alive(x) and x != holder and up(x, holder) <= 3 and common(x, holder) != x"
order      = "(up(x, holder), [(life_course.sex(y) == sex.female, life_course.birth_tick(y), y) for y in line(common(x, holder), x)])"

[succession_laws.ultimogeniture]
# Sex rule: male-preference cognatic. As primogeniture, youngest first within each set of siblings.
candidates = "kin(holder, 4)"
eligible   = "person.alive(x) and x != holder and up(x, holder) <= 3 and common(x, holder) != x"
order      = "(up(x, holder), [(life_course.sex(y) == sex.female, -life_course.birth_tick(y), y) for y in line(common(x, holder), x)])"

[succession_laws.seniority]
# Sex rule: agnatic (males through fathers only; a person in a father column is male by construction).
# Agnatic seniority (the rota): the eldest male of the most senior generation of the male line, generations
# counted relative to the holder (uncles before brothers before sons), then by age.
candidates = "kin(holder, 4)"
eligible   = "person.alive(x) and x != holder and life_course.sex(x) == sex.male and up_agnatic(x, holder) <= 4 and common_agnatic(x, holder) != x"
order      = "(count(y in line_agnatic(common_agnatic(x, holder), x)) - up_agnatic(x, holder), life_course.birth_tick(x))"

[succession_laws.partition]
# Sex rule: male-preference, children only. Partible: the holder's living children, sons by age then daughters by age. M7 F1 holds one title per holder, so
# the first takes the title; what the others take is open question Q3 (§12).
candidates = "children(holder)"
eligible   = "person.alive(x) and x != holder"
order      = "(life_course.sex(x) == sex.female, life_course.birth_tick(x))"

[succession_laws.elective]
# Sex rule: none (the court as it stands). Election by the magnates: the court of the title (vassal title holders and office holders), the holder of the
# highest-tier vassal title first (the M7 spec's resolution rule for a sovereign with no candidate), the eldest first
# within a tier, office holders without a title last. Votes are open question Q4 (§12).
candidates = "court(title)"
eligible   = "person.alive(x) and x != holder"
order      = "(min(t in titles(x): ordinal(title.tier(t)) else 3), life_course.birth_tick(x))"

[succession_laws.tanistry]
# Sex rule: agnatic, adult males. The derbfine: adult males of the male line within three generations up and three down of a common ancestor;
# the eldest first (the election of the tánaiste is Q4).
candidates = "kin(holder, 4)"
eligible   = "person.alive(x) and x != holder and life_course.sex(x) == sex.male and up_agnatic(x, holder) <= 3 and count(y in line_agnatic(common_agnatic(x, holder), x)) <= 3 and common_agnatic(x, holder) != x and now - life_course.birth_tick(x) >= floor(params.majority_age * q16(calendar.days_per_year))"
order      = "life_course.birth_tick(x)"

[succession_laws.sortition]
# Sex rule: none. Lots among the court: a keyed draw per candidate (05.5 §2.1's rng_permutation). M7 must declare the purpose
# `sortition` in its [[rng]] (Q5).
candidates = "court(title)"
eligible   = "person.alive(x) and x != holder"
order      = "draw(sortition, x, 0)"
```

Reads these programs need from `M7.dynasty`: `person.alive` (M2, declared); `life_course.person_id`, `life_course.mother`, `life_course.father`, `life_course.birth_tick` and `life_course.sex` (M2, `same_instant`; M7's and T1's specs must add them, Q11); `title.tier`, `title.holder`, `title.liege`, `office.title`, `office.holder` (owned). `params.majority_age` is M7's. A dead candidate's `person.alive(x)` is a missing read after its death instant (the row is compacted) and so reads `false`, which is the right answer for eligibility. Every other fact about a person on a line (sex, birth tick) is read from the log, never from the ledger: a dead intermediary's ledger row is gone, and a ledger read of it would silently give the zero (the golden case `sum_int_from_the_ledger_misses_the_compacted_dead` shows the pitfall). The fixture's expected lines are in the golden file (`succession_*` cases).

## 9. Verb effects and vow predicates (the consumers' programs)

Verb effects (SPR-024, `T1.minds`): the four break acts of `[mental_breaks]` and the feud attack are the v0 verbs with effects. The golden file checks these programs against T1's spec (`[[commands]]`: `ScheduleDeath` and `Marry` to M2; `PetitionSeat`, `PressClaim`, `RenounceClaim`, `Resign` to M7):

```toml
MURDER_ATTEMPT.effects = "when chance(act_outcome, actor, 0, params.attempt_success): ScheduleDeath(person: target, cause: death_cause.murder, culprit: actor)"
FLED.effects           = "for o in offices(actor): Resign(person: actor, office: o)"
DESPAIR_PACT.effects   = "for c in claims_of(actor) where claim.status(c) == claim_status.active: RenounceClaim(person: actor, claim: c)"
HOLDINGS_BURNED.effects = "BurnHoldings(person: actor)"     # rejected until M4 is registered: E_OWNER_NOT_REGISTERED (the P1a world)
```

`DESPAIR_PACT` also adds `DESPAIR_PACT_WITH` opinion modifiers both ways (T1's spec, `break_act`); that is a command from T1 to itself (`OpinionModifier`), which T1's spec lists only under `applies`, not `[[commands]]`. The golden file carries both outcomes: rejected (`E_UNDECLARED_COMMAND`) against the spec as drafted, accepted against an environment where T1 declares the self-send (Q6, §12).

Vow predicates (S7, P1b): S7's spec says the completion predicate per kind and the conflict signal are effect-language programs (`fulfil`, `feed`). The programs the v0 columns can express:

```toml
AVENGE_KIN.fulfilled    = "not person.alive(target) and person.death_cause(target) == death_cause.murder"   # needs S7 to read person.death_cause (Q7)
GRUDGE.fulfilled        = "not person.alive(target)"
RECOVER_RELIC.fulfilled = "artifact.bearer(target) == vower"
DESTROY_RELIC.fulfilled = "artifact.state(target) == artifact_state.destroyed"
CLAIM_TITLE.fulfilled   = "title.holder(target) == vower"
PROTECT.fulfilled       = "false"      # ends by breaking or voiding only; Q7
PILGRIMAGE.fulfilled    = "false"      # needs person.residence (S3, P1b); Q7
AVENGE_KIN.conflict     = "if any(o in offices(vower): any(t in titles(target): office.title(o) == t)) then 1.0 else 0.0"   # serving the target; needs office reads (Q7)
```

## 10. Error classes (compile-time rejection)

A program is compiled once, at Registry load, against the environment of §7.4. A rejection names one of these codes; the golden file pins the code for each rejected program (the message text is not contractual).

| Code | Rejects |
|---|---|
| `E_SYNTAX` | a token or structure outside §2, including a chained comparison, a shadowed variable, a trailing `;` |
| `E_FLOAT_LITERAL` | a numeric token that is not `Int` or `Decimal` (`1e3`, `1.`, `.5`, `0x10`, `1_000`) |
| `E_LITERAL_RANGE` | a literal that does not fit its resolved type |
| `E_UNKNOWN_NAME` | an unresolved identifier, column, enum, enum value, command, relation, parameter or purpose |
| `E_TYPE` | an operand, argument, branch, key element or program result of the wrong type; `int` mixed with fixed point; two fixed types mixed; a comparison of enums with `<` |
| `E_ARITY` | a builtin, relation or command with the wrong number of arguments, or a command argument missing or repeated |
| `E_CONST` | a degree, draw index or other constant-only argument that is not an `Int` literal |
| `E_DIV` | a divisor that is not a non-zero constant (§4.4) |
| `E_BOUND` | nesting deeper than 2, a bound product above 65,536, a `kin` or `related` degree outside 1..4, an `ancestors` or `descendants` generation count outside 1..6 |
| `E_PLANE` | a read of a column whose plane is not `A` |
| `E_UNDECLARED_READ` | a column read, or a relation whose columns, the declaring system neither owns nor lists in `[[reads]]` |
| `E_UNDECLARED_COMMAND` | a command the declaring system's spec does not list in `[[commands]]` |
| `E_COMMAND_OWNER` | a command listed with a target that is not the command's owner |
| `E_OWNER_NOT_REGISTERED` | a command whose owner is not registered in the world |
| `E_RNG_PURPOSE` | a `draw` or `chance` purpose the declaring system does not list in `[[rng]]` |

A program with several faults is rejected with one of their codes (the checker stops at the first fault it meets, walking the program left to right, innermost first); every rejected golden program has exactly one fault, so its code is pinned. A program that compiles has no run-time failure mode (§4.8).

## 11. Versioning and change classes

- The language is versioned `MAJOR.MINOR.PATCH`, recorded in `[effect_language] version` of the Registry and in the golden file's header; v0 is `0.1.0`. The Rust evaluator refuses a Registry whose language version it does not implement.
- **Additive** changes (a builtin, a relation, a program kind, a new error class that only rejects programs that were already invalid) bump MINOR and are **C2** (Registry schema): `state_hash` is unchanged on every golden world because no existing program's value changes.
- A change to the **value of any existing construct** (a rounding rule, an order, a missing-read result, a bound, a tie rule, the draw mapping) bumps MAJOR and is **C3** as soon as any registered system evaluates a program on a golden world; until then (before SPR-021 lands) it is C2 with the golden file re-approved by the director. The golden file is the detector: a change that alters any expected value is at least the class above.
- Editing a **program** in the Registry (a law, a verb's effects) is a Registry edit: C2 for the schema, and C3 if a golden world's hash moves (`docs/13` §4.2: CI detects the class independently). Adding a law or a verb with effects is `data_only` (S4), C2.
- Reordering an id list that any program takes the `ordinal` of, or compares in a key, is C3 (§3.1).
- A PR that changes `effect_ref.py` must keep every golden case; a PR that changes the golden file says which rows moved and why.

## 12. Decisions taken here, and open questions

Each decision is the Architect's recommendation with a confidence; the director approves the set at the SPR-017 plan gate. Each open question carries a recommendation.

**Decisions**

| # | Decision | Why | Confidence |
|---|---|---|---|
| D1 | One rounding rule everywhere, including `int` division: round half to even, then saturate; `div`/`mod` are the Euclidean integer forms for calendar arithmetic | Matches `rh-core::fixed` (`Money::checked_div_int` already rounds a count division half to even); one golden table, one reference | 8/10 |
| D2 | A read of a missing entity is the zero of its type plus a diagnostic, not an error | Totality with no `Option` type in the language; the specs already use `0` for "none"; invariants can require zero missing reads | 7/10 |
| D3 | Division needs a non-zero constant divisor (literal, calendar constant, or a parameter whose bounds exclude zero); otherwise `safe_div` with a default | The literal reading of `05.5` §2.1; the checker can prove it from the Registry bounds | 8/10 |
| D4 | `primogeniture` is male-preference cognatic and lineal; `seniority` and `tanistry` are agnatic; `line`/`common` prefer the father at ties; variants are Registry entries | The Registry has one id per law; sex-rule variants are exactly the data-only extensions the language exists for | 6/10 |
| D5 | Ascendants never inherit from descendants (`common(x, holder) != x`) | The usual reading of primogeniture; without it a living mother precedes the holder's brothers | 7/10 |
| D6 | Keys may hold bounded lists, compared lexicographically with a prefix first | The only way to state lineal primogeniture without recursion or a Rust-side rank | 7/10 |
| D7 | Enums carry no order; `ordinal` makes list order an explicit dependency | Keeps id-list reordering a C0 edit unless a program depends on it | 8/10 |
| D8 | Relations and accessors are code in contract crates (the prelude); programs are data | `05.5` §2.1 says iteration is over *declared* relations; a relation's bound and reads must be auditable | 8/10 |
| D9 | `draw` returns the 64-bit draw shifted right by one (an `int` in [0, 2⁶³)); `chance` is `draw_hits` on the unshifted draw | `int` is signed; `chance` keeps the exact mapping of `draw.rs` | 8/10 |
| D10 | A `[commands]` Registry section with parameter lists, lint-checked against the specs | The checker cannot type a command argument otherwise; the specs' `[[commands]]` carry no signature | 7/10 |
| D11 | Effects are commands only: no timer, no event, no entity creation from a program | `05.5` §2.1 lists `schedule_outcome` and `emit_event`; neither is needed before SPR-024, and each adds host state the checker cannot see; v0.2 if a consumer needs them | 7/10 |
| D12 | Bounds: `kin` degree ≤ 4, paths and generations ≤ 6, nesting ≤ 2, bound product ≤ 65,536; relation bounds as in §5.3 | Dormancy's `dormancy_degrees` is bounded `[1, 6]` in the Registry, but every law here needs at most 4; 65,536 keeps a program under a millisecond at `08`'s budgets | 6/10 |
| D13 | Genealogy is read from the life-course log (`life_course.person_id`, `.mother`, `.father`, `.birth_tick`, `.sex`), `same_instant` (the log is append-only), never from the ledger's `person.mother`/`person.father`; the review's `kin(a, b, n)` is `related(a, b, n)` here because `kin(p, n)` is already the relation `05.5` §2.1 names | The log covers the dead (REV-P1a NEW-3); the ledger compacts them, which would cut every line through a dead intermediary. One name for a relation and a predicate would make the grammar ambiguous | 8/10 |

**Open questions** (the Registry or a spec is silent)

| # | Question | Recommendation | Confidence |
|---|---|---|---|
| Q1 | Should `person.vocation == holy_orders` be in the laws' `eligible`, or stay M7's exclusion (the spec says M7 excludes "the deposed ruler and persons in holy orders")? | Keep it M7's: the law states the lineage rule; the court's exclusions change with institutions, not with the law. The golden file shows the alternative as `primogeniture_lay` | 7/10 |
| Q2 | Does `kin(holder, 4)` reach far enough for a realm whose houses are three generations deep (`PRJ-002` SPR-019)? | Yes for F1: cousins' lines are degree 4; second cousins are F2 ("cadet branches"). Raise `KIN_DEGREES_MAX` to 6 only if SPR-030's dynastic fixture shows empty lines | 6/10 |
| Q3 | `partition`: what do the other eligible children receive when M7 F1 cannot create titles? | The first of the line takes the title; each other eligible child gets a claim (`CLAIM_CREATED`), so partition reads as a contested inheritance until F2 divides seats | 6/10 |
| Q4 | `elective` and `tanistry` are orders, not votes. Is a vote needed in P1a? | No: M7's arbitration already weighs claims and offices; a vote with opinions is M6-lite or P1b. The orders here are the deterministic fallback M7's spec names for a sovereign with no candidate | 7/10 |
| Q5 | `sortition` needs an RNG purpose that M7's spec does not declare | Add `[[rng]] purpose = "sortition", key = ["title", "tick", "person"]` to M7's spec in PR #32's next round | 9/10 |
| Q6 | A verb effect of T1 on T1's own columns (`DESPAIR_PACT`'s opinion modifiers) is a self-sent command; T1's spec lists `OpinionModifier` only under `applies` | T1 declares the self-send in `[[commands]]` (`target = "T1.minds"`); `rh-core` already allows an owner to send to itself. The alternative, applying inline inside `break_act`, is what `05.5` §2.1 forbids | 7/10 |
| Q7 | Vow predicates need reads S7 does not declare (`person.death_cause`, `office.*`, `person.residence`), and `PROTECT` has no completion | S7's P1b sprint adds the reads (`death_cause` same-instant through the death path, offices lagged); `PROTECT` ends only by breaking (the beneficiary harmed by the vower, S7 `strain`) or voiding; `PILGRIMAGE` waits for S3 | 6/10 |
| Q8 | `emitted_by = "rh-minds"` on the older verbs (`ARREST`, `PURGE`, `REVENGE`, …) is a crate name, not a system id | Rename to `T1.minds` in the Registry with SPR-017's loader change; the checker needs a system id to find the spec | 9/10 |
| Q9 | Who evaluates: the host system at its binding, or `rh-core`? | The host, through an `rh-registry` API (`compile(program, env) -> Compiled`, `Compiled::eval(&SystemView, &Bindings) -> Value`): the reads then pass through the same `SystemView` that enforces `AccessSpec` at run time, so the compile-time check and the run-time guard agree | 8/10 |
| Q10 | Should overlays (`derive`) be in v0 at all? | Reserve the kind and its signature only; its accessors (values, fear, success streaks) do not exist before P3.2b | 8/10 |
| Q11 | M7's and T1's specs declare `person.mother`/`person.father` reads, not the life-course columns the genealogy relations and the laws read | In PR #32's next round, M7 and T1 add `[[reads]]` of `life_course.person_id`, `life_course.mother`, `life_course.father`, `life_course.birth_tick` and `life_course.sex` (`same_instant`, `identity`); the `person.mother`/`person.father` reads can go. `life_course.sex` is REV-P1a NEW-7's column, due in M2's skeleton spec | 8/10 |

## 13. Conformance

- The golden file is the contract's test: every grammar rule, every row of §4.3, every relation and accessor of §5.3, every aggregate, every program kind, the seven laws and the three extensions, and every error class, each with an expected value or code.
- `python -I specs/registry/effect_ref.py --check` exits 0 iff every golden case reproduces; `bin/check_all.sh` runs it, with a positive control (an altered expected value must fail).
- SPR-017's Rust evaluator (`rh-registry`, V-WC-10) reads the same file and must reproduce every case bit for bit, in debug and release builds, as `rh-sample`'s golden vectors do at both optimisation levels.
- The Python reference is standard-library only, exact (`fractions.Fraction`), and shares no code with the engine.
