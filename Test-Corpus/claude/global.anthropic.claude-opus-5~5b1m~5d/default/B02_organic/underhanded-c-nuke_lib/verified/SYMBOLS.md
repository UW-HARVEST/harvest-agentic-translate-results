# SYMBOLS.md — exported-surface parity

Reference C shared object, built exactly as the task prescribes:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-SppITj.so   (project name == parent dir name)
```

Rust shared object:

```
cd translation && cargo build --release
# -> translation/target/release/libunderhanded_c_nuke_lib.so
```

> Note: `translation/Cargo.toml` originally declared `[lib] name =
> "underhanded-c-nuke_lib"`, which Cargo rejects (`library target names cannot
> contain hyphens`) — the crate did not even parse. Renamed to
> `underhanded_c_nuke_lib` (Cargo's own hyphen→underscore normalisation) so the
> crate builds. That was the only `cargo check` error.

## `nm -D --defined-only` — C

```
0000000000001322 T match
00000000000015cd T spectral_contrast
```

## `nm -D --defined-only` — Rust

```
0000000000012e70 T match
0000000000013080 T spectral_contrast
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `match`             | `T` | `T` | `int match(float_t *test, float_t *reference, int bins, double threshold)`. Exported from Rust as `#[unsafe(no_mangle)] pub unsafe extern "C" fn r#match(*mut f64, *mut f64, c_int, f64) -> c_int`. |
| 2 | `spectral_contrast` | `T` | `T` | `double spectral_contrast(float_t *a, float_t *b, int length)`. **`float_t` here is `float`, not `double`** — see below. Exported from Rust as `#[unsafe(no_mangle)] pub unsafe extern "C" fn spectral_contrast(*mut f32, *mut f32, c_int) -> f64`. |

Symbol diff (C-exported minus Rust-exported): **empty**. Nothing is missing, and
the Rust `.so` exports no extra `T` symbols of its own.

## Undefined (imported) symbols

| symbol | C | Rust | ok? |
|--------|---|------|-----|
| `memcpy@GLIBC_2.14`     | `U` | inlined (`ptr::copy_nonoverlapping`) | yes — libc |
| `sqrt@GLIBC_2.2.5`      | `U` | `sqrtsd` inlined by LLVM              | yes — libc/intrinsic |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*registerTMCloneTable` | `w` | crt/runtime provided | yes — toolchain glue, not API |

`nm -D -u` on the Rust `.so` lists only libc/`libgcc`/runtime symbols; there are
**0 missing or undefined non-libc symbols**.

## Local (non-exported) symbols — intentionally NOT in `nm -D`

All of these are `static` in the C and therefore appear as lowercase `t` in
`nm` (not in `nm -D`). They are translated as private Rust `fn`s and must NOT
be exported:

| C symbol | file | Rust counterpart |
|----------|------|------------------|
| `total`         | `src/match.c`             | `unsafe fn total` |
| `smoothen`      | `src/match.c`             | `unsafe fn smoothen` |
| `differentiate` | `src/match.c`             | `unsafe fn differentiate` |
| `preprocess`    | `src/match.c`             | `unsafe fn preprocess` |
| `dot_product`   | `src/spectral_contrast.c` | `unsafe fn dot_product` |
| `normalize`     | `src/spectral_contrast.c` | `unsafe fn normalize` |

Every C source file in `c_src/src` (`match.c`, `spectral_contrast.c`) and every
function in them is translated; no module was skipped, and no symbol is stubbed.

## The `float_t` split — why the two entry points disagree on element size

`c_src/include/match.h`:

```c
#define N_SMOOTH 16
typedef double float_t;
int match(float_t *test, float_t *reference, int bins, double threshold);
double spectral_contrast(float_t *a, float_t *b, int length);
```

* `match.c` includes `"match.h"` → `float_t` == `double` (stride 8).
* `spectral_contrast.c` includes **only** `<math.h>` and never `match.h`.
  glibc's `<math.h>` defines its own `float_t`; on x86-64
  `__FLT_EVAL_METHOD__ == 0`, so `float_t` == `float` (stride 4).

Confirmed from the disassembly of the built `.so`:

```
dot_product:  lea 0x0(,%rax,4),%rdx    ; 4-byte stride
              movss (%rax),%xmm1       ; f32 load
              mulss %xmm1,%xmm0        ; f32 multiply
              cvtss2sd %xmm0,%xmm0     ; widen to f64
total:        lea 0x0(,%rax,8),%rdx    ; 8-byte stride
              movsd (%rax),%xmm0       ; f64 load
```

So the true ABI of the exported `spectral_contrast` is
`double spectral_contrast(float *, float *, int)`, and `match` — which passes
its `double` VLAs to it — makes it reinterpret the low half of those buffers as
`float`s. Both facts are reproduced verbatim by the Rust.
