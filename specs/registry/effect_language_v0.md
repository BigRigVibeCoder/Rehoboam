# Registry effect language v0

*Status: Draft for the director's approval at the SPR-017 plan gate (C2). Version `0.5.0` of the language (round 5 of PR #41, the closing nits r4-1 to r4-5, on top of rounds 2–4). Checked against the frozen system specs on `main`: `M7.dynasty` 0.3.2, `T1.minds` 0.3.1, `M2.demography` 0.2.0. Contract for WP P1.15 (`docs/11`), SPR-017 (`CODEX/05_PROJECT/PRJ-002_P1aPlan.md`) and its dependants SPR-021 (M7 succession) and SPR-024 (minds' verbs). Sources: `05.5` §2.1 and §3.2, `05.1` §2, §4.3 and §6, ADR-0006, ADR-0010 (amendment v1.1 item 4), ADR-0024, ADR-0025, ADR-0033, ADR-0034, ADR-0035, `docs/01` §5 principles 8 and 9, `crates/rh-core/src/fixed/`. Golden cases: `specs/registry/effect_language_golden_v0.toml`. Reference implementation: `specs/registry/effect_ref.py` (standard-library Python; `--check` reproduces every golden case).*

> **BLUF.** The effect language is the small, typed, total expression language in which the Registry states succession laws (who may inherit a title and in what order), verb effects (which lifecycle commands an act queues), and the vows' completion and conflict predicates. A program reads only the columns its declaring system has declared, iterates only over Registry relations with declared bounds, draws randomness only from the keyed RNG with a declared purpose, and changes the world only by queuing typed commands to the column's owner. Every well-typed program terminates and yields a value: there is no recursion, every loop is bounded, and division by zero, overflow and reads of missing entities have defined results (the saturating, round-half-to-even rules of `rh-core::fixed`). Floats do not exist in the language. This document is the grammar, the type system, the evaluation rules, the v0 prelude of relations and accessors, the seven succession laws of the Registry as programs, the error classes a program can be rejected with, and the versioning rule. The Rust evaluator of SPR-017 (`rh-registry`) must reproduce every golden case bit for bit; the Python reference is the independent oracle, as `bin/gen_fixed_vectors.py` is for `rh-core::fixed`.

## 0. Scope and non-goals

**In v0:**
- Expressions over fixed-point numbers, integers, booleans, Registry enums and entity references (§3).
- Bounded aggregates (`count`, `any`, `all`, `sum`, `min`, `max`) over declared relations, nested at most two deep (§5.4); the lineage relation that gives a succession its candidates and the tenure-log relation that gives a title its blood (§5.3).
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
| Keyed RNG only, with a declared purpose | `05.1` §6 item 2; ADR-0006 | `draw` and `chance` take a purpose the declaring system lists in its `[[rng]]`, an entity and a draw index; every call site is unique per system, a draw inside a loop is keyed on the loop variable, and a key drawn twice in one step is an invariant failure; no other randomness exists (§4.6) |
| Single writer; effects are queued commands to the owner, never inline writes | `05.1` §2 rules 1–2, 5; `05.5` §2.1 | An effect program can only emit commands the declaring system's spec lists in `[[commands]]`, to the command's owner (§6); there is no assignment |
| Plane A never reads plane B or C | ADR-0024 | A read of a column whose plane is not `A` is rejected (`E_PLANE`); a read the declaring system does not declare in `[[reads]]` or own is rejected (`E_UNDECLARED_READ`); a declared read of another system's column without a neutral value is rejected (`E_NO_NEUTRAL`, §4.5) |
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
enum_lit    = Ident "." ( Ident | keyword ) ;       (* enum_type.value; never followed by "("; a value may be a keyword: death_cause.none, vocations.none *)
keyword     = "and" | "or" | "not" | "if" | "then" | "else" | "when" | "for" | "in" | "where" | "true" | "false" | "none" | "div" | "mod" ;
call        = Ident "(" [ expr { "," expr } ] ")" ; (* a builtin, an accessor, params.x and calendar.x
                                                       are written as Ident "." Ident without "(" (see §4.7) *)
