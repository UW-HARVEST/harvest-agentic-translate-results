# CONFIGS.md — Phase A configuration surface (valid inputs)

Axes derived mechanically from the C source, not guessed.

## Public entry points (all of them, lowest level first)

| id | entry point | source |
|----|-------------|--------|
| `A` | `int array[262144]` — exported `.bss` object; a caller can **read and write it directly**, so it is both an input and an output channel | `src/long.c:33` |
| `B` | `void perform_expensive_operations(void)` — the lowest-level operation (`f^100` element-wise); not declared in the header but exported | `src/long.c:36` |
| `C` | `void long_exec(unsigned int seed)` — the one-shot wrapper: `srand` + `rand()` fill + 2000×`B` + XOR + `printf("%d\n")` | `src/long.c:49`, `include/long.h:26` |

## Runtime option / mode axes present in the C

Grepping for `if`, `switch`, `#if`, flags, or any global other than `array`
returns nothing (see `ERRORS.md` for the greps): the C library has **no runtime
options, no modes, no flags and no conditional compilation**. Its entire
configuration surface is therefore

1. **which entry point** is called (`B` alone vs. `C`, and in what order),
2. **how many times** `B` is invoked in a row (`k`); `C` fixes `k = ITERATIONS = 2000`,
3. **the data shape held in `array`** when `B` runs, and
4. **the `seed` value** given to `C`.

