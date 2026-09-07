# CONFIGS.md — Phase B configuration surface table

## Axes derived from the C source

The whole library is `c_src/src/driver.c` (46 lines). Enumerating what the code
actually branches on:

**Runtime options / modes / flags:** *none*. There is no init function, no
context struct, no global state, no setter, no mode enum, and no `#ifdef` in
either `driver.c` or `driver.h`. The only "configuration" a caller can express
is the arguments themselves. (`grep -c '#if' c_src/src/driver.c` → the only
preprocessor conditional in the tree is the `DRIVER_H_` include guard.)

**Public entry points (both, including the low-level one):**

* `fma_array(out, mul1, mul2, add, len)` — the low-level primitive. Exported
  (non-`static`) even though it is absent from `driver.h`, so it is part of the
  ABI and is driven **directly**, not only through `driver`.
* `driver(data, len)` — the one-shot convenience wrapper: VLA copy →
  `inner` → `fma_array` over four aliased pointers → `printf("%d\n")` per element.
* `inner` is `static`; it is exercised transitively via `driver` (there is no way
  for an external caller to reach it otherwise, and it is absent from both
  `.so` symbol tables).

**Input shapes the code distinguishes:**

* *length*: `len` drives both loop guards → shapes `0` / `1` / `2` / small (3–8)
  / many (hundreds) / a page-crossing size. Non-positive lengths are in ERRORS.md.
* *aliasing of the four `fma_array` pointers*: the C writes `out[i]` inside the
  loop and reads `mul1[i]`,`mul2[i]`,`add[i]` in the same iteration, so aliasing
  changes the result. Distinct configurations: all four disjoint; `out` disjoint
  but `mul1==mul2`; `out==mul1` (write-then-read alias); `out==add`;
  all four identical (`inner`'s call). Each is a genuinely different code path
  through the same loop body.
* *element value classes*: zeros; small positives; negatives; mixed sign;
  `INT_MAX`/`INT_MIN` extremes; values at the `int` multiply-overflow boundary
  (`46340`/`46341`, `65536`); full-range random `i32`. These change whether the
  multiply and/or the add wrap.
* *stdout shape for `driver`*: `%d` formatting of negative numbers, `INT_MIN`
  (`-2147483648`, the value with no positive counterpart), and multi-line output
  ordering / buffering.

## Table — one row per combination the C treats differently

Every row is driven with **many randomized inputs** (`SplitMix64`, fixed seed
`0x243F_6A88_85A3_08D3`), not a single hand-picked value, and asserted
byte-for-byte between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `fma_array` | 4 disjoint buffers, `len==1`, random full-range `i32` values | [x] |
| 2 | `fma_array` | 4 disjoint buffers, `len==2`, random full-range `i32` | [x] |
| 3 | `fma_array` | 4 disjoint buffers, `len` random in 3..=8, random full-range `i32` | [x] |
| 4 | `fma_array` | 4 disjoint buffers, `len` random in 64..=512 ("many"), random full-range `i32` | [x] |
| 5 | `fma_array` | 4 disjoint buffers, `len==1024` (page-crossing), random full-range `i32` | [x] |
| 6 | `fma_array` | 4 disjoint buffers, all elements `0` | [x] |
| 7 | `fma_array` | 4 disjoint buffers, small positive values only (no overflow possible) | [x] |
| 8 | `fma_array` | 4 disjoint buffers, small negative values only | [x] |
| 9 | `fma_array` | 4 disjoint buffers, values drawn from the extremes set `{INT_MIN, INT_MIN+1, -1, 0, 1, 46340, 46341, 65535, 65536, INT_MAX-1, INT_MAX}` (multiply/add overflow boundary) | [x] |
| 10 | `fma_array` | `mul1 == mul2` (squaring), `out`/`add` disjoint, random values | [x] |
| 11 | `fma_array` | `out == mul1` (in-place, write observed by nothing later but changes read order), others disjoint, random values | [x] |
| 12 | `fma_array` | `out == add`, `mul1`/`mul2` disjoint, random values | [x] |
| 13 | `fma_array` | `out == mul1 == mul2 == add` (all four aliased — exactly the call `inner` makes), random values, random `len` | [x] |
| 14 | `fma_array` | `out == mul1 == mul2 == add`, extremes value set (aliased **and** overflowing) | [x] |
| 15 | `driver` | `len==1`, random full-range `i32` → stdout compared byte-for-byte | [x] |
| 16 | `driver` | `len==2`, random full-range `i32` → stdout | [x] |
| 17 | `driver` | `len` random in 3..=8, random full-range `i32` → stdout | [x] |
| 18 | `driver` | `len` random in 64..=512, random full-range `i32` → stdout | [x] |
| 19 | `driver` | `len==1024`, random full-range `i32` → stdout | [x] |
| 20 | `driver` | all elements `0` → stdout (`"0\n"` repeated) | [x] |
| 21 | `driver` | small positive values only → stdout | [x] |
| 22 | `driver` | negative values only → stdout (`%d` sign formatting) | [x] |
| 23 | `driver` | extremes value set incl. `INT_MIN`/`INT_MAX` → stdout (wrapped `%d` output) | [x] |
| 24 | `driver` + `fma_array` | **cross-check of the composed pipeline**: `driver`'s printed lines must equal the values `fma_array(buf,buf,buf,buf,len)` leaves in `buf`, for random `len`/values, in both libraries | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
and `translation/Cargo.toml` declares only `crate-type = ["cdylib"]` with no
`[[bin]]` / `src/main.rs`. **Neither side builds a binary driver**, so the
"compare the two executables' stdout" clause has no subject. `driver`'s stdout is
nevertheless compared byte-for-byte in rows 15–24 by capturing fd 1 around each
FFI call.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table → one combination only
(default == `--no-default-features`). Rows 1–24 and all ERRORS.md rows are run
under both spellings.
