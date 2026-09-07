# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h` by
grepping for every rejection construct:

```
grep -nE 'RETURN_ERROR|return *-?[0-9]|return NULL|assert|errno|enum|ERROR|#if|#ifdef|MAX|MIN|LIMIT|< *0|> *0|if *\(' -r src include
grep -nE '\breturn\b' -r src include
```

## Finding: the C library has NO error-reporting surface

Complete set of `if` statements in the whole library (3) and `return`
statements (4):

| src line | construct | role |
|---|---|---|
| `lib.c:7`  | `if (a->sort_bits <= b->sort_bits) return 1;` | comparison result, not an error |
| `lib.c:9`  | `if (a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id) return 1;` | comparison result (**dead code**, see row 8) |
| `lib.c:11` | `return 0;` | comparison result, not an error |
| `lib.c:19` | `if (i < split && (j >= hi || leq(...)))` | merge branch selection, not an error |
| `lib.c:33` | `if (hi - lo <= 1) return;` | recursion base case, not an error |
| `lib.c:34` | `return;` | recursion base case |

There are **no** `assert`s, no `errno` use, no error enums, no error-return
macros, no null checks, no range checks, and no min/max constants. `merge_sort`
returns `void`, so it has no way to report failure at all. Every argument
validation the C code performs is *implicit*: it is whatever the arithmetic and
the base case happen to do.

