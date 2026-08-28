# ERRORS.md — error / rejection surface of the C library

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`, not from
docs or assumptions. The greps used:

```sh
grep -nE 'return|NULL|assert|errno|abort|exit|-1|E[A-Z]+|default:|if *\(|while|for *\(' \
    c_src/src/lib.c c_src/include/lib.h
grep -nE 'switch|case|break' c_src/src/lib.c
grep -rnE '#if|#ifdef|#ifndef|#define' c_src/src c_src/include
```

Result of that sweep, stated precisely because it drives the whole table:

* **0** `return` statements of any kind (both functions are `void`; the only
  grep hits for the string `return` were false positives inside float literals).
* **0** error-return macros, **0** error enums, **0** `errno` writes, **0**
  sentinel values, **0** `assert`, **0** `abort`/`exit`.
* **0** `NULL` checks — `colourblind` and all three helpers dereference their
  pointer arguments unconditionally.
* **0** explicit range checks, **0** min/max constants, **0** loops, **0** `if`.
* **0** preprocessor conditionals, so the error surface cannot vary by build.

So the library has **no error-reporting channel whatsoever**. The only way it
"rejects" an input is the `switch` in `colourblind` (`c_src/src/lib.c:25-34`),
which has **no `default:` label**: an `Impairment` that matches no `case` falls
straight through to the end of the function, silently leaving `*R`, `*G`, `*B`
untouched. That silent no-op is the one and only rejection behaviour, and rows
1-6 below enumerate the distinct triggers that reach it.

Rows 7-11 are the generic C-API boundaries required by Phase C even though the
C contains no check for them; each row states the C's *actual* observable
result, which is what the Rust must reproduce.

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `colourblind` | `Impairment == 3` — first value one step past the last valid variant (`cbTritanopia == 2`); `cmpl $0x2 / ja` sends it to the fall-through | silent no-op: `*R`,`*G`,`*B` bit-identical to input; no crash, no diagnostic | `err_row01_impairment_one_past_end` | ✅ |
| 2 | `colourblind` | `Impairment == 4 .. 64` — a sweep of small out-of-range enum values (guards against a jump table indexed without a bound check) | silent no-op for every value | `err_row02_impairment_small_out_of_range_sweep` | ✅ |
| 3 | `colourblind` | `Impairment == u32::MAX` (and `MAX-1`, `0x8000_0000`, `0x7FFF_FFFF`) — extreme unsigned values | silent no-op | `err_row03_impairment_extreme_unsigned` | ✅ |
| 4 | `colourblind` | `Impairment` given as a **negative** `int` (`-1`, `-2`, `-3`, `INT_MIN`). A C enum accepts any `int`; the ABI passes it in `edi` and gcc compares it **unsigned** (`cmpl $0x2,-0x4(%rbp); ja`), so `-1` becomes `0xFFFFFFFF` and is *not* treated as `< 0` | silent no-op — in particular `-1` must **not** alias `cbProtanopia` or wrap into a valid case | `err_row04_impairment_negative_int` | ✅ |
| 5 | `colourblind` | `Impairment` whose **upper 32 bits are dirty** (e.g. `0x1_0000_0000` or `0xDEAD_BEEF_0000_0002` passed in `rdi`). The C reads only `edi`, so the low half decides | low 32 bits decide: `…0002` transforms as `cbTritanopia`, `…0000` as `cbProtanopia`, `0x1_0000_0003` is a no-op | `err_row05_impairment_dirty_high_bits` | ✅ |
| 6 | `colourblind` | out-of-range `Impairment` **combined with** NaN / infinite payload data, to prove the no-op leaves even non-finite bit patterns (incl. sNaN payloads) untouched rather than quieting them | silent no-op, bit-exact, sNaN stays signalling | `err_row06_out_of_range_preserves_exotic_bits` | ✅ |
| 7 | `colourblind` | `R`, `G`, `B` = **NULL** (each individually, and all three at once), with a *valid* impairment | undefined behaviour: the C dereferences unconditionally and faults (`SIGSEGV`). Verified out-of-process so the harness survives; C and Rust must agree on the signal | `err_row07_null_pointers_fault_identically` | ✅ |
| 8 | `colourblind` | `R`, `G`, `B` = NULL with an **out-of-range** impairment | **no fault** — the fall-through path never dereferences, so this is a benign no-op in both C and Rust | `err_row08_null_pointers_with_invalid_impairment_are_safe` | ✅ |
| 9 | `colourblind` | **misaligned** `float*` (byte offsets 1..3 into a buffer) with a valid impairment | `movss` is alignment-agnostic, so the transform succeeds normally; Rust must not fault or reorder | `err_row09_misaligned_pointers` | ✅ |
| 10 | `colourblind` | **aliased** pointers (`R==G`, `R==B`, `G==B`, `R==G==B`) — degenerate/"zero-length" aliasing, the closest analogue of a bad length for a pointer-triple API | not rejected: all three inputs are read into locals *before* any store, then stores happen in the order Red, Green, Blue, so the last store wins. Must match store-for-store | `err_row10_aliased_pointers` | ✅ |
| 11 | `colourblind` | values one step past every meaningful float boundary: `±0`, `±MIN_POSITIVE`, smallest subnormal, largest subnormal, `±MAX`, `±INF`, qNaN, sNaN, `-NaN` (results overflow to `±INF` and underflow to subnormal/zero) | no clamping, no saturation, no error: plain IEEE-754 single-precision results, including `INF - INF = NaN` and sNaN→qNaN quieting | `err_row11_float_boundary_values` | ✅ |

## Notes on why there are no other rows

* No allocation ⇒ no out-of-memory path.
* No length/count/stride parameter exists ⇒ no zero-length or oversized-length
  row is expressible beyond row 10's degenerate aliasing.
* No output-buffer parameter distinct from the input ⇒ no truncation path.
* `Tritanopia`'s `case` has no `break`, but it is the **last** label, so there is
  no fall-through bug to reproduce — and no `default:` for it to fall into.
