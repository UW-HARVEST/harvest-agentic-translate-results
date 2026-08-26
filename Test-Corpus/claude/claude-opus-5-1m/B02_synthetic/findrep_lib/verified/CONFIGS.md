# CONFIGS.md — Configuration surface (valid inputs) of `c_src/src/lib.c`

## Axes derived mechanically from the C source

**Build-time axes:** none. `Cargo.toml` has no `[features]`; `CMakeLists.txt`
sets no `-D` options; `lib.c` has no `#if/#ifdef`. → exactly **one** build
configuration (default / `--no-default-features`), so every row below is
verified once per row, under that single configuration.

**Public entry points (all 8 exported symbols, low-level first):**

| level | symbols |
|-------|---------|
| leaf, pure | `validate_and_normalize`, `process_octal_string`, `find_and_replace_char` |
| leaf, stateful | `add_to_accumulator`, `multiply_with_multiplier`, `subtract_from_accumulator`, `divide_multiplier` |
| composed | `findrep` (the only function in `include/lib.h`; calls all of the above through the `operations[4]` function-pointer table) |

**Runtime "options"/modes the C code branches on** (there are no flag
parameters; the *hidden `static` state* is the option state, and it is settable
only through the public ABI — so it is an axis of every test):

| axis | states the C distinguishes | source |
|------|----------------------------|--------|
| `accumulator` | `== 0` (`both_active` false) / `<= 0150` (=104, subtract skipped) / `> 0150` (subtract runs) / negative | lib.c:142,153,157 |
| `multiplier` | `== 0` (`both_active` false) / `<= 0100` (=64, divide skipped) / `> 0100` (divide runs) / negative / `INT_MIN` (trap on `/-1`) | lib.c:55,154,161 |
| `operation_count` | any; contributes `operation_count * 010` to `findrep`'s result | lib.c:166 |
| `active_params` | `0` (no op), `1` (add only), `2`,`3`,`4` (add + multiply) | lib.c:107,132,137 |
| per-param normalization class | `v <= 0` (pass-through) / `0 < v < 0100` (→64) / `0100 <= v <= 0777` (identity) / `v > 0777` (→511) | lib.c:81-87 |
| `octal_val` shape | `0` / small positive / `> 0777` / negative (`%o` prints as `unsigned`, `%d` as signed) / `INT_MIN` / `INT_MAX` — governs rendered length 12…41 bytes | lib.c:63 |
| needle byte class | present-first / present-middle / present-last / absent / `0` / `>0xFF` / negative / high-bit (`0x80..0xFF`) | lib.c:68 |
| haystack shape | empty / 1 byte / many bytes / repeated needle (only 1st replaced) / high-bit bytes / already-`'X'` | lib.c:68-71 |
| call-sequence shape | single call / long randomized interleaved sequence (state carried across calls; this is the only way to reach `accumulator > 104`, `multiplier > 64`, wrap-around, `multiplier == 0`) | whole file |

## Rows (one per combination the C treats differently)

