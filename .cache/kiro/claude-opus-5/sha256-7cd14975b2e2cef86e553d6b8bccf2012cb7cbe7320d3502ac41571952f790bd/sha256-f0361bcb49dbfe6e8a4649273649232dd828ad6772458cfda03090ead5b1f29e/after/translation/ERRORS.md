# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical extraction

```
$ grep -n 'return'                       src/lib.c include/lib.h
src/lib.c:23:    return 0;

$ grep -nE 'assert|RETURN_ERROR|ERROR|NULL|errno|-1|goto|if *\(' src/lib.c include/lib.h
(no matches — exit 1)

$ grep -nE 'while|if|\?|switch|#if'      src/lib.c
src/lib.c:11:    while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
src/lib.c:13:        b = b > bits ? bits : b;

$ grep -noE '[0-9]+' src/lib.c | sort -u -t: -k2
5:1                       # mask shift amount
5:18446744073709551615    # UINT64_MAX, base of `mask`
8:8                       # 8 * sizeof(tflac_uint)
10:0                      # int i = 0
11:100                    # loop iteration cap
23:0                      # the sole return value
```

## Findings

`bitwriter_add` performs **no validation whatsoever**:

* no `assert`
* no null-pointer check on `bw`
* no range check on `bits` (the header documents no range; nothing rejects
  `bits == 0`, `bits > 64`, or `bits == UINT32_MAX`)
* no bounds check against `bw->len` / `bw->pos` (those fields are never read
  or written by this function)
* no error enum, no `-1`, no `NULL` return, no `errno`
* exactly one `return` statement: `return 0`

The only guard in the function is the `i < 100` loop cap, which is a
*termination* bound, not an input rejection: it silently truncates the
bit-consuming loop instead of signalling an error.

Therefore the "rejection" rows below are the *complete* set of ways the C code
can be fed invalid/out-of-range input, together with what the C **actually
does** (which is the contract the Rust must reproduce). Each row is covered by
a differential test in `tests/differential.rs`.

| #  | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|----|----------|------------------------------------------|-------------------|------|-----|
| 1  | `bitwriter_add` | any input at all — the sole `return` is `return 0` (`src/lib.c:23`); no branch can return anything else | returns `0` unconditionally; `bw` mutated in place | `err_row01_return_is_always_zero` | [x] |
| 2  | `bitwriter_add` | `bits == 0` → `val <<= (64 - 0)` = shift-by-64, out of range for `uint64_t` (UB in C; `shlq %cl` masks count to 6 bits ⇒ shift by 0) | no rejection; `val` unshifted; `tot += 0`; loop entered iff `bw->bits >= 64` | `err_row02_bits_zero` | [x] |
| 3  | `bitwriter_add` | `bits == 64` → `val <<= 0`, boundary of the documented width | no rejection; returns `0` | `err_row03_bits_exactly_64` | [x] |
| 4  | `bitwriter_add` | `bits == 65` — one step past the maximum meaningful width (`8*sizeof(tflac_uint)`) | no rejection; `64 - 65` wraps in `unsigned int` ⇒ shift count `& 63 == 63`; returns `0` | `err_row04_bits_65_one_past_range` | [x] |
| 5  | `bitwriter_add` | `bits` grossly oversized: `100`, `128`, `4096`, `0x7FFFFFFF`, `0x80000000`, `UINT32_MAX` | no rejection; wrapped shift counts and wrapped `bits -= b`; returns `0` | `err_row05_bits_oversized` | [x] |
| 6  | `bitwriter_add` | `bw->bits` already `== 64` (accumulator "full", out of the `0..=63` range the algorithm implies) | no rejection; `b = 63 - 64` wraps to `0xFFFFFFFF`, clamped to `bits` by the ternary | `err_row06_bw_bits_at_64` | [x] |
| 7  | `bitwriter_add` | `bw->bits` out of range: `65`, `0xFFFFFFFF`, and values making `63 - bw->bits` wrap | no rejection; wrapped `b`; `>> (bw->bits & 63)` | `err_row07_bw_bits_out_of_range` | [x] |
| 8  | `bitwriter_add` | `bw->bits + bits` overflows `tflac_u32` (e.g. `bw->bits = 0xFFFFFFFF`, `bits = 1` ⇒ sum `0`) so the `>= 64` loop test is evaluated on the wrapped sum | no rejection; loop **not** entered; returns `0` | `err_row08_bw_bits_plus_bits_u32_wrap` | [x] |
| 9  | `bitwriter_add` | `bw->tot + bits` overflows `tflac_u32` (`tot` near `UINT32_MAX`) | no rejection; `tot` wraps modulo 2^32 | `err_row09_tot_u32_wrap` | [x] |
| 10 | `bitwriter_add` | loop cap `i < 100` (`src/lib.c:11`) reached — input that would need >100 iterations | loop exits with `bits` still non-zero; remaining bits merged by the trailing `bw->val \|= val >> bw->bits`; returns `0` | `err_row10_loop_cap_100` | [x] |
| 11 | `bitwriter_add` | `bits -= b` underflows `tflac_u32` inside the loop (`b > bits` is clamped, so this needs the wrapped-`b` path) | no rejection; `bits` wraps modulo 2^32 | `err_row11_bits_minus_b_underflow` | [x] |
| 12 | `bitwriter_add` | `bw == NULL` — no null check exists; the first access is `bw->tot` (a write) | both implementations dereference null ⇒ same fatal signal (`SIGSEGV`) | `err_row12_null_pointer_parity` | [x] |
| 13 | `bitwriter_add` | `val == 0` and `val == UINT64_MAX` (extreme value operands, incl. all bits above `bits` set) | no rejection; returns `0` | `err_row13_val_extremes` | [x] |
| 14 | `bitwriter_add` | garbage in the fields this function never validates or uses (`bw->pos`, `bw->len`, `bw->buffer` = wild pointer) | no rejection, no deref of `buffer`; `pos`/`len`/`buffer` left byte-identical | `err_row14_unused_fields_untouched` | [x] |

