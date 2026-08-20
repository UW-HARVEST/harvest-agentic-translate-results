# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

## How this table was derived (mechanical, not guessed)

The whole compiled C surface is `c_src/src/lib.c` (59 lines) plus the one-line
header `c_src/include/lib.h`. Exhaustive grep for every rejection mechanism a C
library can use:

```sh
grep -nE 'return|assert|NULL|errno|-1|error|ERROR|exit|abort|if *\(|switch|case|default|#if' \
     c_src/src/lib.c c_src/include/lib.h
```

Full result:

```
lib.c:12:    if (s == 0) {
lib.c:16:        return;
lib.c:24:    switch (i) {
lib.c:25:    case 0:
lib.c:30:    case 1:
lib.c:35:    case 2:
lib.c:40:    case 3:
lib.c:45:    case 4:
lib.c:50:    default:
```

Consequently, mechanically established facts about the error surface:

* `hsv_to_rgb` returns `void` — there is **no** error code, no sentinel, no
  out-parameter status. `grep -c 'return [^;]' lib.c` = 0.
* There are **no** `assert`s, no `NULL` checks, no `errno` writes, no
  `RETURN_ERROR`-style macros, no error enums, no `#define` min/max constants,
  and no explicit range checks on `h`, `s` or `v`.
* There is **no** enum type anywhere in the API, hence no "out-of-range enum
  value across the FFI boundary" to test — the only parameters are two
  pointers. (The nearest analogue, an integer with no valid variant selecting a
  `switch` arm, *does* exist internally as `i` and is covered by rows 3–6 and
  by `CONFIGS.md`.)

The library therefore rejects/degrades input only through (a) the `s == 0`
early-return, and (b) the `switch` `default:` fall-through that absorbs every
`i` value with no `case`. Everything else that "goes wrong" is undefined
behaviour in C (null/misaligned pointers, out-of-`int`-range float→int cast).
Each of those is a real input an external caller can pass, so each gets a row
and a differential test that asserts C and Rust produce the *same* observable
result (identical output bits, or identical fatal signal).

