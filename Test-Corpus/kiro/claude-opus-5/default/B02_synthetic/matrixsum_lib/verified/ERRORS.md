# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`. Every rejection / early-error
return in the C source, one row each. Grep used:

```sh
grep -n "return\|if (!\|assert\|NULL" c_src/src/lib.c
```

There are **no** `assert`s, no error enums, no `RETURN_ERROR`-style macros and
no explicit numeric range checks in this library. Every rejection is either a
NULL-pointer guard or an allocation-failure guard. The complete set of
error-producing sites in the C is lines 47, 50-52, 61, 66-67, 76, 79-80 and
154-155.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `init_array` (line 47) | `malloc(sizeof(DynamicArray))` returns NULL (heap exhausted for a 24-byte request) | returns `NULL`; nothing allocated | `e1_init_array_struct_malloc_fail` (unreachable in practice — documented; covered indirectly by E2/E3 which use the same allocator path) | [x] |
| E2 | `init_array` (lines 50-52) | `malloc(initial_capacity * sizeof(int))` returns NULL — reachable with a huge `initial_capacity` (e.g. `SIZE_MAX/4`, `1<<60`) | frees the struct, returns `NULL` (no leak, no partially-initialised object escapes) | `e2_init_array_data_malloc_fail` | [x] |
| E3 | `init_array` (lines 50-52) | `initial_capacity * sizeof(int)` **wraps** `size_t` to a small/zero byte count (e.g. `1<<62` → 0 bytes, `SIZE_MAX` → `2^64-4` bytes) | wrapping multiply is what C computes: `1<<62` → `malloc(0)` succeeds → non-NULL array whose `capacity` field is `1<<62`; `SIZE_MAX` → huge request → `NULL` | `e3_init_array_size_overflow_wrap` | [x] |
| E4 | `expand_array` (line 61) | `arr == NULL` | returns `0`, no dereference | `e4_expand_array_null` | [x] |
| E5 | `expand_array` (lines 66-67) | `realloc` returns NULL — reachable via a capacity whose doubled byte size cannot be satisfied (huge `capacity`), **or** via `capacity == 0` where `realloc(ptr, 0)` frees and returns NULL under glibc | returns `0`; `arr->data` and `arr->capacity` are left **unchanged** (C does not roll back / clear them) | `e5_expand_array_realloc_fail` | [x] |
| E6 | `add_element` (line 76) | `arr == NULL` | returns `0`, `value` discarded | `e6_add_element_null` | [x] |
| E7 | `add_element` (lines 79-80) | `arr->size >= arr->capacity` **and** the inner `expand_array` fails (capacity 0, or capacity so large the doubled realloc fails) | returns `0`; `arr->size` is **not** incremented and no element is written | `e7_add_element_expand_fail` | [x] |
| E8 | `free_array` (implicit guard, `if (arr)`) | `arr == NULL` | no-op, returns cleanly (must not crash) | `e8_free_array_null` | [x] |
| E9 | `matrixsum` (lines 154-155) | `init_array(2)` returns `NULL` (allocation failure for the fixed capacity-2 array) | returns `-1` | `e9_matrixsum_init_fail` (unreachable for the hard-coded capacity 2 — the `-1` sentinel path is documented and asserted absent for all inputs in both libs) | [x] |

## Generic FFI boundary cases also covered (not distinct C branches)

| # | case | expected behaviour | test | status |
|---|------|--------------------|------|--------|
| G1 | NULL passed to every pointer-taking export (`expand_array`, `add_element`, `free_array`) | `0`, `0`, no-op respectively | `g1_all_null_pointers` | [x] |
| G2 | zero length: `init_array(0)` | glibc `malloc(0)` returns a unique non-NULL pointer → array with `capacity == 0`, `size == 0`, non-NULL `data` | `g2_zero_capacity` | [x] |
| G3 | oversized length: `init_array` with `SIZE_MAX`, `SIZE_MAX/4`, `1<<62`, `1<<63` | identical NULL / non-NULL verdict and identical `capacity` field in both libs | `g3_oversized_capacity` | [x] |
| G4 | one step past a valid range: `add_element` exactly at `size == capacity` (the growth boundary) and one element beyond | identical success/failure, identical resulting `size`/`capacity`/buffer contents | `g4_growth_boundary` | [x] |
| G5 | out-of-range "enum" values across FFI: `process_flags` / `matrixsum` receive ints with no valid flag variant — bits outside `FLAG_READ|WRITE|EXECUTE|DELETE` (`0x10`, `0xFF`, `-1`, `INT_MIN`, `INT_MAX`, all 2^32 low-bit patterns sampled) | C masks with `&` and `!!`, so extra bits are ignored; both libs must return the identical count | `g5_out_of_range_flag_values` | [x] |
| G6 | `matrixsum` with `INT_MIN` / `INT_MAX` params (signed overflow in `sum * 0x10 + ...`) | C wraps (gcc two's-complement in practice); Rust uses `wrapping_*` — must be byte-identical | `g6_int_extremes` | [x] |
| G7 | mutated `matrix` global making the checksum exceed `0xFFF`, and negative checksums | `matrix_sum & 0xFFF` masking must match exactly, including for negative sums | `g7_matrix_mutation_mask` | [x] |

## Suite detection power (mutation check)

The differential suite was validated by temporarily mutating
`translation/src/lib.rs` (reverted afterwards; `c_src/` never touched) and
confirming the tests catch each divergence:

| mutation applied to the Rust lib | tests that failed |
|----------------------------------|-------------------|
| `process_flags`: drop the `FLAG_DELETE` term | 8 |
| `matrixsum`: mask `& 0xFFF` → `& 0xFFFF` | 3 |
| `matrixsum`: `hex_multiplier` `0x10` → `0x11` | 6 |
| `expand_array`: doubling `*2` → `*3` | 8 |
| `add_element`: store `value + 1` (off-by-one payload) | 12 |
| `matrix` factory value `0xD4` → `0xD5` | 2 |
| `init_array`: element size `4` → `8` | 2 |
| `add_element`: `size >= capacity` → `size > capacity` | 6 |
| `expand_array(NULL)` returns `1` instead of `0` | 2 (`e4`, `g1`) |
| `add_element(NULL, v)` returns `1` instead of `0` | 2 (`e6`, `g1`) |
| `free_array`: drop the NULL guard | `e8` — SIGSEGV, run aborts |
| `matrixsum`: `-1` sentinel → `0` | 0 (path unreachable, see E9) |

The only mutation with no observable effect is the one on the provably
unreachable `-1` sentinel (E9), which is consistent with the analysis above
rather than a gap in coverage.
