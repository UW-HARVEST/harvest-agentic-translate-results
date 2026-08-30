# Configuration surface

The public dynamic surface consists of the low-level `fma_array` entry point
and the composed `driver` entry point. There are no runtime option flags,
compile-time Cargo features, enums, format selectors, element-type selectors,
or byte-order modes.

The C branches only on each loop's `i < len` condition. Pointer aliasing is
nevertheless part of the public low-level input shape because the parameters
are not `restrict`-qualified, and shifted overlap changes later loop
iterations. Randomized positive-length rows cover lengths from 1 through 64.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `fma_array` | `len == 0`; all pointers may be null; empty/no-op shape | [x] |
| 2 | `fma_array` | positive randomized length; four disjoint buffers; values constrained so multiply-add does not overflow | [x] |
| 3 | `fma_array` | positive randomized length; four disjoint buffers; full-width `int` values including wrapping/edge cases | [x] |
| 4 | `fma_array` | positive randomized length; `out == mul1`; `mul2` and `add` disjoint | [x] |
| 5 | `fma_array` | positive randomized length; `out == mul2`; `mul1` and `add` disjoint | [x] |
| 6 | `fma_array` | positive randomized length; `out == add`; `mul1` and `mul2` disjoint | [x] |
| 7 | `fma_array` | positive randomized length; `out == mul1 == mul2`; `add` disjoint | [x] |
| 8 | `fma_array` | positive randomized length; `out == mul1 == add`; `mul2` disjoint | [x] |
| 9 | `fma_array` | positive randomized length; `out == mul2 == add`; `mul1` disjoint | [x] |
| 10 | `fma_array` | positive randomized length; all four pointers exactly alias | [x] |
| 11 | `fma_array` | positive randomized length; `mul1 == mul2`; `out` and `add` disjoint | [x] |
| 12 | `fma_array` | positive randomized length; `mul1 == add`; `out` and `mul2` disjoint | [x] |
| 13 | `fma_array` | positive randomized length; `mul2 == add`; `out` and `mul1` disjoint | [x] |
| 14 | `fma_array` | positive randomized length; `mul1 == mul2 == add`; `out` disjoint | [x] |
| 15 | `fma_array` | randomized length at least 2; `out` starts one element after a shared `mul1`/`mul2`/`add` buffer (forward shifted overlap) | [x] |
| 16 | `fma_array` | randomized length at least 2; shared `mul1`/`mul2`/`add` starts one element after `out` (backward shifted overlap) | [x] |
| 17 | `driver` | `len == 0`; null data; no output | [x] |
| 18 | `driver` | `len == 1`; randomized scalar values including `INT_MIN`, `INT_MAX`, negative, zero, and positive | [x] |
| 19 | `driver` | randomized `len` from 2 through 64; randomized arrays including edge and multiply-add-overflow values; decimal newline output | [x] |

## Feature combinations

`Cargo.toml` declares no features. The complete build matrix is therefore:

| # | Cargo invocation mode | [ ] |
|---|-----------------------|-----|
| 1 | default features | [x] |
| 2 | `--no-default-features` (equivalent empty feature set) | [x] |
