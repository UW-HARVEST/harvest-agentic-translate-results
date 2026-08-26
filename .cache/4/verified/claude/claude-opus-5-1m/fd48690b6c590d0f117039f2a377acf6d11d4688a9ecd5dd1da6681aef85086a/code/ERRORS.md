# ERRORS.md — Error-surface table (Phase A → gated Phase C)

Derived mechanically by grepping **every** `return` / `if (!...)` / null check /
allocation-failure branch in `c_src/src/lib.c`. There are no `assert`s, no error
enums, and no explicit numeric range checks in this library — every rejection is
either a NULL-pointer guard or an allocation-failure guard. All 13 `return`
statements in the C file were classified; the ones below are the rejecting ones.

Sentinel conventions in this C library:
- `init_array` → `NULL` on failure, non-NULL on success.
- `expand_array`, `add_element` → `0` on failure, `1` on success.
- `matrixsum` → `-1` on failure.
- `free_array` → `void`, NULL is a silent no-op.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `init_array` | `malloc(sizeof(DynamicArray))` fails (line 47 `if (!arr) return NULL;`) — not reachable by input alone (16-byte allocation); covered structurally via the data-alloc path below | `NULL` | `err01_init_array_struct_alloc_failure_documented` | [x] |
| 2 | `init_array` | `malloc(initial_capacity * sizeof(int))` fails (line 50) — `initial_capacity` whose byte product is unsatisfiable: `SIZE_MAX`, `SIZE_MAX/2`, `SIZE_MAX/4`, `SIZE_MAX/8`, `(1<<62)-1`, `1<<61` (all `> PTRDIFF_MAX` after the multiply, or an unmappable size) | `NULL` (and the `DynamicArray` is `free`d, no leak) | `err02_init_array_huge_capacity_returns_null` | [x] |
| 3 | `init_array` | `initial_capacity * sizeof(int)` **wraps to 0** (`initial_capacity == 1<<62`, `2<<62`, `3<<62`): `size_t` multiply overflow → `malloc(0)` → glibc returns a unique non-NULL pointer, so this is **NOT** an error: returns a valid handle with `capacity == 1<<62` and a 0-byte buffer | non-`NULL`, `size==0`, `capacity==initial_capacity` | `err03_init_array_capacity_product_wraps_to_zero` | [x] |
| 4 | `init_array` | `initial_capacity == 0` → `malloc(0)`; glibc returns non-NULL, so **not** an error: valid handle with `capacity == 0` | non-`NULL`, `size==0`, `capacity==0` | `err04_init_array_zero_capacity_is_not_an_error` | [x] |
| 5 | `expand_array` | `arr == NULL` (line 61 `if (!arr) return 0;`) | `0` | `err05_expand_array_null_returns_zero` | [x] |
| 6 | `expand_array` | `realloc` fails (line 66) — `capacity*2*4` unsatisfiable, e.g. capacity `SIZE_MAX/8`; C leaves `data` and `capacity` **unmodified** | `0`, struct unchanged | `err06_expand_array_realloc_failure_returns_zero` | [x] |
| 7 | `expand_array` | `capacity == 0` → `new_capacity = 0` → `realloc(data, 0)`; glibc **frees** `data` and returns `NULL` → treated as failure. C returns `0` and does **not** update `data`, leaving a dangling pointer (preserved behaviour, must not be "fixed") | `0`, `capacity` still `0` | `err07_expand_array_zero_capacity_realloc_to_zero` | [x] |
| 8 | `expand_array` | `capacity` such that `capacity*2` wraps `size_t` (`capacity >= 1<<63`, e.g. `1<<63` → `new_capacity == 0` → `realloc(p,0)` → NULL) | `0` | `err08_expand_array_capacity_doubling_wraps` | [x] |
| 9 | `add_element` | `arr == NULL` (line 76 `if (!arr) return 0;`) | `0` | `err09_add_element_null_returns_zero` | [x] |
| 10 | `add_element` | `size >= capacity` **and** `expand_array` fails (lines 78-81) → failure propagates; element is **not** stored and `size` is not incremented | `0`, `size` unchanged | `err10_add_element_expand_failure_propagates` | [x] |
| 11 | `add_element` | `size >= capacity` with `capacity == 0` → `expand_array` hits row 7 → returns `0` | `0`, `size` still `0` | `err11_add_element_on_zero_capacity_array` | [x] |
| 12 | `add_element` | `size > capacity` (strictly greater — the check is `>=`, reachable by a caller that mutates the struct directly) → still routes through `expand_array` | matches C (`0` or `1` per realloc outcome) | `err12_add_element_size_greater_than_capacity` | [x] |
| 13 | `free_array` | `arr == NULL` (line 89 `if (arr)`) → silent no-op, must not crash | no crash, no return value | `err13_free_array_null_is_noop` | [x] |
| 14 | `matrixsum` | `init_array(2)` returns `NULL` (lines 154-155 `return -1;`) — the only error return of the public header function; unreachable by input (fixed 8-byte request) since the params cannot influence the allocation | `-1` (never observed in practice; `matrixsum` is total over all `int` inputs) | `err14_matrixsum_never_returns_error_sentinel` | [x] |
| 15 | `init_array` | `initial_capacity * sizeof(int)` wraps to a **small non-zero** product: `(1<<62)+1` → `malloc(4)` **succeeds**, so this is **NOT** an error — it returns a valid handle whose `capacity` (`2^62+1`) is astronomically larger than its 4-byte buffer. Both implementations must agree on this non-rejection (the Rust must use wrapping, not checked, multiplication) | non-`NULL`, `size==0`, `capacity==(1<<62)+1` | `err15_init_array_capacity_product_wraps_to_small` | [x] |

