# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.
Tests live in `tests/phase_c_errors.rs`; the harness is `tests/common/mod.rs`.

## Mechanical grep of the C source

```
$ grep -nE 'return|assert|NULL|<=|>=|if' c_src/src/lib.c
```

* `return 1;` x2, `return 0;` x1 — in `spritebatch_internal_sprite_less_than_or_equal`
* `return;` x1 — the `hi - lo <= 1` early-out in `spritebatch_internal_merge_sort_recurse`
* `assert` — **0 occurrences**
* `NULL` / null checks — **0 occurrences**
* explicit range / bounds checks — **0 occurrences**
* error enums, error codes, `errno`, `RETURN_ERROR`-style macros — **0 occurrences**
* min/max constants — **0 occurrences**

**Consequence:** the public API is `void merge_sort(spritebatch_sprite_t *a,
spritebatch_sprite_t *b, int size)`. It has *no error channel whatsoever*: no
return value, no out-parameter status, no sentinel, no assertion. Every row below
is therefore either a *rejection-by-early-return* or an *undefined-behaviour /
fault* boundary, and the differential assertion is "same observable effect"
(identical memory state, or identical fatal outcome at the identical faulting
address) rather than "same error code".

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `merge_sort` | `size == 0`, `a`/`b` valid non-null | `memcpy(b,a,0)` touches nothing; `recurse(b,0,0,a)` hits `hi-lo == 0 <= 1` and returns. **Both buffers bit-for-bit unchanged.** | `err01_size_zero_valid_pointers_is_a_noop` | [x] |
| 2 | `merge_sort` | `size == 0` with `a == NULL` and/or `b == NULL` | `memcpy(NULL,NULL,0)` performs no access; early-out as row 1. **Returns normally, no fault** (asserted as `Exited(0)` for both). | `err02_size_zero_null_pointers_returns_normally` | [x] |
| 3 | `merge_sort` | `size == 1` | `memcpy` copies exactly 16 bytes `a -> b`; `recurse` early-outs (`hi-lo == 1 <= 1`). `a` untouched, `b[0] == a[0]`, `b[1..]` untouched. Element 1 never dereferenced. | `err03_size_one_copies_only_one_element` | [x] |
| 4 | `spritebatch_internal_merge_sort_recurse` (via `merge_sort`) | `hi - lo <= 1` — the only guard in the file; reached at every leaf of the recursion | `return;` immediately. Nothing outside `[0, size)` is ever written (asserted with guard elements). | `err04_recursion_leaf_guard` | [x] |
| 5 | `spritebatch_internal_sprite_less_than_or_equal` | `a->sort_bits > b->sort_bits` — the only path that reaches `return 0` | returns `0`, so the merge takes its `else` branch. **NOTE: the second `if` (`sort_bits == ... && texture_id <= ...`) is DEAD CODE** — the first `if` already returns 1 when the keys are equal, so `texture_id` NEVER influences ordering. Asserted directly: swapping only the texture ids does not change the output order. | `err05_predicate_reject_path_and_dead_branch` | [x] |
| 6 | `spritebatch_internal_sprite_less_than_or_equal` | extreme signed operands: `INT_MIN` vs `INT_MAX` and every pair from `{INT_MIN, INT_MIN+1, -1, 0, INT_MAX-1, INT_MAX}` | plain signed `<=`, no subtraction, therefore no overflow: `INT_MIN <= INT_MAX` -> 1, `INT_MAX <= INT_MIN` -> 0 | `err06_predicate_extreme_signed_operands` | [x] |
| 7 | `merge_sort` | `size < 0` (`-1, -2, -3, -4, -7, -8, -15, -16, -17, -31, -32, -100, -127, -128, -255, -256, -257, -1000, -4096, -65535, -65536, -2^20, -2^24`, plus 64 random negatives) | `sizeof(t) * (size_t)size` = `16 * (2^64 + size)` mod 2^64 = an astronomically large byte count, which `memcpy` then runs off the end of the buffer with. See "the two regimes" below — this is UB, and the *outcome* depends on the address space, so the assertion is on the exact faulting address. | `err07_negative_size` | [x] |
| 8 | `merge_sort` | `size > 0` with `a == NULL` and/or `b == NULL` (null pointer, non-zero length) | `memcpy` dereferences NULL -> **`SIGSEGV` (signal 11)**, asserted to be the same signal in both, and asserted non-vacuously (the C really does die). | `err08_null_pointers_with_nonzero_size_are_fatal_in_both` | [x] |
| 9 | `merge_sort` | `size` larger than the caller's logical element count (`logical=0..7`, `size=8..128`) | out-of-range read+write; made deterministic by running inside a 128-element over-allocated arena, so both implementations must produce identical bytes across the whole arena. | `err09_oversized_size_within_overallocated_arena` | [x] |
| 10 | `merge_sort` | `a == b` (aliased buffers), sizes `0..=24` | `memcpy` with `dst == src` (UB per the standard, idempotent in glibc), then the merge reads and writes the same storage. Must match byte-for-byte. | `err10_aliased_buffers` | [x] |
| 11 | `merge_sort` | `size == INT_MIN` (and `INT_MIN+1`, `INT_MIN+2`, `INT_MIN+16`, `INT_MIN/2`) | `(size_t)INT_MIN * 16` wraps to `0xFFFFFFF800000000` -> huge count -> `memcpy` faults. Same regime as row 7. | `err11_int_min_size` | [x] |
| 12 | `merge_sort` | out-of-range "enum"-like integer argument. The C prototype takes `int`, so **any** 32-bit value is a legal FFI input — exactly the class of "C enums accept any int" bugs. Covered: `INT_MAX`, `INT_MAX-1`, `2^30`, `2^24`, `2^20`, `10^8`, `65537/65536/4097/4096/1000/513/512/257/256/255`, 128 random `i32` values, and every `2^k - 1 / 2^k / 2^k + 1` for `k = 0..30` in both signs. Plus in-bounds boundary values `0, 1, 7, 8, 9, 10, 63, 64` on a real 64-element buffer. | `err12_arbitrary_int_argument` | [x] |