### Out-of-range enum values

`include/lib.h` declares **no `enum` types** — the only parameters are
`tflac_u32 bits` and `tflac_uint val`, both plain integers, and every value of
both is exercised as an "out-of-range" input by rows 2–5 and 13. There is
therefore no enum-with-no-valid-variant case to cross the FFI boundary.

## Divergence found and fixed

**Row 12 (NULL `bw`) initially FAILED.** The C died with `SIGSEGV` (11); the
Rust died with `SIGABRT` (6).

Cause: the translation opened with

```rust
let bw: &mut tflac_bitwriter = unsafe { &mut *bw };
```

Forming a Rust reference from the raw pointer — and, once that was removed, the
plain `(*bw).tot` raw dereference in a `debug_assertions` build — makes rustc
emit a UB check that reports *"null pointer dereference occurred"* as a
non-unwinding panic, i.e. `abort()`. The C has no null check at all and simply
faults on its first access to `bw->tot`.

Fix (`src/lib.rs`): never form a reference from the parameter; operate through
raw-pointer places `(*bw).field` only. Verified that the shipped release `.so`
and the C `.so` now both terminate with `SIGSEGV` under a neutral C `dlopen`
driver as well as under the Rust test.

## Robustness note (behaviour is not compiler-dependent)

`bitwriter_add` shifts a `uint64_t` by counts that can reach or exceed 64
(`val <<= 64 - bits` with `bits == 0`, `val >> bw->bits` with `bw->bits >= 64`),
which is UB in C. The generated code uses `shlq %cl` / `shrq %cl`, which mask
the count to its low 6 bits; the Rust reproduces this with explicit `& 63` in
`c_shl_u64` / `c_shr_u64`.

To confirm this is not an artifact of the unoptimized cmake build, the C source
was additionally compiled at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` and each
resulting `.so` was compared against the Rust release `.so` through a neutral C
`dlopen` driver over **560,000 cases each** (a full cross-product of the
interesting `bw->bits` × `bits` boundary values plus 400,000 fully random
cases): **0 mismatches at every optimization level.**
