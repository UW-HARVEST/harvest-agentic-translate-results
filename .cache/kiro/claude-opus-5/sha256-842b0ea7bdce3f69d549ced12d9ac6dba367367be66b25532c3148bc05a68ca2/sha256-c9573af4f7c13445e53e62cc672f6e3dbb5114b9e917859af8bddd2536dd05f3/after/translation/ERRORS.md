# ERRORS.md — error / rejection surface of the C library

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep

```sh
$ grep -n 'return\|assert\|NULL\|errno\|-1\|if \|for \|while ' c_src/src/lib.c
7:    if (a->sort_bits <= b->sort_bits)
8:        return 1;
9:    if (a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id)
10:        return 1;
11:    return 0;
18:    for (int k = lo; k < hi; k++) {
19:        if (i < split &&
20:            (j >= hi ||
24:        } else {
33:    if (hi - lo <= 1)
34:        return;
35:    int split = (lo + hi) / 2;
```

Findings, stated exactly:

* There is **no error-return macro** (`RETURN_ERROR`, …), **no `return -1`**,
  **no `return NULL`**, **no error enum**, **no `assert`**, **no `errno` use**,
  and **no explicit null / range validation** anywhere in the library.
* The only public function, `merge_sort`, returns `void`. It therefore has no
  error channel at all: every input is either processed or produces undefined
  behaviour. "Rejection" in this library means an *early return that does no
  work*, plus the implicit conversions/overflows the C performs on bad inputs.
* There are **no enums** in the public header, so "out-of-range enum value
  across the FFI boundary" has no instance here. The only scalar parameter is
  `int size`, and the whole `int` range is covered below (rows 7–11).
* `sizeof(spritebatch_sprite_t)` is 16 and the struct assignment `b[k] = a[i]`
  compiles to a **full 16-byte copy including the tail padding**
  (`mov (%rax),%rax; mov %rax,(%rcx); mov 0x8(%rax),%rdx; mov %rdx,0x8(%rcx)`),
  so padding bytes are part of the observable output — see row 12.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `spritebatch_internal_sprite_less_than_or_equal` (lib.c:7) | `a->sort_bits <= b->sort_bits` | returns `1` immediately; `texture_id` never inspected |
| 2  | `spritebatch_internal_sprite_less_than_or_equal` (lib.c:9) | `a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id` | **unreachable**: row 1 already returns `1` for every `==`. The `texture_id` tiebreak is dead code, so `texture_id` has *zero* influence on the sort order. Rust must be equally "wrong". |
| 3  | `spritebatch_internal_sprite_less_than_or_equal` (lib.c:11) | `a->sort_bits > b->sort_bits` | returns `0` |
| 4  | `spritebatch_internal_merge_sort_iteration` (lib.c:19) | left run exhausted (`i >= split`) | takes the `else` branch: `b[k] = a[j]`, `j++` — **without any bound check on `j`**, so `j` can run past `hi` and read out of range when `lo/split/hi` are inconsistent |
| 5  | `spritebatch_internal_merge_sort_iteration` (lib.c:20) | right run exhausted (`j >= hi`) | short-circuits: `a + j` is **not** dereferenced; `b[k] = a[i]` is taken. Rust must not evaluate the comparator here either (it would read out of bounds). |
| 6  | `spritebatch_internal_merge_sort_recurse` (lib.c:33) | `hi - lo <= 1` (includes `hi == lo` and `hi < lo`) | returns immediately, writes nothing |
| 7  | `merge_sort` | `size == 0` | `memcpy(b, a, 0)` — no-op; recursion returns at row 6. Both `a` and `b` left **completely untouched**. |
| 8  | `merge_sort` | `size == 1` | `memcpy` copies 16 bytes `a`→`b`; recursion returns at row 6. `a` unchanged, `b[0] == a[0]` (all 16 bytes incl. padding). |
| 9  | `merge_sort` | `a == NULL && b == NULL && size == 0` | `memcpy(NULL, NULL, 0)` is a no-op in practice on glibc; no crash, no write. Must not crash in Rust either. |
| 10 | `merge_sort` | `size < 0` (e.g. `-1`, `-1000`, `INT_MIN`) | `sizeof(t) * size` widens `int`→`size_t` (`cltq; shl $4`), giving `16 * (2**64 + size) mod 2**64`, a near-2**64 byte count. The recursion then returns immediately (`hi - lo = size - 0 <= 1`), so the `memcpy` is the only effect. **Observed on this platform** (tested out-of-process, see `errors_row10_negative_size_same_fatal_outcome`): `size == -1` (length `2**64 - 16`) returns normally with a specific resulting byte image, while `size == -1000` and `size == INT_MIN` die on a fatal signal. Both implementations must produce the *same* outcome AND the same resulting bytes for each value — verified for `-1, -2, -16, -1000, INT_MIN, INT_MIN+1`. |
| 11 | `merge_sort` | `size == INT_MAX` (and any `size` large enough that `lo + hi` overflows `int` in lib.c:35) | signed overflow; gcc emits wrapping `add` then `shr $0x1f; add; sar $1` (truncate-toward-zero divide), so `split` goes negative → out-of-range indexing. Not reachable in-process (needs ≈32 GiB of buffers). The Rust uses `wrapping_add` + Rust's truncating `/`, emitting the identical arithmetic. Documented; the same midpoint code path is exercised at the largest affordable sizes (up to 131072 elements, 17 recursion levels). |
| 12 | `merge_sort` | struct tail padding holds non-zero garbage | the 16-byte struct copy carries the padding across, so garbage padding is **propagated, not normalised**. Rust must copy 16 bytes, not 12. |

