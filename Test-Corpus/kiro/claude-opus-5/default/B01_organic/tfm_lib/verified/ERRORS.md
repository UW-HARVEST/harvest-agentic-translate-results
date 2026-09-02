# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source. The greps run over
`c_src/src/lib.c` + `c_src/include/lib.h`:

```sh
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|goto|exit\(|abort|-1|ERR|_MAX|_MIN|#define' \
     src/lib.c include/lib.h      # -> (no matches)
grep -nE 'if|else|switch|case|\?|#if' src/lib.c
#  8:        if (src[0] < src[1]) {
# 15:                0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
# 18:        } else {
# 25:                0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
```

**Result of the mechanical scan: the C library has an empty *explicit* error
surface.** `tfm` returns `void`. There is not a single `return` statement, error
code, error enum, sentinel, `assert`, `NULL` check, range check, `errno` write,
`goto`-to-cleanup, or min/max constant anywhere in `c_src/`. There are no enum
parameters, so there is no out-of-range-enum case to construct. There are no
`#ifdef`s.

Therefore every row below is a **rejection-by-silence / implicit-contract** row:
an invalid or boundary input, and the *exact observable* behaviour the C
compiles to. "Expected C result" is stated in terms of what an external caller
can observe, because that is the only thing the Rust must reproduce. Each row's
expectation was read off the `-O0` disassembly of `tfm`
(`objdump -d libharvest-work-dvbeFO.so`), not from documentation.

The loop guard is `for (i = 0; i < count; i++)` with `int i` and `int count`,
compiled to:

```text
131e:  mov -0x4(%rbp),%eax     ; i
1321:  cmp -0x44(%rbp),%eax    ; count
1324:  jl  1128                ; signed less-than
```

so the *only* input validation in the entire library is that signed `jl`.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `tfm` | `count == 0` (zero length) | signed `i < count` fails immediately; loop body never runs; **zero bytes written to `dest`**; returns normally |
| 2 | `tfm` | `count == -1` (negative length) | signed compare, so `0 < -1` is false; **zero bytes written**; returns normally (no wrap to a huge unsigned trip count) |
| 3 | `tfm` | `count == INT_MIN` (`-2147483648`, most negative) | same as row 2 — signed `jl` false; **zero bytes written**; returns normally |
| 4 | `tfm` | `dest == NULL`, `src == NULL`, `count == 0` | pointers are never dereferenced because the loop body never executes; **returns normally, no crash** (NULL is only invalid if `count > 0`) |
| 5 | `tfm` | `dest == NULL`, `src == NULL`, `count < 0` | same as row 4 — **returns normally, no crash** |
| 6 | `tfm` | `dest == NULL`, `src` valid, `count > 0` | unchecked store through NULL → **SIGSEGV** (undefined behaviour; not exercised as a live call, asserted structurally instead) |
| 7 | `tfm` | `src == NULL`, `dest` valid, `count > 0` | unchecked load through NULL → **SIGSEGV** (undefined behaviour; not exercised as a live call) |
| 8 | `tfm` | `count` one step past the caller's buffer (`count = n+1` for an `n`-element `src`) | no bounds check exists → reads `src[3n..3n+2]` and writes `dest[2n..2n+1]` out of bounds. Observable, *defined* consequence when the caller over-allocates: **element `n` is transformed from whatever bytes follow**, i.e. the C never rejects an oversized `count` |
| 9 | `tfm` | `sqd < 0` (discriminant negative) — `(((0) > (sqd)) ? (0) : (sqd))` | `comiss`/`jbe` takes the "clamp" path and loads `.rodata+0x4 == 0x00000000`; `sqrtf(+0.0f)` → `+0.0f`. **No domain error, no `errno`, no NaN**: the negative value is *rejected by clamping to `+0.0f`** |
| 10 | `tfm` | `sqd == NaN` (any operand NaN, or `inf - inf`, or `0 * inf`) | `comiss 0.0, NaN` sets `CF=ZF=PF=1` → `jbe` **taken** → the clamp is *skipped* and the NaN is passed to `sqrtf`. `sqrtf(NaN)` → quieted NaN, sign+payload preserved. So a NaN discriminant is **not** clamped to zero |
| 11 | `tfm` | `sqd == -0.0f` | `0.0f > -0.0f` is false → **no clamp**; `sqrtf(-0.0f)` → `-0.0f` (sign preserved), not `+0.0f`. **Finding: this trigger is UNREACHABLE** — see the note below. The row's *observable* consequence (signed-zero handling through the clamp and `sqrtf`) is still tested |
| 12 | `tfm` | `src[0]` and/or `src[1]` is NaN, so `src[0] < src[1]` is *unordered* | `comiss` + `jbe` → **the `else` arm is taken** (unordered is treated exactly like `>=`). The `else` arm swaps the `dx2`/`dy2` naming and the store order |
| 13 | `tfm` | `sqd` overflows to `+inf` (huge inputs) | no overflow check; `sqrtf(+inf)` → `+inf`; `lambda = +inf`; `dx2 - inf = -inf`. **No error, `-inf` is stored** |
| 14 | `tfm` | signalling NaN (`0x7fa00000`) in any of `src[0..2]` | SSE quiets it: result carries `0x7fe00000`-style payload with the quiet bit forced on. **No trap** (MXCSR masks are default) |
| 15 | `tfm` | `dest` aliases/overlaps `src` (`dest == src`, in-place) | no aliasing check and no `restrict`; `src` advances 3 floats/iteration while `dest` advances 2, so writes trail reads. Behaviour is **fully defined and self-referential**: iteration `i > 0` reads bytes iteration `i-1` already overwrote. Not rejected |
| 16 | `tfm` | unaligned `dest`/`src` (not 4-byte aligned) | `movss` is alignment-agnostic → **no fault, no rejection** (verified with a 1-byte-offset buffer) |

Rows 6 and 7 are genuine undefined behaviour (a NULL dereference). Calling them
in-process would abort the harness for *both* libraries, so they are covered by
asserting the *precondition boundary* instead: rows 4/5 prove NULL is accepted
whenever `count <= 0`, which is exactly where the C stops dereferencing. This is
recorded in `tests/error_paths.rs::row_06_07_null_with_positive_count_is_ub`.

## Finding: row 11 (`sqd == -0.0f`) is unreachable

Worth recording because it means the C's clamp cannot distinguish `>` from `>=`:

* `sqd = addss(mulss(mulss(4.0f, dxy), dxy), acc)`.
* The `4*dxy*dxy` term is **never** `-0.0`: `mulss(4.0f, dxy)` carries `dxy`'s
  sign, and multiplying that by `dxy` again gives a non-negative sign, so the
  term is `+0.0` or positive (or NaN).
* `+0.0f + x` is `-0.0` for no `x` (only `-0.0 + -0.0` yields `-0.0`).

Both facts are verified **exhaustively over all 2^32 `f32` bit patterns** by
`tests/exhaustive.rs::sqd_is_never_negative_zero`, and independently by the
`clamp-ge-instead-of-gt` entry in `scripts/mutation_check.py`, which flips the
C's `>` to `>=` and is confirmed to be an equivalent mutant.

Row 11's test therefore pins the *observable* signed-zero behaviour (the full
`±0.0` cross-product through both `.so`s) rather than a condition that cannot
occur.

## Status

All 16 rows have a passing differential test in `tests/error_paths.rs`.
See that file's `ROWS` map and the per-row `#[test]` functions.