Legend for "expected C result": `dest[0..3]` bit patterns, or the fatal signal.
`i` denotes `(int)floorf(h / 60.0f)`.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `hsv_to_rgb` | `src[1]` (`s`) is `+0.0f` → `s == 0` true, degenerate "no saturation" input; `h` and `v` are ignored entirely, including when `h` is garbage | early `return` after writing `dest[0]=dest[1]=dest[2]=v`; `h` never read again, `floorf` never called | `err_01_s_plus_zero_early_return` | [x] |
| 2 | `hsv_to_rgb` | `src[1]` is `-0.0f` — `-0.0 == 0` is **true** in IEEE-754, so this negative-zero saturation is also absorbed by the early return (no separate check exists) | same as #1: `dest = {v,v,v}` | `err_02_s_minus_zero_early_return` | [x] |
| 3 | `hsv_to_rgb` | `i` has **no matching `case`** because `i >= 5` (`h >= 300`, e.g. `h = 360`, `h = 1e6`) | `switch` `default:` arm → `r=v, g=p, b=q` (silent fall-through, *not* an error) | `err_03_switch_default_i_ge_5` | [x] |
| 4 | `hsv_to_rgb` | `i` has no matching `case` because `i < 0` (`h < 0`, e.g. `h = -1`, `h = -1e6`). GCC compiles the `switch` as an **unsigned** `cmpl $4 / ja`, so all negatives take `default:` | `default:` arm → `r=v, g=p, b=q` | `err_04_switch_default_i_negative` | [x] |
| 5 | `hsv_to_rgb` | `h` is `+inf` / `-inf` → `floorf` returns `±inf` → `(int)±inf` is **UB in C**; on x86-64 `cvttss2si` yields the integer-indefinite value `0x80000000` = `INT_MIN` | `i = INT_MIN` → `default:` arm; `f = ±inf - (-2147483648.0f)` (`+inf`, or `NaN` for `-inf`) so `q` is `inf`/`NaN`-tainted | `err_05_h_infinite_int_indefinite` | [x] |
| 6 | `hsv_to_rgb` | `h` is `NaN` (quiet **and** signaling, several payloads) → `floorf(NaN)=NaN` → `(int)NaN` is UB; `cvttss2si` yields `INT_MIN` | `i = INT_MIN` → `default:` arm; `f`, `q` become `NaN` | `err_06_h_nan_int_indefinite` | [x] |
| 7 | `hsv_to_rgb` | `h` finite but `h/60` outside `[-2^31, 2^31)` (one step past the representable `int` range: `h = ±1.3e11`, `±FLT_MAX`, and the exact boundaries `h/60 == 2147483648.0` and `h/60 == -2147483648.0`) → out-of-range float→`int` cast, UB in C; `cvttss2si` yields `INT_MIN` except for the exactly-representable `-2^31` | `i = INT_MIN` → `default:` arm | `err_07_h_out_of_int_range` | [x] |
| 8 | `hsv_to_rgb` | `src[1]` (`s`) is `NaN` — `NaN == 0` is false, so the early return is **not** taken and the NaN flows into `p`, `q`, `t` | main path taken; all three outputs `NaN`-tainted per arm | `err_08_s_nan_not_equal_zero` | [x] |
| 9 | `hsv_to_rgb` | `s` / `v` outside the documented `[0,1]` range (no clamp exists): `s > 1` (makes `p` negative), `s < 0`, `v < 0`, `v > 1`, `v` huge | no rejection, plain arithmetic; results may be negative / `inf` | `err_09_s_v_out_of_unit_range` | [x] |
| 10 | `hsv_to_rgb` | `s` or `v` is `±inf` → produces `inf * 0` / `inf - inf` invalid operations | no rejection; `NaN` (x86 indefinite `0xffc00000`) appears in outputs | `err_10_s_v_infinite` | [x] |
| 11 | `hsv_to_rgb` | `src[2]` (`v`) is `-0.0f` with `s == 0` → early return copies the sign of zero | `dest = {-0.0, -0.0, -0.0}` (sign bit preserved, not normalised to `+0.0`) | `err_11_v_negative_zero_sign_preserved` | [x] |
| 12 | `hsv_to_rgb` | denormal (subnormal) `h`, `s`, `v` — no flush-to-zero, no special case | plain arithmetic on subnormals, bit-exact | `err_12_subnormal_inputs` | [x] |
| 13 | `hsv_to_rgb` | `dest == src` (full aliasing). Neither pointer is `restrict`, and the C reads `src[0..2]` into locals *before* any store, so aliasing is well-defined but observable | in-place update; `dest[0..3]` overwritten with the RGB triple | `err_13_alias_dest_eq_src` | [x] |
| 14 | `hsv_to_rgb` | partially overlapping buffers: `dest == src + 1` and `dest == src - 1` (writes clobber neighbouring source slots) | reads happen first, so results equal the non-aliased case, shifted in memory | `err_14_alias_partial_overlap` | [x] |
| 15 | `hsv_to_rgb` | `src == NULL` → unconditional `src[0]` load with no null check (UB) | fatal `SIGSEGV` | `err_15_null_src_segv` (subprocess probe) | [x] |
| 16 | `hsv_to_rgb` | `dest == NULL` (with `s == 0`, i.e. the early-return store path) → unconditional `dest[0]` store, no null check (UB) | fatal `SIGSEGV` | `err_16_null_dest_early_path_segv` (subprocess probe) | [x] |
| 17 | `hsv_to_rgb` | `dest == NULL` **and** `s != 0` (the long path, so the fault happens after `floorf`/`switch`) | fatal `SIGSEGV` | `err_17_null_dest_main_path_segv` (subprocess probe) | [x] |
| 18 | `hsv_to_rgb` | both pointers `NULL` | fatal `SIGSEGV` (on the `src` load, which happens first) | `err_18_null_both_segv` (subprocess probe) | [x] |
| 19 | `hsv_to_rgb` | misaligned `src` / `dest` (`float*` off by 1 byte). C has no alignment requirement on x86-64 for `movss`, so this silently works | same output bits as the aligned call | `err_19_misaligned_pointers` (release `.so` only) | [x] |
| 20 | `hsv_to_rgb` | "oversized"/short buffer analogue: exactly-3-element `src`/`dest` allocations, i.e. the function must read *no more* than `src[0..3]` and write *no more* than `dest[0..3]`. Reading/writing a 4th element would be an out-of-bounds access | canary words immediately before and after the 3-float window are untouched | `err_20_no_out_of_bounds_access` | [x] |
| 21 | `hsv_to_rgb` | Generic "one step past the range" sweep across the *whole* argument domain: every one of the 512 sign+exponent fields × 4 mantissa patterns, in each of the three slots, crossed with the early-return / NaN / infinite / negative presets. This is the stand-in for "out-of-range enum value" — the API takes no enum, so the equivalent is "every bit pattern that is not a documented valid input" | identical output bits for all of them (the C never rejects anything) | `err_21_generic_boundary_sweep` | [x] |

