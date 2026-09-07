# CONFIGS.md — Phase B configuration-surface table

## Axes mechanically derived from the C source

`c_src/src/long.c` + `c_src/include/long.h` (68 + 29 lines, one translation unit).

**Runtime options / modes / flags:** NONE. There is no option struct, no global
mode variable, no `#ifdef` in the source (`grep -c '#if' c_src/src/long.c` → 0
other than the header guard), no `switch`, and only counted `for` loops (no
`if` at all). The single scalar parameter is `unsigned int seed`.

**Compile-time configuration:** `#define ARRAY_SIZE (256 * 1024)`,
`#define ITERATIONS 2000`, inner loop bound `100`. Not settable by a caller.

**Cargo features:** `translation/Cargo.toml` declares **no `[features]`
section**, so there is exactly ONE feature combination (`--no-default-features`
is equivalent to the default). Verified by
`grep -n 'features' translation/Cargo.toml` → no match.

**Public entry points (full set, lowest level first):**

1. `array` — exported global `int[262144]`; the *state* both functions read and
   write. Callers can read and write it directly through the `.so`, so its
   contents are an input axis in their own right.
2. `perform_expensive_operations()` — the lowest-level function; one in-place
   pass over `array`. Not in the header, but exported, so a real consumer can
   drive it.
3. `long_exec(unsigned int seed)` — the convenience/one-shot wrapper:
   `srand(seed)` → fill `array` from `rand()` → 2000x
   `perform_expensive_operations()` → xor-reduce → `printf("%d\n", ...)`.

**Input SHAPE axes the code distinguishes** (all of them are element-value
shapes of `array`, since the length and count are compile-time constants):

* sign of the element (drives `x >> 3` arithmetic-shift and the sign of `x % 7`)
* signed-overflow edges (`INT_MIN`, `INT_MAX`, values near `±2^31/3`)
* multiples of 7 and 2 (drives `x % 7 == 0` and `x / 2` exactness)
* zero (fixed point of the whole transform)
* uniform vs. varying-per-element content (a uniform array cannot detect an
  indexing / lane-shuffling bug; the Rust code processes 8 lanes at a time, so
  *per-index-distinct* content is mandatory)
* alignment/position within the 8-wide vector chunk and the chunk remainder
  (`ARRAY_SIZE % 8 == 0`, so the remainder path is dead — still probed)
* the whole 2^32 value domain (exhaustively, in contiguous blocks)

**Output axes:** the mutated `array` (1 MiB compared byte-for-byte) and, for
`long_exec`, the single `printf`ed line on stdout.

## Table

