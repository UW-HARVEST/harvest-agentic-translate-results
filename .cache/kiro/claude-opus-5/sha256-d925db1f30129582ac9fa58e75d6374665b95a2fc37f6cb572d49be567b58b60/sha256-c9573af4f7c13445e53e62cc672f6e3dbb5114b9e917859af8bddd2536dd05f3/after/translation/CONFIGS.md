# CONFIGS.md — configuration surface table (Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h` +
`c_src/CMakeLists.txt` + `translation/Cargo.toml`.

## Axes the C code actually branches on

**Compile-time options:** none. There is no `#ifdef` in `lib.c`, no
`target_compile_definitions` in `CMakeLists.txt`, and no `[features]` in
`Cargo.toml`. The full feature-combination set is the single empty combination.

**Runtime "options" / modes.** There is no options struct and no setter. The
library's mode is carried entirely by **three file-scope `static` variables**
that persist across calls and are mutated by the public API itself:

| state | init | mutated by | branched on at |
|-------|------|-----------|----------------|
| `accumulator`      | `0` | `add_to_accumulator`, `subtract_from_accumulator` | `if (accumulator > 0150)` (lib.c:142), `!!accumulator` (lib.c:153) |
| `multiplier`       | `1` | `multiply_with_multiplier`, `divide_multiplier`    | `if (multiplier > 0100)` (lib.c:161), `!!multiplier` (lib.c:154) |
| `operation_count`  | `0` | all four operation functions                      | `result += operation_count * 010` (lib.c:166) |

Because these are hidden, mutable, and *never reset*, **call-sequence position
is itself a configuration axis**: `findrep(1,2,3,4)` returns a different value
on the 1st, 2nd and 3rd invocation. Every row below is therefore driven as a
*sequence* against a freshly `dlopen`ed pair of libraries (each test copies both
`.so`s to a unique path so glibc gives it private, un-shared statics).

**Dispatch axes inside `findrep`:**

* `active_params = !!p1 + !!p2 + !!p3 + !!p4` ∈ {0,1,2,3,4} — compared against
  `mode_add = 01` and `mode_multiply = 02`, giving 3 distinct dispatch shapes:
  `active==0` (neither op), `active==1` (add only), `active>=2` (add + multiply).
* `accumulator > 0150` (104) gate → optional 3rd op (`subtract`).
* `both_active = !!accumulator && !!multiplier` gate → optional state term.
* `multiplier > 0100` (64) gate → optional 4th op (`divide`, always with `b==2`).
* `result == 0` → `0777` sentinel.

**Input-shape axes for `validate_and_normalize`** (applied to each of the 4
`findrep` params before dispatch, and reachable directly as an export):
`value == 0`; `0 < value < 64`; `value == 64`; `64 < value < 511`;
`value == 511`; `value > 511`; `value < 0`; `INT_MIN`; `INT_MAX`.

**Input-shape axes for `process_octal_string`** (`sprintf "%o"`/`"%d"` of an
`int`): `0`; small positive; positive needing all 11 octal digits; negative
(reinterpreted as `unsigned` by `%o` but signed by `%d`); `-1` (`37777777777`);
`INT_MIN`; `INT_MAX`.

**Input-shape axes for `find_and_replace_char`**: empty string; length 1;
long string; needle at index 0 / middle / last index; needle absent; needle
repeated (only the *first* is replaced); needle `== 0`; needle out of
`unsigned char` range; string containing high (`>= 0x80`) bytes, which is where
the `char`-signedness of `memchr`'s comparison matters.

## Rows (cross-product, pruned to combinations the C distinguishes)

Every row is exercised with **many randomized inputs** (SplitMix64, fixed seed
`0x5EED_1234_ABCD_0001`) against a freshly loaded pair of libraries, comparing
return values (and, for the pointer functions, the entire 256-byte destination
buffer) byte-for-byte.

