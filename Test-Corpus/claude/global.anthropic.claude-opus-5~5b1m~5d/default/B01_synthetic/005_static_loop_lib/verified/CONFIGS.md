# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

## Axes the C code actually branches on

Derived from `c_src/include/staticloop.h` (the full public API) and every
branch/loop in `c_src/src/staticloop.c`:

**Public entry points (complete set, 2 of 2):**

| entry point | signature | level |
|-------------|-----------|-------|
| `static_sum` | `int static_sum(int update)` | **lowest level** — mutates the function-scope `static int sum` and returns it |
| `driver`     | `void driver(int stride)`     | convenience wrapper — loops `i = 0..9`, calls `static_sum(i * stride)`, `printf("%d\n", ...)` each iteration |

**Runtime options / modes / flags:** none. There is no init function, no
options struct, no global config, no `#ifdef`-selected behaviour, no
environment variable read. The only "configuration" the library carries is its
**hidden persistent state**: the function-scope `static int sum`, initialised
to `0` once per loaded library image.

**Therefore the real configuration axes are:**

- **A. Entry point:** `static_sum` alone / `driver` alone / the two interleaved.
- **B. Hidden-state configuration (the accumulator's value on entry):** fresh
  (`sum == 0`) / small positive / small negative / `INT_MAX` / `INT_MIN` /
  arbitrary random. This is the axis a naive test misses entirely, because
  every result of both functions is a function of it.
- **C. Call multiplicity ("empty / one / many"):** 0 calls, 1 call, 2 calls,
  many (hundreds/thousands) calls — because state accumulates, the *sequence*
  is the input, not the scalar.
- **D. Input value shape for `update` / `stride`:** `0` / `+1` / `-1` / small
  positive / small negative / powers of two / `INT_MAX` / `INT_MIN` /
  `INT_MAX-1` / `INT_MIN+1` / random full-domain `i32`.
- **E. Overflow regime** (the only value-dependent behaviour in the code):
  no overflow / `i * stride` (the multiply in `driver`) overflows /
  `sum += update` (the add) overflows / both overflow.
- **F. Observation channel:** the `int` return value of `static_sum` /
  the `printf`-produced **stdout bytes** of `driver` (10 lines,
  `"%d\n"` each). Both must be compared byte-for-byte.

`driver`'s loop bound is the constant `10`, so iteration count is not an axis;
however the loop makes `driver` a *composed pipeline* over `static_sum`, which
is why rows 12–20 drive `driver` from non-fresh state rather than only fresh.

Each test acquires a **freshly `dlopen`ed private copy** of each `.so` (copied
to a unique temp path so glibc gives it a distinct image with a re-initialised
`sum`), so row-level state configurations are exact and independent.

## Rows (cross-product, pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `static_sum` | fresh state, **0 calls** — assert the fresh accumulator by observing that the first call returns exactly its own argument | [x] |
| 2 | `static_sum` | fresh state, **1 call**, `update = 0` (degenerate) | [x] |
| 3 | `static_sum` | fresh state, **1 call**, randomized small `update` ∈ [-1000, 1000], 512 seeded samples (fresh instance each) | [x] |
| 4 | `static_sum` | fresh state, **1 call**, randomized full-domain `update: i32`, 512 seeded samples (fresh instance each) | [x] |
| 5 | `static_sum` | fresh state, **2 calls**, both randomized full-domain — first accumulation step | [x] |
| 6 | `static_sum` | fresh state, **many calls** (1024), randomized small values, no overflow regime | [x] |
| 7 | `static_sum` | fresh state, **many calls** (4096), randomized full-domain values — add-overflow regime hit repeatedly | [x] |
| 8 | `static_sum` | fresh state, **many calls**, all-zero sequence (identity/no-op sequence) | [x] |
| 9 | `static_sum` | fresh state, **many calls**, monotone `+1` sequence then monotone `-1` sequence (walks state up and back to 0) | [x] |
| 10 | `static_sum` | fresh state, sequence of the boundary set `{0, ±1, ±2^k, ±(2^k-1), INT_MAX, INT_MIN}` in seeded random order | [x] |
| 11 | `static_sum` | **pre-seeded extreme state** (`INT_MAX`, then `INT_MIN`) followed by randomized full-domain updates | [x] |
| 12 | `driver` | fresh state, `stride = 0` (no-op stride, no overflow) — 10 stdout lines | [x] |
| 13 | `driver` | fresh state, `stride = 1` and `stride = -1` (minimal ± strides) | [x] |
| 14 | `driver` | fresh state, randomized small `stride` ∈ [-1000, 1000], 256 seeded samples, fresh instance each — no-overflow regime | [x] |
| 15 | `driver` | fresh state, randomized full-domain `stride: i32`, 256 seeded samples, fresh instance each — multiply-overflow **and** add-overflow regimes | [x] |
| 16 | `driver` | fresh state, `stride` from the boundary set `{INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, ±2^k for k=0..31}` — multiply-overflow regime | [x] |
| 17 | `driver` | fresh state, `stride = 0x2000_0000` — multiply does **not** overflow for small `i` but the accumulated `sum` **does** mid-loop (add-overflow-only regime) | [x] |
| 18 | `driver` | **`driver` called repeatedly** (8 consecutive calls) on one instance with a randomized stride — 80 stdout lines, state carried across calls | [x] |
| 19 | `driver` | **non-fresh state**: N randomized `static_sum` calls first, then `driver(stride)` — composed pipeline over dirty state | [x] |
| 20 | `static_sum` + `driver` **interleaved** | randomized script of 256 operations, each randomly either `static_sum(v)` (return value compared) or `driver(v)` (stdout bytes compared), on one shared instance — full pipeline, both channels, shared hidden state | [x] |
| 21 | binary executable | **N/A** — `c_src/CMakeLists.txt` builds only `add_library(StaticLoop SHARED ...)`; there is no `add_executable`, so no driver binary stdout to compare. Asserted in `configs::row21_no_binary_target`. | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the default build is the
only configuration; `check_all_features.sh` enumerates and re-runs the suite
for every combination that exists (namely: default, and `--no-default-features`
which is identical).