aggregate   = agg "(" Ident "in" relation [ "where" expr ] [ ":" expr ] [ "else" expr ] ")" ;
agg         = "count" | "any" | "all" | "sum" | "min" | "max" ;
relation    = Ident "(" [ expr { "," expr } ] ")" ;  (* a prelude relation, §5.3 *)
tuple       = "(" expr "," expr { "," expr } ")" ;   (* two or more elements; key type only *)
list_key    = "[" expr "for" Ident "in" relation "]" ;  (* key type only *)
```

Disambiguation rules:
- `Ident "." Ident "("` is a column read; `Ident "." Ident` without `(` is an enum literal, a parameter (`params.<name>`) or a calendar constant (`calendar.<name>`), decided by the first identifier. After the dot a keyword is an ordinary name (`vocations.none` is the enum value `none`; `sex.none` is `E_UNKNOWN_NAME` because `sex` has no such value); the parameter and calendar names are never keywords.
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
| `event` | opaque | the verb's own event, or `none` | `none` | `DraftRef`, rewritten to an `EventId` at the merge |

**`event`** is opaque (N3): the variable `event` of an effect program (§7.1) and the literal `none` are its only values, and an event-typed command parameter (`cause_event`, `source_event`) is its only use. It has no arithmetic, no comparison, no `exists`, is not a draw key and not a key element (`E_TYPE` for each). The host submits an event-typed argument as its `DraftRef` and the `rh-core` merge rewrites it, as it does `Command::cause`; the golden file renders it as the text `event` (`none` as `0`) because its numeric value does not exist before the merge.

`05.5` §2.1 writes the fixed types `q8.8` and `q16.16`; they are the same types, spelled here as the column types of the system specs (`q8_8`, `q16_16`). There is no money type in v0 (`Money` arrives with M4 F0, P1b) and no float type at all.

**Enum types** are the Registry's id lists (`sex`, `death_cause`, `vocations`, `title_tiers`, `succession_laws`, `vow_kinds`, `opinion_modifiers`, …) and the per-column enums the system specs declare (`title.state`, `claim.status`, `artifact.state`, …). Every enum declares a **default** value, the one a missing read yields (§4.5); it is named, never "the first value", so reordering a list cannot change what a missing read returns. The environment the checker sees (§7.4) names each enum column's type and default. Enums have no order in the language: `ordinal(e)` gives the declaration index as an `int` when an order is wanted, which makes the dependency on list order explicit. An enum crosses into a command as its ordinal (§6), so reordering an id list that any program passes to a command or takes the `ordinal` of is a C3 change (§11).

**Entity kinds** are the `entity` values of `[[owns]]` across the system specs: `person`, `life_course`, `tenure`, `title`, `office`, `claim`, `house`, `polity`, `artifact`, `vow`, `region`, … The value `none` belongs to every kind and is the id 0, the convention of every `… or 0` column of the specs (`person.father`, `title.holder`). `exists(e)` is true iff `e` is not `none` and a row with that id exists in the snapshot.

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
| `a == b`, `a != b` | same scalar type | equality; fixed point compares bits; entities compare ids (a `none` literal needs a kind from the other side) |
| `a < b`, `<=`, `>`, `>=` | `num × num → bool` | numeric order (fixed point: order of bits) |
| `not a`, `a and b`, `a or b` | `bool` | logic; `and` and `or` **evaluate both sides** (the right side's reads count, §4.5) |
| `if c then a else b` | `bool × T × T → T` | conditional; **only the taken branch is evaluated**; `T` any type, keys included (structurally equal, §3.4) |
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
| `draw(purpose, e, k)` | `purpose × entity × (int \| entity) → int` | keyed uniform draw, §4.6 |
| `chance(purpose, e, k, p)` | `… × q16_16 → bool` | `draw_hits` of `rh-core::fixed::draw`, §4.6 |
| `up(x, p)`, `common(x, p)`, `up_agnatic(x, p)`, `common_agnatic(x, p)` | `person × person → int`, `→ person` | genealogical accessors, §5.3 |
| `related(a, b, n)` | `person × person × degree → bool` | `a` and `b` are distinct persons within `n` degrees of kinship (`n` a degree constant in 1..6, §4.4): the predicate form of `kin`, §5.3 |
| `descends(x, a)` | `person × person → bool` | `x` is `a` or a descendant of `a` within 6 generations; false when either is `none` (the title-blood rule of §8) |
| `entity.column(e)` | entity `K` → the column's type | a declared read, §4.5 |
| `params.name` | → `int` or `q16_16` | a Registry parameter of the declaring system, §4.7 |
| `calendar.days_per_year`, `calendar.minutes_per_day` | → `int` | the manifest's year and the engine's day (`rh_core::time::MINUTES_PER_DAY`), §4.7; ticks are `SimTime` minutes |
| aggregates | §5.4 | |

### 3.4 Keys

The `order` program of a succession law evaluates to a **key**. A key is a scalar, a tuple of two or more keys, or a bounded list of keys built by `[ key_expr for y in relation ]`. Keys are **structurally typed**: a tuple's type is its arity and the type of each element (`tuple(int,list(tuple(bool,int)))`), a list's type is its element type, and the two branches of a conditional, like every key an `order` program can produce, must have the same key type (`(true, 2)` against `(5, 1)` is `E_TYPE`); lists may differ in length. Keys compare by a total order:

- scalars: `int` and fixed point by value; `bool` with `false < true`; an enum by declaration index (so `ordinal` is implied inside a key); an entity by id with `none` first;
- tuples: element by element, lexicographically;
- lists: lexicographically, element by element; when one list is a prefix of the other, the shorter comes first (an ancestor before its line).

Candidates sort by key ascending, then by entity id ascending (§7.2), so two candidates with equal keys (twins on the same line) are ordered by `life_course.person_id`, and a key need not name the candidate. Key values never enter columns: a key exists only between an `order` program and the host's sort.

### 3.5 What has no type

Relations, programs and command names are not values: a relation appears only after `in` or as the `candidates` program; a command only as an action. Keys exist only as the result of an `order` program and inside its tuples and lists. An `event` exists only as an event-typed command argument (§3.1). The checker rejects any other use (`E_TYPE`).

## 4. Semantics

### 4.1 Type resolution

1. Column reads, variables, parameters, accessors and builtins with fixed signatures have concrete types.
2. In a binary operator, a comparison, `min`/`max`/`clamp`/`safe_div`, a conditional and the body-or-else of an aggregate, the operands unify: a concrete type is adopted by a polymorphic literal; two literals default (`Int` to `int`; any `Decimal` to `q16_16`); two different concrete types are `E_TYPE`.
3. A literal passed to a parameter of a known type (a command argument, `q8(n)`, `chance(..., p)`) adopts that type, and so does the whole program's value when its kind declares a result type (§7.1): `conflict = "0.0"` is a `q8_8` program.
4. A literal with no context (a key element, the body of `sum` over nothing concrete) takes its default.
5. A degree argument (`kin`, `related`, `ancestors`, `descendants`, `lineage`, `holders_of`) is an `Int` literal or an `int` parameter whose declared bounds lie inside the relation's range (§4.4); `draw`'s index is an `int` expression or an entity.
6. `none` adopts an entity kind or `event` from its context; an event-typed parameter accepts `event` or `none` and nothing else (`0` is `E_TYPE`).

### 4.2 Evaluation order

Evaluation is innermost first, left to right, in the order of the syntax tree; `and` and `or` evaluate both operands, `if` evaluates its condition and the taken branch only. The order is observable only through the ordered emission of effects (§6) and through the diagnostics of §4.5; `draw` is keyed by its arguments, not by evaluation order, so no construct can change which draw a call returns. Expression nesting is bounded (64 levels, `E_BOUND`), so the evaluator's recursion is too. A relation yields its elements in its canonical order (§5.2); an aggregate folds over them in that order; a `for` effect emits in that order. The host evaluates each program over one snapshot of plane A (the lagged or same-instant view its `[[reads]]` declare; lags are the host's business, not the language's).

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

A **constant** is an `Int` or `Decimal` literal (under unary minus or not), a calendar constant, or `params.<name>`. The divisor of `/`, `div` and `mod` must be a constant **whose bits, in the division's type, cannot be zero**: a literal is compiled to the type first (`q8(1) / 0.001` is `E_DIV`, because `0.001` is 0 `q8_8` bits), and a parameter's bounds are rounded to the type before the test (`[0.000001, 1.0]` reaches 0 `q16_16` bits, so it is `E_DIV`; `[0.3, 1.0]` is fine). Every other division is written `safe_div(a, b, d)` with an explicit default. This is `05.5` §2.1's "no division by a non-constant without a guard" made checkable. A **degree** (`kin`, `related`: 1..6; `ancestors`, `descendants`: 1..6; `lineage`: 1..3; `holders_of`: 1..16) is an `Int` literal or an `int` parameter whose declared bounds lie inside that range (`E_CONST` for anything else, `E_BOUND` outside the range), so M7's `dormancy_degrees` (bounds `[1, 6]`) can be a degree.

### 4.5 Reads

`entity.column(e)` reads column `entity.column` of the entity `e`, which must have the kind the column is keyed by: the table's own entity, except that the life-course log (`life_course.*`) is keyed by `life_course.person_id` and is read with a `person`. The checker accepts the read only if the declaring system owns the column or lists it in `[[reads]]` (`E_UNDECLARED_READ`), and only if the column's plane is `A` (`E_PLANE`). A relation (§5.3) reads the columns of its definition; using a relation requires those reads of the declaring system too. **Missing entity:** a read through `none`, or of an id with no row in the snapshot, yields the **zero of the column's type** (`0`, `0.0`, `false`, the enum's declared default, `none`), and the evaluator counts it: `missing_reads` is the number of `entity.column(e)` evaluations whose `e` was `none` or had no row, over one evaluation of one program, counted at each evaluation (a read inside an aggregate counts once per element). Because `and` and `or` evaluate both sides, `exists(e) and entity.column(e)` still counts a missing read; the idiom that avoids one is the conditional, which evaluates only its taken branch: `if exists(e) then entity.column(e) else d`. The count is not part of the value and does not enter hashes; invariant tests may require it to be zero. **Neutral values and absent owners (NEW-4, minor 5):** every `[[reads]]` entry of a column another system owns carries a machine-readable `neutral` field, a literal of the column's type (`Int`, `Decimal`, `true`/`false`, an enum value, or `0` for a reference), the coupling's value when the owner is absent; the spec lint (SPR-014) checks the field, and the checker rejects a program's read of a foreign column whose entry has none (`E_NO_NEUTRAL`), **whatever the world registers**, so a Registry load never depends on the world. At registration `rh-core` drops a read whose owner the world does not register and reports it, and the view supplies the neutral value, which the evaluator counts as `neutral_reads`; a read whose owner is registered reads the world. The ledger's dead rows are compacted (SPR-011 ruling 5); a read of a compacted person is a missing read, which is why the laws read the life-course log (§5.3).

### 4.6 Randomness

`draw(purpose, e, k)` is the keyed draw `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` of `05.1` §6 item 2, with `system_id` the declaring system's, `t = now` (the instant, in minutes), `entity_id = e` (an entity reference whose kind is the kind of the expression `e`; `none` is id 0; the kind is part of the key through the typed `EntityRef` of `05.1` §4.3) and `k` an `int` expression or an entity, whose id is then `k`. That is how T1's purposes are written: `attack_outcome` keyed `(actor, k = target, t)` is `chance(attack_outcome, actor, target, p)`. Its value is the 64-bit draw shifted right by one bit, an `int` in [0, 2⁶³). `chance(purpose, e, k, p)` is `rh_core::fixed::draw_hits(u, p)` on the unshifted 64-bit draw `u`: true iff `u / 2⁶⁴ < p`, so `p <= 0` never hits and `p >= 1` always hits. `purpose` must be a purpose the declaring system lists in `[[rng]]` (`E_RNG_PURPOSE`). Because the key fixes the value, two calls with the same arguments return the same value; so that no two call sites share a draw by accident (the call-site rule of `05.1` §6 item 3), the rule has two layers (minor 3). **Static (`E_RNG_REUSE`):** every `(purpose, entity expression, k expression)` call site is unique within one program, and across the programs of one system that can run in the same step. What runs in the same step (r4-2): the succession laws of one system are mutually exclusive per title, and re-evaluating a law is idempotent (N6), so a data-only variant may reuse a law's site (`sortition_lots_then_age` in the golden file; minor 5); the verb effect programs of one system are never re-evaluated, each event evaluates its program exactly once, and every verb program of the system may run in one step, so their sites are unique across all of them, and a draw in a verb program must key on an entity of its event (`actor` or `target`, in its entity or index expression; `E_RNG_REUSE` otherwise), so that two events of one verb at one instant draw independently. The site is compared where an expression is compared after constant folding (`1 - 1` is the site `0`; `Int` arithmetic and `calendar.minutes_per_day` fold, parameters do not) and carries its type (`k = holder`, a person, and `k = title`, a title, are different sites even when the ids coincide); and a `draw` inside an aggregate, a list comprehension, a `for` effect or the per-candidate `eligible`/`order` of a law must mention every enclosing loop variable in its entity or index expression, since an iteration that does not key on its variable repeats one draw. SPR-017's loader runs the lint across the programs that share a step. A program of a verb whose event carries a drawn outcome (`[verbs.<verb>] payload`, §7.1) may not call `draw` or `chance` at all (`E_RNG_REUSE`: the host drew the outcome once, and a second draw would re-roll it; minor 2). **Run time (invariant `E_RNG_REUSE`):** for a law the ledger is scoped to **one evaluation of one program for one binding** (N6): within it the evaluator refuses a repeated `(purpose, entity kind and id, k)` as a hard invariant failure (a `SimError`), which covers two variables bound to the same entity (`draw(p, x, 0) + draw(p, holder, 0)` with `x = holder`); a second evaluation of the same program with the same inputs in one step is idempotent by design (the same draws in the same order, pinned by `succession_sortition_evaluated_twice_in_one_step`) and raises nothing. For verb programs the ledger spans all of one system's verb evaluations in one step, since none is re-evaluated: two events of a drawing verb at one instant whose draws share a key are the invariant failure, and keyed by the actor they pass (the golden cases `verb_two_events_same_key_at_one_instant` and `verb_two_events_keyed_by_actor`, run as `events` of one case). The sortition law keys `(candidate, k = the title, t)` (M7 0.3.2, minor 4), so one arbitration draws once per candidate and two titles arbitrated at one instant never share a draw. The host supplies the draws; the golden cases list them explicitly, so the reference and the Rust evaluator see the same numbers without re-implementing ChaCha8.

### 4.7 Names

Every identifier resolves at compile time to exactly one of: a keyword; a bound variable of the program kind (§7.1) or of an enclosing aggregate, `for` or list comprehension; a builtin (§3.3); a prelude relation or accessor (§5.3); the prefix of a column read, an enum literal, `params` or `calendar`. An unresolved name is `E_UNKNOWN_NAME`; a variable that shadows another is `E_SYNTAX`. `params.<name>` is a parameter of the declaring system's Registry entry, typed `int` when both bounds are written as integers and `q16_16` otherwise, with the manifest's value compiled by the rule of `05.5` §3.2. `calendar.days_per_year` is the manifest's calendar (`int`); `calendar.minutes_per_day` is the engine constant `rh_core::time::MINUTES_PER_DAY` (1,440; minor 6), spelled as a calendar name so that programs never carry the number; a tick is a `SimTime` minute, so an age in years is `(now - life_course.birth_tick(x)) div (calendar.days_per_year * calendar.minutes_per_day)` and a threshold in years is converted the other way, in `int`.

### 4.8 Totality

Every well-typed program terminates and yields a value of its declared type. Proof sketch: the syntax has no recursion and no user-defined names, so evaluation is structural over a finite tree; the only iteration is over relations, each of which yields at most its declared bound (§5.2), and nesting is limited to depth 2 with a product of bounds of at most 65,536 (`E_BOUND`); every partial operation has a defined result (§4.3: saturation; §4.4: guarded division; §4.5: missing reads; §5.3: empty paths and absent ancestors; §5.4: `else` for empty `min` and `max`); draws are pure functions of their keys. The checker enforces every precondition at compile time, so the evaluator has two failure paths only, both hard invariant failures (a `SimError` in the host, never a silent repair): a `candidates` relation above its bound after pruning (`E_TRUNCATED`, §5.2) and a draw key repeated within one step (`E_RNG_REUSE`, §4.6). Its result is otherwise the value, plus diagnostics (`missing_reads`, `neutral_reads`, `truncated_relations`, `dropped_actions`).

## 5. Relations and iteration

### 5.1 What a relation is

A relation is a named, bounded, ordered set of entities computed from declared columns (`05.5` §2.1: "only over declared relations with a declared upper bound"). Relations are code, not data, and the code lives in **`rh-registry`**, the evaluator's crate: a contract crate holds types and accessors only (`05.1` §2 rule 1; REV-P1a NEW-3), so it can hold neither a genealogy walk nor the children-by-parent index that `children` needs. The evaluator reads every column through the host's `SystemView`, which enforces the declaring system's `AccessSpec` at run time, and takes the derived children index from the host. The prelude (§5.3) is the v0 set, each with its owner (the system whose columns it reads), its arguments, the columns it reads, its bound and its order. Adding a relation is a `needs_code` gap in `rh-registry`, never an effect-language change.

### 5.2 Canonical order and bounds

A relation yields its elements in **ascending entity id** (`none` never appears), except the path relations `line` and `line_agnatic`, which yield in path order, `ancestors` and `descendants`, which yield by generation then id, and `lineage`, which yields in `(up, line)` order (§5.3). A relation yields at most its bound; when the underlying set is larger, the elements beyond the bound in canonical order are dropped and the evaluator counts a `truncated_relations` diagnostic (an invariant test may require zero). **`lineage` is the exception:** it is the candidates relation of every lineage law, and an heir dropped by truncation would be a silent wrong succession, so exceeding its bound is a hard invariant failure (`E_TRUNCATED`; a `SimError` in the host), never a diagnostic. So that the bound never aborts an ordinary dynasty (N2), `lineage` **prunes before it bounds**: a dead member with no living descendant among the members is dropped (it is never eligible, and no key of another member depends on it; `line` walks the log, not the membership), and the bound of 4,096 applies to what remains (sized in D12). The golden file pins the bound on generated fan trees (`[worlds.<name>.fan]`, the regular tree its header defines, which the Rust test builds from the same table): 583 members at depth 3, a hard failure at depth 4 (4,679 living members), and 583 again at depth 4 when the 4,096 leaves are dead and childless.

### 5.3 The v0 prelude

**Genealogy comes from the life-course log** (M2: `life_course.person_id`, `life_course.mother`, `life_course.father`, `u64` with 0 for unknown; `life_course.birth_tick`, `i64`; `life_course.sex`, the enum of `person.sex` at entry; REV-P1a NEW-3 and NEW-7), not from the live ledger: the ledger's dead rows are compacted at Bookkeeping of the death instant (SPR-011 ruling 5), while the log keeps every person who ever lived, so a line through a dead intermediary (a grandson whose father predeceased the holder) stays complete. The log is append-only, so its users declare these reads `same_instant`; its rows are keyed by `life_course.person_id` (a read `life_course.x(p)` takes the person, §4.5), never by a row id. The genealogy relations read `person_id`, `mother` and `father` (`lineage` also `birth_tick` and the ledger's `person.alive`, for pruning); the law programs read `birth_tick` and `sex` of the persons on a line. Siblings are ordered by `life_course.birth_tick`, ties (twins) by `life_course.person_id`, the host's final tie-break (§7.2). "Ancestor-or-self of `x` at height `h`" is the set `A_h(x)`: `A_0 = {x}`, `A_{h+1} = {father(y), mother(y) : y ∈ A_h} \ {none}`; the agnatic variant follows fathers only. Heights are bounded by `KIN_DEGREES_MAX = 6` for `kin`, `related`, `up` and `common` (M7's `dormancy_degrees` and the Registry's `kin(6)` need it), generations by `DESCENT_MAX = 6` for `ancestors`, `descendants` and the lines, `lineage`'s height by 3 and `holders_of`'s window by `TENURES_MAX = 16`. Children are ordered by `life_course.person_id`, so every order below is deterministic without hashing.

**Tenure comes from M7's tenure log** (B3): `tenure.title`, `tenure.holder` (`u64`), `tenure.start`, `tenure.end` (`i64` `SimTime`, `end = i64::MAX` while held), one row per tenure, append-only (a row at each change of holder, `end` written once, the initial holder's row written at world creation), in tenure order. `holders_of(t, n)` reads it so that a title's blood is the blood of its recent holders, not of a fixed founder, which six generations of descent could not reach (§8). **Precondition (N5):** the blood rule assumes the tenure log reaches back at least to the holders whose blood defines the line. A log of one row (the initial holder's, which is all world creation writes today) makes a childless holder's death an empty line under every blood law, and leaves a brother or an uncle out even when a son is in (the golden case `one_row_log_childless_holder_gives_an_empty_line` pins the failure mode). Meeting it is a worldgen requirement, not a language one: for a dynastic world `rh-worldgen` back-fills pre-epoch history, life-course rows with parent links for the dynasty's ancestors back to the manifest's dynasty `generations` and tenure rows for the title's past holders (recorded on SPR-019, population synthesis, in PRJ-002).

| Name | Owner | Yields | Reads | Bound | Definition |
|---|---|---|---|---|---|
| `children(p)` | M2 | `person` | the three genealogy columns | 64 | every person whose mother or father is `p`, alive or dead; id order |
| `kin(p, n)` | M2 | `person` | genealogy | 256 | every person other than `p` reachable from `p` in at most `n` parent-or-child steps (`n` a degree in 1..6); alive or dead; id order |
| `related(a, b, n)` | M2 | `bool` | genealogy | | `b` is in `kin(a, n)` (false when either is `none`); the predicate form the review calls `kin(a, b, n)` |
| `ancestors(p, g)` | M2 | `person` | genealogy | 126 | the ancestors of `p` within `g` generations (`g` a degree in 1..6), by generation (parents first), then id; 126 is 2 + 4 + … + 64 |
| `descendants(p, g)` | M2 | `person` | genealogy | 256 | the descendants of `p` within `g` generations, alive or dead, by generation (children first), then id |
| `lineage(p, h)` | M2 | `person` | genealogy, `birth_tick`, `person.alive` | 4096 | every descendant within 6 generations of every ancestor-or-self of `p` at height 0..`h` (`h` a degree in 1..3), `p` itself excluded, each once, **minus the dead members with no living descendant among the members** (§5.2); ordered by `up(x, p)` ascending, then by the `(birth_tick, person_id)` sequence of `line(common(x, p), x)`, lexicographically (an ancestor of `p` in the set keys `(up, [])`, before its own line); the candidates relation of the lineage laws (§7.2, §8); exceeding the bound after pruning is `E_TRUNCATED` (§5.2) |
| `descends(x, a)` | M2 | `bool` | genealogy | | `a` is an ancestor-or-self of `x` within 6 generations; false when either is `none` |
| `line(a, x)` | M2 | `person` | genealogy | 6 | the persons on the shortest parent-chain from `x` up to `a`, listed from `a`'s child down to `x` (`a` excluded, `x` included); among equally short chains, the one that takes the father at the first step where the chains differ; `line(x, x)` and a pair with no chain of length ≤ 6 yield the empty list |
| `line_agnatic(a, x)` | M2 | `person` | `person_id`, `father` | 6 | as `line`, following fathers only |
| `up(x, p)` | M2 | `int` | genealogy | | the least `h` in 0..6 such that some member of `A_h(p)` is an ancestor-or-self of `x` within 6 generations; `7` if there is none |
| `common(x, p)` | M2 | `person` | genealogy | | the member of `A_h(p)` found by `up`; when several qualify at that height, the first in father-first enumeration order (`L_0 = [p]`, `L_{h+1}` = for each `y` in `L_h`: `father(y)`, then `mother(y)`, each once, `none` skipped); `none` when `up` is 7 |
| `up_agnatic(x, p)`, `common_agnatic(x, p)` | M2 | `int`, `person` | `person_id`, `father` | | as `up` and `common` through fathers only, on both sides |
| `titles(p)` | M7 | `title` | `title.holder` | 16 | titles whose holder is `p`; id order |
| `vassals(t)` | M7 | `title` | `title.liege` | 64 | titles whose liege is `t`; id order |
| `offices(p)` | M7 | `office` | `office.holder` | 16 | offices whose holder is `p`; id order |
| `court(t)` | M7 | `person` | `title.liege`, `title.holder`, `office.title`, `office.holder` | 128 | the holders of `t`'s vassal titles and of `t`'s offices, `none` excluded, each once; id order |
| `claims_on(t)` | M7 | `claim` | `claim.title` | 64 | claims on `t`; id order |
| `claims_of(p)` | M7 | `claim` | `claim.claimant` | 16 | claims of `p`; id order |
| `vows_of(p)` | S7 | `vow` | `vow.vower` | 8 | vows of `p`, every status; id order |
| `holders_of(t, n)` | M7 | `person` | `tenure.title`, `tenure.holder` | 16 | the holders of `t`'s last `n` tenure rows (`n` a degree in 1..16), most recent first, each person once at its most recent tenure, `none` skipped; the current holder first while the title is held; empty for `none` or a title never held |

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

- An action `Cmd(person: e, a: v, …)` names a command of the Registry `[commands]` table (§7.4) and gives every parameter once by name (`E_ARITY` otherwise). The `person` argument is the queue key (`rh_core::command::Command::person`); the remaining arguments are the command's parameters in the declared order. The signatures are the owner's `[[applies]]` payloads, lint-checked both ways (§7.3): `ScheduleDeath {victim, tick, cause, culprit, cause_event}` and `Marry {a, b, tick}` from M2's spec (0.2.0, frozen); `OpinionModifier {holder, about, kind, magnitude, source_event}`, `ApplyThought {holder, overlay, delta, source_event}`, `Promote {person}`, `Demote {person}` and `BurnHoldings {actor, household, cause_event}` from T1's (0.3.1, frozen). A payload field named `cause_event` or `source_event` is **event-typed** (§3.1): it takes the program's `event` (this verb's own event; the host passes its `DraftRef` and the merge rewrites the field, as it does `Command::cause`, an engine requirement of the P1a plan) or `none` (no inciting event, as M7's hazard murder sends), and nothing else. A tick-valued argument is a `SimTime` minute, computed in `int` from `now` and the calendar.
- The checker accepts an action only if the declaring system's spec lists the command in `[[commands]]` with `target` equal to the command's owner (`E_UNDECLARED_COMMAND` when not listed; `E_COMMAND_OWNER` when the listed target is not the owner, which is the rejected "write to a non-owned column"). An action whose owner is not registered in the world **compiles** (NEW-4): at registration `rh-core` drops the send and reports it, and the evaluator emits nothing for it and counts `dropped_actions`. A command to the declaring system's own columns is still a queued command, to itself, and must be listed the same way.
- `when c: A` emits `A` iff `c`; `for y in R where c: A` emits `A` once per element of `R` passing `c`, in canonical order.
- **Encoding into `Command::args`** (`Vec<i64>`): `int` as itself; `q8_8` and `q16_16` as their raw bits; `bool` as 0 or 1; an enum as its ordinal; an entity as its id reinterpreted as two's-complement `i64` (`none` is 0); an event-typed field as the `EventId` the merge resolves (`none` as 0). The golden file renders a command as `Name(person=<id>, args=[<i64>, …])`, the exact queue form, with the text `event` in an event-typed field because its id does not exist before the merge.

Neither `draw` nor a read is an effect: a program is pure except for the commands it lists. It cannot emit an event (the verb's event is the host's), schedule a timer, create an entity or touch a plane-B or plane-C value.

## 7. Program kinds, environments and the Registry schema

### 7.1 Kinds and their signatures

| Kind | Registry location | Bound variables | Result type |
|---|---|---|---|
| `candidates` | `[succession_laws.<law>]` | `holder: person` (the previous holder, dead or deposed, readable in the instant per SPR-011 ruling 5), `title: title`, `now: int` | relation of `person` |
| `eligible` | same | as above plus `x: person` (the candidate) | `bool` |
| `order` | same | as `eligible` | `key` |
| `effects` | `[verbs.<verb>]` | `actor: person`, `target: person` (`none` if the verb has no target), `instrument: artifact` (`none` if none), `event: event` (the verb's own event, §3.1), `now: int`, plus the verb's declared **payload fields** (`[verbs.<verb>] payload`, e.g. `success: bool` for `MURDER_ATTEMPT`: the outcome the host drew and wrote into the event, N4) | effects |
| `fulfilled` | `[vow_kinds.<kind>]` | `vower: person`, `target` of the kind's `target` entity kind, `now: int` | `bool` |
| `conflict` | same | as `fulfilled` | `q8_8`, expected in [0, 1] (S7 clamps) |
| `derive` | `[overlays.<overlay>]` | `self: person`, `now: int` | `q8_8` (reserved: the psychological core it reads is P3.2b; no v0 Registry entry declares one) |

`now` is the instant's tick (`int`, `SimTime` minutes from the epoch, `05.1` §4.1; a day is `calendar.minutes_per_day`). The declaring system is: the owner of the law's consumer, `M7.dynasty`, for succession laws; the verb's `emitted_by` for effects; `S7.vows` for vow predicates; `T1.minds` for overlays.

### 7.2 The succession protocol

For a title `t` whose holder `h` died or was deposed at `now`, under law `L` (`title.law(t)`), the host computes the law's line:

1. `C` = the elements of `candidates` evaluated with `holder = h`, `title = t`: for a lineage law, `lineage(holder, 3)`, which reaches every line of the holder's ancestors up to great-grandparents, pruned of the dead without a living line, and is never truncated silently (§5.2).
2. `E` = `[x in C where eligible(x)]`, in canonical order.
3. The **line** is `E` sorted by `(order(x), id(x))` ascending, `life_course.person_id` as the final tie-break (§3.4), which is what orders twins.

The first of the line is the legal heir; M7's arbitration (`M7.dynasty` submodel `arbitrate`) applies its own exclusions (the deposed ruler, persons in holy orders) and scoring after this step. The regent rule ("the nearest adult kin in the law's order") reads the same line. The law never reads claims, offices or opinions: those are M7's arbitration terms, not the law.

### 7.3 Registry schema (the C2 change SPR-017 lands with the loader)

The programs live next to the ids they belong to. `ids` stays the enumeration manifests reference; the per-id tables carry the programs; a lint (SPR-017) requires every id with a program table to be in `ids`, and every law in `ids` to have the three programs from the first world that registers `M7.dynasty`.

```toml
[effect_language]
version = "0.5.0"                 # this document's version; the Registry's registry_version covers it

[succession_laws]
ids = ["primogeniture", "ultimogeniture", "seniority", "partition", "elective", "tanistry", "sortition"]

[succession_laws.primogeniture]   # §8
candidates = "lineage(holder, 3)"
eligible   = "…"
order      = "…"

[verbs]
MURDER_ATTEMPT = { act_classes = ["harm"], emitted_by = "T1.minds", payload = { success = "bool" }, effects = "when success: ScheduleDeath(person: target, tick: now + calendar.minutes_per_day, cause: death_cause.murder, culprit: actor, cause_event: event)" }

[vow_kinds]
CLAIM_TITLE = { kind_weight = 0.8, target = "title", fulfilled = "title.holder(target) == vower", conflict = "0.0" }

[commands]                        # the signatures the checker and the queues share (new section; = the owners' [[applies]])
ScheduleDeath = { owner = "M2.demography", person = "victim", params = [{ name = "tick", type = "int" }, { name = "cause", type = "enum:death_cause" }, { name = "culprit", type = "person" }, { name = "cause_event", type = "event" }] }
```

And in a system spec, the `neutral` field of a read (minor 5), which the lint of SPR-014 requires on every read of a foreign column:

```toml
[[reads]]
column = "polity.exec_constraint"
neutral = "0.0"                   # a literal of the column's type: Int, Decimal, true/false, an enum value, or 0 for a reference
```

The `[commands]` section is new: today a command's owner is declared by the owner's `AccessSpec::applies` and named by each sender's `[[commands]] target` (SPR-006), and the owner's spec documents the payload in `[[applies]]` (M2: `ScheduleDeath { victim, tick, cause, culprit, cause_event }`, `Marry { a, b, tick }`). The checker needs the parameter list as data, so the Registry gains it; SPR-017's lint checks `[commands]` against the owners' `[[applies]]` payloads and the senders' `[[commands]]` tables in both directions, and types every `cause_event` and `source_event` field `event`. A verb's `payload` table names the event payload fields the host binds for its effects (§7.1). The lineage laws read M7's tenure log (`tenure.*`, four new columns, §5.3). The Rust loader's `IdList` form of `[succession_laws]` and the `Verb` type gain the optional program fields; the Python reference validator reads only `ids`, so no manifest changes.

### 7.4 The environment the checker sees

A program is checked against: the Registry enums with their defaults and the `[commands]` signatures; every column of the system specs with its entity, key, type, enum type and plane (`[[owns]]`); the declaring system's `[[reads]]`, `[[commands]]`, `[[rng]]`, the neutral values of its couplings and its Registry parameters with their bounds; the set of registered systems; the manifest's calendar. The golden file carries one such environment in full (`[registry]`), so the checker's decisions are reproducible without the engine.

## 8. The succession laws of the Registry, as programs

The seven laws are the ids of `[succession_laws]`; M7's spec names all seven (`exit_hazards`: the law multiplier covers every id; `arbitrate`: "the legal heir by the title's law"). Each program below is checked and evaluated in the golden file on a fixture dynasty (`world.dynasty`) and its expected line is pinned. Every lineage law takes its candidates from `lineage(holder, 3)` (§5.3: the lines of the holder's ancestors up to great-grandparents, pruned of the dead without a living line, never silently truncated, B2, N2), keeps the holder's own ancestors out with `not descends(holder, x)` (D5, N1: the lineage holds them at heights 1–2, and without the guard a living father keys `(1, [])` ahead of his grandson and a living grandfather heads `seniority`; the `common(x, holder) != x` form of v0.3.0 leaked under uncle–niece marriage, which T1 allows: a grandfather who is also an uncle has a father-first `common` above himself, round 4 minor 1, pinned on `world.niece`), orders siblings by `life_course.birth_tick` with ties broken by `life_course.person_id` (a key never names the candidate; the host's tie-break orders twins), and inherits **by representation**: a dead elder child's line comes before a living younger child (the golden cases `succession_primogeniture` and `succession_representation_grandson_before_living_younger_sons`). **Title blood (D14, for the director):** a candidate is of the title's blood iff it descends from the holder, or from one of the title's last sixteen holders, the relation's maximum: `descends(x, holder) or any(h in holders_of(title, 16): descends(x, h))` over M7's tenure log (B3, N5, §5.3; a seniority rota of nine tenures in two generations put a first cousin's only holder ancestor, his grandfather, outside an eight-tenure window, pinned on `world.rota`). The rule depends on no fixed founder: `descends` reaches six generations, so a founder rule (`descends(x, title.founder)`, v0.2.0) failed seven generations below the founder, inside M7's thousand-year horizon, where it excluded the holder's own sons and let `primogeniture` pick the dead holder's mother; each holder in the window renews the anchor, so a dynasty that keeps the seat keeps its blood indefinitely, while a collateral of no recent holder's blood (a maternal half-sibling: Yrsa in the fixture, Yngve in the deep world) is excluded. The law's order applies within that set. The rule is the default of every lineage law here; the unrestricted form is the Registry variant `primogeniture_open` (both are pinned on the deep world, `world.deep`, nine holders in a line), so the director's choice is a one-line Registry edit either way. The sex rule of each law is stated in its comment and is a design decision (D4, §12): the Registry has one `primogeniture`, so it is male-preference cognatic; `seniority` and `tanistry` are agnatic, as their sources are; the golden file also carries `absolute_primogeniture` and `agnatic_primogeniture` as data-only extensions, to show that a variant is a Registry edit. Under male-preference cognatic order a dead daughter's son comes after every son's line and before a living younger daughter's line: the daughter's own key (female, her birth tick) places her line, and she is represented by it.

```toml
[succession_laws.primogeniture]
# Sex rule: male-preference cognatic (sons by age, then daughters by age, each represented by their line).
# Primogeniture by representation, lineal. Descendants first (sons by age, each son's line before the next
# son, then daughters likewise), then the lines of the nearest common ancestor (siblings and their lines, then
# uncles and aunts and theirs). The holder's ancestors are in the lineage (heights 1-2) and never inherit (D5).
# Title blood: the holder's descendants, and collaterals of the last sixteen holders' blood (B3, N5, D14).
candidates = "lineage(holder, 3)"
eligible   = "person.alive(x) and x != holder and not descends(holder, x) and (descends(x, holder) or any(h in holders_of(title, 16): descends(x, h)))"
order      = "(up(x, holder), [(life_course.sex(y) == sex.female, life_course.birth_tick(y)) for y in line(common(x, holder), x)])"

[succession_laws.primogeniture_open]   # the variant without the title-blood rule, for the director's choice (D14)
candidates = "lineage(holder, 3)"
eligible   = "person.alive(x) and x != holder and not descends(holder, x)"
order      = "(up(x, holder), [(life_course.sex(y) == sex.female, life_course.birth_tick(y)) for y in line(common(x, holder), x)])"

[succession_laws.ultimogeniture]
# Sex rule: male-preference cognatic. As primogeniture, youngest first within each set of siblings.
candidates = "lineage(holder, 3)"
eligible   = "person.alive(x) and x != holder and not descends(holder, x) and (descends(x, holder) or any(h in holders_of(title, 16): descends(x, h)))"
order      = "(up(x, holder), [(life_course.sex(y) == sex.female, -life_course.birth_tick(y)) for y in line(common(x, holder), x)])"

[succession_laws.seniority]
# Sex rule: agnatic (males through fathers only; a person in a father column is male by construction).
# Agnatic seniority (the rota): the eldest male of the most senior generation of the male line, generations
# counted relative to the holder (uncles before brothers before sons), then by age.
candidates = "lineage(holder, 3)"
eligible   = "person.alive(x) and x != holder and not descends(holder, x) and life_course.sex(x) == sex.male and up_agnatic(x, holder) <= 3 and (descends(x, holder) or any(h in holders_of(title, 16): descends(x, h)))"
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
# the eldest first (the election of the tánaiste is Q4). The age test is in int minutes (B1).
candidates = "lineage(holder, 3)"
eligible   = "person.alive(x) and x != holder and not descends(holder, x) and life_course.sex(x) == sex.male and up_agnatic(x, holder) <= 3 and count(y in line_agnatic(common_agnatic(x, holder), x)) <= 3 and (descends(x, holder) or any(h in holders_of(title, 16): descends(x, h))) and now - life_course.birth_tick(x) >= floor(params.majority_age * q16(calendar.days_per_year)) * calendar.minutes_per_day"
order      = "life_course.birth_tick(x)"

[succession_laws.sortition]
# Sex rule: none. Lots among the court. 05.5 §2.1's rng_permutation(purpose = "sortition") is exactly this order plus the
# host's tie-break of §7.2: candidates in ascending draw keyed (person = the candidate, k = the title, t = the
# arbitration instant), ties in ascending PersonId, which is how M7's spec states it (0.3.2; the title key
# by minor 4). The language needs no permutation built-in: a per-candidate draw as the sort key is the
# permutation, and the title in the key keeps two arbitrations at one instant apart (Q5, closed).
candidates = "court(title)"
eligible   = "person.alive(x) and x != holder"
order      = "draw(sortition, x, title)"
```

Reads these programs need from `M7.dynasty`: `person.alive` (M2, declared; `lineage` reads it to prune); `life_course.person_id`, `life_course.mother`, `life_course.father`, `life_course.birth_tick` and `life_course.sex` (M2, `same_instant`, declared since PR #32 round 4c); `tenure.title` and `tenure.holder` (owned, the new tenure log, B3); `title.tier`, `title.holder`, `title.liege`, `office.title`, `office.holder` (owned). `params.majority_age` is M7's. The fixture's expected lines are in the golden file; the two data-only variants `absolute_primogeniture` and `agnatic_primogeniture` and the Q1 variant `primogeniture_lay` carry the same guard and blood rule; `sortition_lots_then_age` shows a variant reusing a law's draw site (§4.6). A dead candidate's `person.alive(x)` is a missing read after its death instant (the row is compacted) and so reads `false`, which is the right answer for eligibility. Every other fact about a person on a line (sex, birth tick) is read from the log, never from the ledger: a dead intermediary's ledger row is gone, and a ledger read of it would silently give the zero (the golden case `sum_int_from_the_ledger_misses_the_compacted_dead` shows the pitfall). The fixture's expected lines are in the golden file (`succession_*` cases).

## 9. Verb effects and vow predicates (the consumers' programs)

Verb effects (SPR-024, `T1.minds`): the four break acts of `[mental_breaks]` and the feud attack are the v0 verbs with effects. The golden file checks these programs against exactly the reads and sends T1 0.3.1 (frozen) declares (`[[commands]]`: `ScheduleDeath` and `Marry` to M2; `OpinionModifier` to itself; `PetitionSeat`, `PressClaim`, `RenounceClaim`, `Resign` to M7; `SwearVow` to S7; `Transfer`, `Search`, `Destroy` to S6; `BurnHoldings` to M4):

```toml
MURDER_ATTEMPT.payload = { success = "bool" }   # the outcome T1 drew (break_outcome for a break's attempt, attack_outcome for a feud attack; MIN-3) and wrote into the event
MURDER_ATTEMPT.effects = "when success: ScheduleDeath(person: target, tick: now + calendar.minutes_per_day, cause: death_cause.murder, culprit: actor, cause_event: event)"   # the program draws nothing (N4); the death at the next day start (MAJ-4)
FLED.effects           = "for o in offices(actor): Resign(person: actor, office: o)"
DESPAIR_PACT.effects   = "for c in claims_of(actor) where claim.status(c) == claim_status.active: RenounceClaim(person: actor, claim: c); OpinionModifier(person: actor, about: target, kind: opinion_modifiers.DESPAIR_PACT_WITH, magnitude: 30.0, source_event: event)"
HOLDINGS_BURNED.effects = "BurnHoldings(person: actor, household: household_of(actor), cause_event: event)"   # P1b: T1 declares the send {actor, household, cause_event}, but v0 has no accessor for a person's household (M4's table): E_UNKNOWN_NAME until M4 adds one (Q14); once it exists, the send is dropped and reported at registration while M4 is absent (NEW-4)
```

`DESPAIR_PACT`'s opinion modifiers both ways are a command from T1 to itself (`OpinionModifier`), which T1 declares in `[[commands]]` (Q6, closed); the magnitude above is illustrative (T1's `[mental_breaks]` entry names the value). T1 declares everything these programs need; `HOLDINGS_BURNED` alone waits for an M4 accessor of a person's household (Q14, P1b).

Vow predicates (S7, P1b): S7's spec says the completion predicate per kind and the conflict signal are effect-language programs (`fulfil`, `feed`). The programs the v0 columns can express:

```toml
AVENGE_KIN.fulfilled    = "(if exists(target) then not person.alive(target) else true) and life_course.death_cause(target) == death_cause.murder"   # the log's cause (M2 0.2.0: one death_cause enum, none..accident), readable after the death instant; no missing read
GRUDGE.fulfilled        = "not person.alive(target)"
RECOVER_RELIC.fulfilled = "artifact.bearer(target) == vower"
DESTROY_RELIC.fulfilled = "artifact.state(target) == artifact_state.destroyed"
CLAIM_TITLE.fulfilled   = "title.holder(target) == vower"
PROTECT.fulfilled       = "false"      # ends by breaking or voiding only; Q7
PILGRIMAGE.fulfilled    = "false"      # needs person.residence (S3, P1b); Q7
AVENGE_KIN.conflict     = "if any(o in offices(vower): any(t in titles(target): office.title(o) == t)) then 1.0 else 0.0"   # serving the target; needs office reads (Q7); the literals are q8_8 by the kind's result type (§4.1)
```

## 10. Error classes (compile-time rejection)

A program is compiled once, at Registry load, against the environment of §7.4. A rejection names one of these codes; the golden file pins the code for each rejected program (the message text is not contractual).

| Code | Rejects |
|---|---|
| `E_SYNTAX` | a token or structure outside §2, including a chained comparison, a shadowed variable, a trailing `;` |
| `E_FLOAT_LITERAL` | a numeric token that is not `Int` or `Decimal` (`1e3`, `1.`, `.5`, `0x10`, `1_000`) |
| `E_LITERAL_RANGE` | a literal that does not fit its resolved type |
| `E_UNKNOWN_NAME` | an unresolved identifier, column, enum, enum value, command, relation, parameter or purpose |
| `E_TYPE` | an operand, argument, branch, key element or program result of the wrong type; `int` mixed with fixed point; two fixed types mixed; a comparison of enums with `<`; an `event` anywhere but an event-typed payload field, or an `Int` in one (§3.1) |
| `E_ARITY` | a builtin, relation or command with the wrong number of arguments, or a command argument missing or repeated |
| `E_CONST` | a degree that is neither an `Int` literal nor an `int` parameter; a purpose that is not an identifier |
| `E_DIV` | a divisor that is not a constant with non-zero bits in the division's type (§4.4) |
| `E_BOUND` | nesting deeper than 2, a bound product above 65,536 (reachable: `kin` × `lineage`, `lineage` × `children`), expression nesting above 64, a degree outside its relation's range (§4.4) |
| `E_PLANE` | a read of a column whose plane is not `A` |
| `E_UNDECLARED_READ` | a column read, or a relation whose columns, the declaring system neither owns nor lists in `[[reads]]` |
| `E_NO_NEUTRAL` | a read of a column another system owns whose `[[reads]]` entry declares no `neutral` value, whatever the world registers (§4.5). **Sequencing:** the field is a spec prerequisite of SPR-017 (moved there from SPR-014 in PRJ-002); until the system specs carry it, every law over a foreign read (`person.alive`, the log) is `E_NO_NEUTRAL`, so the loader cannot run the laws before the specs do |
| `E_UNDECLARED_COMMAND` | a command the declaring system's spec does not list in `[[commands]]` |
| `E_COMMAND_OWNER` | a command listed with a target that is not the command's owner |
| `E_RNG_PURPOSE` | a `draw` or `chance` purpose the declaring system does not list in `[[rng]]` (purposes resolve only against that list, so an unknown name is this code, not `E_UNKNOWN_NAME`) |
| `E_RNG_REUSE` | two `draw` or `chance` call sites with the same purpose, entity expression and index expression after constant folding, in one program or in programs that share a step; a draw inside a loop that is not keyed on the loop variable; any draw in the program of a verb whose payload carries a drawn outcome (§4.6); the same code names the run-time invariant for a key drawn twice in one evaluation |

A command to an owner the world does not register is not a rejection: it compiles and is dropped at registration (§6). A program with several faults is rejected with one of their codes (the checker stops at the first fault it meets, walking the program left to right, innermost first); every rejected golden program has exactly one fault, so its code is pinned. A program that compiles has two run-time failure modes, both hard invariants (§4.8).

## 11. Versioning and change classes

- The language is versioned `MAJOR.MINOR.PATCH`, recorded in `[effect_language] version` of the Registry and in the golden file's header; this document is `0.5.0` (`0.1.0` to `0.4.0` were the earlier rounds of PR #41; nothing evaluates them, so the value changes of rounds 3 to 5, the `death_cause` ordinals, the event encoding, the sortition key, the blood rule, its window and the guard, are re-approved golden rows rather than a MAJOR bump). The Rust evaluator refuses a Registry whose language version it does not implement.
- **Additive** changes (a builtin, a relation, a program kind, a new error class that only rejects programs that were already invalid) bump MINOR and are **C2** (Registry schema): `state_hash` is unchanged on every golden world because no existing program's value changes.
- A change to the **value of any existing construct** (a rounding rule, an order, a missing-read result, a bound, a tie rule, the draw mapping) bumps MAJOR and is **C3** as soon as any registered system evaluates a program on a golden world; until then (before SPR-021 lands) it is C2 with the golden file re-approved by the director. The golden file is the detector: a change that alters any expected value is at least the class above.
- Editing a **program** in the Registry (a law, a verb's effects) is a Registry edit: C2 for the schema, and C3 if a golden world's hash moves (`docs/13` §4.2: CI detects the class independently). Adding a law or a verb with effects is `data_only` (S4), C2.
- Reordering an id list that any program takes the `ordinal` of, compares in a key, or passes to a command (enum arguments are encoded as ordinals, §6) is C3 (§3.1); reordering one that only missing reads depend on is not, because the default is named (§3.1).
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
| D5 | Ascendants never inherit from descendants: every lineage law carries `not descends(holder, x)` | The usual reading of primogeniture; the lineage holds the holder's ancestors at heights 1–2, and without the guard a living father keys `(1, [])` ahead of his grandson and a living grandfather heads `seniority` (restored in round 3, N1; pinned on Ragnar as holder of 102). The `common(x, holder) != x` form leaks under uncle–niece marriage (a grandfather who is also an uncle: `common` finds the great-grandfather first), so the guard is the direct descent test (round 4 minor 1, `world.niece`) | 8/10 |
| D6 | Keys may hold bounded lists, compared lexicographically with a prefix first | The only way to state lineal primogeniture without recursion or a Rust-side rank | 7/10 |
| D7 | Enums carry no order; `ordinal` makes list order an explicit dependency | Keeps id-list reordering a C0 edit unless a program depends on it | 8/10 |
| D8 | Relations and accessors are code in `rh-registry` (the prelude), read through the host's `SystemView` with the host's derived children index; programs are data | `05.5` §2.1 says iteration is over *declared* relations; a contract crate holds types and accessors only (`05.1` §2 rule 1, NEW-3), so the walks cannot live there | 8/10 |
| D9 | `draw` returns the 64-bit draw shifted right by one (an `int` in [0, 2⁶³)); `chance` is `draw_hits` on the unshifted draw | `int` is signed; `chance` keeps the exact mapping of `draw.rs` | 8/10 |
| D10 | A `[commands]` Registry section with parameter lists, lint-checked against the specs | The checker cannot type a command argument otherwise; the specs' `[[commands]]` carry no signature | 7/10 |
| D11 | Effects are commands only: no timer, no event, no entity creation from a program | `05.5` §2.1 lists `schedule_outcome` and `emit_event`; neither is needed before SPR-024, and each adds host state the checker cannot see; v0.2 if a consumer needs them | 7/10 |
| D12 | Bounds: `kin` and `related` degree ≤ 6, generations and paths ≤ 6, `lineage` height ≤ 3, `holders_of` window ≤ 16, aggregate nesting ≤ 2, bound product ≤ 65,536, expression nesting ≤ 64; relation bounds as in §5.3: `lineage` **4,096 after pruning**, a hard failure above it | `dormancy_degrees` is bounded `[1, 6]` and the Registry's `kin(6)` needs 6; 65,536 keeps a program under a millisecond at `08`'s budgets. The lineage bound is sized against a stated fertility case (N2): five births per couple of which half marry and reproduce (2.5 effective), every line marrying out, three living generations (the holder's, its children's, its grandchildren's) and the dead above them carrying lines. One great-grandparent couple then has about 2.5³ + 2.5⁴ + 2.5⁵ ≈ 153 living descendants at the holder's three generations; the four couples of height 3 give about 610, with the pruned dead carriers about 650, and the reviewer's figure of about 1,300 (five surviving births, or two of the lines doubled by cousin marriage) is the pessimistic case; 4,096 is three times that. `lineage` nests only with the small relations under the product limit (`line` 6, `titles` and `offices` 16: 65,536 exactly); `kin` × `lineage` (1,048,576) and `lineage` × `children` (262,144) are `E_BOUND`, which is the reachable rejection the golden file pins. **For the director** with D4 and D14 | 7/10 |
| D13 | Genealogy is read from the life-course log (`life_course.person_id`, `.mother`, `.father`, `.birth_tick`, `.sex`), `same_instant` (the log is append-only), keyed by `person_id`, never from the ledger's `person.mother`/`person.father`; the review's `kin(a, b, n)` is `related(a, b, n)` here because `kin(p, n)` is already the relation `05.5` §2.1 names | The log covers the dead (REV-P1a NEW-3, NEW-7); the ledger compacts them, which would cut every line through a dead intermediary. One name for a relation and a predicate would make the grammar ambiguous | 8/10 |
| D14 | **Title blood** (B3, N5): a candidate is of the title's blood iff it descends from the holder or from one of the title's last sixteen holders (`holders_of(title, 16)`, the relation's maximum, over M7's tenure log); the law's order applies within that set; the unrestricted form is the Registry variant `primogeniture_open`. The rule rests on two things outside the language: the worldgen back-fill of pre-epoch tenure and life-course history (§5.3, SPR-019), without which a one-row log gives a childless holder an empty line; and the sixteen-tenure window, so a rota longer than sixteen tenures per bloodline is outside v0. **For the director** | A fixed founder fails seven generations down (`descends` reaches six), inside M7's horizon: the v0.2.0 rule then excluded the holder's sons and promoted his dead mother. Each holder in the window renews the anchor, so a seat that stays in a house keeps its blood indefinitely, and a maternal half-sibling still cannot outrank the holder's son; the cost is a four-column tenure log M7 keeps anyway for reign lengths. Eight tenures were too few for a rota (nine tenures in two generations left a first cousin's grandfather outside it); sixteen spans a two-generation rota of brothers and cousins, or four centuries of father-to-son tenures | 8/10 |
| D15 | Absent owners follow NEW-4: a read or send to an unregistered owner compiles; registration drops and reports it; the read takes the `neutral` value its `[[reads]]` entry declares, a field the checker requires on every foreign read whatever the world registers (`E_NO_NEUTRAL`, minor 5) | One rule in `rh-core` (SPR-014) instead of a per-program rejection that would make a Registry load depend on the world; a static field keeps the lint and the checker world-independent | 8/10 |
| D16 | Ticks are `SimTime` minutes; age tests are `int` arithmetic from `now`, `calendar.days_per_year` and `calendar.minutes_per_day`, the engine constant `rh_core::time::MINUTES_PER_DAY` (minor 6) | `rh_core::time::SimTime` is minutes (B1); a `q16_16` cannot hold a tick; the day length is the engine's, not a manifest field | 9/10 |
| D17 | `event` is an opaque type (N3): the verb's own event or `none`, usable only in an event-typed payload field (`cause_event`, `source_event`), rewritten by the `rh-core` merge | An EventId does not exist before the merge, so a program must not compute with it; typing it keeps `0`, arithmetic, comparison and draw keys out by construction | 9/10 |
| D18 | A verb's effects read the outcome the host drew from the event's payload (`[verbs.<verb>] payload`, bound like `actor`), and never redraw it (N4): `MURDER_ATTEMPT` binds `success`, and a program of a verb with a payload that calls `draw` or `chance` is rejected (`E_RNG_REUSE`, round 4 minor 2) | T1 draws `break_outcome` or `attack_outcome` once, writes the outcome into the event, and the effect follows the event; a draw in the program would roll a second, different outcome from the same purpose and break the one-draw-per-decision rule (MIN-3) | 9/10 |
| D19 | The draw-site rule has two layers (minor 3): the static lint folds constants, types the key and requires every loop variable in a loop's draw key, within one program and across programs that share a step; the evaluator refuses a key drawn twice within one evaluation of one program for one binding as an invariant failure (N6) | The static lint cannot see that two variables name one entity; the invariant cannot explain which site is wrong; together they make `05.1` §6 item 3 (one draw per call site) total. The ledger stops at the evaluation because a repeated evaluation with the same inputs in one step is idempotent by design, and a `SimError` there would abort normal runs | 8/10 |

**Open questions** (the Registry or a spec is silent)

| # | Question | Recommendation | Confidence |
|---|---|---|---|
| Q1 | Should `person.vocation == holy_orders` be in the laws' `eligible`, or stay M7's exclusion (the spec says M7 excludes "the deposed ruler and persons in holy orders")? | Keep it M7's: the law states the lineage rule; the court's exclusions change with institutions, not with the law. The golden file shows the alternative as `primogeniture_lay` | 7/10 |
| Q2 | *Closed (B2):* candidates come from `lineage(holder, 3)`, which reaches every line of the holder's ancestors to great-grandparents; a first cousin's son is in it, and truncation is a hard failure | |
| Q3 | `partition`: what do the other eligible children receive when M7 F1 cannot create titles? | The first of the line takes the title; each other eligible child gets a claim (`CLAIM_CREATED`), so partition reads as a contested inheritance until F2 divides seats | 6/10 |
| Q4 | `elective` and `tanistry` are orders, not votes. Is a vote needed in P1a? | No: M7's arbitration already weighs claims and offices; a vote with opinions is M6-lite or P1b. The orders here are the deterministic fallback M7's spec names for a sovereign with no candidate | 7/10 |
| Q5 | *Closed (minor 4):* M7 (0.3.2) declares `sortition` keyed `(candidate, k = the title, t)`; the law is `draw(sortition, x, title)` | |
| Q6 | *Closed (minor 1):* T1 (0.3.1) declares the `OpinionModifier` self-send in `[[commands]]`; `DESPAIR_PACT` compiles | |
| Q7 | Vow predicates need reads S7 does not declare (`office.*`, `person.residence`), and `PROTECT` has no completion | `AVENGE_KIN` reads `life_course.death_cause` (PR #37's M2, written once at exit), which S7 declares in PR #32's next round (the Architect is asking for it): closed on that point. The office reads come with S7's P1b sprint; `PROTECT` ends only by breaking (the beneficiary harmed by the vower, S7 `strain`) or voiding; `PILGRIMAGE` waits for S3 | 6/10 |
| Q8 | `emitted_by = "rh-minds"` on the older verbs (`ARREST`, `PURGE`, `REVENGE`, …) is a crate name, not a system id | Rename to `T1.minds` in the Registry with SPR-017's loader change; the checker needs a system id to find the spec | 9/10 |
| Q9 | Who evaluates: the host system at its binding, or `rh-core`? | The host, through an `rh-registry` API (`compile(program, env) -> Compiled`, `Compiled::eval(&SystemView, &ChildrenIndex, &Bindings) -> Value`): the reads pass through the same `SystemView` that enforces `AccessSpec` at run time, the prelude's walks run inside `rh-registry` over it, and the host supplies its derived children-by-parent index (M2) | 8/10 |
| Q10 | Should overlays (`derive`) be in v0 at all? | Reserve the kind and its signature only; its accessors (values, fear, success streaks) do not exist before P3.2b | 8/10 |
| Q11 | *Closed:* M7 (0.3.2) reads `life_course.person_id`, `mother`, `father`, `birth_tick` and `sex` at `same_instant`, T1 (0.3.1) the genealogy columns; M2 (0.2.0) declares `life_course.sex` and `life_course.death_cause`; M7's `on_birth` keeps `person.mother`/`person.father` | |
| Q12 | *Closed (B3):* `title.founder` is withdrawn; the blood rule reads M7's tenure log (`tenure.title`, `tenure.holder`, `tenure.start`, `tenure.end`), which PR #32 is being asked to add in place of the founder column | |
| Q13 | `ScheduleDeath.cause` is the `death_cause` enum, whose first value `none` M2 reserves for the living | Keep one enum (PR #37's choice) and let M2's `apply_death` reject `none` at the queue, as it rejects any malformed payload; a program that sends `death_cause.none` is a Registry bug, not a language case | 7/10 |
| Q14 | `HOLDINGS_BURNED`: T1 (0.3.1) declares `BurnHoldings {actor, household, cause_event}` to M4, but no v0 accessor yields a person's household, which is M4's table (P1b) | The program waits for an M4 accessor (`household_of(p)`, with M4 F0); until then it is `E_UNKNOWN_NAME` (pinned), and a `BurnHoldings` with no household compiles and is dropped at registration while M4 is absent (NEW-4, pinned) | 8/10 |
| Q15 | *Closed (N5):* a dynastic world's tenure log and life-course log must reach back before the epoch for the blood rule to anchor collaterals; `rh-worldgen` back-fills both (SPR-019 in PRJ-002), and §5.3 states the precondition | |

### 12.1 For the director (at the SPR-017 plan gate)

| Choice | Options | Recommendation | Confidence |
|---|---|---|---|
| Title blood (D14) | (a) the holder's descendants and the blood of the title's last sixteen holders, over M7's tenure log, resting on the worldgen back-fill of pre-epoch history (SPR-019) and the sixteen-tenure window (the default here); (b) unrestricted cognatic succession (`primogeniture_open`) | (a). A title that follows the blood of its holders reads as a dynasty and never outlives `descends`'s reach; (b) lets a maternal half-sibling outrank the holder's son. Both are pinned on the fixture, the deep world and the rota world, so either is a one-line Registry edit; a rota longer than sixteen tenures per bloodline is outside v0 | 8/10 |
| Sex rules (D4) | `primogeniture` male-preference cognatic, `seniority` and `tanistry` agnatic; variants (`absolute_primogeniture`, `agnatic_primogeniture`) as Registry entries | As stated: one id per law in the Registry, variants as data | 6/10 |
| Bounds (D12) | kin degree 6, generations 6, lineage height 3 and 4,096 members after pruning (a hard failure above), holders window 16, nesting 2, bound product 65,536 | As stated, with the fertility case of D12 (about 650 members expected, 1,300 pessimistic, 4,096 bound); raise the lineage bound only if SPR-030's dynastic fixture world trips it | 7/10 |

## 13. Conformance

- The golden file is the contract's test: every grammar rule, every row of §4.3, every relation and accessor of §5.3, every aggregate, every program kind, the seven laws and the four extensions on the fixture, the deep world, the rota world and the uncle–niece world, the lineage bound on generated fan trees, both invariants, and every error class, each with an expected value or code.
- `python -I specs/registry/effect_ref.py --check` exits 0 iff every golden case reproduces (558 cases at v0.5.0); `bin/check_all.sh` runs it, with a positive control (an altered expected value must fail).
- SPR-017's Rust evaluator (`rh-registry`, V-WC-10) reads the same file and must reproduce every case bit for bit, in debug and release builds, as `rh-sample`'s golden vectors do at both optimisation levels.
- The Python reference is standard-library only, exact (`fractions.Fraction`), and shares no code with the engine.
