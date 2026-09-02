# CONFIGS.md — configuration-surface table

Derived mechanically from the C source and the public header.

## Axes the C code actually distinguishes

Enumerated by grepping the public header for entry points and the source for
every runtime branch:

```sh
grep -n ';' c_src/include/driver.h        # -> only `void driver(int x);`
grep -rn 'if\|switch\|#ifdef\|#if \|getenv\|static ' c_src/src/driver.c   # -> no match
```

* **Runtime options / modes / flags: none.** No setter, no global state, no
  `#ifdef`, no environment lookup, no `static` variable. The function is pure
  apart from its stdout side effect, so there is no option cross-product.
* **Public entry points: exactly one — `driver`.** It *is* the lowest-level
  entry point; there is no convenience wrapper layered over anything else, so
  "test the low-level API too" collapses onto this one symbol.
* **Input shapes:** the single parameter is a by-value `int`. The shapes the
  code and the `%d` conversion actually treat differently are:
  * sign of the result (`-` printed or not) — flips at `x = -150`;
  * result exactly zero (`0` printed);
  * decimal digit count of the result (1 … 10 digits) — affects `printf` output
    width;
  * whether `2*x` overflows (`|x| > INT_MAX/2`);
  * whether `y += 300` overflows (`x >= 1073741674`);
  * call multiplicity / statelessness (0, 1, many calls: output ordering and
    absence of carried state).

## Rows (each meaningful combination)

| #   | entry point(s) | configuration (options set + input shape) | randomized | [x] |
|-----|----------------|-------------------------------------------|------------|-----|
| C1  | `driver` | `x = 0` — result `300`, no overflow, positive 3-digit | fixed | [x] |
| C2  | `driver` | small positive `x ∈ [1, 999]`, no overflow, positive result | 400 seeded values | [x] |
| C3  | `driver` | small negative `x ∈ [-999, -1]`, spans the sign flip and zero crossing of the result | 400 seeded values | [x] |
| C4  | `driver` | `x = -150` — result exactly `0` (single-digit, no sign) | fixed | [x] |
| C5  | `driver` | `x = -149` / `x = -151` — one step either side of the zero result (`2` and `-2`, sign appears) | fixed | [x] |
| C6  | `driver` | `x` chosen so the result has exactly `d` decimal digits, for every `d ∈ 1..=10`, both signs | seeded per digit count | [x] |
| C7  | `driver` | mid-range positive `x ∈ [1000, INT_MAX/2 - 1]`, no overflow | 500 seeded values | [x] |
| C8  | `driver` | mid-range negative `x ∈ [INT_MIN/2 + 1, -1000]`, no overflow | 500 seeded values | [x] |
| C9  | `driver` | `x` in the multiply-overflow positive band `[INT_MAX/2 + 1, INT_MAX]` | 500 seeded values | [x] |
| C10 | `driver` | `x` in the multiply-overflow negative band `[INT_MIN, INT_MIN/2 - 1]` | 500 seeded values | [x] |
| C11 | `driver` | `x` in the add-overflow-only band `[1073741674, 1073741823]` (`2*x` fits, `+300` does not) | full band, 150 values | [x] |
| C12 | `driver` | exact boundary values `{INT_MIN, INT_MIN+1, INT_MIN/2-1, INT_MIN/2, INT_MIN/2+1, -151, -150, -149, -1, 0, 1, 1073741673, 1073741674, INT_MAX/2-1, INT_MAX/2, INT_MAX/2+1, INT_MAX-1, INT_MAX}` | fixed set | [x] |
| C13 | `driver` | uniform random over the **entire** `i32` range (all bit patterns reachable) | 4000 seeded values | [x] |
| C14 | `driver` | many sequential calls in one process, interleaved C→Rust→C, checking statelessness and output ordering | 200-call seeded sequence | [x] |
| C15 | `driver` | zero calls (library loaded, symbol resolved, nothing invoked) — no spurious output at load/unload time | fixed | [x] |
| C16 | `driver` | powers of two and `±2^k ± 1` for `k ∈ 0..=31` — value-dependent carry/overflow patterns | full sweep | [x] |

All rows are driven through `libloading` against **both** `.so` files and
compared byte-for-byte on captured stdout. Seed is fixed (`0x5EED_1234_ABCD_EF01`)
for reproducibility.

## Feature combinations

`Cargo.toml` defines no `[features]`, so the default build is the only
combination. The test runner script still loops over the discovered set
(`{default}`) so the check is mechanical rather than assumed.