Each row is a differential test in `tests/differential.rs`: the same call is
issued to the C `.so` and the Rust `.so` and *every* observable is compared —
return value, the full 256-byte destination buffer (with `0xAA` canaries), and
the post-call hidden state (probed with the state-neutral calls
`add_to_accumulator(0,0)` / `divide_multiplier(0,1)`). Rows marked
"randomized" use ≥256 pseudo-random inputs from a fixed-seed xorshift PRNG.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `validate_and_normalize` | all 8 documented boundaries `{0,1,63,64,65,510,511,512}` + `INT_MIN`, `INT_MAX`, `-1` | `cfg_validate_boundaries` | [x] |
| 2  | `validate_and_normalize` | randomized full `int` range (uniform 32-bit), 4096 values | `cfg_validate_random_full_range` | [x] |
| 3  | `validate_and_normalize` | randomized in each clamp class separately (`<=0`, `1..63`, `64..511`, `512..INT_MAX`) | `cfg_validate_random_per_class` | [x] |
| 4  | `process_octal_string` | `octal_val == 0` → `"Octal: 00, Decimal: 0"` (shortest output, 21 bytes) | `cfg_process_octal_string_boundaries` | [x] |
| 5  | `process_octal_string` | small positive (`1`, `7`, `8`, `0123`, `0777`) — 1–3 octal digits | `cfg_process_octal_string_boundaries` | [x] |
| 6  | `process_octal_string` | `INT_MAX` (`017777777777`, 11 octal digits, 10 decimal digits) | `cfg_process_octal_string_boundaries` | [x] |
| 7  | `process_octal_string` | negative values `-1`, `-8`, `INT_MIN` → `%o` prints the unsigned bit pattern (11 digits), `%d` prints the sign (longest output, 41 bytes) | `cfg_process_octal_string_boundaries` | [x] |
| 8  | `process_octal_string` | randomized full `int` range, 2048 values, full 256-byte buffer + canary compare | `cfg_process_octal_string_random` | [x] |
| 9  | `process_octal_string` | same `dest` reused for a long → short rendering (stale bytes after the NUL must match) | `cfg_process_octal_string_reused_buffer` | [x] |
| 10 | `find_and_replace_char` | needle at index 0 / middle / last index of a multi-byte haystack | `cfg_find_and_replace_positions` | [x] |
| 11 | `find_and_replace_char` | needle occurs many times → only the **first** occurrence becomes `'X'` | `cfg_find_and_replace_repeated` | [x] |
| 12 | `find_and_replace_char` | 1-byte haystack, hit and miss | `cfg_find_and_replace_positions` | [x] |
| 13 | `find_and_replace_char` | haystack containing high-bit bytes `0x80..0xFF`, needle given as a negative `int` and as `>0xFF` `int` (unsigned-char narrowing) | `cfg_find_and_replace_high_bit` | [x] |
| 14 | `find_and_replace_char` | randomized haystacks (random length 0..80, random bytes 1..255) × random needles from full `int` range, 4096 cases, full-buffer compare | `cfg_find_and_replace_random` | [x] |
| 15 | `add_to_accumulator` | randomized `(a,b)` full `int` range incl. `INT_MIN/INT_MAX`, 1024 calls, return + probed state compared each call | `cfg_add_random` | [x] |
| 16 | `subtract_from_accumulator` | randomized `(a,b)` full range, 1024 calls | `cfg_sub_random` | [x] |
| 17 | `multiply_with_multiplier` | randomized `(a,b)` full range, 1024 calls (drives `multiplier` to 0, to negatives, and through wrap-around) | `cfg_mul_random` | [x] |
| 18 | `divide_multiplier` | `b` ∈ `{0, 1, -1, 2, -2, 3, INT_MAX, INT_MIN}` × current `multiplier` sign/magnitude classes (guarding the `INT_MIN / -1` trap, covered in ERRORS row 4) | `cfg_divide_special_divisors` | [x] |
| 19 | `divide_multiplier` | randomized `b` (full range, incl. 0), 1024 calls | `cfg_divide_random` | [x] |
| 20 | all 4 stateful ops | long randomized **interleaved** sequence (8192 ops, random op index 0..3, random args) — verifies `accumulator`, `multiplier`, `operation_count` stay bit-identical across the composed pipeline | `cfg_interleaved_state_sequence` | [x] |
| 21 | `findrep` | `active_params == 0` (all params 0): add/multiply skipped; result depends only on state + `operation_count` | `cfg_findrep_active_param_counts` | [x] |
| 22 | `findrep` | `active_params == 1` (each of the 4 positions non-zero in turn): add runs, multiply skipped | `cfg_findrep_active_param_counts` | [x] |
| 23 | `findrep` | `active_params == 2` and `3` (all C(4,2)+C(4,3) = 10 non-zero position masks) | `cfg_findrep_active_param_counts` | [x] |
| 24 | `findrep` | `active_params == 4`, params in each normalization class (`<0`, `1..63`, `64..511`, `>511`) — 4^4 = 256 class combinations | `cfg_findrep_normalization_classes` | [x] |
| 25 | `findrep` | boundary params `{INT_MIN,-1,0,1,63,64,65,104,105,510,511,512,INT_MAX}` cross-product on the first two params, others fixed | `cfg_findrep_boundary_cross` | [x] |
| 26 | `findrep` | with `accumulator > 0150` pre-set (subtract branch taken) | `cfg_findrep_accumulator_over_threshold` | [x] |
| 27 | `findrep` | with `multiplier > 0100` pre-set (divide branch taken) and with `multiplier == 0` (`both_active` false) | `cfg_findrep_multiplier_states` | [x] |
| 28 | `findrep` | randomized full-range params, 4096 calls, state carried over between calls (the realistic consumer pattern) | `cfg_findrep_random` | [x] |
| 29 | `findrep` interleaved with the 4 leaf ops | randomized mixed sequence over all 8 entry points (8192 steps) — the full composed pipeline, state + buffers compared at every step | `cfg_full_api_random_sequence` | [x] |
| 30 | all 8 entry points | first-call behaviour from the pristine initial state (`accumulator=0, multiplier=1, operation_count=0`) — verified in a dedicated child process so the state is untouched | `cfg_pristine_initial_state` (subprocess) | [x] |
| 31 | `process_octal_string` | every `%o`/`%d` digit-count transition: `8^k`, `8^k ± 1` for k=0..10 and `10^k`, `10^k ± 1` for k=0..9, plus their negatives (this is where a hand-written formatter drifts from glibc) | `cfg_process_octal_string_digit_boundaries` | [x] |
| 32 | `validate_and_normalize` | exhaustive: every single value in `-2048..=4096` (covers all four clamp classes and both thresholds byte for byte) | `cfg_validate_exhaustive_small_range` | [x] |
| 33 | `findrep` | full 4-way grid over the class representatives `{0,1,64,65,511,512}` = 1296 calls with state carried across them | `cfg_findrep_full_small_grid` | [x] |
| 34 | all 8 entry points | long deterministic random walk (3000 steps, fixed seed) from the pristine state in a child process, full transcript compared line by line | `cfg_pristine_initial_state` / `scenario_transcripts_match` (`long_random`) | [x] |

