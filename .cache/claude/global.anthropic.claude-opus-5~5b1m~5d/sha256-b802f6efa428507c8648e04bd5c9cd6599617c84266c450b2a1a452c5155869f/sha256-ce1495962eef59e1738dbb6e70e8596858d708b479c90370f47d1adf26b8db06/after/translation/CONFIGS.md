# CONFIGS.md — Configuration surface table (valid inputs)

## Mechanical derivation of the axes

### Public entry points (the FULL set, incl. the lowest level)

| symbol | level | declared in | notes |
|--------|-------|-------------|-------|
| `fma_array(out, mul1, mul2, add, len)` | **lowest level** — the actual kernel | `src/driver.c` only (external linkage, absent from the public header) | must be driven **directly**, not only via `driver` |
| `driver(data, len)` | convenience one-shot wrapper | `include/driver.h` | copies input into a VLA, calls `inner` |
| `inner(out, len)` | `static` — not exported | `src/driver.c` | reachable only through `driver`; it is what fixes the *aliasing* configuration `out == mul1 == mul2 == add` and the `printf("%d\n", …)` output |

`driver` exercises exactly ONE configuration of `fma_array` (total 4-way
aliasing). Every other aliasing configuration is reachable only by calling
`fma_array` directly — hence the rows below.

### Runtime options / modes / flags

**None.** Grepping the whole C source for `#if`, `if`, `switch`, `#define`,
`enum`, and global variables yields only the `DRIVER_H_` include guard. The
library is stateless: there is no init function, no context struct, no
option setter, no mode flag, no byte-order or element-type selector, and no
compile-time configuration. `translation/Cargo.toml` likewise declares no
`[features]`, so the only build configuration is the default one.

Consequently the configuration surface is entirely the **cross-product of the
axes the code branches or depends on data-wise**:

* **Axis A — entry point:** `fma_array` (direct) | `driver`.
* **Axis B — aliasing of the 4 pointers** (only meaningful for `fma_array`;
  `driver` pins this to A4). Line 31 reads `mul1[i]`, `mul2[i]`, `add[i]` and
  then writes `out[i]`, all at the same index `i`, so aliasing is observable:
  * A0 all four buffers distinct
  * A1 `out == mul1`, others distinct
  * A2 `out == mul2`, others distinct
  * A3 `out == add`, others distinct
  * A4 `out == mul1 == mul2 == add` (**the configuration `inner` uses**)
  * A5 `mul1 == mul2`, `out` and `add` distinct (squaring)
  * A6 `mul1 == mul2 == add`, `out` distinct
  * A7 `out == mul1 == mul2`, `add` distinct
  * A8 `out == mul1`, `mul2 == add` (two pairs)
  * A9 partially overlapping ranges (`out = buf`, inputs = `buf + k`) — the
        loop is a forward element-wise walk in both implementations, so the
        result is deterministic and must agree
* **Axis C — `len` / count shape** (empty / one / many / boundary):
  `0`, `1`, `2`, `3`, `4`, `5`, `7`, `8`, `15`, `16`, `17`, `31`, `32`, `33`,
  `63`, `64`, `65`, `100`, `1000`, `4096`, `100_000`.
  Sizes around powers of two matter because they are the boundaries at which a
  vectorising compiler splits main loop / epilogue on either side.
