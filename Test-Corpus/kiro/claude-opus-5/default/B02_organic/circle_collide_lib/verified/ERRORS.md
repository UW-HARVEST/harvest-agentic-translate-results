# ERRORS.md — Error / rejection surface table (Phase A → gates Phase C)

Derived **mechanically** from `c_src/src/lib.c`. The greps used:

```sh
grep -n "return"                     src/lib.c   # 15 hits
grep -n "assert\|NULL\|ERROR\|errno" src/lib.c   #  0 hits
grep -n "if\|switch\|case\|default"  src/lib.c   #  9 hits
```

Findings of the mechanical sweep:

* **0** `assert` / `static_assert`.
* **0** null-pointer checks.
* **0** explicit range / min / max / bounds checks.
* **0** error enums, error codes, `errno` writes, `RETURN_ERROR`-style macros.
* **0** `return -1` / `return NULL` sentinels.
* **1** explicit rejection branch: `default: return 0;` at `src/lib.c:112-113`.
* **0** `#if` / `#ifdef` / `#define` conditionals.

This library rejects nothing except an unrecognised `C2_TYPE`. Every other
"weird" input (NaN, ±inf, inverted AABB, degenerate capsule, misaligned or null
pointer) is fed straight into the arithmetic / dereference with no guard — which
is itself the behaviour the Rust must reproduce bit-for-bit. Rows 2..12 below
therefore record the *implicit* rejection/boundary behaviours the C source
actually exhibits, so each still gets a differential test.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `c2Collided` | `typeB` matches no `case` label — the `default:` arm at `src/lib.c:112`. Exercised with `3, 4, 7, 255, 256, 1000, i32::MAX, -1, -2, i32::MIN` and 512 random ints outside `{0,1,2}` | returns `0` exactly (no dereference of `A`/`B` at all) | `err_row01_c2collided_out_of_range_enum` | [x] |
| 2  | `c2Collided` | `typeB` out of range **and** `A == NULL && B == NULL`. C reaches `default:` before any load, so the null pointers are never touched | returns `0`; must NOT fault | `err_row02_c2collided_null_ptrs_with_bad_type` | [x] |
| 3  | `c2Collided` | `typeB` = negative one-step-past-range (`-1`) — C `switch` on `int`, so negatives fall to `default:` | returns `0` | `err_row03_c2collided_negative_enum` | [x] |
| 4  | `c2Collided` | `typeB` = `3` = exactly one step past the last valid variant `C2_TYPE_CAPSULE` (2) | returns `0` | `err_row04_c2collided_one_past_last_variant` | [x] |
| 5  | `c2Collided` | valid `typeB` but the *first* operand is not really a circle: C unconditionally does `*(c2Circle*)A` regardless of `typeB`, so a 12-byte buffer of arbitrary bytes must be reinterpreted identically | same `int` as reinterpreting those bytes as `c2Circle` | `err_row05_c2collided_blind_cast_of_A` | [x] |
| 6  | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` ⇒ `c2Dot(n,n) == 0` ⇒ unguarded `da / 0.0f` at `src/lib.c:93`. No check exists | `0.0/0.0 = NaN` or `±x/0.0 = ±inf` flows into `d2`; final `d2 < r*r` is `0` when `d2` is NaN | `err_row06_capsule_degenerate_div_by_zero` | [x] |
| 7  | `c2CircletoCircle` / `c2CircletoAABB` / `c2CircletoCapsule` | negative radius `r < 0` (never validated) | `r*r`/`(A.r+B.r)^2` is still ≥ 0, so a negative radius behaves like its magnitude; asserted equal | `err_row07_negative_radius` | [x] |
| 8  | `c2CircletoAABB` | inverted / oversized AABB (`min > max` per axis, incl. `±inf` bounds). `c2Clampv` = `max(lo, min(a,hi))` with no ordering check, so `min>max` yields `lo` | whatever the raw clamp chain produces; asserted equal | `err_row08_inverted_aabb` | [x] |
| 9  | all float entry points | NaN in any float argument (`>` / `<` comparisons are false ⇒ ternaries take the *second* operand; `comiss`+`jbe`) | the "false" branch every time ⇒ `0` from the `d2 < r2` predicates; `c2Maxv`/`c2Minv` return operand `b` | `err_row09_nan_inputs` | [x] |
| 10 | all float entry points | `±inf` arguments ⇒ `inf - inf = NaN`, `inf * 0 = NaN` inside `c2Dot` | asserted equal bit-for-bit / int-for-int | `err_row10_infinity_inputs` | [x] |
| 11 | all float entry points | subnormal / `f32::MIN_POSITIVE`/`MAX` operands ⇒ overflow to `inf` in `c2Dot`, underflow to `0` in `c2Mulvs` (no FTZ set by either lib) | asserted equal bit-for-bit | `err_row11_denormal_and_overflow` | [x] |
| 12 | `c2Collided` | `typeB` valid but pointers *unaligned* (offset-by-1 inside a byte buffer). C does an unaligned `movss`-based struct copy, which x86 permits | same result as the aligned copy of the same bytes | `err_row12_unaligned_pointers` | [x] |

### Not testable (would be UB in C, no defined result to compare against)

`c2Collided(NULL, NULL, C2_TYPE_CIRCLE)` — a *valid* `typeB` with null pointers
dereferences address 0 in **both** implementations and raises `SIGSEGV`. There is
no return value to differentially compare, so it is deliberately excluded rather
than asserted; row 2 covers the null case on the one path where C defines it.

## Divergences found and fixed

Two real defects were found by these tests and fixed in `src/lib.rs` (the C was
never touched):

1. **`c2Dot` NaN payload (rows 9/11, `regression_nan_order.rs`).** The C's final
   `addss` takes the **y term** as `src1`, and its y-term `mulss` takes **`b.y`**
   as `src1`. x86 forwards `src1` quieted when it is NaN, so a naive
   `a.x * b.x + a.y * b.y` returned the *x* term's payload/sign instead. Observed
   at 126 mismatches per 2 M random bit patterns; now 0. Fixed by modelling the
   x86 operand selection in `mul_ss` / `add_ss`, because LLVM commutes
   `fadd`/`fmul` freely and Rust source order cannot pin it down.
2. **`c2Collided` unaligned loads (rows 12 / `err_row12`, debug profile only).**
   `*(c2Circle *)A` in C compiles to alignment-agnostic x86 loads, so a caller may
   pass an unaligned pointer. The Rust `*(A as *const c2Circle)` tripped the
   debug-assertions "misaligned pointer dereference" check and aborted. Fixed
   with `core::ptr::read_unaligned`. This one was invisible in the release
   profile — it is why the suite runs under **both** cargo profiles.

## Note on NaN payloads and the C build configuration

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the documented build
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`) compiles at
`-O0`, and that `.so` is the ground truth these tests load. gcc's choice of SSE
`src1` operand is **not stable across `-O` levels** — measured on this toolchain:

| build | `c2Dot` y-term `src1` | `c2Dot` add `src1` | `c2Mulvs` `src1` |
|-------|----------------------|--------------------|------------------|
| `-O0` (the documented build) | `b.y` | y term | vector component |
| `-O1` / `-O2` / `-Os` | `a.y` | x term | vector component |
| `-O3` (auto-vectorised `mulps`) | `a.y` | x term | **scalar** |

No single translation can match all of these simultaneously, so the Rust matches
`-O0`, i.e. the build the task specifies. If the C is ever rebuilt at another
optimisation level, `regression_nan_order.rs` fails with an explicit
"precondition" message naming the operand the C picked, which is the signal to
flip the operand order in `mul_ss` / `add_ss` in `src/lib.rs`. Every non-NaN
result is `-O`-independent and unaffected.
