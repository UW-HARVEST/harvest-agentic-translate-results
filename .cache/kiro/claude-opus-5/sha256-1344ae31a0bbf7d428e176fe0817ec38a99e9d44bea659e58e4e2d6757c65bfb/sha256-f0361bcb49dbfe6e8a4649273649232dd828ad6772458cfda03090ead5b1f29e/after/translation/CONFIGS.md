# CONFIGS.md — configuration-surface table (valid inputs)

## Axes mechanically derived from `c_src/src/lib.c`

There is no compile-time configuration (`#ifdef`) and no runtime "option
struct". The library's configuration surface is the *state the public API can
put a `ProcessState` into* plus the *shape of each argument*. The axes the C
actually branches on:

* **A1 — entry point.** All six external symbols, including the low-level ones
  (`create_state`, `destroy_state`, `update_flags`, `process_buffer`,
  `confuse_types`), not just the `confusion` one-shot wrapper.
* **A2 — `create_state.capacity`.** Controls `malloc` size *and* the `snprintf`
  truncation bound: `0`, `1`, shorter-than-output (truncating), exactly-fitting,
  generous (`128`), huge.
* **A3 — `create_state.initial_val`.** Decides the buffer text *and* the union
  bit pattern read later by `confuse_types`: `0`, small +, small −, `INT_MIN`,
  `INT_MAX`, and bit patterns that are NaN / ±inf / denormal / >2^31 as `float`.
* **A4 — `update_flags.param`.** Branches on bits 0,1,2 (`flag1..3`) and bits
  3–5 (`mode`, 8 values), and on the *sign* of `param` (`param >> 3` is an
  arithmetic shift). Also the **call count**, since `counter` is a 5-bit field
  incremented per call and wrapping at 32.
* **A5 — `process_buffer.target`.** Byte classes: absent, present once, present
  many times, `'\0'`, ASCII, high-bit-set (signed `char` vs `unsigned char`
  comparison in `memchr`).
* **A6 — `confuse_types.operation`.** The four `switch` arms `0,1,2,3` (each a
  different reinterpretation of the union) plus out-of-range.
* **A7 — union state at the time `confuse_types` runs.** Fresh from
  `create_state` vs. after a `confuse_types(state, 0)` overwrite
  (`1078530011` == `3.14159f`). This is the *interaction* axis: arm 1/2/3 read
  whatever arm 0 wrote.
* **A8 — `confusion.param1..param4`.** `param3 % 10` selects the search byte
  (negative → non-digit); `param4 % 4` selects the arm (negative → no arm).

