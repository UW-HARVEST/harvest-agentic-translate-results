# CONFIGS.md — Phase B valid-path configuration surface table

Derived mechanically from `c_src/src/lib.c`. The decisive property of this library is
that it is **stateful**: three file-scope `static` variables (`accumulator = 0`,
`multiplier = 1`, `operation_count = 0`) are mutated by every operation and are then
**branched on** by `findrep`. So the "runtime options" of this API are not flags passed
by the caller — they are the *reachable states of those three statics*, which the caller
selects by choosing a **call sequence**. A row is therefore
`(entry point) × (pre-seeded static state) × (input shape)`.

## Axis 1 — Public entry points (all 8, low-level ones included)

`add_to_accumulator`, `multiply_with_multiplier`, `subtract_from_accumulator`,
`divide_multiplier` (the four `operations[]` table members, callable directly),
`process_octal_string`, `find_and_replace_char`, `validate_and_normalize` (leaf helpers),
`findrep` (the composed pipeline, and the only function in `include/lib.h`).

## Axis 2 — State-dependent branches the C actually takes

| C line | branch condition | state axis it reads |
|--------|------------------|---------------------|
| L132 | `active_params >= mode_add` (`>= 1`) | input shape |
| L137 | `active_params >= mode_multiply` (`>= 2`) | input shape |
| L142 | `accumulator > 0150` (`> 104`) | `accumulator` |
| L157 | `has_accumulator && has_multiplier` | `accumulator != 0`, `multiplier != 0` |
| L161 | `multiplier > 0100` (`> 64`) | `multiplier` |
| L169 | `!result_exists` (`result == 0`) | computed result |
| L54 | `b != 0` | input shape |
| L81/82/84 | `value > 0`, `< 0100`, `> 0777` | input shape |
| L69/L126 | `memchr` hit vs. miss | input shape |

## Axis 3 — Input shapes `validate_and_normalize` special-cases

`INT_MIN` · negative · `0` · `1..63` (clamped up to 64) · `63` · `64` · `65..510` ·
`511` · `512..` (clamped down to 511) · `INT_MAX`

## Configuration table

