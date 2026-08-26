# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep results

```
$ grep -nE 'return|RETURN|goto'        c_src/src/lib.c c_src/include/lib.h   -> (none)
$ grep -nE 'assert|abort|exit\('       c_src/src/lib.c c_src/include/lib.h   -> (none)
$ grep -nE 'NULL|nullptr|!= *0|== *0'  c_src/src/lib.c c_src/include/lib.h   -> (none)
$ grep -nE 'enum|switch|#ifdef|#if |errno' c_src/src/lib.c c_src/include/lib.h -> (none)
$ grep -nE 'if|for|while|\?'           c_src/src/lib.c
    8:    for (int i = 0; i < flips; ++i) {
   11:        for (int j = 0; j < w; ++j) {
```

**The library has NO error-return mechanism at all.** `flip_horizontal` returns
`void`; there is no error macro, no error enum, no sentinel return, no `assert`,
no null check, no explicit range check, and no min/max constant. Consequently
every "rejection" this library performs is implicit and takes exactly one of two
forms:

* **(S) silent no-op** — a loop guard (`i < flips` or `j < w`) is false on the
  first evaluation, so the function returns having written nothing and having
  left `*img` untouched.
* **(V) fatal memory fault** — the C unconditionally dereferences a pointer, so
  an invalid pointer terminates the process with `SIGSEGV`.

"Same error/rejection" for this API therefore means: the same one of {S, V},
and for (S) a byte-identical, completely unmodified buffer + unmodified
`cp_image_t`, and for (V) the same terminating signal.

There are **no enums in the public API**, so there is no out-of-range enum
value to pass across the FFI boundary. The only "enum-like" inputs are the two
`int` fields `w` and `h`, whose entire `int` range is exercised below
(including `INT_MIN` / `INT_MAX`, which have no "valid variant" meaning).

## Error-surface table

Every row is a distinct condition under which the C source refuses to do work
or faults. Rows 1-3 are the two dereference sites (`img->…` at lib.c:4-6, and
`*a` / `*b` at lib.c:12-14). Rows 4-13 are the two loop guards (lib.c:8 and
lib.c:11) evaluating false on entry, enumerated over every distinct way each
guard can be falsified.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|----------------------------------------------|-------------------|------|-----|
| 1  | `flip_horizontal` | `img == NULL` — lib.c:4 dereferences `img->pix` with no null check | (V) `SIGSEGV` | `err01_null_img_segv` | [x] |
| 2  | `flip_horizontal` | `img->pix == NULL`, `w >= 1`, `h >= 2` — outer+inner loops entered, lib.c:12 dereferences `*a` at address 0 | (V) `SIGSEGV` | `err02_null_pix_with_work_segv` | [x] |
| 3  | `flip_horizontal` | `img->pix` non-null but buffer shorter than `w*h`: **not an error the C detects** — no bounds check exists, C writes out of bounds | out-of-bounds write (undefined); Rust must write the *same* addresses. Verified with an oversized buffer + a poison canary (row 20 of CONFIGS.md) | `cfg_padding_canary` | [x] |
| 4  | `flip_horizontal` | `h == 0` → `flips = 0/2 = 0`, guard `0 < 0` false (lib.c:8) | (S) no-op | `err04_h_zero` | [x] |
| 5  | `flip_horizontal` | `h == 1` → `flips = 1/2 = 0`, guard false (lib.c:8) | (S) no-op | `err05_h_one` | [x] |
| 6  | `flip_horizontal` | `h == -1` → `flips = -1/2 = 0` (C truncates toward zero), guard false | (S) no-op | `err06_h_neg_one` | [x] |
| 7  | `flip_horizontal` | `h == -2` → `flips = -1`, guard `0 < -1` false | (S) no-op | `err07_h_neg_two` | [x] |
| 8  | `flip_horizontal` | `h < 0` generally (randomized negatives) → `flips < 0`, guard false | (S) no-op | `err08_h_negative_random` | [x] |
| 9  | `flip_horizontal` | `h == INT_MIN` → `flips = -1073741824`, guard false (note: `INT_MIN/2` does not overflow) | (S) no-op | `err09_h_int_min` | [x] |
| 10 | `flip_horizontal` | `w == 0`, `h >= 2` → outer loop runs, inner guard `0 < 0` false every iteration; no deref of `pix` at all | (S) no-op (even with `pix == NULL`) | `err10_w_zero` | [x] |
| 11 | `flip_horizontal` | `w == -1`, `h >= 2` → inner guard `0 < -1` false; `pix + w*i` computes an out-of-range pointer that is never dereferenced | (S) no-op | `err11_w_neg_one` | [x] |
| 12 | `flip_horizontal` | `w` large negative (e.g. `-(1<<20)`), `h >= 2` → pointer arithmetic wraps far below the allocation, never dereferenced | (S) no-op, no fault | `err12_w_large_negative` | [x] |
| 13 | `flip_horizontal` | `w == INT_MIN`, `h >= 2` → `w*i` signed-overflows in `int` (C: UB, gcc -O0: wraps); never dereferenced | (S) no-op, no fault | `err13_w_int_min` | [x] |
| 14 | `flip_horizontal` | `h == 1` combined with `pix == NULL` (guard short-circuits before any `pix` use) | (S) no-op | `err14_null_pix_h_one` | [x] |
| 15 | `flip_horizontal` | `h == INT_MAX`, `w == 0` → `flips = 1073741823` outer iterations, every inner guard false | (S) no-op after ~2^30 empty iterations | `err15_h_int_max_w_zero` | [x] |
| 16 | `flip_horizontal` | `w == INT_MAX`, `h == 0` → guard on `flips` false before `w` is ever used | (S) no-op | `err16_w_int_max_h_zero` | [x] |
| 17 | `flip_horizontal` | `w` one step past "empty": `w == 1, h == 2` (smallest input that does real work) — boundary between (S) and work | 1 swap of 1 pixel | `err17_smallest_working_input` | [x] |
| 18 | `flip_horizontal` | `h` one step past "empty": `h == 2` vs `h == 1` (boundary of `flips` becoming non-zero) | `h==1` no-op, `h==2` one swap | `err18_h_boundary_one_vs_two` | [x] |
| 19 | `flip_horizontal` | all four `(w, h)` sign combinations incl. both negative | (S) no-op for every combination where `w<=0` or `h<=1` | `err19_sign_matrix` | [x] |