### Low-level exports, driven directly (not via `findrep`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C1 | `validate_and_normalize` | all 9 value buckets + 4096 randomized `i32` (full range) | [x] |
| C2 | `validate_and_normalize` | exhaustive sweep of every boundary neighbourhood: `-1..1`, `63..65`, `510..512`, `INT_MIN±`, `INT_MAX±` | [x] |
| C3 | `process_octal_string` | all 7 value shapes + 2048 randomized `i32`; full 256-byte dest buffer compared | [x] |
| C4 | `find_and_replace_char` | needle present at index 0 / middle / last, over randomized ASCII strings | [x] |
| C5 | `find_and_replace_char` | needle absent; needle repeated (first-match-only); length 0 and 1 | [x] |
| C6 | `find_and_replace_char` | strings containing high bytes `0x80..0xFF`, needle in `0x80..0xFF` (signed-`char` comparison path) | [x] |
| C7 | `find_and_replace_char` | 2048 randomized (string, needle) pairs over the full byte domain, needle drawn from full `i32` | [x] |
| C8 | `add_to_accumulator` | fresh state, 4096 randomized `(a,b)` applied as a *sequence* — accumulates, so also covers `accumulator` sign changes and two's-complement wrap | [x] |
| C9 | `multiply_with_multiplier` | fresh state, 4096 randomized `(a,b)` as a sequence — drives `multiplier` to `0` and through overflow wrap | [x] |
| C10 | `subtract_from_accumulator` | fresh state, 4096 randomized `(a,b)` as a sequence | [x] |
| C11 | `divide_multiplier` | fresh state, 4096 randomized `(a,b)` as a sequence, `b` biased to include `0`, `±1`, small divisors (`INT_MIN/-1` excluded — see ERRORS.md E3) | [x] |
| C12 | interleaved `add`/`multiply`/`subtract`/`divide` | fresh state, 8192 randomized ops in randomized order — the composed-pipeline state machine, invisible to per-function tests | [x] |

### `findrep` — dispatch × normalization × hidden-state combinations

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C13 | `findrep` | `active_params == 0` (all four params `0`) on **fresh** state → neither dispatch runs; hits the `both_active` and sentinel logic | [x] |
| C14 | `findrep` | `active_params == 1`, each of the 4 positions being the nonzero one, × each normalization bucket for that param | [x] |
| C15 | `findrep` | `active_params == 2`, all 6 position pairs × normalization buckets | [x] |
| C16 | `findrep` | `active_params == 3`, all 4 position triples × normalization buckets | [x] |
| C17 | `findrep` | `active_params == 4`, all params nonzero × normalization buckets | [x] |
| C18 | `findrep` | all 16 zero/nonzero param masks, exhaustive, on fresh state (one fresh library pair per mask) | [x] |
| C19 | `findrep` | inputs chosen so `accumulator` lands just below / at / just above `0150` (104) before the gate → subtract-op gate off/off/on | [x] |
| C20 | `findrep` | inputs chosen so `multiplier` lands just below / at / just above `0100` (64) → divide-op gate off/off/on | [x] |
| C21 | `findrep` | inputs driving `multiplier` to exactly `0` (so `both_active == 0` and the state term is skipped) | [x] |
| C22 | `findrep` | inputs driving `accumulator` to exactly `0` (so `both_active == 0`) | [x] |
| C23 | `findrep` | inputs driving the final `result` to exactly `0` → `0777` sentinel (row E13 from the valid side). Reached **by construction**: on fresh state, `multiply_with_multiplier(M, 1)` then `findrep(1,0,0,0)` gives `result = 153 + M`, so `M == -153` lands exactly on 0 and the C returns `0o777`. Also swept over `M ∈ -400..200` for four param shapes; the neighbours `M = -154 / -152` are pinned to `-1 / 1` so the branch edge itself is asserted, not just the value. | [x] |
| C24 | `findrep` | params at the normalization boundaries `{INT_MIN, -1, 0, 1, 63, 64, 65, 510, 511, 512, INT_MAX}` — full 11^4 = 14641 cross-product, driven in batches of 64 against a freshly loaded pair, so every combination is seen both from a clean slate and mid-sequence | [x] |
| C25 | `findrep` | **repeated invocation on the same state**: 512-call sequence with randomized params, comparing every intermediate return — the hidden-static accumulation axis | [x] |
| C26 | `findrep` | 4096 fully randomized `(p1,p2,p3,p4)` over the full `i32` range as a single long sequence (value-dependent + overflow paths) | [x] |

