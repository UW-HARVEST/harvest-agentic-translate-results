# SYMBOLS.md — public ABI surface parity

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-DwDtPG.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  i.e. **no** `CMAKE_BUILD_TYPE` → `-O0`)
* Rust `.so`: `translation/target/release/libunderhanded_c_nuke_lib.so`

## Defined (exported) symbols

`nm -D --defined-only` on each object:

| # | symbol | in C `.so` | in Rust `.so` | C signature (as compiled) | notes |
|---|--------|-----------|---------------|---------------------------|-------|
| 1 | `match`             | `T` | `T` | `int match(double *test, double *reference, int bins, double threshold)` | `match.c` includes `match.h`, so its `float_t` is `double`. Exported from Rust as `#[unsafe(no_mangle)] extern "C" fn r#match`. |
| 2 | `spectral_contrast` | `T` | `T` | `double spectral_contrast(float *a, float *b, int length)` | **`float`, not `double`** — see below. |

Symbol diff: **empty**. No symbol exported by the C `.so` is missing from the
Rust `.so`, and the Rust `.so` exports no extra `T`/`D`/`B` symbols.

```
$ nm -D --defined-only libharvest-work-DwDtPG.so
0000000000001322 T match
00000000000015cd T spectral_contrast

$ nm -D --defined-only libunderhanded_c_nuke_lib.so
0000000000011c90 T match
0000000000011ee0 T spectral_contrast
```

## `static` (non-exported) functions — translated but deliberately not exported

These are `static` in C and therefore absent from `nm -D`. The Rust translation
keeps them private for the same reason; exporting them would be a *surplus*
symbol, not parity.

| C function | file | Rust counterpart |
|---|---|---|
| `static double total(float_t *, int)`                 | `src/match.c`             | `matching::total`            |
| `static void smoothen(float_t *, int)`                | `src/match.c`             | `matching::smoothen`         |
| `static void differentiate(float_t *, int)`           | `src/match.c`             | `matching::differentiate`    |
| `static void preprocess(float_t *, float_t *, int)`   | `src/match.c`             | `matching::preprocess`       |
| `static double dot_product(float_t *, float_t *, int)`| `src/spectral_contrast.c` | `spectral_contrast::dot_product` |
| `static void normalize(float_t *, int)`               | `src/spectral_contrast.c` | `spectral_contrast::normalize`   |

## The `float_t` type split (the reason the two entry points disagree)

`include/match.h` has `typedef double float_t;`, but `src/spectral_contrast.c`
**never includes `match.h`** — it includes only `<math.h>`. So inside that
translation unit `float_t` is C99's `<math.h>` `float_t`, which on x86-64 glibc
(`FLT_EVAL_METHOD == 0`) is `float`:

```
$ ./fe            # printf("FLT_EVAL_METHOD=%d sizeof(float_t)=%zu")
FLT_EVAL_METHOD=0 sizeof(float_t)=4
```

Confirmed in the compiled object — `dot_product`/`normalize` use a 4-byte
element stride and single-precision ops:

```
14e6:  lea    0x0(,%rax,4),%rdx     ; i * 4  -> float stride
14f5:  movss  (%rax),%xmm1          ; a[i]
150d:  movss  (%rax),%xmm0          ; b[i]
1511:  mulss  %xmm1,%xmm0           ; single-precision multiply
```

whereas `total`/`smoothen`/`differentiate` in `match.c` use an 8-byte stride
(`lea 0x0(,%rax,8),%rdx`, `movsd`, `addsd`).

Consequence, reproduced verbatim by the translation: `match` builds two
`double` VLAs and hands them to `spectral_contrast`, which reinterprets the
first `bins * 4` bytes of each as `bins` `float` lanes, normalises them in
place, and dots them. `match` therefore only ever looks at the low halves of
its first `bins/2` preprocessed `double`s.

## Undefined symbols

C `.so`: `memcpy@GLIBC_2.14`, `sqrt@GLIBC_2.2.5` plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` — all libc.

Rust `.so`: libc (`malloc`, `memcpy`, `memmove`, `memset`, `free`, …) and the
`libgcc` unwinder (`_Unwind_*`) pulled in by the standard library.
**0 missing/undefined non-libc symbols.**
