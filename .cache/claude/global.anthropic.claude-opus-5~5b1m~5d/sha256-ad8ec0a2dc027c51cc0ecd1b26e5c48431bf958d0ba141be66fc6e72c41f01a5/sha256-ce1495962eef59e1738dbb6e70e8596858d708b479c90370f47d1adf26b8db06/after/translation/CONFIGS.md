# CONFIGS.md — Phase A: valid-input configuration surface

Mechanically derived from the branches the C code actually takes. `lib.c` has no
runtime option struct and no `#ifdef`s, so the "options" of this library are the
**argument-value classes that select a branch**, plus the **shapes of the buffers**
the pointer arguments describe. The axes below are exactly the conditions the C
source tests:

| axis | values the C code distinguishes | source |
|---|---|---|
| `shift_array` guard | `positions <= 0` \| `positions >= size` \| `0 < positions < size` | `lib.c:36` |
| `shift_array` shape | `size` = 1, 2, 3, 4, 8, 64, 1024; `positions` = 1, middle, `size-1`; byte count = `(size-positions)*4` | `lib.c:37` |
| `process_string` guard | `*str == 0` \| `*str != 0` | `lib.c:45` |
| `process_string` shape | length 0, 1, 2, 5, 63, 255, 1024; ASCII vs. high-bit bytes (negative `char`); NUL in the interior | `lib.c:46` |
| `apply_bitmask` operation | `0` (`& 0xF0`) \| `1` (`& 0x0F`) \| `2` (`\| 0xAA`) \| `3` (`^ 0x55`) \| default | `lib.c:57-67` |
| `apply_bitmask` value | `0`, small, negative, `i32::MIN`, `i32::MAX`, `-1`, random 32-bit | `lib.c:58-65` |
| `init_matrix` | always writes exactly `3*4` `int`s, values `1..12`; row-major `int (*)[4]` | `lib.c:71-83` |
| `compare_allocations` order | `ptr1 < ptr2` → 1 \| `ptr1 > ptr2` → 2 \| `ptr1 == ptr2` → 3 (unsigned compare, `jae`/`jbe`) | `lib.c:102-108` |
| `compare_allocations` bonus | `val1 > 0` → `+10` \| `val1 <= 0` → `+0` | `lib.c:111` |
| `arity4` bitmask selector | `param1 % 4` ∈ `{0,1,2,3}` for `param1 >= 0`, `{0,-1,-2,-3}` for `param1 < 0` (C truncating remainder → `default:`) | `lib.c:142` |
| `arity4` scaling | `param3 == 0` (skip) \| `param3 != 0` → `(result*param3)/100`, positive / negative / overflowing | `lib.c:152-154` |
| `arity4` offset | `param4 == 0` (skip) \| `param4 != 0` → `result += param4` | `lib.c:156-158` |
| `arity` dispatch | `len<2` → `-1` \| `==2` → `arity2` \| `==3` → `arity3` \| `else` → `arity4`; `len` is truncated to `unsigned char` | `lib.c:171-181` |
| allocator state (implicit input) | `ptr1 < ptr2` \| `ptr1 > ptr2` \| `ptr1 == ptr2` — the address ordering the allocator happens to produce is a *hidden argument* of `compare_allocations` | `lib.c:86-108` |
| pointer alignment (implicit input) | aligned \| misaligned `int*` buffers — the C code has no alignment requirement and uses plain `mov` | `lib.c:37,80,175-179` |

**The allocator state is a hidden input, and it is normalised.**
`compare_allocations` (and therefore `arity4`/`arity3`/`arity2`/`arity`) returns a
value that depends on the state of the process-wide glibc allocator: freeing
`ptr1` then `ptr2` makes the next `malloc` pair come back in the opposite address
order (tcache is LIFO), so a *bare* call sequence alternates between `1+bonus`
and `2+bonus`. This is a property of the C code, not of the translation:
`tests/probe_alloc.rs` loads the **same C `.so` twice** (two `dlopen`s of two
copies) and shows the two C instances diverging from each other in exactly the
same way.

Rather than assume anything about that state, every row below that reaches
`malloc` calls `common::normalize_allocator(order)` immediately before the
library call. That helper takes `tcache_count` chunks of `sizeof(int)` out of the
allocator and releases them highest-address-first or lowest-address-first, which
*forces* the library to observe `ptr1 < ptr2` or `ptr1 > ptr2`. Consequently:

* the differential comparison is fully deterministic (the tcache is
  thread-local, so parallel test threads cannot interfere — verified by 40
  repeated runs of the suite);
* **both** branches of `lib.c:102-108` are exercised on purpose, and each row's
  expected value (`order + bonus`) is asserted, not just C-vs-Rust equality;
* the third branch, `ptr1 == ptr2` (`result = 3`), cannot be produced by a real
  allocator and is covered by row C52 through an interposed `malloc`.

Nothing may allocate between `normalize_allocator` and the call it protects,
because a small Rust allocation would land in the same tcache bin.

