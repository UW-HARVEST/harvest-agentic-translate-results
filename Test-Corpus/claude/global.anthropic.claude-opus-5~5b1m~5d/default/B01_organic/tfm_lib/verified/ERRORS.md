# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (32 lines) and `c_src/include/lib.h`
(1 line). Method: grep the C source for every `return`, `assert`, `NULL`,
`errno`, error enum, explicit range check, and every min/max clamp constant.

```
$ grep -rn 'return\|assert\|NULL\|errno\|EINVAL\|-1' c_src/src c_src/include
(no matches)
```

**`tfm` is `void` and contains no `return`, no `assert`, no null check, and no
explicit error code.** Its entire rejection surface therefore consists of:

* the loop guard `i < count` — the *only* input validation in the library; and
* the clamp `(((0) > (sqd)) ? (0) : (sqd))` — the only value-domain guard, which
  silently substitutes a value instead of erroring; and
* the implicit numeric "rejections" of IEEE-754 (NaN comparisons falling to the
  `else` branch, overflow to infinity, invalid operations producing NaN).

Every distinct rejection/guard branch gets one row. `[x]` = a differential test
exists and passes against **both** the C `.so` and the Rust `.so`.

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `tfm` | `count == 0` (loop guard `0 < 0` false) | returns immediately; **zero** loads from `src`, **zero** stores to `dest`; `dest` left bit-identical to its prior contents | [x] |
| E2 | `tfm` | `count == -1` (loop guard `0 < -1` false) | same as E1: no-op, `dest` untouched. Not clamped, not an error code — silently nothing | [x] |
| E3 | `tfm` | `count == INT_MIN` (`-2147483648`) | same as E1: no-op, `dest` untouched (no overflow, guard is a plain signed `<`) | [x] |
| E4 | `tfm` | `count` = other negatives (`-2`, `-1000`, `INT_MIN+1`) | no-op, `dest` untouched | [x] |
| E5 | `tfm` | `dest == NULL`, `src == NULL`, `count <= 0` | no-op; **no dereference**, no crash (loop body never runs) | [x] |
| E6 | `tfm` | `dest == NULL`, `src` valid, `count > 0` | UB in C: stores through NULL → `SIGSEGV`. Verified to fault identically for C and Rust in a forked child process | [x] |
| E7 | `tfm` | `src == NULL`, `dest` valid, `count > 0` | UB in C: loads through NULL → `SIGSEGV`. Verified identically for C and Rust in a forked child | [x] |
| E8 | `tfm` | `src[0] == src[1]` — relational `src[0] < src[1]` is **false** | takes the `else` branch: `dy2=src[0]`, `dx2=src[1]`, `dest[0]=dxy`, `dest[1]=dx2-lambda`. (Boundary one step from the `if` branch) | [x] |
| E9 | `tfm` | `src[0] > src[1]` | `else` branch, as E8 | [x] |
| E10 | `tfm` | `src[0]` is NaN (either sign, any payload) — `NaN < x` is **false** | `else` branch (unordered compare rejects the `if`), *not* the `if` branch | [x] |
| E11 | `tfm` | `src[1]` is NaN — `x < NaN` is **false** | `else` branch | [x] |
| E12 | `tfm` | both `src[0]` and `src[1]` NaN | `else` branch | [x] |
| E13 | `tfm` | `sqd < 0` — clamp `0 > sqd` is **true**. Mathematically `sqd = (dy2-dx2)² + 4dxy² ≥ 0`, so this is reachable **only through rounding**: near-equal `dx2`/`dy2` make `dy2*dy2 - 2*dx2*dy2 + dx2*dx2` round negative (see "unreachable claims" below for the exact constructor) | `sqrtf` receives the `float` `0.0f`, **never** a negative; returns `+0.0f`; no `EDOM` | [x] |
| E14 | `tfm` | `sqd == -0.0f` — clamp `0 > -0.0f` is **false**, so `-0.0f` is *not* replaced | `sqrtf(-0.0f)` = `-0.0f` (IEEE-754 §6.3); `lambda = 0.5f*(dy2+dx2+(-0.0f))`. Distinguishes the C ternary from `fmaxf`, which would also return `-0.0`/`+0.0` ambiguously | [x] |
| E15 | `tfm` | `sqd` is NaN — clamp `0 > NaN` is **false**, so NaN is *not* replaced | NaN reaches `sqrtf`, which returns a quiet NaN with the sign+payload preserved and mantissa MSB forced on; NaN propagates into `lambda` and out to `dest` | [x] |
| E16 | `tfm` | `sqd == +inf` (overflow of `4.0f*dxy*dxy` or `dx2*dx2`) | clamp false; `sqrtf(+inf)=+inf`; `lambda=+inf`; `dest[0]=dx2-inf` (or `dest[1]`) | [x] |
| E17 | `tfm` | invalid operation `inf - inf` inside `sqd` (both `dy2*dy2` and `2*dx2*dy2` overflow to `+inf`) | produces the x86 QNaN indefinite `0xFFC00000` (negative sign bit), which then flows through E15 | [x] |
| E18 | `tfm` | invalid operation `0 * inf` inside `2.0f*dx2*dy2` (`dx2=0`, `dy2=±inf`) | produces `0xFFC00000`, then E15 | [x] |
| E19 | `tfm` | invalid operation `inf + (-inf)` in `dy2 + dx2` (`dy2=+inf`, `dx2=-inf`) | `0xFFC00000` propagated into `lambda` | [x] |
| E20 | `tfm` | subnormal / underflow-to-zero inputs (`±1e-45`, `±FLT_MIN/2`) — no flush-to-zero is requested by the C build | gradual underflow, subnormal results preserved bit-exactly (MXCSR FTZ/DAZ off in both) | [x] |
| E21 | `tfm` | *signalling* NaN in `src[0..2]` (`0x7FA0_0000` / `0xFFA0_0000`) | no trap (SSE exceptions masked); the sNaN is **quieted** to `0x7FE0_0000`/`0xFFE0_0000` by the first arithmetic op that consumes it, or copied **verbatim** when it only passes through `dest[i] = dxy` (a plain store, not an FP op) | [x] |
| E22 | `tfm` | NaN with a *non-canonical payload* (e.g. `0x7F80_0001`, `0xFFBF_FFFF`) | payload is preserved through the quieting rules; the surviving payload is the SSE **destination** operand's | [x] |
| E23 | `tfm` | out-of-range enum value passed across FFI | **N/A — the API declares no enum.** The only non-pointer parameter is `int count`; its full `int` range is covered by E1–E4 and B-rows, including `INT_MIN`, `-1`, `0`, `1` and large positives | [x] |
| E24 | `tfm` | "oversized length": `count` larger than the logical element count of the caller's data (buffer still allocated large enough that no OOB access occurs) | no bounds check exists — C happily processes the extra trailing elements; Rust must read/write exactly the same extra elements | [x] |
| E25 | `tfm` | `count == 1` with a buffer sized for exactly 1 element (`3` in, `2` out) — the tight lower boundary of the loop | exactly 3 loads and 2 stores; **no** access to `src[3]` / `dest[2]` (checked with guard canaries either side) | [x] |
| E26 | `tfm` | unaligned-for-vectorization but `float`-aligned pointers (`src`/`dest` offset by 1, 2, 3 floats inside a larger allocation) | identical results; no alignment fault (the C `-O0` build is scalar, and Rust must not require 16-byte alignment) | [x] |

