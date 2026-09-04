# ERRORS.md — error / rejection surface table

Mechanically derived from `c_src/`, then **verified against the compiled C
shared object** (every `SIGSEGV` below was reproduced with a standalone C driver
and is re-verified in `tests/errors.rs` by running the faulting call in an
isolated child process).

**The C library contains no error-handling machinery at all.** Grepping the whole
tree finds

* `0` occurrences of `assert`, `errno`, `RETURN_ERROR`, `goto`, `exit`,
* `0` `NULL` / range / size / divide-by-zero checks,
* `0` `enum` declarations — so there is no "out-of-range enum value" input class
  to cross the FFI boundary. The only non-pointer, non-float parameter is a
  plain `int` count, and every out-of-range `int` value is covered by rows 3–8.
* exactly `4` `return` statements in total (`total`, `dot_product`, and the two
  in `match`), and
* exactly `1` named constant: `#define N_SMOOTH 16`.

The rejection surface is therefore made of (a) the two value-based rejections
`match` performs and (b) the *implicit* degenerate/undefined paths the unguarded
code falls into.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `match`  | `total(test,bins) < threshold * total(reference,bins)` — energy gate rejects (`match.c:37`; compiled as `mulsd -0x70(%rbp),%xmm1` with `%xmm1 = total(reference)`, then `comisd -0x78(%rbp),%xmm0` / `jbe`) | `= 0`, and `preprocess` / `spectral_contrast` are **never called** | [x] |
| 2  | `match`  | gate passes but `spectral_contrast(t,r,bins) < threshold` (`match.c:40`, `comisd` + `setae`) | `= 0` | [x] |
| 3  | `match`  | **`bins == 0`** → `SIGSEGV`. The zero-length VLAs `t`/`r` are both placed at `align_up(%rsp, 8)` (size rounds to `0`, so `sub $0x0,%rsp`), and `differentiate`'s `v[length-1] = 0` becomes `v[-1]` = `%rsp - 8` — **exactly the slot holding the return address pushed by `call preprocess`**. `preprocess` then returns to address `0`. | `SIGSEGV` for *every* `threshold` (both totals of an empty array are `+0.0`, so the gate can never reject: `0.0 < ±0.0` and `0.0 < NaN` are both false) | [x] |
| 4  | `differentiate` | `length == 0`: loop bound `i < length - 1` is `0 < -1` → zero iterations; then the unconditional `v[length - 1] = 0` store (`lea -0x8(%rax),%rdx`) writes 8 bytes *below* the buffer | the out-of-bounds store itself; it is what makes row 3 fault | [x] |
| 5  | `match`  | **`bins < 0`** → `SIGSEGV`. Gate cannot reject (row 3), then `preprocess` calls `memcpy(v, source, (size_t)(bins * 8))` — `movslq` sign-extends `bins` and `lea 0x0(,%rax,8)` yields ≈ `2^64 - 8|bins|` | `SIGSEGV` | [x] |
| 6  | `match`  | **`bins` huge** (e.g. `10^9`, `INT_MAX`): two VLAs of `bins*8` bytes via `sub %rax,%rsp`, allocated *before* the gate is evaluated | `SIGSEGV` (stack exhaustion) | [x] |
| 7  | `match`  | **`bins == INT_MIN`**: `movslq` then `*8` wraps, `sub %rax,%rsp` moves `%rsp` by a nonsense amount | `SIGSEGV` | [x] |
| 8  | `match`  | `test == NULL` with `bins >= 1` → `total` dereferences `v[0]` (`movsd (%rax),%xmm0`) | `SIGSEGV` | [x] |
| 9  | `match`  | `reference == NULL` with `bins >= 1` → same, on the second `total` call | `SIGSEGV` | [x] |
| 10 | `match`  | `test == NULL` **and** `reference == NULL` with `bins == 0`: no dereference happens, but row 3 still applies | `SIGSEGV` (from row 3, not from the null pointers) | [x] |
| 11 | `match`  | `threshold = NaN` (quiet or signalling, either sign): the gate is `x < NaN` → `comisd` sets CF=ZF=PF=1 → `jbe` taken → **does not reject**; all the preprocessing runs; the final `contrast >= NaN` is false | `= 0` | [x] |
| 12 | `match`  | `threshold = +inf` with `total(reference) == 0.0` → `+inf * 0.0 = NaN` → gate does not reject; `contrast >= +inf` is false | `= 0` | [x] |
| 13 | `match`  | `threshold = -inf` → `-inf * total(reference)` is `∓inf` (or `NaN` when the total is `0`); the gate rejects only if that exceeds `total(test)`; the final `contrast >= -inf` is **true unless `contrast` is `NaN`** | `= 1` iff the gate passed and `contrast` is not `NaN`, else `0` | [x] |
| 14 | `match`  | `total(test)` or `total(reference)` is `NaN` (`NaN` input, or an `+inf`/`-inf` mix) → gate comparison is unordered → **does not reject** | proceeds; result decided by row 2 | [x] |
| 15 | `spectral_contrast` | `length <= 0`: all three loops are `for(i = 0; i < length; i++)` → zero-trip; `dot_product` returns `+0.0`; `sqrt(+0.0) = +0.0`; `normalize` writes nothing. **The pointers are never dereferenced, so `NULL` is safe.** | `= +0.0` (bit pattern `0x0000000000000000`) | [x] |
| 16 | `spectral_contrast` | `NULL` pointer (either argument) with `length >= 1` → `movss (%rax),%xmm1` inside `dot_product`, reached from `normalize` | `SIGSEGV` | [x] |
| 17 | `normalize` | `magnitude == 0.0` (all-zero input, or every lane flushing to `0.0f`): `v[i] /= 0.0` with **no divide-by-zero check** → `0.0/0.0` is the SSE indefinite `-NaN` (`0xFFC00000`), `x/0.0` is `±inf` | every lane becomes `-NaN`; `dot_product` then returns `NaN`, so `spectral_contrast` returns `NaN` and `match` returns `0` (unordered `comisd`) | [x] |
| 18 | `normalize` | `magnitude` is `NaN` (input contains `NaN`, or `Σx²` produces `inf - inf`) → every `v[i] / NaN` is `NaN` | `spectral_contrast` returns `NaN`; `match` returns `0` | [x] |
| 19 | `normalize` | `magnitude == +inf` (input contains `±inf`, or `Σx²` overflows `double`) → `v[i]/inf` is `±0.0` for finite `v[i]`, `NaN` for `±inf` | all-finite case leaves zeros → `dot_product` returns `+0.0` | [x] |
| 20 | `normalize` | `cvtsd2ss` writeback **overflow**: `abs(v[i]/magnitude) > FLT_MAX` (reachable with subnormal `magnitude`) → `±inf` stored into the `float` lane | silent `±inf`, no error | [x] |
| 21 | `normalize` | `cvtsd2ss` writeback **underflow**: quotient below `FLT_TRUE_MIN` → `±0.0` (sign preserved) or a subnormal `float` | silent, no error | [x] |
| 22 | `dot_product` | `Σ a[i]*b[i]` overflows to `±inf`, then adding the opposite infinity → `-NaN` | silent `NaN` | [x] |
| 23 | `dot_product` | `a[i] == 0.0f` and `b[i] == ±inf` → `mulss` invalid operation → `-NaN` | silent `NaN` | [x] |
| 24 | `spectral_contrast` | **aliased arguments** `a == b` (`include/match.h` has no `restrict`): `normalize(a)` then `normalize(b)` normalise the *same* buffer twice; the second pass sees an already-unit vector | returns `dot_product(a,a)` of the twice-normalised buffer (≈ `1.0`, not exactly), and leaves the buffer twice-normalised | [x] |
| 25 | `match`  | aliased `test == reference`: legal; both totals are equal and `t`/`r` receive identical preprocessed data | gate becomes `x < threshold*x`; `contrast` is `dot_product(u,u)` for a unit vector | [x] |
| 26 | `smoothen` | tail rows `i > length - N_SMOOTH`: fewer than `N_SMOOTH` samples are summed but the divisor is **still 16**, never renormalised (`match.c:18`; `divsd` against the `16.0` constant in `.rodata`) | tail is attenuated; reproduced verbatim, not "fixed" | [x] |
| 27 | `match`  | `bins` odd: `spectral_contrast` reads `bins` 4-byte lanes out of `bins` 8-byte slots, so the last lane is the **low half** of `t[(bins-1)/2]` and that slot's high half is never read | silent; part of normal operation | [x] |
| 28 | `match`  | `bins == 1`: `differentiate` immediately writes `v[0] = 0` (its loop is zero-trip), then `smoothen` gives `0/16 = 0` → both vectors are `{0.0}` → `magnitude = 0` → row 17 → `contrast = NaN` | `= 0` for every non-`-inf` threshold that passes the gate | [x] |
| 29 | both | preprocessed `double`s whose **low 32 bits** form a `float` `NaN`/`inf`/subnormal — routine, since those are a `double`'s low mantissa bits | silent `NaN`/`inf` inside `spectral_contrast`; must be bit-reproduced | [x] |
| 30 | both | **signalling `NaN`** in the input: `cvtss2sd` / `mulss` / `addsd` quiet it (set the significand MSB) and propagate the payload. For two `NaN` operands x86 returns the **destination** operand quieted, and GCC at `-O0` makes `b[i]` the `mulss` destination and the *product* the `addsd` destination | payload-exact `NaN` bits, observable in `spectral_contrast`'s return value and buffers | [x] |

## Rows 3, 5, 6, 7, 10 — a documented, intentional divergence

These five rows are pointer/size **undefined behaviour whose only "result" is a
fault**. There is no defined C semantics to match: the C program's behaviour for
`bins <= 0`, `bins` huge, or `bins == INT_MIN` is not merely surprising, it is
absent — the abstract machine has no meaning for a negative-length VLA or for a
store through `v[-1]` that lands on a return address.

The Rust translation therefore clamps `bins <= 0` to an empty buffer, which is
exactly what every loop in `match.c`'s translation unit already does
(`for(i = 0; i < length; i++)`), and returns `(0.0 >= threshold) as c_int`.
`tests/errors.rs` verifies in an isolated child process that the **C really does
fault** on each of these inputs, and asserts the Rust side's documented
alternative, so the divergence is pinned rather than hidden. Rows 8, 9 and 16
*are* reproduced faithfully: Rust dereferences the same null pointer and faults
the same way.