Axis 3 is the interesting one, because `f` is value-dependent in four separate
ways (sign-dependent arithmetic right shift, left shift of negatives, division
truncating toward zero, remainder taking the dividend's sign) and wraps on
signed overflow. Axis 4 matters because the Rust `long_exec` does **not** run
the nested loop: it computes the same `f^200000` through exact
function-iteration algebra (`src/fast.rs`), a completely different code path
that is *only* reachable via `C`. `B` always runs the naive `f^100` in both
libraries.

Rust feature axis: `[features]` declares exactly one optional feature,
`debug-stats` (stderr diagnostics only), and no defaults, so the complete power
set is `{}` and `{debug-stats}`. Both must satisfy every row
(`tools/check_features.sh`).

## Configuration table

`R(s)` = 262144 values from the fixed-seed splitmix32 stream (full `i32` range,
negatives included) — identical in `tests/harness/mod.rs` and `tools/runner.c`.
`Rnn(s)` = the same stream masked to `0 ..= INT_MAX`, i.e. the shape `rand()`
itself produces. Every row runs through **both** `.so`s via their exported
symbols and compares the whole 1 MiB `array` byte-for-byte, plus the exact
`printf` bytes wherever `C` is involved.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | A→B | `array` as loaded (all-zero `.bss`), `k = 1`; also asserts both objects really are zero at load | [x] `bss_initial.rs` |
| 2 | A→B | all zeros, `k = 0` — no-op; array must be bit-unchanged in both | [x] `row02` |
| 3 | A→B | all elements `= 1`, `k = 1` | [x] `row03` |
| 4 | A→B | all elements `= -1`, `k = 1` | [x] `row04` |
| 5 | A→B | all elements `= INT_MAX`, `k = 1` | [x] `row05` |
| 6 | A→B | all elements `= INT_MIN`, `k = 1` | [x] `row06` |
| 7 | A→B | sentinel sweep (37 values: `0, ±1, ±2, ±3, ±6, ±7, ±8, ±14, ±2^15, ±2^16, ±2^30, INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, ±INT_MAX/3, 0x55555555, 0xAAAAAAAA, …`) tiled, `k = 1` and `k = 3` | [x] `row07`, `row07b` |
| 8 | A→B | contiguous window `array[i] = i - 131072` — every value in `-131072 ..= 131071`, `k = 1` | [x] `row08` |
| 8b | A→B | contiguous window anchored at `INT_MIN`, `k = 1` | [x] `row08b` |
| 8c | A→B | contiguous window anchored at `INT_MAX`, `k = 1` | [x] `row08c` |
| 9 | A→B | `R(1)`, `k = 1` | [x] `row09` |
| 10 | A→B | `R(2)`, `k = 2` | [x] `row10` |
| 11 | A→B | `R(3)`, `k = 3` | [x] `row11` |
| 12 | A→B | `R(4)`, `k = 5` | [x] `row12` |
| 13 | A→B | `Rnn(5)` — the `rand()` shape, `k = 7` | [x] `row13` |
| 14 | A→B | `R(6)`, `k = 20` | [x] `row14` |
| 15 | A→B | `R(7)`, `k = 81` | [x] `row15` |
| 16 | A→B | `R(8)`, `k = 82` | [x] `row16` |
| 17 | A→B | `R(9)`, `k = 83` | [x] `row17` |
| 18 | A→B | sparse: single non-zero elements at index 0 / 1 / mid / last (`INT_MIN`, `INT_MAX`, `-1`, `123456789`), rest zero, `k = 3` | [x] `row18` |
| 19 | A→B | values already **on the cycles of `f`** (taken from a post-`long_exec` image, a shape random `i32`s never reach), `k = 1` and `k = 7` | [x] `row19` |
| 20 | A→B→A→B | two separate `B` calls with a read-back in between; must agree at *both* points and equal a single `k = 2` run — proves the exported global is the shared state | [x] `row20` |
| 20b | A→B | randomized matrix: `R(11)`, `R(12)` × `k ∈ {1,2,4,9,17}`, plus `R(21)` × `k ∈ {81,82}` | [x] `randomized_matrix` |
| 20c | A→B | **exhaustive**: `array[i] = base + i` over all 16384 disjoint windows, i.e. **every one of the 2^32 `int` values**, `k = 1`, compared by whole-array checksum | [x] `tools/sweep.sh`, 0 mismatches |
| 21 | C | `seed = 0` (glibc: aliases `srand(1)`) — full 2000 iterations; compare `printf` bytes **and** final array | [x] |
| 22 | C | `seed = 1` | [x] |
| 23 | C | `seed = 2` | [x] |
| 24 | C | `seed = 3` | [x] |
| 25 | C | `seed = 7` | [x] |
| 26 | C | `seed = 42` | [x] |
| 27 | C | `seed = 12345` | [x] |
| 28 | C | `seed = 999983` (large prime) | [x] |
| 29 | C | `seed = 2147483648` (2^31, sign bit set) | [x] |
| 30 | C | `seed = 4294967295` (`UINT_MAX`), and `-1 as u32` reaching the same bit pattern | [x] |
| 30b | C | 32 further seeds: `4, 5, 6, 8, 9, 10, 11, 13, 17, 19, 23, 29, 97, 100, 128, 255, 256, 777, 1000, 4096, 31337, 54321, 65535, 88888888, 123456789, 1000003, 16777216, 2000000000, 2147483647, 3000000000, 4000000000, 4294967294` | [x] |
| 31 | C→A | after `long_exec`, read the **whole** final `array` back and compare (not just the printed XOR) | [x] every seed row |
| 32 | C→B | `long_exec(42)` then `perform_expensive_operations()` — the second op must consume the post-`long_exec` state | [x] `row32` |
| 33 | C→C→C | `long_exec(42)`, `long_exec(42)`, `long_exec(7)` — three printed lines, idempotent in the seed, final image is seed 7's | [x] `row33` |
| 34 | A→B→C | dirty the array (`R(99)`), run `B`, then `long_exec(42)` — the `rand()` fill must discard everything | [x] `row34` |
| 35 | A→B(×2000) vs C | the accelerated path cross-check: `srand(seed)` + `rand()` fill + **2000 naive `B` calls** must equal `long_exec(seed)`. Confirms `src/fast.rs` computes exactly the nested loop, on the **C** library (ground truth) and on the Rust one | [x] `tools/runner.c fill:libcrand:S pxo:2000`, `accelerated_equals_naive_through_ffi` |
| 35b | B, C | **stream parity**: the C writes only to stdout, never a byte to stderr; the default-feature Rust must match on both fds | [x] `stderr_parity` |
| 36 | all | every row above under `--no-default-features` | [x] `tools/check_features.sh` |
| 37 | all | every row above under `--features debug-stats` — stdout and the final array stay byte-identical; the feature adds stderr diagnostics only, which is its documented purpose and the one intentional difference from the C | [x] `tools/check_features.sh`, `stderr_parity` |

Rows 1–20c run in-process inside `tests/differential.rs` / `tests/bss_initial.rs`.
Rows 21–35 involve at least one C `long_exec` (2000 × 100 × 262144 ≈ 5.2·10^10
kernel applications, ~470 s of CPU each with the `-O0` build
`c_src/CMakeLists.txt` configures), so the C side is generated once out of
process by `tools/gen_reference.sh`, cached byte-for-byte under
`tests/reference/`, and compared against the Rust `.so` by
`tests/long_exec_diff.rs`. `long_exec_live_c` (`#[ignore]`d) re-derives one of
those rows live from the C `.so` in-process on demand.