## Notes on what is deliberately *absent*

* No `errno` is ever set by `tfm`. The clamp at E13 guarantees `sqrtf` is never
  called with a negative argument, so glibc never raises `EDOM`. This is
  verified *indirectly*, not by reading `errno`: `e13_negative_sqd_is_clamped_to_zero`
  asserts that no NaN escapes to the output for finite inputs, which is exactly
  what an unclamped `sqrtf(negative)` would produce. (Reading `errno` across a
  `dlopen`ed boundary would compare the harness's TLS slot, not the callee's, so
  it would not be a meaningful assertion.)
* There is no way for `tfm` to report failure to its caller: it is `void` and
  has no out-param status. Consequently every row above is verified by
  comparing the *written output buffer* (and process exit status / signal for
  E6/E7), not by comparing return codes.
* `float` is IEEE-754 binary32 in both languages on this target
  (`x86_64-unknown-linux-gnu`, SSE2 baseline); no `long double` / x87 excess
  precision can leak in.

---

## Verification result

All **26** rows have a passing differential test. The mapping is 1:1 and
mechanically checkable:

```
$ diff <(grep -oE '^\| E[0-9]+ ' ERRORS.md   | grep -oE '[0-9]+' | awk '{printf "e%02d\n",$0}' | sort) \
       <(grep -oE '^fn e[0-9]+' tests/phase_c.rs | grep -oE 'e[0-9]+' | sort)
# (no output — 1:1 match)
```

