# ERRORS.md — Phase A: error-surface table

Every distinct rejection / error return in `c_src/src/lib.c`, found by grepping
for `return NULL`, `return 0`, `return -1`, `if (!...)` guards and every
allocation-failure branch. There are no `assert`s, no error enums, and no
explicit range checks in this library — the entire error surface is
null-pointer guards plus allocation-failure paths.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|---------------------------------------------|-------------------|------|
| E1 | `init_array` (`lib.c:47`) | `malloc(sizeof(DynamicArray))` returns NULL (struct allocation fails) | returns `NULL` | `e1_init_array_struct_alloc_failure` (not directly forceable through the ABI; covered by inspection + the E2 sibling path, and by the shared-allocator argument in `SYMBOLS.md`) |
| #  | `init_array` (`lib.c:50-53`) | `malloc(initial_capacity * sizeof(int))` returns NULL → `free(arr)` then return | returns `NULL` | see E2 |
| E2 | `init_array` | `initial_capacity` so large that `initial_capacity * sizeof(int)` (size_t, wraps) is still an un-servicable size, e.g. `SIZE_MAX/4`, `SIZE_MAX/2`, `SIZE_MAX`, `1<<62 - 1`, `usize::MAX/4+1` | returns `NULL` | `e2_init_array_data_alloc_failure` |
| E3 | `init_array` | `initial_capacity` whose byte product WRAPS to a servicable value: `initial_capacity = 2^62` → `2^62 * 4 == 0 (mod 2^64)` → `malloc(0)` succeeds → non-NULL array with `capacity == 2^62` and `size == 0` | returns non-NULL (NOT an error — the C does not range-check) | `e3_init_array_capacity_wraps_to_zero_bytes` |
| E4 | `expand_array` (`lib.c:61`) | `arr == NULL` | returns `0` | `e4_expand_array_null` |
| E5 | `expand_array` (`lib.c:66-68`) | `realloc` fails: `arr->capacity * 2 * sizeof(int)` is un-servicable (e.g. capacity `SIZE_MAX/8`, `SIZE_MAX/2`, `1<<61`) | returns `0`, and `arr->data` / `arr->capacity` are left UNCHANGED | `e5_expand_array_realloc_failure` |
| E6 | `expand_array` | `arr->capacity == 0` → `new_capacity == 0` → `realloc(data, 0)` (glibc: frees `data`, returns NULL) | returns `0` | `e6_expand_array_zero_capacity` |
| E7 | `add_element` (`lib.c:76`) | `arr == NULL` (any `value`) | returns `0` | `e7_add_element_null` |
| E8 | `add_element` (`lib.c:79-81`) | `arr->size >= arr->capacity` and `expand_array` fails (capacity 0, or capacity so large the doubling cannot be allocated) | returns `0`, `arr->size` NOT incremented | `e8_add_element_expand_failure` |
| E9 | `free_array` (`lib.c:89`) | `arr == NULL` | no-op, no crash (void) | `e9_free_array_null` |
| E10 | `matrixsum` (`lib.c:154-156`) | `init_array(2)` returns NULL (allocation failure) | returns `-1` | `e10_matrixsum_alloc_failure` (not forceable through the ABI: `init_array(2)` needs 16+8 bytes; documented + asserted never taken for all tested inputs) |

## Generic FFI boundary cases also covered (Phase C)

| # | case | covered by |
|---|------|-----------|
| G1 | NULL pointer to every pointer-taking entry point (`expand_array`, `add_element`, `free_array`) | E4, E7, E9 |
| G2 | Zero length / zero capacity (`init_array(0)`, then `add_element`, `expand_array`, `free_array` on it) | `g2_zero_capacity_lifecycle` |
| G3 | Oversized lengths — every power-of-two capacity `2^0 … 2^63` plus `SIZE_MAX`, `SIZE_MAX-1`, `SIZE_MAX/4`, `SIZE_MAX/4±1` through `init_array` | `g3_capacity_sweep` |
| G4 | One step past a range boundary: `capacity = 2^62 - 1 / 2^62 / 2^62 + 1` (the byte-count wrap point), `size == capacity - 1 / == capacity / == capacity + 1` for `add_element` | `g3_capacity_sweep`, `g4_add_element_boundary` |
| G5 | Out-of-range "enum" values across FFI: the library has no `enum`, but `process_flags` and `matrixsum` take `int` **flag** words. Values with no valid flag bit set, values with every reserved/high bit set, negative values, `INT_MIN`, `INT_MAX`, `-1`, `0x7FFFFFFF`, `0xFFFFFFF0`-style words are all passed and compared. `matrixsum` likewise gets `INT_MIN`/`INT_MAX` (signed overflow of `sum`, `sum * 0x10`) | `g5_out_of_range_flag_words`, `g5_matrixsum_extremes` |
| G6 | Corrupted/hand-built `DynamicArray` passed in from the caller (`data` non-NULL, bogus `size`/`capacity` relationship) — the C does not validate it | `g6_caller_built_struct` |
| G7 | `free_array` on an array whose `data` is NULL (`free(NULL)` is a no-op) | `g7_free_array_null_data` |
| G8 | `expand_array` where `capacity*2*sizeof(int)` wraps to a small **non-zero** byte count, so `realloc` SUCCEEDS and the C stores the absurd doubled capacity: `capacity = 2^61+1` → bytes `= 8`; also `2^62+1`, `2^61+2`, `2^62+2`, `2^63+1` | `x1_expand_array_bytes_wrap_to_small_nonzero` |
| G9 | `init_array` where `capacity*sizeof(int)` wraps to a small **non-zero** byte count (`2^62+n` → `4n` bytes), then the array is actually written through | `x2_init_array_bytes_wrap_then_use` |
| G10 | `calculate_matrix_checksum` invoked with arguments — legal for the C's empty (unprototyped) parameter list `int calculate_matrix_checksum()`; the args must be ignored | `x3_checksum_unprototyped_extra_args` |

