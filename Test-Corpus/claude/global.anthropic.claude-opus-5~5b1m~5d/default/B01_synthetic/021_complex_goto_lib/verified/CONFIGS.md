# CONFIGS.md — Phase B configuration surface table

## Axes derived from the C source

`c_src/src/driver.c` has no options, flags, modes, `#ifdef`s, enums, formats or
byte-order handling — the entire configuration surface is the value pair
`(x, y)` and the five conditions the code branches on:

| axis | source line | branch |
|------|-------------|--------|
| **G** loop guard | `while (x > 0 \|\| y > 0)` | enter body vs return immediately (4 sign quadrants) |
| **S** special skip | `if (x == 1 && y == 4) goto label2;` | skip the `label1` block for the first pass of an iteration. Only reachable on **entry** (see note) |
| **X** x-decrement | `if (x > 0) { printf("x\n"); x--; }` | emit `"x\n"` / decrement, or not |
| **Y** y-zero | `if (y == 0) continue;` | jump back to the loop guard (skipping `"y\n"`/`y--`/the `x<3` test) |
| **B** back-goto | `if (x < 3) goto label1;` | backwards jump inside the same iteration (inner loop) vs fall through to the guard |

Note on **S**: the check runs only at the top of a `while` iteration. An
iteration is (re-)entered either by falling off the end of the body (requires
`x >= 3`, so `x == 1` is impossible) or by `continue` (requires `y == 0`, so
`y == 4` is impossible). Therefore `goto label2` fires **only on the initial
call** with exactly `(1, 4)` — row 5 is the sole configuration that exercises it.

**Public entry points:** exactly one — `driver` (the lowest-level and only
symbol; there is no convenience wrapper layer). Every row calls it through the
`.so` export of both libraries and compares the stdout bytes.

**Excluded, non-executable region** (see `ERRORS.md` rows 14-15): `x > 0 && y < 0`
(C-side signed-overflow UB / unbounded output) and `|x|,|y|` near `INT_MAX` with
both arguments positive (~2^32 lines). All rows below stay outside those.

## Configuration rows

Randomized rows use a fixed-seed xorshift PRNG (`SEED = 0x2026_0828_D21F_11A3`)
and the stated number of samples; each sample is compared byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `driver` | **G**=false: `x <= 0 && y <= 0`, randomized over `[-3000,0]²` plus `INT_MIN` corners — immediate return, empty output (128 samples) | [x] |
| 2  | `driver` | **Y** always taken: `y == 0, x > 0` random `x ∈ [1,3000]` — `"loop\nx\n"` repeated, `continue` every iteration (96 samples) | [x] |
| 3  | `driver` | **X** never taken: `x == 0, y > 0` random `y ∈ [1,3000]` — `"loop\n"` once then only `"y\n"`, **B** always true (96 samples) | [x] |
| 4  | `driver` | **X** never taken, negative `x`: `x ∈ [-3000,-1], y ∈ [1,3000]` — `x < 3` always true, `x` never decremented (96 samples) | [x] |
| 5  | `driver` | **S**: exactly `(x, y) = (1, 4)` — the only input taking `goto label2` (single deterministic case, plus asserted against neighbours) | [x] |
| 6  | `driver` | **S** near-miss on `y`: `x == 1`, `y ∈ {0,1,2,3,5,6,7,8}` — must *not* skip `label1` | [x] |
| 7  | `driver` | **S** near-miss on `x`: `y == 4`, `x ∈ {-1,0,2,3,4,5}` — must *not* skip `label1` | [x] |
| 8  | `driver` | **B** true path with a decrementing `x`: `x == 2`, `y ∈ [1,3000]` random (64 samples) | [x] |
| 9  | `driver` | **B** boundary: `x == 3` (`x < 3` false ⇒ fall through to guard), `y ∈ [1,3000]` random (64 samples) | [x] |
| 10 | `driver` | **B** mixed: `x ∈ [4,60]`, `y ∈ [1,60]` random — iterations start by falling through, later switch to the back-goto once `x` drops below 3 (256 samples) | [x] |
| 11 | `driver` | large `x`, small `y`: `x ∈ [500,20000]`, `y ∈ [0,5]` (24 samples) | [x] |
| 12 | `driver` | small `x`, large `y`: `x ∈ [0,5]`, `y ∈ [500,20000]` (24 samples) | [x] |
| 13 | `driver` | both large: `x, y ∈ [200,4000]` random (24 samples) | [x] |
| 14 | `driver` | ordering relations: `y > x`, `y == x`, `y < x` with `x, y ∈ [1,400]` random, 3 sub-shapes (3 × 64 samples) | [x] |
| 15 | `driver` | exhaustive small grid: every `(x, y) ∈ [-4,12]²` except the excluded `x > 0 && y < 0` region (289 − 48 = 241 cases) | [x] |
| 15b | `driver` | near-exhaustive medium grid: every `(x, y) ∈ [-6,40]²` outside the UB region (2209 − 240 = 1969 cases) — covers every guard × skip × continue × back-goto interaction for small magnitudes instead of sampling it | [x] |
| 16 | `driver` | broad randomized sweep over `[-2000,2000]²` minus the excluded region — value-dependent property test (768 samples) | [x] |
| 17 | `driver` | integer extremes that still terminate: `x = INT_MIN` with `y ∈ [0,10]`; `y = INT_MIN`/`INT_MAX` with `x <= 0`; `x = INT_MAX` with `y <= 0`… plus `0x5A5A5A5A`-style arbitrary bit patterns paired with a non-positive partner | [x] |
| 18 | `driver` | statelessness / repeated invocation: a fixed sequence of 40 randomized calls issued back-to-back into one capture, so any residual state or output-buffer difference between the two libraries would show up (2 sequences) | [x] |
| 19 | `driver` | interleaved libraries: alternate C call, Rust call, C call… within a single sequence to confirm neither library perturbs the other's shared `stdout` buffering | [x] |

All 20 rows checked off — see `translation/tests/differential.rs`
(`configs_row01_*` … `configs_row19_*`), passing under the default (only)
feature combination.

## Harness notes

* Every call in every row goes through `dlsym("driver")` on **both** `.so`s
  (`libloading`); the Rust crate is never called directly, so the
  `#[no_mangle] extern "C"` export is exercised too.
* Output is compared by redirecting file descriptor 1 to a temporary file around
  each call, so it captures the exact bytes libc writes for both libraries.
  Because that redirect is process-global, `.cargo/config.toml` pins
  `RUST_TEST_THREADS = "1"` and the harness refuses to run otherwise.
* `cargo test` does **not** rebuild a `cdylib`-only lib target, so the harness
  builds it itself into `target/so-under-test` and asserts the artifact is newer
  than `src/lib.rs` — otherwise the suite would silently verify a stale `.so`.
* Harness sensitivity was validated by mutation testing: six semantic mutations
  of `src/lib.rs` (`y == 4`→`y == 5`, `||`→`&&` in the guard, `x < 3`→`x <= 3`,
  dropping the `skip_label1` reset, `continue`→`break`, `x > 0`→`x >= 0`) were
  each detected by multiple rows.