## Rows (pruned cross-product — combinations the C treats differently)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `create_state` + `destroy_state` | `capacity = 128` (generous), `initial_val` randomized over full `i32` | [x] |
| 2  | `create_state` + `destroy_state` | `capacity = 1` (NUL only), randomized `initial_val` | [x] |
| 3  | `create_state` + `destroy_state` | `capacity = 2..16` (truncating `snprintf`), randomized `initial_val` | [x] |
| 4  | `create_state` + `destroy_state` | `capacity` exactly `strlen("State:%d:Mode:3")+1` for the given `initial_val` (boundary) | [x] |
| 5  | `create_state` + `destroy_state` | `capacity` huge but plausible (`1<<20`), randomized `initial_val` | [x] |
| 6  | `create_state` | `initial_val ∈ {0, 1, -1, INT_MIN, INT_MAX}` × `capacity ∈ {1,8,16,128}` (boundary matrix) | [x] |
| 7  | `update_flags` | single call, `param` randomized over full `i32` (covers all `flag1..3` × `mode` combinations and both signs) | [x] |
| 8  | `update_flags` | `param = 0..63` exhaustively (all 8 `mode` values × all 8 low-bit combinations) | [x] |
| 9  | `update_flags` | 40 successive calls on the same state → `counter` saturates through 31 and **wraps to 0** (5-bit field) | [x] |
| 10 | `update_flags` | negative `param` (`INT_MIN`, `-1`, randomized negatives) → arithmetic `>>3` into `mode` | [x] |
| 11 | `process_buffer` | target present exactly once in the buffer (a digit of `initial_val`) | [x] |
| 12 | `process_buffer` | target present many times (`':'`, or a repeated digit e.g. `initial_val = 1111111`) | [x] |
| 13 | `process_buffer` | target absent (`'Z'`, `'~'`), randomized `initial_val` | [x] |
| 14 | `process_buffer` | `target = '\0'` (never inside the `strlen` bound) | [x] |
| 15 | `process_buffer` | `target` high-bit-set / negative `char` (`-1`, `-128`, randomized `i8 < 0`) | [x] |
| 16 | `process_buffer` | called on a truncated buffer (`capacity = 2..16`) — occurrence count depends on truncation | [x] |
| 17 | `process_buffer` | called twice in a row on the same state (idempotence — the loop must not consume state) | [x] |
| 18 | `confuse_types` | `operation = 0` on a fresh state, randomized `initial_val` (writes `1078530011`) | [x] |
| 19 | `confuse_types` | `operation = 1` on a **fresh** state → `float_val` is the raw `initial_val` bit pattern; randomized over full `i32`, so NaN, ±inf, denormals and values whose `*100` overflows `int` all occur | [x] |
| 20 | `confuse_types` | `operation = 1` **after** `operation = 0` → reads `3.14159f`, returns `314` | [x] |
| 21 | `confuse_types` | `operation = 2` on a fresh state, randomized `initial_val` (`uint_val & 0xFF`) | [x] |
| 22 | `confuse_types` | `operation = 2` after `operation = 0` | [x] |
| 23 | `confuse_types` | `operation = 3` on a fresh state, randomized `initial_val` (signed `char` byte sum, can be negative) | [x] |
| 24 | `confuse_types` | `operation = 3` after `operation = 0` | [x] |
| 25 | `confuse_types` | targeted `initial_val` bit patterns for arm 1: `0x7F800000` (+inf), `0xFF800000` (−inf), `0x7FC00000` (NaN), `0x00000001` (denormal), `0x4F000000` (2^31, overflows on `*100`), `0x00000000` (+0), `0x80000000` (−0) | [x] |
| 26 | `create_state`→`update_flags`→`process_buffer`→`confuse_types` | full low-level pipeline in C order, all four inputs randomized, comparing every intermediate return value **and** the final `flags`/`data` bytes read back through the struct | [x] |
| 27 | pipeline with reordering | `confuse_types(0)` *before* `process_buffer` (buffer text is unaffected — union and buffer are independent) | [x] |
| 28 | `confusion` | all four params randomized over full `i32` (1000+ cases, fixed seed) | [x] |
| 29 | `confusion` | `param3 ≥ 0` → digit search byte; `param3 < 0` → non-digit byte from negative `%` | [x] |
| 30 | `confusion` | `param4 % 4 ∈ {0,1,2,3}` each forced, and `param4 < 0` → `{-1,-2,-3}` no-arm | [x] |
| 31 | `confusion` | `param1 ∈ {0,1,-1,INT_MIN,INT_MAX}` × `param2 ∈ {0..63}` × `param3 ∈ {-10..10}` × `param4 ∈ {-4..4}` boundary matrix | [x] |
| 32 | `confusion` | repeated invocations in one process (no cross-call state leaks; also exercises the stdout stream) | [x] |
| 33 | stdout comparison | every one of rows 1–32 additionally compared on **captured stdout bytes** (all six functions print), via `dup2` redirection into a temp file | [x] |

## Phase D — feature combinations

`Cargo.toml` has no `[features]` table, so the complete set of feature
combinations is `{default}` == `{no-default-features}`. Both were exercised:

```
cargo test --release
cargo test --release --no-default-features
cargo test --release --all-features
```

No `[[bin]]` target exists, so there is no driver executable to compare
stdout for; the stdout comparison is done at the FFI level instead (row 33).

## Verification record

All rows above are checked: each is covered by a test in `tests/phase_b.rs`
(`row01_…` … `row32_…`), driven with a fixed-seed RNG (`Rng::new`, splitmix64)
over many inputs per row, and every one compares the C and Rust `.so`s on
return values, the `flags` bit-field word, the `data` union word, `capacity`,
the `buffer` bytes, AND the exact captured stdout (row 33).

Additional coverage beyond the table, in `tests/deep.rs` and `tests/interop.rs`:

| sweep | calls per side | what it pins down |
|-------|----------------|-------------------|
| `deep_op1_strided_full_float_space` via `sweep_full.sh` | 536,870,912 | every 8th binary32 pattern across the WHOLE 2^32 space through arm 1 |
| `deep_op1_float_to_int_overflow_band` | 63,947 | exponents 140–170 x 1024 mantissa steps x both signs (where `cvttss2si` differs from a saturating cast) |
| `deep_op1_float_to_int_structured` | 36,896 | all 256 exponents x both signs (zero, denormal, normal, inf, NaN) |
| `deep_op1_float_to_int_low_exponents` | 30,051 | exponents 0–140 + every small integer and /100, /3 |
| `deep_op2_and_op3_full_byte_coverage` | 85,536 x 3 arms | all 65,536 low-two-byte combinations |
| `deep_update_flags_wide` | 240,001 | params -70k..70k exhaustively + 100k random i32 |
| `deep_process_buffer_wide` | 40,286 cases / 165,944 matches | random byte soup incl. 0x80–0xff, dense 2-symbol alphabets, uniform buffers to length 1000, every one of the 256 target values |
| `deep_confusion_wide` | 45,120 | all four params random + inf/NaN/overflow-inducing `param1` |
| `cross_library_pipeline_agrees` | 300 x 4 combos | a state allocated by one `.so` driven by the other `.so`'s functions |

`tests/interop.rs` additionally pins `sizeof`/field offsets and the bit-field
positions inside `PackedFlags` by having each implementation read back state
that the other one wrote.
