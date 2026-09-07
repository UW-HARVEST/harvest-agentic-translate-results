# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the complete C source. The library is one function:

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) {
        printf("%d %d\n", i, j);
    }
}
```

## Axes the C code actually branches on

Enumerated by grepping the public header and every `if` / `switch` / `#ifdef`
/ loop guard in the C sources:

| axis | values the C distinguishes | evidence |
|------|---------------------------|----------|
| A. runtime options / modes / flags | **none** — no globals, no setters, no option struct, no env lookup, no `#ifdef` other than the header include guard | `grep -nE 'static\|extern\|getenv\|#if' c_src/src/driver.c` finds nothing |
| B. sign of `x` (the only branch) | `x <= 0` → 0 iterations; `x > 0` → `x` iterations | loop guard `i < x` |
| C. iteration count / input shape | 0 (empty), 1 (one), 2, small many, larger many | `i < x` is the only shape driver |
| D. printed field width of `i` | 1-digit, 2-digit, 3-digit, 4+-digit — changes the byte layout of `"%d %d\n"` | `printf("%d %d\n", i, j)` |
| E. printed field width of `j = 2*i` | `j` crosses each decimal decade one step ahead of `i` (e.g. `i=5 → j=10`, `i=50 → j=100`), so `j`'s width differs from `i`'s within the same run | `j += 2` |
| F. output volume vs. stdio buffering | output smaller than one `stdout` buffer vs. output spanning many buffer flushes (multi-KB) | both sides call the same libc `printf`; buffering is observable in ordering/flush behaviour |
| G. call sequencing / statefulness | single call vs. many successive calls (no state is carried; must be identical) | no `static` storage exists |
| H. entry points | **one**: `driver` — the lowest-level and only public symbol; there is no convenience wrapper and no lower layer | `driver.h` declares exactly one function |

Axes A (options) and H (multiple entry points) are empty/singleton by
construction, so the configuration cross-product reduces to B × C × D × E × F × G
over the `int` domain of `x`.

## Untestable boundary (documented, intentionally not a row)

`j = 2*i` overflows `int` once `i > INT_MAX/2` (`x > 1073741823`). In C that is
signed-overflow UB; reaching it requires >10^9 `printf` calls, so it cannot be
executed within the task's time bounds. The Rust translation uses
`wrapping_add`, matching the wrap-around that the C compiler emits in practice.
No row asserts on it.

## Configuration-surface table

Every row is exercised by a differential test that loads **both** `.so` files
via `libloading`, calls `driver` through the FFI export, captures the raw
`stdout` bytes of each, and asserts they are byte-identical. Rows marked
"randomized" use many seeded-random values (fixed seed → reproducible), not a
single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `driver` | B:`x<=0` / C:empty — `x == 0`, zero iterations, empty output | `cfg_row01_zero_iterations` | [x] |
| 2 | `driver` | B:`x<=0` / C:empty — randomized negative `x` over `INT_MIN..0`, zero iterations | `cfg_row02_random_negative` | [x] |
| 3 | `driver` | B:`x>0` / C:one — `x == 1`, single line, D:1-digit `i`, E:1-digit `j` | `cfg_row03_single_iteration` | [x] |
| 4 | `driver` | B:`x>0` / C:two — `x == 2`, D:1-digit, E:1-digit | `cfg_row04_two_iterations` | [x] |
| 5 | `driver` | B:`x>0` / C:many, D:1-digit only, E:`j` crosses into 2 digits (`x == 6` ⇒ `j` reaches 10) | `cfg_row05_j_width_crosses_first` | [x] |
| 6 | `driver` | B:`x>0` / C:many, D:`i` crosses 1→2 digits, E:`j` crosses 1→2→3 digits (`x == 60`) | `cfg_row06_i_and_j_cross_decades` | [x] |
| 7 | `driver` | B:`x>0` / C:many, D:`i` reaches 3 digits, E:`j` reaches 4 digits (`x == 600`) | `cfg_row07_three_and_four_digits` | [x] |
| 8 | `driver` | B:`x>0` / C:many, D/E: exact decade boundaries `x ∈ {5,6,9,10,11,49,50,51,99,100,101,499,500,501,999,1000,1001,4999,5000,5001}` | `cfg_row08_decade_boundaries` | [x] |
| 9 | `driver` | B:`x>0` / C:many — randomized `x` in `1..=64` (small counts, all widths ≤ 3) | `cfg_row09_random_small` | [x] |
| 10 | `driver` | B:`x>0` / C:many — randomized `x` in `1..=5000`, F:output spans many stdio buffer flushes (tens of KB) | `cfg_row10_random_large_buffered` | [x] |
| 11 | `driver` | G:call sequencing — randomized *sequence* of calls mixing `x<=0` and `x>0`, all captured in one stdout stream (checks no residual state and identical flush ordering) | `cfg_row11_random_call_sequences` | [x] |
| 12 | `driver` | F:largest bounded volume — `x == 20000` (≈200 KB of output, many buffer refills) | `cfg_row12_large_volume` | [x] |
| 13 | `driver` | B/D/E: full-range randomized `x` over the entire `c_int` domain, clamped to a bounded number of iterations for positives (mixes accepted and zero-iteration inputs drawn from the real input space) | `cfg_row13_random_full_int_domain` | [x] |

## Binary executable

`c_src/CMakeLists.txt` defines only `add_library(driver SHARED src/driver.c)`;
there is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no binary
driver**, so the "compare C and Rust binary stdout" gate is not applicable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one (equivalently
`--no-default-features`). Both were exercised; see the run log in the final
report.
