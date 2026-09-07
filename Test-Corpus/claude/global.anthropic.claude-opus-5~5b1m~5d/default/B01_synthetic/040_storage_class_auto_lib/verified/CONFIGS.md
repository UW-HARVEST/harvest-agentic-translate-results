# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source. The complete public surface is one
function with one scalar parameter:

```c
void driver(int x);            /* c_src/include/driver.h */

void driver(int x) {           /* c_src/src/driver.c    */
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

## Axes the C actually branches on

| axis | values the C distinguishes | evidence |
|---|---|---|
| runtime options / modes / flags | **none** | no global state, no setter, no option struct, no env var read, no `if`/`switch` anywhere in `c_src` |
| conditional compilation | **none** | only the `DRIVER_H_` include guard |
| public entry points | exactly **one**: `driver` (this *is* the lowest-level entry point — there is no convenience wrapper layered over anything) | `nm -D --defined-only` shows one `T` symbol; header declares one function |
| input element type / width / byte order | fixed: one `int` (32-bit, by value, in `edi` per SysV AMD64) | signature |
| input count / shape | one scalar per call; the only shape axis left is the **value** of `x`, and the number/order of successive calls | signature |

So the cross-product reduces to: *value class of `x`* × *call multiplicity*.
The value classes below are exactly the ones the two arithmetic steps
(`2*x`, then `+= 300`) treat differently with respect to sign and 32-bit
wraparound; `printf("%d\n", …)` additionally distinguishes negative (emits a
`-`), zero, and multi-digit widths, so digit-count classes are enumerated too.

## Configuration table

Every row is exercised against **both** `.so`s through `libloading`, with many
randomized inputs per row (LCG seeded with a fixed constant, so runs are
reproducible), and the captured stdout bytes are compared byte-for-byte.
(`driver` returns `void`, so the printed bytes are the complete observable
result.) `[x]` = passing.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x = 0` — the single value making `y` exactly the additive constant `300` | [x] |
| 2 | `driver` | small positive `x` in `1..=1000`, exhaustive — `2*x` and `2*x+300` both in range, result positive | [x] |
| 3 | `driver` | small negative `x` in `-1000..=-1`, exhaustive — spans the sign flip of the result (`2*x+300` crosses 0 near `x = -150`) | [x] |
| 4 | `driver` | `x = -150` exactly (`y == 0`, prints `0`) and `x = -149`/`-151` (one either side: smallest positive and largest negative result) | [x] |
| 5 | `driver` | randomized mid-magnitude positive `x` in `1001..=2^29`, no overflow, result 4–9 digits | [x] |
| 6 | `driver` | randomized mid-magnitude negative `x` in `-2^29..=-1001`, no overflow, negative result (`-` sign path) | [x] |
| 7 | `driver` | `x` where `2*x` fits but `2*x + 300` overflows: `x ∈ [1073741524, 1073741823]`, exhaustive — add-only wraparound | [x] |
| 8 | `driver` | `x` at the multiply-overflow boundary, positive side: `INT_MAX/2 = 1073741823` and `1073741824` (first `x` whose `2*x` overflows), plus randomized `x ∈ [2^30, INT_MAX]` | [x] |
| 9 | `driver` | `x` at the multiply-overflow boundary, negative side: `INT_MIN/2 = -1073741824` and `-1073741825`, plus randomized `x ∈ [INT_MIN, -2^30]` | [x] |
| 10 | `driver` | extremes: `x = INT_MAX`, `x = INT_MIN`, `x = INT_MAX - 1`, `x = INT_MIN + 1`, `x = 1`, `x = -1` | [x] |
| 11 | `driver` | digit-width sweep: `x` chosen so the printed field is 1, 2, 3, …, 10 characters wide, and both `300`-adjacent and power-of-ten-adjacent results (`y = 9/10`, `99/100`, …) | [x] |
| 12 | `driver` | uniformly random `x` over the **full** `i32` range, 20 000 values (mixes all of the above classes, catching value-dependent bugs) | [x] |
| 13 | `driver` | call multiplicity / ordering: many successive calls to the same `.so` (state leakage), and C-then-Rust interleaved per input within one process (stdout buffering divergence) | [x] |
| 14 | `driver` | both `.so`s loaded simultaneously in one process, sharing one libc `stdout` — verifies the Rust export does not disturb the C library's buffered stream | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the default build is the
only configuration; the suite is nonetheless run under both the default build
and `--no-default-features` to satisfy the gate.

## Binary driver

`c_src/CMakeLists.txt` builds only a shared library (no `add_executable`, no
`main()`), so there is no binary whose stdout could be diffed. The equivalent
stdout comparison is performed through the FFI boundary in every row above,
since printing *is* this function's entire observable behavior.

## Additional coverage beyond the table

| test | coverage |
|---|---|
| `phase_b_heavy_stride_sweep` (`#[ignore]`d; run with `-- --ignored`) | every 4093rd value across the **entire** `i32` range — 1 049 385 inputs, all byte-identical |

## How to reproduce

```sh
# 1. build the C shared library
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. run the differential suite (both profiles, all feature combos)
cd translation
cargo test                      # debug
cargo test --release            # release (panic = "abort")
cargo test --no-default-features
cargo test --all-features
cargo test --release -- --ignored   # the 1M-input heavy sweep
```

`.cargo/config.toml` pins `RUST_TEST_THREADS = 1`: the tests capture the
process's fd 1 to compare the bytes `printf` emits, so the harness must not
write to stdout concurrently from another test thread.