## Generic FFI boundaries (required even though the table above has no rows for them)

| trigger | expected C result | test | [x] |
|---|---|---|---|
| out-of-range "enum" values | **no enums exist in this API.** The only `int` inputs are `w`/`h`, whose entire `i32` range (2000 randomized bit patterns, plus `INT_MIN`/`INT_MAX`) is swept | `generic_full_int_range_sweep` | [x] |
| zero-length buffer with a non-null dangling-aligned pointer | no-op for every `(w,h)` where a guard fails | `generic_zero_length_buffer` | [x] |
| misaligned `cp_image_t *` (offsets 1..7) | plain unaligned `int` loads; works on x86-64 | `generic_misaligned_image_struct` | [x] |
| `pix` in the middle of a bigger allocation | writes confined to `[pix, pix+w*h)` | `generic_pix_offset_into_buffer` | [x] |
| oversized length (`w`/`h` far larger than the buffer) | no check exists; identical out-of-bounds footprint | `err03` / `cfg_padding_canary` | [x] |
| repeated invocation on rejected shapes | still a no-op | `generic_repeated_calls_on_noop_shapes` | [x] |

## A real divergence this table caught

Row 1 initially **FAILED**: C died with `SIGSEGV` (11) but the Rust died with
`SIGABRT` (6). Cause: rustc instruments a *place-expression* dereference
(`(*img).pix`) with a null/alignment check whenever `-C debug-assertions` is on;
the resulting panic cannot unwind out of an `extern "C"` fn, so it aborts. Fixed
by loading the three header fields through `core::ptr::read(&raw const …)`,
which performs the same plain hardware load the C does — and which is already
how the pixel loads in the loop behave (this is why row 2 passed from the
start). Both rows now report `Signal(11)` for both implementations.

Similarly, `pix + w*i` was translated with `wrapping_offset`/`wrapping_add`
rather than `offset`/`add`, because the latter carry a checked
"must not wrap the address space" precondition that would abort on the
large-negative-`w` inputs of rows 12/13 that C accepts silently.

## Notes on rows 1-2 (the fatal ones)

Both are verified by re-executing the test binary as a child process
(`std::process::Command`) so that the C and the Rust `.so` each get their own
process, then comparing `ExitStatus::signal()`. Asserting "both died with
signal 11" is a comparison of the *same* rejection, not merely "both failed".