## Result

All rows pass. Every row was run four times: {Rust dev profile, Rust release
profile (`panic = "abort"`)} × {C default cmake build (`-O0`), C `-O2`
`CMAKE_BUILD_TYPE=Release` build}, in the single valid feature configuration
(there is only one). Driver: `./verify_all.sh` (see `$TMPDIR/verify_all.log`),
52 tests per run, `52 passed; 0 failed` in all four.

On top of that, `RUN_EXHAUSTIVE=1 ./verify_all.sh` adds three brute-force sweeps
(`#[ignore]`d by default because of their runtime) and they pass in all four
artifact combinations as well:

| sweep | coverage |
|-------|----------|
| `exhaustive_validate_all_i32` | **all 2^32** `int` inputs of `validate_and_normalize`, C vs Rust |
| `exhaustive_process_octal_wide` | every value in `-2^22..=2^22` plus every 4096th value of the whole `int` range (full buffer compare each) |
| `exhaustive_find_and_replace_needles` | every needle in `-2^17..=2^17` and every 251st value of the whole `int` range, against haystacks covering all 255 non-NUL bytes |

A 300 000-step randomized differential fuzz over all 8 entry points was also
run from the pristine state (`HARVEST_STEPS=300000`): both children produced
600 000 transcript lines that are byte-identical (`cmp` clean).

Suite stability was also checked with `--test-threads=1`, `4` and `16` (the
libraries share hidden state, so every call is issued to both libraries in
lockstep under one mutex; results are order-independent).

## Harness validation (mutation testing)

To prove the differential suite actually has teeth (a suite that passes because
it compares nothing is worse than no suite), 13 mutants of `src/lib.rs` were
compiled into a separate cdylib with `rustc` and fed to the suite through
`HARVEST_RUST_LIB` (the checked-in `src/lib.rs` was never modified). **All 13
were killed:**

| mutant | tests failed |
|--------|--------------|
| `upper_threshold 0o777 → 0o776` | 30 |
| `lower_threshold 0o100 → 0o101` | 31 |
| `validate_and_normalize` clamps negatives too (drop `value > 0`) | 31 |
| replacement byte `'X' → 'Y'` | 9 |
| `memchr` length `strlen(s) → strlen(s)+1` | 7 |
| `'p'` search byte → `'P'` | 18 |
| `operation_count++` removed from `add_to_accumulator` | 18 |
| `operation_count * 010 → * 011` | 18 |
| `%o → %x` in the rendered message | 7 |
| sentinel `0777 → 0776` | 2 |
| `accumulator > 0150 → >=` | 2 |
| `multiplier > 0100 → >=` | 3 |
| `mode_multiply 02 → 03` | 13 |
| `both_active` `&& → ||` | 17 |
| division truncation → floor (`div_euclid`) | 2 |
| divide guard `b != 0 → b != 1` | whole run aborts with SIGFPE (hard failure) |