### Coverage checklist

- [x] 1
- [x] 2
- [x] 3
- [x] 4
- [x] 5
- [x] 6
- [x] 7
- [x] 8
- [x] 9
- [x] 10
- [x] 11 (documented-unreachable: needs ~32 GiB; arithmetic verified against gcc's `add`/`shr`/`sar` sequence)
- [x] 12

## Where each row is tested

All in `tests/error_paths.rs`, run against both `.so` files loaded with
`libloading`:

| row(s) | test |
|---|---|
| 1, 2, 3 | `errors_row01_03_comparator_exits` (all three exits, plus an absolute assertion that the C really keeps input order on `sort_bits` ties, so a "fixed" Rust comparator cannot pass) |
| 4 | `errors_row04_left_run_exhausted` |
| 5 | `errors_row05_right_run_exhausted_short_circuit` |
| 6 | `errors_row06_recurse_early_return` |
| 7 | `errors_row07_size_zero_no_writes` (differential + absolute "not one byte written") |
| 8 | `errors_row08_size_one_copies_all_16_bytes` |
| 9 | `errors_row09_null_pointers_size_zero` (both null, `a` null, `b` null) |
| 10 | `errors_row10_negative_size_same_fatal_outcome` (out-of-process, compares exit code, signal, **and** an FNV digest of both buffers, for `-1, -2, -16, -1000, INT_MIN, INT_MIN+1`) |
| 11 | `errors_row11_midpoint_arithmetic_at_largest_affordable_size` |
| 12 | `errors_row12_padding_propagated_not_normalised` (differential + absolute check that the C carries `DE AD BE EF` padding through) |
| generic boundaries | `boundary_sizes_around_documented_ranges`, `undersized_length_leaves_tail_untouched`, `oversized_length_reported_to_api`, `exhaustive_small_int_size_sweep` (every `size` in `0..=512`) |

### Out-of-range enum values

`c_src/include/lib.h` declares **no enums**, so this class has no instance in
this API. The equivalent — an `int` parameter with no valid meaning — is the
`size` argument, and it is covered exhaustively: `0..=512` one value at a time
(`exhaustive_small_int_size_sweep`), the oversized region (`oversized_length_*`),
and the entire negative half of the range via representative values including
`INT_MIN` (`errors_row10_*`).

### Harness discrimination

These tests are not vacuous: injecting a mutation that clamps a negative `size`
instead of letting it wrap fails `errors_row10_*`; a 12-byte sprite copy fails 9
of the 15 error-path tests; a `u32` comparator fails 8. See CONFIGS.md for the
full mutation table.