`tests/phase_c.rs`: 26 row tests + 1 `#[ignore]`d helper
(`zz_null_pointer_crash_child`, the child process used by E6/E7).

### Divergence found and fixed: E6 / E7 (NULL pointer, `count > 0`)

The only real defect this phase uncovered.

* **C** (`libharvest-work-qlgOWs.so`): dies with **`SIGSEGV` (11)**.
* **Rust, `release` profile**: `SIGSEGV` (11) — already correct.
* **Rust, `dev` profile**: died with **`SIGABRT` (6)**:
  ```
  thread '<unnamed>' panicked at src/lib.rs:270:17:
  null pointer dereference occurred
  thread caused non-unwinding panic. aborting.
  ```
  rustc's UB sanitizer — switched on implicitly by `debug-assertions` — turns
  the raw-pointer store into a checked operation, so the debug `.so` rejected an
  input that the C happily faults on.

**Fix (Rust side only):** `[profile.dev] debug-assertions = false` in
`translation/Cargo.toml`, with `overflow-checks = true` retained so nothing else
is weakened. Both profiles now fault identically, verified by comparing the
terminating signal of a forked child:

```
which=dest impl=c -> SIGSEGV(11)   impl=rust -> SIGSEGV(11)
which=src  impl=c -> SIGSEGV(11)   impl=rust -> SIGSEGV(11)
which=both impl=c -> SIGSEGV(11)   impl=rust -> SIGSEGV(11)
```

This is exactly the class of bug the "every configuration" gate exists to catch:
the *shipped* `release` object was always right, and only the `dev` object was
wrong, so testing one profile would have missed it entirely.

### Two ERRORS.md claims that turned out to be *unreachable*, and how they were handled

Rather than assert an unreachable condition (which would silently pass while
proving nothing), each is now actively confirmed unreachable by search, and the
*reachable* neighbours are tested instead:

* **E14, `sqd == -0.0f`.** `sqd = dxy_term + acc` where
  `dxy_term = (4*dxy)*dxy` is a square (never `-0.0`), and `acc` could only be
  `-0.0` if `dx2*dx2` were. IEEE round-to-nearest yields `-0.0` from an addition
  only when *both* addends are `-0.0`, so the regime cannot occur.
  `b19_sqd_negative_zero` and `e14_*` assert **0 hits** across the exhaustive
  24³ alphabet, the cancellation family and 400 000 random triples, then test the
  reachable neighbours (`sqd == +0.0`, `sqd < 0`) and push `-0.0` through every
  input lane instead.
* **E13, negative `sqd`, is reachable but only via rounding.** Mathematically
  `sqd = (dy2-dx2)² + 4dxy² ≥ 0`; it only goes negative through
  catastrophic cancellation. The harness constructs it deliberately with
  near-equal operands `1 + p·2⁻²³` vs `1 + q·2⁻²³`, whose residual is
  `(rn(p²/2²³) + rn(q²/2²³) − 2·rn(pq/2²³))·2⁻²³` — e.g. `p=2048, q=2049`
  gives `0 + 1 − 2 = −1`, i.e. `sqd = −2⁻²³`. **400 negative-`sqd` triples
  found, split across both C branches**, so the clamp is genuinely exercised
  rather than assumed.

### Rows whose "expected result" is a branch choice, not a value

E8–E12 claim the C falls into the `else` branch. That claim is now *observable*
rather than inferred: the C writes `dxy` **verbatim** (a plain `movss`, not an FP
op) to `dest[1]` in the `if` branch and to `dest[0]` in the `else` branch, so
comparing `dest[i]` bits against `src[2]` bits reveals which branch ran. Each of
E8–E12 asserts the observed branch is the expected one for both objects, and
fails if the branch was indistinguishable for every input (so the test cannot
pass vacuously).

### `errno`

Never set: the clamp (E13) guarantees `sqrtf` is never called with a negative
argument, so glibc raises no `EDOM`. Confirmed indirectly — E13 verifies no NaN
escapes for finite inputs, which is what an unclamped `sqrtf(negative)` would
produce.