* **Axis D — element value shape** (the value-dependent paths, incl. the
  overflow-capable ones and the `%d` formatting paths):
  * V0 all zeros
  * V1 all ones
  * V2 small positives `1..=9` (no overflow)
  * V3 small negatives `-9..=-1` (exercises the `-` sign in `%d`)
  * V4 mixed small signed `-9..=9` (includes 0)
  * V5 "safe" magnitudes `|x| <= 46340` (`≈ sqrt(INT_MAX)`, product still fits)
  * V6 just past that boundary: `46340`, `46341`, `-46340`, `-46341`, `65535`,
        `65536` (first products to overflow)
  * V7 extremes: `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `-1`, `0`, `1`
  * V8 uniform random over the whole `i32` range (overflow-heavy, seeded)
  * V9 random from a pool of the extremes above (seeded)
* **Axis E — output width of `%d`** (a `driver`-only shape, since only `inner`
  prints): 1-digit, multi-digit, negative-with-sign, and the widest possible
  values `-2147483648` / `2147483647`. Covered by combining C×D on `driver`.

Both `.so`s are compared **byte-for-byte** on (a) the full contents of every
output buffer after the call and (b) the exact bytes written to file
descriptor 1 during the call.

Every row is driven with **many seeded random inputs** (`SPLITMIX64`, fixed
seed `0x9E3779B97F4A7C15`), not one hand-picked value: each row runs the whole
`len` list of Axis C and, per `len`, several independent random draws.

## The table

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `fma_array` | A0 distinct buffers × all Axis-C lens × V0 zeros | [x] |
| C2 | `fma_array` | A0 distinct × all lens × V1 all ones | [x] |
| C3 | `fma_array` | A0 distinct × all lens × V2 small positives, randomized | [x] |
| C4 | `fma_array` | A0 distinct × all lens × V3 small negatives, randomized | [x] |
| C5 | `fma_array` | A0 distinct × all lens × V4 mixed small signed, randomized | [x] |
| C6 | `fma_array` | A0 distinct × all lens × V5 safe magnitudes `|x|<=46340`, randomized | [x] |
| C7 | `fma_array` | A0 distinct × all lens × V6 overflow-boundary values `46340/46341/65535/65536` ± | [x] |
| C8 | `fma_array` | A0 distinct × all lens × V7 extremes `INT_MAX/INT_MIN/±1/0` (all positions) | [x] |
| C9 | `fma_array` | A0 distinct × all lens × V8 full-range random (overflow-heavy) | [x] |
| C10 | `fma_array` | A0 distinct × all lens × V9 random pool of extremes | [x] |
| C11 | `fma_array` | A1 `out == mul1` × all lens × V8 full-range random | [x] |
| C12 | `fma_array` | A1 `out == mul1` × all lens × V7 extremes | [x] |
| C13 | `fma_array` | A2 `out == mul2` × all lens × V8 full-range random | [x] |
| C14 | `fma_array` | A2 `out == mul2` × all lens × V9 extreme pool | [x] |
| C15 | `fma_array` | A3 `out == add` × all lens × V8 full-range random | [x] |
| C16 | `fma_array` | A3 `out == add` × all lens × V5 safe magnitudes | [x] |
| C17 | `fma_array` | A4 all four aliased (the `inner` configuration) × all lens × V8 full-range random | [x] |
| C18 | `fma_array` | A4 all four aliased × all lens × V7 extremes | [x] |
| C19 | `fma_array` | A4 all four aliased × all lens × V2 small positives | [x] |
| C20 | `fma_array` | A5 `mul1 == mul2` (squaring) × all lens × V8 full-range random | [x] |
| C21 | `fma_array` | A5 `mul1 == mul2` × all lens × V6 overflow-boundary magnitudes | [x] |
| C22 | `fma_array` | A6 `mul1 == mul2 == add`, distinct `out` × all lens × V8 random | [x] |
| C23 | `fma_array` | A7 `out == mul1 == mul2`, distinct `add` × all lens × V8 random | [x] |
| C24 | `fma_array` | A8 `out == mul1` and `mul2 == add` × all lens × V8 random | [x] |
| C25 | `fma_array` | A9 partial forward overlap `out = buf`, inputs `= buf + k` for k = 1,2,3,8 × all lens × V8 random | [x] |
| C26 | `fma_array` | A9 partial reverse overlap `out = buf + k`, inputs `= buf` for k = 1,2,3,8 × all lens × V8 random | [x] |
| C27 | `fma_array` | A0 distinct, `len` smaller than the allocated buffer (asserts the tail past `len` is untouched in both) × V8 random | [x] |
| C28 | `driver` | len shape "empty": `len == 0` → no stdout bytes | [x] |
| C29 | `driver` | len shape "one": `len == 1` × V7 extremes and V8 random | [x] |
| C30 | `driver` | len shape "many, small": lens 2,3,4,5,7,8 × V2 small positives (single-digit `%d`) | [x] |
| C31 | `driver` | lens 2..8 × V3 small negatives (leading `-` in `%d`) | [x] |
| C32 | `driver` | lens 2..8 × V4 mixed small signed incl. zeros | [x] |
| C33 | `driver` | power-of-two-boundary lens 15,16,17,31,32,33,63,64,65 × V8 full-range random | [x] |
| C34 | `driver` | lens 100, 1000 × V8 full-range random (multi-KiB stdout, buffer-flush boundaries) | [x] |
| C35 | `driver` | len 4096 × V8 full-range random (crosses the 4 KiB stdio buffer) | [x] |
| C36 | `driver` | len 100_000 × V8 full-range random (large but still inside the 8 MiB C stack VLA limit) | [x] |
| C37 | `driver` | all lens × V5 safe magnitudes (no overflow anywhere) | [x] |
| C38 | `driver` | all lens × V6 overflow-boundary values → widest `%d` output | [x] |
| C39 | `driver` | all lens × V7 extremes `INT_MAX/INT_MIN` → `%d` prints `-2147483648`, `2147483647`, `0` | [x] |
| C40 | `driver` | all lens × V9 random pool of extremes (mixes overflow and non-overflow per element) | [x] |
| C41 | `driver` | `data` buffer larger than `len` (asserts only the first `len` elements are read; caller's buffer unmodified because `data` is `const`) × V8 random | [x] |
| C42 | `driver` + `fma_array` | composed pipeline: `driver(data,len)` stdout must equal `fma_array(t,d,d,d,len)` then `%d\n`-formatting `t`, cross-checked C↔Rust and C↔C | [x] |
| C43 | `driver` | repeated / interleaved invocations (C then Rust then C then Rust, 200 rounds, random len+values) — catches residual state and stdio-buffer coupling | [x] |
| C44 | `fma_array` | repeated invocations on the same buffer (iterated `out = out*out+out`, 50 rounds) — value-dependent divergence amplifier | [x] |

Feature combinations: the crate has no features, so `default` and
`--no-default-features` are the same build; both are run by `run_all.sh`.

## Verification result

All 44 rows pass. Test targets:

| file | tests | covers |
|------|-------|--------|
| `tests/phase_b_fma.rs` | 30 | C1..C27, C44 (+ the full Alias x Shape cross-product, and an exhaustive 15x15x15 value grid at `len == 1`) |
| `tests/phase_b_driver.rs` | 17 | C28..C43 (+ the full Shape x Len cross-product for `driver`) |
| `tests/phase_c_errors.rs` | 18 | ERRORS.md E1..E16 |
| `tests/phase_d_symbols.rs` | 4 | symbol parity |
| `tests/smoke.rs` | 1 | harness self-check with hard-coded C reference values |

Comparison method per row:
* `fma_array`: **every** buffer involved (not just `out`) is compared element by
  element after the call, so an unintended write through an aliased input
  pointer is caught too. A sentinel-filled tail past `len` (row C27) proves
  neither implementation over-runs the loop bound.
* `driver`: file-descriptor-level capture of fd 1, compared byte-for-byte, plus
  an independent `%d\n`-of-`x*x+x` oracle, plus a check that the `const int *`
  input buffer is unmodified.

Verified under: profile `debug` and `release` (the latter also uses
`panic = "abort"`), feature sets `--all-features` and `--no-default-features`
(identical, as the crate declares none), and against both the default and an
`-O2` build of the C sources. Driven by `./run_all.sh`.

### Note on the valid `len` ceiling for `driver`

`driver` allocates its scratch buffer as a VLA on the stack, so its valid
domain is bounded by the stack limit: measured on this host (8 MiB stack),
`len = 2_000_000` succeeds and `len = 2_100_000` faults. Rows C28..C43 stay well
inside that bound; the boundary itself is row E13 of `ERRORS.md`.