Every row is exercised with `ITERS = 400` pseudo-random inputs (xorshift64\*,
fixed seed derived from the row id, so runs are reproducible) drawn from the
value classes of that row, comparing C vs. Rust **byte for byte** (return values
and, for the pointer-taking entry points, the full contents of the output buffer
including guard bytes on both sides).

Test file: `tests/phase_b_valid.rs`. `[x]` = passing across all randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1  | `shift_array` | `size=1`, `positions` random in `1..=1` … guard always fails (`positions >= size`) → no-op; random contents | `c1_shift_size1` | [x] |
| C2  | `shift_array` | `size=2`, `positions=1` (only in-range value); random contents | `c2_shift_size2_pos1` | [x] |
| C3  | `shift_array` | `size=3`, `positions ∈ {1,2}`; random contents | `c3_shift_size3` | [x] |
| C4  | `shift_array` | `size=4`, `positions ∈ {1,2,3}` (the shape `arity4` uses); random contents | `c4_shift_size4` | [x] |
| C5  | `shift_array` | `size=8`, `positions ∈ 1..=7`; random contents incl. `i32::MIN/MAX` | `c5_shift_size8` | [x] |
| C6  | `shift_array` | `size=64`, `positions ∈ 1..=63`; overlapping `memmove` of many bytes | `c6_shift_size64` | [x] |
| C7  | `shift_array` | `size=1024`, `positions ∈ {1, 512, 1023}`; large overlapping move | `c7_shift_size1024` | [x] |
| C8  | `shift_array` | `positions == size-1` (minimum move: 1 element) for `size ∈ 2..=64` | `c8_shift_pos_is_size_minus_1` | [x] |
| C9  | `shift_array` | `positions == 1` (maximum move: `size-1` elements) for `size ∈ 2..=64` | `c9_shift_pos_1_varied_size` | [x] |
| C10 | `shift_array` | guard-boundary sweep: `positions ∈ {-1,0,1,size-1,size,size+1}` × `size ∈ 0..=8`, exhaustive | `c10_shift_guard_boundary_sweep` | [x] |
| C11 | `process_string` | length 1 string, random non-zero byte (incl. `0x80..=0xFF`, i.e. negative `char`) | `c11_process_len1` | [x] |
| C12 | `process_string` | length 2..=8, random printable bytes | `c12_process_short` | [x] |
| C13 | `process_string` | length 5 = the literal `"Hello"` used by `arity4` | `c13_process_hello` | [x] |
| C14 | `process_string` | length 63/255/1024, random non-zero bytes | `c14_process_long` | [x] |
| C15 | `process_string` | interior NUL: `strlen` stops early although the buffer continues | `c15_process_interior_nul` | [x] |
| C16 | `process_string` | all-`0xFF` buffer (every `char` negative) of random length | `c16_process_high_bytes` | [x] |
| C17 | `apply_bitmask` | `operation=0` (`value & 0xF0`); random `value` incl. extremes | `c17_bitmask_op0` | [x] |
| C18 | `apply_bitmask` | `operation=1` (`value & 0x0F`) | `c18_bitmask_op1` | [x] |
| C19 | `apply_bitmask` | `operation=2` (`value \| 0xAA`) | `c19_bitmask_op2` | [x] |
| C20 | `apply_bitmask` | `operation=3` (`value ^ 0x55`) | `c20_bitmask_op3` | [x] |
| C21 | `apply_bitmask` | `operation` random over the full `i32` range × `value` random (cross-product incl. all `default:` values) | `c21_bitmask_random_op` | [x] |
| C22 | `init_matrix` | exact `3×4` `int` buffer surrounded by guard words; pre-filled with random garbage | `c22_init_matrix_exact` | [x] |
| C23 | `init_matrix` | called twice in a row on the same buffer (idempotence) and on an oversized buffer, checking only 12 words change | `c23_init_matrix_repeat_and_oversized` | [x] |
| C24 | `compare_allocations` | `val1 > 0` (`+10` bonus taken), `val2` random | `c24_cmp_alloc_val1_pos` | [x] |
| C25 | `compare_allocations` | `val1 == 0` (bonus skipped), `val2` random | `c25_cmp_alloc_val1_zero` | [x] |
| C26 | `compare_allocations` | `val1 < 0` (bonus skipped), `val2` random incl. `i32::MIN` | `c26_cmp_alloc_val1_neg` | [x] |
| C27 | `compare_allocations` | `val1 = i32::MAX` / `i32::MIN` / `1` / `-1` boundary values | `c27_cmp_alloc_boundaries` | [x] |
| C28 | `arity4` | `param1 % 4 == 0`, `param3 == 0`, `param4 == 0` | `c28_arity4_m0_p3z_p4z` | [x] |
| C29 | `arity4` | `param1 % 4 == 1`, `param3 == 0`, `param4 == 0` | `c29_arity4_m1_p3z_p4z` | [x] |
| C30 | `arity4` | `param1 % 4 == 2`, `param3 == 0`, `param4 == 0` | `c30_arity4_m2_p3z_p4z` | [x] |
| C31 | `arity4` | `param1 % 4 == 3`, `param3 == 0`, `param4 == 0` | `c31_arity4_m3_p3z_p4z` | [x] |
| C32 | `arity4` | `param1 < 0` with `param1 % 4 ∈ {-1,-2,-3}` (negative remainder → `default:`), `param3 == 0`, `param4 == 0` | `c32_arity4_negmod_p3z_p4z` | [x] |
| C33 | `arity4` | `param3 > 0` small (`1..=100`), `param4 == 0` — scaling path, all four `param1 % 4` classes | `c33_arity4_p3_small_pos` | [x] |
| C34 | `arity4` | `param3 < 0` small (`-100..=-1`), `param4 == 0` — negative scaling, truncation toward zero | `c34_arity4_p3_small_neg` | [x] |
| C35 | `arity4` | `param3` huge (`i32::MIN`, `i32::MAX`, random) → `result*param3` overflows and wraps, `param4 == 0` | `c35_arity4_p3_overflow` | [x] |
| C36 | `arity4` | `param3 == 0`, `param4 != 0` (offset only), random incl. extremes | `c36_arity4_p4_only` | [x] |
| C37 | `arity4` | `param3 != 0` **and** `param4 != 0` (both paths, fully random) | `c37_arity4_p3_and_p4` | [x] |
| C38 | `arity4` | all four parameters fully random over `i32` (unconstrained cross-product) | `c38_arity4_fully_random` | [x] |
| C39 | `arity4` | boundary corners: every combination of `{i32::MIN,-100,-4,-1,0,1,4,100,i32::MAX}` for `param1`/`param3`, random `param2`/`param4` | `c39_arity4_corner_grid` | [x] |
| C40 | `arity2` | `param3`/`param4` forced to `0` by the wrapper; `p1` covering all `% 4` classes, `p2` random | `c40_arity2_random` | [x] |
| C41 | `arity2` | `p1`/`p2` boundary values (`i32::MIN`, `i32::MAX`, `0`, `±1`) | `c41_arity2_boundaries` | [x] |
| C42 | `arity3` | `p3 == 0` (scaling skipped) | `c42_arity3_p3_zero` | [x] |
| C43 | `arity3` | `p3 != 0`, small and overflowing, `p1` covering all `% 4` classes | `c43_arity3_p3_nonzero` | [x] |
| C44 | `arity` | `len == 2` → `arity2`, 2-element `params` buffer, random contents | `c44_arity_len2` | [x] |
| C45 | `arity` | `len == 3` → `arity3`, 3-element buffer | `c45_arity_len3` | [x] |
| C46 | `arity` | `len == 4` → `arity4`, 4-element buffer | `c46_arity_len4` | [x] |
| C47 | `arity` | `len ∈ 5..=255` → `arity4` (only the first 4 elements are read), longer buffers | `c47_arity_len_5_to_255` | [x] |
| C48 | `arity` | `len` aliasing through the `unsigned char` truncation: `258→2`, `259→3`, `260→4`, `65538→2`, `-1→255`, `i32::MAX→255` | `c48_arity_len_truncation_aliases` | [x] |
| C49 | `arity` | exhaustive `len ∈ 0..=511` (both valid and rejecting) with a fixed 4-element buffer | `c49_arity_len_exhaustive_0_511` | [x] |
| C50 | pipeline | `arity` → `arity3`/`arity2` → `arity4` → `process_string` + `shift_array` + `apply_bitmask` + `init_matrix` + `compare_allocations` driven end-to-end through the **dispatcher only**, fully random `len`/`params` (composed-pipeline check) | `c50_pipeline_random_end_to_end` | [x] |
| C51 | all 9 exports | randomized *interleaved* call sequence across every entry point (shared-state check: no entry point may leave state that changes a later one), replayed per library under both address orderings | `c51_interleaved_all_entry_points` | [x] |
| C52 | `compare_allocations` | address ordering forced with an interposed `malloc`: `ptr1 < ptr2` (→1), `ptr1 > ptr2` (→2) and `ptr1 == ptr2` (→3, unreachable with a real allocator; also pins that the `+10` bonus is decided by the value *in memory*, i.e. `val2`, not by `val1`) × 9 `(val1,val2)` sign/boundary combinations | `phase_c_errors::e24_pointer_order_branches` | [x] |
| C53 | `arity`, `init_matrix`, `shift_array`, `process_string` | **misaligned** buffers (`+1`, `+2`, `+3` bytes) — the C API imposes no alignment and uses plain `mov`, so this is a valid input shape | `phase_c_errors::e25_misaligned_pointers` | [x] |
| C54 | every entry point | both cargo profiles (`dev`, `release`) and all three feature selections (`default`, `--no-default-features`, `--all-features`), plus each profile's tests run against the *other* profile's `.so` via `RUST_LIB_PATH` | `run_all.sh` | [x] |