## Silent (non-return-value) failure modes

`free_array` returns `void`, so a missing `free()` cannot be observed through any
return value. These rows are checked with glibc `mallinfo2()` accounting, which
is sound because both `.so`s share glibc's allocator (see `SYMBOLS.md`).

| # | condition | expected C result | test |
|---|-----------|-------------------|------|
| L1 | 200 000 × (`init_array(1000)` → 2 × `add_element` → `free_array`) | net allocator growth ≈ 0 (a missed `free(arr->data)` = ~800 MB, a missed `free(arr)` = ~4.8 MB) | `l1_free_array_reclaims_everything_in_both` |
| L2 | 500 000 × `matrixsum` (each call does its own `init_array`/`free_array`) | net allocator growth ≈ 0 | `l2_matrixsum_does_not_leak_in_either` |
| L3 | 20 000 × (`init_array(4)` → 8 × `expand_array` → `free_array`) — the old block must go back to the allocator via `realloc` | net allocator growth ≈ 0 | `l3_expand_array_does_not_leak_in_either` |

## Crash-mode and side-effect-ordering rows

The call never returns in some of these, so a return-value comparison cannot see
them. Each runs in a CHILD PROCESS and the child's termination signal, exit code
and printed observables are compared (`tests/phase_c_crash.rs`).

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| Z1 | `add_element` | `data` points into a `PROT_NONE` page, `size < capacity` → the element store faults | dies with `SIGSEGV` (11) | `z1_add_element_store_faults_size_already_incremented` |
| Z2 | `add_element` | `data == NULL`, `size (0) < capacity (4)` → `arr->data[0] = value` dereferences NULL | dies with `SIGSEGV` (11), **not** `SIGABRT` | `z2_add_element_null_data_within_capacity` |
| Z3 | `add_element`, `expand_array`, `free_array` | caller passes a `DynamicArray *` at an ODD address (C makes no alignment promise; it just emits unaligned `mov`s) | returns normally (`rc == 1`, `size` bumped); no crash | `z3_misaligned_struct_pointer` |
| Z4 | `add_element` | `data` is 4-byte-MISALIGNED, `size < capacity` | returns `1` and stores the value unaligned; no crash | `z4_misaligned_data_pointer` |
| Z5 | `add_element` | element store faults, and `arr->size` is inspected afterwards (struct in a `MAP_SHARED` mapping) | `arr->size == 1`: GCC commits `arr->size = old + 1` **before** the element store | `z5_add_element_side_effect_order_after_fault` |

## Call-structure rows

| # | function | property the C has | test |
|---|----------|--------------------|------|
| T1 | `matrixsum` | makes **no direct** `malloc`/`realloc`/`free` call — all allocation is delegated to `init_array`/`add_element`/`free_array` | `t1_matrixsum_makes_no_direct_allocator_calls_in_either` |
| T2 | `matrixsum` | makes 8 real helper calls (`init_array`, `add_element` ×4, `process_flags`, `calculate_matrix_checksum`, `free_array`), so both `malloc`s and both `free`s actually happen and both are allocation-failure points that can yield `-1` (E1/E10) | `t2_matrixsum_reaches_its_helpers_through_real_calls` |
| T3 | all helpers | reached through the PLT (`R_X86_64_JUMP_SLOT`), i.e. interposable even for the library's own internal calls | `t3_helpers_are_interposable_in_both` |
| T4 | `add_element` | grows via `expand_array`, never by calling `realloc` itself | `t4_add_element_calls_expand_array_rather_than_inlining_realloc` |