One row per meaningful combination the C actually treats differently.
"randomized" = many inputs per row from a fixed-seed PRNG (seed 0x5EED1234).

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `array` (read) | freshly-loaded `.so`: `array` must be all zeros in `.bss`, size `0x100000`, in both libs | [x] |
| 2  | `array` (write/read round trip) | write a randomized 1 MiB pattern through the exported symbol and read it back; C and Rust expose the same 262144-element `int` window | [x] |
| 3  | `perform_expensive_operations` | uniform `array`: all zeros | [x] |
| 4  | `perform_expensive_operations` | uniform `array`: all `1`, all `-1`, all `2`, all `-2`, all `7`, all `-7`, all `6`, all `8` (small values, both signs, around the `%7` and `/2` boundaries) | [x] |
| 5  | `perform_expensive_operations` | uniform `array`: `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `0x80000000` bit patterns (overflow edges) | [x] |
| 6  | `perform_expensive_operations` | per-index-distinct ramp `array[i] = i` (detects lane/index shuffling in the 8-wide Rust path; also sweeps a contiguous positive block) | [x] |
| 7  | `perform_expensive_operations` | per-index-distinct descending ramp `array[i] = -(int)i` (negative contiguous block) | [x] |
| 8  | `perform_expensive_operations` | per-index-distinct ramp anchored at `INT_MIN` (`array[i] = INT_MIN + i`) — overflow edge *and* distinct per lane | [x] |
| 9  | `perform_expensive_operations` | per-index-distinct ramp anchored so the block straddles `INT_MAX` → `INT_MIN` wrap (`array[i] = INT_MAX - 131072 + i`) | [x] |
| 10 | `perform_expensive_operations` | randomized full-range `array` (`i32` uniform over all 2^32), many independent fillings | [x] |
| 11 | `perform_expensive_operations` | randomized array biased to "hard" values: mixture of `0`, `±1`, `INT_MIN`, `INT_MAX`, multiples of 7, powers of two, and full-range randoms — interaction of shapes *within one vector chunk* (lane-mixed) | [x] |
| 12 | `perform_expensive_operations` | sparse shape: one non-zero element at index `k` for `k` in {0,1,7,8,9,262143, random} — probes chunk boundaries (`k % 8 == 0/7`) and the final chunk | [x] |
| 13 | `perform_expensive_operations` | chunk-remainder shape: only the last `1..=7` elements non-zero (`ARRAY_SIZE % 8 == 0` so the Rust remainder path is dead — confirms it stays dead and correct) | [x] |
| 14 | `perform_expensive_operations` | repeated invocation: 25 back-to-back calls on the same dirty global state, compared after **every** call (composed pipeline, not a single call) | [x] |
| 15 | `perform_expensive_operations` | exhaustive sweep of the 32-bit element domain in contiguous 262144-element blocks, at randomized block offsets covering all four quadrants of the `i32` range | [x] |
| 16 | `perform_expensive_operations` | interleaved with direct `array` writes between calls (consumer-driven state mutation: run, patch some indices, run again) | [x] |
| 17 | `long_exec` | full end-to-end one-shot, `seed = 0` — stdout line compared byte-for-byte, and post-call `array` compared byte-for-byte | [x] |
| 18 | `long_exec` | full end-to-end one-shot, randomized seed(s) (e.g. `42`) — stdout + final `array` | [x] |
| 19 | `long_exec` | full end-to-end, `seed = UINT_MAX` / `seed = 0x80000000` (unsigned-domain edges) | [x] |
| 20 | `long_exec` | `long_exec` called after `array` was deliberately dirtied by the caller (must be seed-determined only, `srand`+`rand` overwrite every element) | [x] |
| 21 | `long_exec` / `srand`+`rand` prefix only | the `srand(seed)` → 262144x `rand()` fill step in isolation, compared for many randomized seeds (isolates the libc PRNG use from the 2000-iteration transform, so the expensive row above needs only a few seeds) | [x] |
| 22 | binary driver | project builds **no** binary/executable target (`[lib] crate-type = ["cdylib"]` only; CMake builds only `add_library(long SHARED ...)`) — nothing to compare; N/A | [x] |

## Cost note

`long_exec` is `2000 * 262144 * 100 ≈ 5.2e10` inner steps: ~470 s in the C
build and ~56 s in the Rust build per call. Rows 17-20 are therefore run in a
dedicated long-running harness (`tests/long_exec_full.rs`, `--ignored`, one
seed per invocation) while rows 1-16 and 21 — which cover exactly the same
per-element arithmetic and the same PRNG fill — run in the fast default suite.

## Phase B result (all rows verified)

Every row above passed with randomized inputs (fixed seed `0x5EED1234`), all
calls made through `dlsym` on both `.so`s:

* `tests/differential.rs` — rows 1-16 and 21, 19 tests, **19 passed / 0 failed**
  (~19 s). Roughly 100 full-array (262144-element) differential passes per run.
* `tests/differential.rs::sweep_wide_random` (`--ignored`) — 200 extra
  randomized full-array rounds, ~52 M distinct element values, **passed** (54 s).
* `tests/long_exec_full.rs` — rows 17-20 end-to-end, 9 recorded seeds
  (`0, 1, 42, 42+poisoned array, 7, 7 via junk 64-bit ABI, INT_MAX,
  0x80000000, UINT_MAX`). For every seed the final 1 MiB `array` is byte-identical
  and the captured stdout is byte-identical:

  | seed | printed line (C == Rust) |
  |------|--------------------------|
  | 0 | `42032659` |
  | 1 | `42032659` (glibc `srand(0)` == `srand(1)`; faithfully reproduced) |
  | 42 | `430392287` |
  | 42, array pre-poisoned | `430392287` (identical to clean run) |
  | 7 | `72337063` |
  | 7 via `0xDEADBEEF_00000007` | `72337063` (high bits ignored, both) |
  | `INT_MAX` | `62428010` |
  | `0x80000000` | `269448949` |
  | `UINT_MAX` | `494145113` |

* Row 22: confirmed there is no binary/executable target in either build, so
  there is no stdout-of-a-driver comparison to make. The stdout of the *library*
  is nevertheless captured and compared (fd-1 redirection) as shown above.

### Harness notes (both were real bugs found while testing, in the harness)

1. `dlopen` refcounts by (path, inode), so loading the same `.so` twice in one
   process returns the SAME mapping and the single global `array` is shared.
   Each `Impl` therefore loads a private copy of the `.so`.
2. glibc `srand`/`rand` is one process-global state shared by both `.so`s, so
   two `long_exec` calls must never overlap. Each recorder process performs
   exactly one `long_exec`; C and Rust sides are compared across processes via
   `target/long_cache/`.

Reproduce with `./run_all.sh` (fast, all feature combos) and
`./record_long_exec.sh` (the ~8-minute `long_exec` recordings).
