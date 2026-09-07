# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -nE "return -1|return NULL|return 0|assert|errno|RETURN_ERROR|if *\(|switch|#ifdef|#if |ERROR|goto|abort|exit\(|NULL" -r src include
(no matches)
```

* error-return macros (`RETURN_ERROR`, ...): **0**
* `return -1` / `return NULL` / `return 0`: **0**
* `assert` / `abort` / `exit`: **0**
* explicit `if` statements, `switch`, `goto`: **0**
* null-pointer checks: **0** (the public API takes/returns a struct **by value**;
  there is no pointer in `lib.h` at all)
* range / bounds checks, clamping, saturation: **0**
* error enums, out-params, status codes, min/max constants: **0**
* `#ifdef` / `#if` conditional compilation: **0**

The only control flow in the entire library is six ternary operators (three
channels x two gamma functions) — those are *valid-path* branches and belong to
`CONFIGS.md`, not here.

## The table

`tritanopia` is a **total function**: its only parameter is a by-value
`cb_rgb_255` of three `unsigned char`, so every one of the 2^24 = 16,777,216
possible inputs is valid and none is rejected. The C therefore has **no
rejection path at all** and the table has no functional rows. Recording that as
a fact rather than inventing rows is the honest outcome of the grep above.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `tritanopia` | *(none — no error return, assert, range check, or null check exists in the C source)* | n/a |

## Generic-boundary rows tested anyway (Phase C)

The task requires covering the generic boundaries every C API has, even when the
table is empty. These rows assert **identical behaviour**, not "both fail",
because the C's documented behaviour for each is "succeed and return a value".

| # | boundary probed | trigger | expected C result | test |
|---|-----------------|---------|-------------------|------|
| E1 | numeric minimum of every channel | `{0, 0, 0}` | returns normally; must byte-match Rust | `err_e1_all_min` |
| E2 | numeric maximum of every channel | `{255, 255, 255}` | returns normally; must byte-match Rust | `err_e2_all_max` |
| E3 | one step past max, i.e. the FFI *cannot* express it — `unsigned char` wraps | caller passes `256`/`-1`/`0x1FF` in the register; only the low byte is ABI-significant | C reads the low byte only (`movzbl`): `256 -> 0`, `-1 -> 255` | `err_e3_out_of_range_channel_wraps` |
| E4 | high garbage in the unused 4th byte of the argument eightbyte | pass `0xDEADBEEF`-style padding above the 3 struct bytes | C ignores byte 3 entirely; result depends only on R,G,B | `err_e4_argument_padding_ignored` |
| E5 | out-of-range "enum" value across FFI | `lib.h` declares **no enum**; the nearest analogue is an arbitrary 32-bit word reinterpreted as the struct — every one of the 2^32 words is accepted by the C | C never validates; low 3 bytes decide the result | `err_e5_arbitrary_u32_as_struct` |
| E6 | returned padding byte | the 4th byte of the return register is unspecified | only R,G,B are ABI-significant; the test compares the three fields (and separately records the raw register bytes for information) | `err_e6_return_padding_note` |
| E7 | float->uchar conversion out of `[0,255]` (the one genuinely UB-adjacent spot) | inputs where the tritanopia matrix drives a channel `<0` or `>1`, e.g. `{0,0,255}` and `{255,255,0}`, so `x*255+0.5` leaves `0..255` | GCC emits `cvttss2si %xmm0,%eax; mov %al,..` — truncate toward zero to `i32`, then **wrap** to 8 bits (no saturation) | `err_e7_cast_out_of_range` |
| E8 | zero / oversized "length" | the API has **no length or pointer parameter**, so no such input exists | n/a — documented as inapplicable | — |
| E9 | null pointer | the API has **no pointer parameter**, so no such input exists | n/a — documented as inapplicable | — |

E7 is the class of bug that happy-path testing misses, and it is *reachable*
from ordinary inputs: `{0,0,255}` sends the red row to `-0.1274`, and
`{255,255,0}` sends it to `1.1274`.