`ap` = `active_params`. "fresh" = statics at their initial values
(`accumulator=0, multiplier=1, operation_count=0`), obtained by `dlopen`ing a *fresh
private copy* of each `.so` so the two libraries always start in lockstep.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `validate_and_normalize` | `value = 0` (identity path, `is_nonzero == 0`) | [x] |
| 2 | `validate_and_normalize` | `value` random in `1..=63` → clamps up to 64 | [x] |
| 3 | `validate_and_normalize` | `value` random in `64..=511` → identity | [x] |
| 4 | `validate_and_normalize` | `value` random in `512..=INT_MAX` → clamps to 511 | [x] |
| 5 | `validate_and_normalize` | `value` random in `INT_MIN..=-1` → identity (never clamped) | [x] |
| 6 | `validate_and_normalize` | exact boundaries `{INT_MIN,-1,0,1,63,64,65,510,511,512,INT_MAX}` | [x] |
| 7 | `validate_and_normalize` | full-range random `i32`, 20 000 draws | [x] |
| 8 | `add_to_accumulator` | fresh state, random `(a,b)` — accumulates over 500 calls, state carried | [x] |
| 9 | `add_to_accumulator` | operands forcing signed overflow (`INT_MAX + INT_MAX`, `INT_MIN + INT_MIN`) | [x] |
| 10 | `multiply_with_multiplier` | fresh state, random `(a,b)` — `multiplier` wraps quickly, 500 calls | [x] |
| 11 | `multiply_with_multiplier` | `(a,b)` where `a*b == 0` → drives `multiplier` to 0 permanently (kills L157/L161) | [x] |
| 12 | `multiply_with_multiplier` | overflow shapes `(INT_MIN,1)`, `(INT_MAX,INT_MAX)`, `(-1,INT_MIN)` | [x] |
| 13 | `subtract_from_accumulator` | fresh state, random `(a,b)`, 500 calls, state carried | [x] |
| 14 | `subtract_from_accumulator` | `a-b` overflow shapes (`INT_MIN - INT_MAX`) | [x] |
| 15 | `divide_multiplier` | `b != 0`, `multiplier` positive → truncation toward zero | [x] |
| 16 | `divide_multiplier` | `b != 0`, `multiplier` negative → C truncation toward zero, not floor | [x] |
| 17 | `divide_multiplier` | `b` negative, sign-mixed dividend/divisor | [x] |
| 18 | `divide_multiplier` | `b == 0` guard taken, `operation_count` still bumped (see ERRORS #1) | [x] |
| 19 | `divide_multiplier` | interleaved with `multiply_with_multiplier` so `multiplier` cycles sign | [x] |
| 20 | `process_octal_string` | `octal_val = 0` → `"Octal: 00, Decimal: 0"` (literal `0` prefix + `%o` of 0) | [x] |
| 21 | `process_octal_string` | `octal_val = 0123` (83), the value `findrep` actually uses | [x] |
| 22 | `process_octal_string` | random positive `octal_val`, 2 000 draws, full 100-byte dest compared | [x] |
| 23 | `process_octal_string` | negative `octal_val` → `%o` as `unsigned`, `%d` as signed | [x] |
| 24 | `process_octal_string` | `INT_MIN`, `INT_MAX`, `-1`, `1`, `7`, `8`, `0777`, `0100` | [x] |
| 25 | `process_octal_string` | dest pre-filled with `0xAA` sentinel — verifies exact NUL placement and that no byte past the terminator is touched | [x] |
| 26 | `find_and_replace_char` | hit at index 0 | [x] |
| 27 | `find_and_replace_char` | hit mid-string / at last character | [x] |
| 28 | `find_and_replace_char` | miss (ERRORS #12) | [x] |
| 29 | `find_and_replace_char` | empty string (ERRORS #13) | [x] |
| 30 | `find_and_replace_char` | multiple occurrences → only first replaced (ERRORS #17) | [x] |
| 31 | `find_and_replace_char` | `search_char` full `i32` range incl. `>255`/negative/low-byte-0 (ERRORS #14–16) | [x] |
| 32 | `find_and_replace_char` | random byte strings (incl. high-bit `0x80..0xFF` bytes) × random `search_char`, 5 000 draws | [x] |
| 33 | `find_and_replace_char` | string already containing `'X'` | [x] |
| 34 | `findrep` | fresh state, `ap = 0` (`0,0,0,0`) — both op blocks skipped, sentinel path | [x] |
| 35 | `findrep` | fresh state, `ap = 1` — add block only (each of the 4 positions) | [x] |
| 36 | `findrep` | fresh state, `ap = 2` — add + multiply blocks (all 6 position pairs) | [x] |
| 37 | `findrep` | fresh state, `ap = 3` (all 4 position triples) | [x] |
| 38 | `findrep` | fresh state, `ap = 4` | [x] |
| 39 | `findrep` | fresh state, params drawn from the `validate_and_normalize` shape classes (clamp-up / identity / clamp-down / negative / zero) — full cross-product | [x] |
| 40 | `findrep` | fresh state, params chosen so `accumulator > 104` on the **first** call → L142 subtract block fires | [x] |
| 41 | `findrep` | fresh state, params chosen so `accumulator <= 104` → L142 skipped | [x] |
| 42 | `findrep` | state where `multiplier > 64` → L161 divide fires | [x] |
| 43 | `findrep` | state where `multiplier == 0` → `both_active == 0`, L158 skipped (ERRORS #26) | [x] |
| 44 | `findrep` | state where `accumulator == 0` but `multiplier != 0` (ERRORS #25) | [x] |
| 45 | `findrep` | **repeated calls, state carried**: same params called 1..50 times in a row — every later call sees a different `accumulator`/`multiplier`/`operation_count` and takes different branches | [x] |
| 46 | `findrep` | **mixed sequence**: low-level ops pre-seed the statics, then `findrep` runs (composed-pipeline coverage that per-function tests cannot reach) | [x] |
| 47 | `findrep` | boundary params `{INT_MIN,-1,0,1,63,64,511,512,INT_MAX}` cross-product over all 4 slots | [x] |
| 48 | `findrep` | randomized fuzz: 20 000 calls, random params, state never reset — deep state divergence detector | [x] |
| 49 | all 8 | randomized **interleaving** of every entry point (random op, random args), 20 000 steps on one shared library instance, comparing every return value and every buffer | [x] |
| 50 | `findrep` | result-forced-to-zero search: params hunted so the computed `result` is exactly 0 → sentinel `0777` (ERRORS #28) | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the cross-product of features is the single
default configuration. Phase D re-runs the whole suite under `--no-default-features` and
`--all-features` to confirm they are identical builds.

## Verification status

All 50 rows pass. Test files, all driving both `.so` files through `libloading`:

| file | rows covered | tests |
|------|--------------|-------|
| `tests/smoke.rs`            | harness self-check + pristine-static-state proof | 3 |
| `tests/leaf_functions.rs`   | 1–7, 20–33 + capacity boundaries                 | 26 |
| `tests/stateful_ops.rs`     | 8–19, 49 (arithmetic subset)                     | 14 |
| `tests/findrep.rs`          | 34–48, 50 + branch-coverage audit                | 23 |
| `tests/all_entry_points.rs` | 49 (full 8-entry-point interleaving)             | 3 |
| `tests/crash.rs`            | fatal paths (ERRORS 3, 18, 19)                   | 6 |

`tests/findrep.rs` additionally runs an INDEPENDENT third implementation of the C
control flow, so each call is checked three ways (C vs Rust vs model), and
`branch_coverage_audit` asserts that all six `findrep` branches were observed both
taken and not taken — this is what keeps the suite from being vacuously green.

Because `dlopen` de-duplicates by `(dev, ino)`, every test loads a **fresh private
copy** of each `.so`, so the three mutable statics start at
(`accumulator=0, multiplier=1, operation_count=0`) on both sides and stay in lockstep.
`tests/smoke.rs::fresh_pair_resets_hidden_static_state` proves this actually works.