## Generic FFI boundary cases (required by Phase C even though not table rows)

| # | case | covered by |
|---|------|-----------|
| G1 | NULL pointer to every pointer-taking entry point (`expand_array`, `add_element`, `free_array`) | rows 5, 9, 13 + `err_g1_g2_null_and_zero_length_across_all_entry_points` |
| G2 | Zero length / zero capacity (`init_array(0)`, `expand_array` on cap 0, `add_element` on cap 0) | rows 4, 7, 11 + `err_g1_g2_null_and_zero_length_across_all_entry_points` |
| G3 | Oversized lengths (`SIZE_MAX`, `SIZE_MAX/4`, `SIZE_MAX/8`) and one-step-past-overflow boundaries (`1<<62`, `(1<<62)+1`, `1<<63`) | rows 2, 3, 6, 8 |
| G4 | Out-of-range "enum"/flag values across FFI: `process_flags` takes an `int` bitmask, so every `int` is in range — including values with **no** valid flag bits set, values with only reserved high bits, `INT_MIN`, `INT_MAX`, `-1`, and `0x7FFFFFF0`. The C masks with `& 0x1/0x2/0x4/0x8` and `!!`s each, so unknown bits must be **ignored**, never counted | `err_g4_process_flags_out_of_range_and_reserved_bits` |
| G5 | Extreme scalar values into `matrixsum` (`INT_MIN`, `INT_MAX`, `±1`) where `sum * 0x10` and the additions **overflow** `int`. Signed overflow is UB in C but gcc wraps; the Rust uses `wrapping_*` to match the emitted code | `err_g5_matrixsum_signed_overflow_extremes` |
| G6 | Mutated global `matrix` (exported `D` symbol) driving `calculate_matrix_checksum` / `matrixsum` to overflow, including values making `matrix_sum` negative so `matrix_sum & 0xFFF` masks a negative | `cfg` rows 14-16 + `err_g6_matrix_mutation_negative_checksum` |

**Not applicable to this library:** `RETURN_ERROR`-style macros, error enums,
`errno` propagation, string/format parsing errors, byte-order handling — none
appear in `c_src/src/lib.c`.