## The two regimes of a pathological `size` (rows 7, 11, 12)

A huge byte count handed to `memcpy` is undefined behaviour, and it was
**measured** that the C's own observable outcome is *not a function of its
arguments*: with an identical `size == -1` and identical buffer *contents*, the C
`.so` alone flips between `Exited(0)` and `SIGSEGV` depending only on the address
at which the buffers happen to be mapped.

```
C size=-1 -> Exited(0)@0x7f2bfcb5f000   Signaled(11)@0x7f2bfcb85000
             Signaled(11)@0x7f2bfcb8a000  Signaled(11)@0x7f2bfcb86000
```

(glibc's `memcpy` selects between forward, backward and non-temporal strategies
partly from `dst - src` and from the magnitude of `n`, and whether the resulting
access pattern hits an unmapped page depends on the surrounding mappings. The C
child and the Rust child necessarily have different layouts, because they load
different shared objects.)

A naive "did both crash?" assertion would therefore be testing the process
layout, not the translation. The tests instead make the observation deterministic:

* `a` and `b` are placed in a **single 4 KiB read/write window at the centre of a
  256 MiB `PROT_NONE` reservation**, so the *first* access outside the window
  faults immediately, at an address that is a pure function of the `(dst, src, n)`
  triple `merge_sort` passed to `memcpy`;
* the child installs a `SIGSEGV`/`SIGBUS` handler that records `si_addr` into
  shared memory and `_exit(42)`s;
* the C child runs first, its window and fault address are snapshotted, the
  window is restored to the seed bytes, and the Rust child then runs **on the
  identical addresses**;
* the assertion compares three observables: termination outcome, the **exact
  faulting address**, and every byte of the window.

Equal fault addresses prove that both implementations handed identical arguments
to the same `memcpy` — which *is* the translated logic under test. Sample of the
captured, matching values (window at `0x7f8fd0000000`, `b` at `+0x800`):

| size | C outcome / si_addr | Rust outcome / si_addr |
|------|---------------------|------------------------|
| `-1` | `Exited(42)` @ `0x7f8fcfffffb0` | `Exited(42)` @ `0x7f8fcfffffb0` |
| `-2` | `Exited(42)` @ `0x7f8fcfffffa0` | `Exited(42)` @ `0x7f8fcfffffa0` |
| `-255` | `Exited(42)` @ `0x7f8fcfffefd0` | `Exited(42)` @ `0x7f8fcfffefd0` |
| `INT_MIN` | `Exited(42)` @ `0x7f87cfffffc0` | `Exited(42)` @ `0x7f87cfffffc0` |
| `INT_MAX` | `Exited(42)` @ `0x7f97cfffffb0` | `Exited(42)` @ `0x7f97cfffffb0` |
| `255` | `Exited(42)` @ `0x7f8fd0001780` | `Exited(42)` @ `0x7f8fd0001780` |
| `3` (valid) | `Exited(0)`, no fault | `Exited(0)`, no fault |

This is why the Rust `merge_sort` calls libc's `memcpy` through an
`extern "C"` declaration rather than `ptr::copy_nonoverlapping`: only the real
`memcpy` reproduces these argument-dependent strategies, the `dst == src` case
and the `n == 0` + `NULL` case exactly.

## Divergences found and fixed during Phase C

| divergence | root cause | fix |
|---|---|---|
| `size == 0` / `a == b` / null-pointer edge cases risked differing | the Rust used `ptr::copy_nonoverlapping` with a hand-written `if bytes != 0 && a != b` guard, which is only *approximately* `memcpy` | `src/lib.rs` now declares and calls libc `memcpy` directly, exactly as the C does — no guard, no approximation |

## Generic FFI boundaries also covered (beyond the table)

* null pointers — rows 2 and 8 (with zero *and* non-zero length);
* zero length — rows 1 and 2;
* oversized length — rows 9 and 12;
* one step past a valid range — row 12 (`size = 9` on an 8-element buffer, `size
  = -1` just below `0`, `2^k ± 1` sweeps);
* out-of-range enum-style integer across FFI — row 12 (the whole `i32` domain);
* aliasing — row 10;
* both optimisation levels of the Rust `.so` (`release` **and** `debug`) — see
  `verify.sh`, because the behaviour being matched is codegen-sensitive.