### Cross-entry-point composition (low-level + high-level interleaved)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C27 | all 8 exports interleaved | fresh state, 8192 randomized calls picking uniformly among all 8 exports with randomized args; every return value and every buffer byte compared. This is the only row that exercises the real consumer pattern where direct low-level state mutation is interleaved with `findrep`. | [x] |
| C28 | all 8 exports interleaved | as C27 but with args biased toward boundary constants (`0`, `±1`, `63/64/65`, `104`, `510/511/512`, `INT_MIN`, `INT_MAX`) so the gates flip frequently | [x] |
| C29 | `add_to_accumulator` / `subtract_from_accumulator` then `findrep` | pre-seed `accumulator` past the `0150` gate via the low-level export, *then* call `findrep` — the subtract-dispatch path that `findrep`-only tests reach rarely | [x] |
| C30 | `multiply_with_multiplier` / `divide_multiplier` then `findrep` | pre-seed `multiplier` past the `0100` gate via the low-level export, then call `findrep` | [x] |

### Feature combinations

| #  | configuration | [x] |
|----|---------------|-----|
| C31 | default features (the only combination) — all rows above | [x] |
| C32 | `--no-default-features` — all rows above re-run | [x] |

## Dead stores: what is deliberately NOT a configuration axis

`findrep` builds two local buffers that never reach its return value.
`grep -n 'message\|result' c_src/src/lib.c` shows:

* `message` is written at lib.c:122 (`process_octal_string(message, 0123)`),
  mutated at lib.c:148 (`find_and_replace_char(message, 'O')`), copied to
  `final_message` at lib.c:151, and **never read again**;
* `result` is only ever written from the `'p'` offset in `search_buffer`
  (lib.c:127), the four operation return values, the
  `accumulator + multiplier` term, `operation_count * 010`, and the sentinel.

So the message *text* is unobservable through any exported symbol, and
`search_buffer` contributes only the **index** of its first `'p'` (9), making
the literal's tail unobservable too. `mutation_check.sh` records the three
mutations that exploit this as `EQUIVALENT` rather than as test gaps, and pins
the distinction with a sharper mutation that edits the literal *before* the
`'p'` (which moves the offset and **is** caught).

`process_octal_string` and `find_and_replace_char` are still verified
exhaustively as exported symbols in their own right (rows C3–C7 and
`tests/exhaustive.rs`); it is only their use *inside* `findrep` that is dead.

## Exhaustive coverage (beyond the sampled rows)

`tests/exhaustive.rs` enumerates whole input domains rather than sampling:

| sweep | domain | size |
|-------|--------|------|
| `exhaustive_validate_and_normalize_full_i32` (`#[ignore]`d; run with `--ignored`) | every `i32` | 4 294 967 296 |
| `exhaustive_validate_and_normalize_low_million` | `-2^20 ..= 2^20` | 2 097 153 |
| `exhaustive_validate_and_normalize_extremes` | `INT_MIN ..` and `.. INT_MAX` bands | 2 × 262 145 |
| `exhaustive_find_and_replace_byte_matrix` | every (string byte, needle low byte) pair × 3 positions × 3 int encodings | 587 520 |
| `exhaustive_process_octal_string_dense_band` | `-100000 ..= 100000` plus every power-of-two boundary, full 256-byte buffer compared | 200 k+ |
| `exhaustive_findrep_small_grid` | 8^4 param grid, fresh state every 32 calls | 4 096 |

The full-`i32` sweep of `validate_and_normalize` completed with all 4 294 967 296
inputs agreeing, which is what makes the two clamp-threshold mutants
*provably* equivalent rather than merely untested.

## How to reproduce

```sh
./run_all.sh          # C build + symbol parity + all tests x all feature combos x both profiles
./mutation_check.sh   # injects 43 bugs into the Rust and checks the suite catches them
cargo test --release --test exhaustive -- --ignored --nocapture   # the 2^32 sweep
```
