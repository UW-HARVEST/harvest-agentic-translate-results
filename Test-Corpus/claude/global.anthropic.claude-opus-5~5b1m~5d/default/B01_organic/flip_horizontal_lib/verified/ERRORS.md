# ERRORS.md — Error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` (19 lines) and `c_src/include/lib.h`.

## Mechanical grep result

```
$ grep -nE 'return|assert|NULL|error|ERROR|if|switch' c_src/src/lib.c
(no matches other than the two `for` loop conditions)
```

**The C function performs ZERO explicit validation.** Concretely it contains:

* no `return` statement (it is `void`),
* no `assert`,
* no null check on `img` or on `img->pix`,
* no range check on `w` or `h`,
* no error enum, no error code, no sentinel value, no out-parameter status,
* no min/max constants,
* **no enums at all** in the public header, so there is no
  "out-of-range enum value across the FFI boundary" case to construct
  (documented here so the absence is a finding, not an oversight).

Consequently the entire "rejection surface" is *implicit*: it consists of the
two loop guards (`i < flips` and `j < w`), which turn invalid/degenerate
dimensions into a silent no-op rather than an error. Each distinct way a guard
can reject work gets its own row below, plus the generic FFI boundary cases.

The observable "result" for every row is therefore **(a) the callee does not
modify the pixel buffer, and (b) it does not modify `img->w/h/pix`**. The tests
assert both, byte-for-byte, against the C.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `flip_horizontal` | `h == 0` → `flips = 0/2 = 0`, outer guard `0 < 0` false | no-op; buffer and `img` unchanged | [x] |
| 2 | `flip_horizontal` | `h == 1` → `flips = 1/2 = 0`, outer guard false | no-op; buffer and `img` unchanged | [x] |
| 3 | `flip_horizontal` | `h < 0` (e.g. `-1`) → `flips = -1/2 = 0` (C truncates toward zero), outer guard false | no-op | [x] |
| 4 | `flip_horizontal` | `h < 0` even (e.g. `-8`) → `flips = -4`, outer guard `0 < -4` false | no-op | [x] |
| 5 | `flip_horizontal` | `h == INT_MIN` → `flips = INT_MIN/2 = -1073741824` (exact, no overflow), outer guard false | no-op | [x] |
| 6 | `flip_horizontal` | `w == 0`, `h >= 2` → outer loop runs `h/2` times, inner guard `0 < 0` false every time | outer loop spins, zero swaps; buffer unchanged | [x] |
| 7 | `flip_horizontal` | `w < 0` (e.g. `-1`), `h >= 2` → row pointers `pix + w*i` / `pix + w*(h-i-1)` are computed out of bounds but never dereferenced; inner guard `0 < w` false | no swaps; buffer unchanged (no fault, because the OOB pointers are only computed) | [x] |
| 8 | `flip_horizontal` | `w == INT_MIN`, `h == 2` → `off_b = INT_MIN*1` wraps; pointer computed, never dereferenced | no swaps; buffer unchanged | [x] |
| 9 | `flip_horizontal` | both degenerate: `w == 0 && h == 0` **and `pix == NULL`** → neither loop body runs, so the null `pix` is never dereferenced | no-op, no fault (null `pix` tolerated when no work is due) | [x] |
| 10 | `flip_horizontal` | `w == 0 && h == 4` with `pix == NULL` → outer loop runs, inner never does, null `pix` never dereferenced | no-op, no fault | [x] |
| 11 | `flip_horizontal` | `h < 0` with `pix == NULL` → outer guard false immediately | no-op, no fault | [x] |
| 12 | `flip_horizontal` | `img == NULL` → the very first statement `img->pix` dereferences address 0 | **SIGSEGV** (no check exists to prevent it) — tested for *signal parity* in a forked child, not merely "both failed" | [x] |
| 13 | `flip_horizontal` | `img->pix == NULL` while `w >= 1 && h >= 2` (work IS due) → inner loop dereferences address 0 | **SIGSEGV** — tested for signal parity in a forked child | [x] |

Rows 1–11 are *silent no-ops* and are asserted as exact buffer+struct equality
between C and Rust. Rows 12–13 are *faults*; they are compared by re-executing
the test binary in a child process and asserting both libraries die from the
**same signal** (not merely that both died).

## Deliberately excluded: genuine undefined behaviour with no defined result

The following inputs make the C invoke UB in a way that has **no observable
result to compare** (it reads/writes wild addresses, so any "match" would be
accidental). They are recorded for completeness and are *not* checkable rows:

| trigger | why excluded |
|---------|--------------|
| `w*h` larger than the actual `pix` allocation | C reads/writes past the buffer; result is whatever memory follows. Not a defined value either side can be required to reproduce. |
| `w > 0`, `h >= 2`, `w * i` overflowing `int` (needs `w` ≳ 2³⁰ *and* a matching multi-GB allocation to be dereferenceable) | signed overflow is UB in C and the required allocation is not constructible in a test. The Rust uses `wrapping_mul`, matching the usual two's-complement codegen. |

Note that row 7/8 are *not* in this excluded set: there the out-of-bounds row
pointer is only ever **computed**, never dereferenced (the inner guard rejects
first), so the observable behaviour — a clean no-op — is well-defined and is
tested.