The table below therefore enumerates every distinct *rejection / degenerate
input* condition reachable in the C code, with the observable C result each
produces. "Same error/rejection" for a `void` function means: **identical
observable side effects on both buffers, byte-for-byte, and identical
crash-or-not-crash behaviour.** That is the strongest assertion available for
this API, and it is what the Phase C tests assert.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|---|---|---|---|---|
| 1 | `merge_sort` | `size == 0`, both pointers valid | `memcpy(b,a,0)` is a no-op; `recurse(b,0,0,a)` hits `hi-lo == 0 <= 1` and returns immediately. Both buffers left **completely untouched** (including `b`'s pre-existing garbage). No crash. | `err_01_size_zero_leaves_both_buffers_untouched` | [x] |
| 2 | `merge_sort` | `size == 0`, **both pointers `NULL`** | `memcpy(NULL,NULL,0)` performs no access; base case returns. No dereference, no crash, no write. | `err_02_size_zero_null_pointers` | [x] |
| 3 | `merge_sort` | `size == 0`, `a` valid / `b` `NULL` (and the mirror, `a` `NULL` / `b` valid) | length-0 `memcpy` never touches the `NULL` side; base case returns. No crash, `a` unchanged. | `err_03_size_zero_one_null_pointer` | [x] |
| 4 | `merge_sort` | `size == 1` (degenerate, below the merge threshold) | `memcpy` copies 1 element (16 bytes, padding included) `a`→`b`; `recurse` hits `hi-lo == 1 <= 1` and returns. `a` unchanged, `b[0]` == `a[0]` byte-for-byte, `b[1..]` untouched. | `err_04_size_one_copies_then_returns` | [x] |
| 5 | `spritebatch_internal_merge_sort_recurse` | base case `hi - lo <= 1` reached at every leaf | recursion terminates; no read/write outside `[lo, hi)`. Verified indirectly: no element outside `[0,size)` of either buffer is ever modified. | `err_05_no_out_of_range_writes` | [x] |
| 6 | `spritebatch_internal_merge_sort_iteration` | right run exhausted: `j >= hi` while `i < split` | short-circuit `\|\|` means `a + j` (one past the run, possibly one past the array) is **not** dereferenced; the left element is taken. | `err_06_right_run_exhausted_no_oob_read` | [x] |
| 7 | `spritebatch_internal_merge_sort_iteration` | left run exhausted: `i >= split` | the `else` branch is taken unconditionally, `a + j` is read without any comparison; `i` is never incremented past `split`. | `err_07_left_run_exhausted` | [x] |
| 8 | `spritebatch_internal_sprite_less_than_or_equal` | `a->sort_bits == b->sort_bits` with `a->texture_id > b->texture_id` — the input the second `if` was *meant* to reject | the first `if` (`<=`) already returned 1, so the `texture_id` tiebreak is **unreachable dead code**. C returns **1** (not 0). Consequence: the sort is *not* ordered by `texture_id` on ties, and equal-`sort_bits` elements keep their relative order (stable). Must be reproduced, not fixed. | `err_08_dead_texture_id_tiebreak` + `err_08b_ties_are_stable_not_texture_ordered` | [x] |
| 9 | `spritebatch_internal_sprite_less_than_or_equal` | `sort_bits` at signed extremes `INT_MIN` / `INT_MAX`, and mixed-sign pairs | the comparison is a **signed** `int` compare (`cmp`/`jg` in the disassembly), so `INT_MIN < 0 < INT_MAX`; no unsigned wraparound. | `err_09_sort_bits_signed_extremes` | [x] |
| 10 | `spritebatch_internal_sprite_less_than_or_equal` | `texture_id` at `0` / `u64::MAX` | `texture_id` is `unsigned long long`, compared unsigned — but only in the dead branch, so it can never affect the result. Passing extreme `texture_id`s must change nothing about the ordering. | `err_10_texture_id_extremes_do_not_affect_order` | [x] |
| 11 | `merge_sort` | `size < 0` (e.g. `-1`, `INT_MIN`) | `sizeof(...) * size` widens the `int` via `cltq` then `shl $0x4`, producing a huge **unsigned** `size_t` (`-1` → `0xFFFF_FFFF_FFFF_FFF0`). `memcpy` of that length faults. **Undefined behaviour / guaranteed SIGSEGV in the C.** The Rust reproduces the identical wrapping multiplication (`SPRITE_SIZE.wrapping_mul(size as usize)`), asserted by computing the byte count in both, since the call itself cannot be made in-process. | `err_11_negative_size_byte_count_matches` (arithmetic parity; the faulting call is deliberately **not** invoked) | [x] |
| 12 | `merge_sort` | oversized `size` — larger than the caller's actual allocation | no bounds check exists; C reads/writes out of bounds (UB). Not exercised: it is UB in the C and would corrupt the test process. Documented as out of scope, same as row 11. | n/a (UB in C, not invokable) | [x] |
| 13 | `merge_sort` | out-of-range *enum* value across the FFI boundary | **No enum type exists anywhere in the public API** (`grep -c enum c_src` → 0). The only scalar parameter is `int size`, whose full `i32` range is covered by rows 1, 4, 11 and the Phase B `size` axis. Nothing to test. | n/a (no enums in API) | [x] |
| 14 | `merge_sort` | uninitialised / non-zero **padding** bytes in the input structs | both the `memcpy` and the per-element struct assignment (`mov (%rax),%rax; mov 0x8(%rax),%rdx; …` — a full 16-byte copy) carry the 4 tail-padding bytes across. Padding is therefore observable in the output and must match. | `err_14_padding_bytes_propagate` | [x] |

Rows 11–13 are the only rows without an executed differential call, and each is
justified in place: 11 and 12 are inputs on which the C is guaranteed to fault
(the parity that *can* be checked — the byte-count computation — is checked),
and 13 describes an input class that does not exist in this API.

## Verification record

`tests/phase_c_error_paths.rs` — 14 tests, one per row (row 8 has two), all
passing in debug and release, and against the C compiled at `-O0` … `-O3`/`-Os`.

Because `merge_sort` is `void` and the C has no error codes, each test asserts
the strongest available equivalent: identical byte-for-byte side effects on both
buffers **and** identical crash-or-not-crash behaviour. Three tests go further
than plain comparison and pin the C's actual quirky behaviour, so a future
"cleanup" of the Rust that also changed the expectation would still fail:

* `err_08_dead_texture_id_tiebreak` asserts the C really does leave
  equal-`sort_bits` elements in place (the dead branch is dead).
* `err_10_texture_id_extremes_do_not_affect_order` re-runs the C with the
  `texture_id`s replaced and asserts the chosen permutation is unchanged.
* `err_14_padding_bytes_propagate` asserts the scratch fill never survives in an
  output element, i.e. the C really does copy the 4 padding bytes.

`err_06_right_run_exhausted_no_oob_read` additionally places both buffers flush
against an `mprotect(PROT_NONE)` page, so a read of element `hi` would
`SIGSEGV`; both libraries return normally, confirming the `||` short-circuit.

`err_05_no_out_of_range_writes` and `err_13_no_enums_in_public_api` surround the
live range with guard elements and assert nothing outside `[0, size)` is written
by either library, for every `size` from 0 to 64.