### Notes on rows 15–19 (undefined behaviour rows)

* Rows 15–18 are compared by **fatal signal**, in a child process
  (`std::process::Command` re-invoking the test binary with
  `HARVEST_CRASH_PROBE=<c|rust>:<kind>`), so a mismatch such as "C segfaults,
  Rust aborts with a Rust panic message" is caught rather than hidden. The
  assertion is `dc.signal == Some(SIGSEGV) && dc == dr`, i.e. it pins the
  *specific* fatal signal, not merely "both died".
* These rows pass in **both** the debug and the release profile. That is only
  true because `src/lib.rs` performs its memory accesses with
  `ptr::read` / `ptr::write` instead of the `*ptr` deref operator: the deref
  operator makes rustc emit debug-profile `null_pointer_dereference` /
  `misaligned_pointer_dereference` UB assertions, which abort with a Rust
  diagnostic (`SIGABRT`) instead of faulting like the C code (`SIGSEGV`). The
  original translation used `*ptr` and therefore diverged in debug builds; this
  was fixed. Verified by disassembly (`core::ptr::const_ptr::read` lowers to a
  bare `mov (%rdi),%eax` with no check) and by the probes themselves.
* Every other row (1–14, 20–21) is asserted bit-for-bit against **both** the
  debug and the release Rust `.so`.

### Divergences found and fixed by this phase

1. **NaN payload propagation order** (affects rows 5, 6, 8, 10, and many
   `CONFIGS.md` rows). x86 `MULSS`/`SUBSS`/`DIVSS` return the *first* source
   operand quieted when it is a NaN, else the second, else the x86 QNaN
   indefinite `0xFFC0_0000` for an invalid operation. Plain Rust `*` / `-`
   lower to whichever operand order LLVM prefers (LLVM treats all NaNs as
   interchangeable), which produced e.g. `0xFFC0_0000` where C produced
   `0x7FC0_0000` for `src = {inf, 0x7FC00000, 0xFFC00000}`. Fixed by adding
   `subss` / `mulss` / `divss` helpers to `src/lib.rs` that reproduce GCC's
   exact emitted operand order (taken from the `objdump` of
   `c_src/build/CMakeFiles/.../lib.c.o`).
2. **`floorf` on NaN** — now handled explicitly (quiet the argument, preserving
   sign and payload), matching glibc's `x + x` / `ROUNDSS` behaviour instead of
   relying on LLVM's NaN-agnostic `llvm.floor.f32` lowering.
3. **Signaling-NaN pass-through on the copy paths** — the `s == 0` early return
   and the `r = v` style assignments now move raw `u32` bit patterns, so an
   sNaN in `src` is copied verbatim (as C's `movss` does) and cannot be quieted
   by an intermediate `f32` materialisation.
4. **Debug-profile pointer UB assertions** — see the note above.
