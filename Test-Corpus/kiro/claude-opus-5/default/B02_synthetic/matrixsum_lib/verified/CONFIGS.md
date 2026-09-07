# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`, not from docs.

**Build-time configuration axes:** none. `CMakeLists.txt` has no `option()`, no
`target_compile_definitions`, and `lib.c` contains no `#ifdef` outside the three
`#include`s. `translation/Cargo.toml` has no `[features]` section, so the only
Rust configuration is the default one (identical to `--no-default-features`).
There is no `[[bin]]`/`main.rs` in either tree, so there is no driver stdout to
diff.

**Runtime option/mode axis (A) — the permission flag bitmask.**
`matrixsum` derives `permissions` from the truthiness of its four `int`
arguments (`lib.c:135-151`) and `process_flags` counts the set bits of the
`FLAG_READ|FLAG_WRITE|FLAG_EXECUTE|FLAG_DELETE` nibble (`lib.c:94-113`).
That is a 4-bit option space: **16 distinct modes**, each taking a different
combination of the four `if`s.

**Runtime state axis (B) — the mutable `matrix` global.**
`matrix` is an exported `D` (writable) object and
`calculate_matrix_checksum` reads it live on every call (`lib.c:115-125`), so
`matrixsum`'s result depends on caller-visible mutable state. States that
matter: pristine, mutated, negative-valued, and values large enough that the
`& 0xFFF` mask and the `int` accumulator behave differently.

**Input-shape axis (C) — `DynamicArray` capacity / size / growth.**
The container entry points branch on `size >= capacity` (`lib.c:78`) and on
`capacity * 2` (`lib.c:63`). Distinct shapes: capacity 0 / 1 / 2 / many;
size below capacity vs. exactly at capacity; growth triggered once vs.
repeatedly; `expand_array` called directly vs. implicitly via `add_element`.

**Input-shape axis (D) — argument value magnitude/sign.**
`sum`, `sum * 0x10`, `flag_count * 0xFF` and `matrix_sum & 0xFFF` are all `int`
arithmetic with no checks, so small / large / negative / overflowing values are
distinct value-dependent paths even when the flag mode is identical.

**Entry points.** All 7 exported functions are covered, *including* the
low-level ones (`init_array`, `expand_array`, `add_element`, `free_array`,
`process_flags`, `calculate_matrix_checksum`) driven directly, not only through
the `matrixsum` one-shot wrapper, plus the exported `matrix` data object.

## Rows (one per combination the C treats differently)

Every row is exercised with many randomized inputs (fixed seed, SplitMix64) via
both `.so`s and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `process_flags` | axis A exhaustive: all 16 low-nibble flag values, no extra bits | [x] |
| 2 | `process_flags` | axis A × high-bit noise: all 16 nibbles OR'd with randomized bits `>= 0x10` (incl. sign bit) — high bits must be ignored | [x] |
| 3 | `process_flags` | randomized full-range `i32` (positive, negative, `INT_MIN`/`INT_MAX` boundaries) | [x] |
| 4 | `calculate_matrix_checksum` | pristine `matrix` global (as loaded), called repeatedly — must be pure/stable | [x] |
| 5 | `calculate_matrix_checksum` | `matrix` global overwritten with randomized full-range `i32` (12 cells, row-major layout must agree), restored after | [x] |
| 6 | `calculate_matrix_checksum` | `matrix` global at boundary shapes: all zero, all `INT_MAX`, all `INT_MIN`, alternating ±, single non-zero cell in each of the 12 positions | [x] |
| 7 | `init_array` + `free_array` | capacity shape 1 (smallest non-zero); verify `size`/`capacity`/non-null `data` field layout matches, then free | [x] |
| 8 | `init_array` + `free_array` | capacity shapes 0, 2, 3, 4, 8, 64, 4096 and randomized capacities in `1..=65536` | [x] |
| 9 | `init_array` + `add_element` (below capacity) | capacity `c` (randomized), insert `n < c` randomized values — no growth path; compare full `size`/`capacity`/element buffer | [x] |
| 10 | `init_array` + `add_element` (exactly at capacity) | insert `n == c` values — `size >= capacity` still false on the last insert; no growth | [x] |
| 11 | `init_array` + `add_element` (single growth) | capacity `c`, insert `c + 1` values — triggers exactly one `expand_array`; compare returned flags, `capacity` (`2c`) and all elements | [x] |
| 12 | `init_array` + `add_element` (repeated growth) | capacity 1 or 2, insert 100–500 randomized values — many doublings; compare `size`, `capacity` and the whole buffer | [x] |
| 13 | `init_array` + `expand_array` (direct, low-level) | expand a fresh array 0..k times *without* adding elements, k randomized in `1..=12`; compare return value and `capacity` doubling sequence | [x] |
| 14 | `init_array` + `add_element` + `expand_array` interleaved | randomized program of interleaved `add_element` / `expand_array` / field reads (the composed pipeline, not per-wrapper calls) | [x] |
| 15 | `matrixsum` | axis A exhaustive: all 16 zero/non-zero argument patterns, with randomized small non-zero magnitudes | [x] |
| 16 | `matrixsum` | axis A × axis D: randomized full-range `i32` arguments (overflowing `sum`, `sum*0x10`) | [x] |
| 17 | `matrixsum` | all-zero arguments (flag_count 0, sum 0) — the degenerate shape | [x] |
| 18 | `matrixsum` | argument boundary values `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` in the cross-product over the 4 parameter positions | [x] |
| 19 | `matrixsum` × axis B | `matrixsum` called *after* the `matrix` global is mutated (randomized, and `& 0xFFF`-boundary values such as `0xFFF`, `0x1000`, negative sums) — the option/state interaction | [x] |
| 20 | full stack, randomized program | randomized sequences mixing `matrix` mutation, `matrixsum`, `process_flags`, `calculate_matrix_checksum` and array lifecycles against one shared library instance each — order-dependence / hidden-state divergence | [x] |
