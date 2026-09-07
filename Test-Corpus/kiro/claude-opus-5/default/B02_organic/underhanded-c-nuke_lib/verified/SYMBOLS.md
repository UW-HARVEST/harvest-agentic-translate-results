# SYMBOLS.md — Exported-symbol parity

Generated mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libharvest-work-ifjFx4.so
Rust: translation/target/release/libunderhanded_c_nuke_lib.so
```

## C `.so` defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | C type | exported by Rust `.so`? |
|---|--------|--------|--------------------------|
| 1 | `match` | `int match(double *test, double *reference, int bins, double threshold)` | YES (`src/match.rs`, `#[unsafe(no_mangle)] extern "C" fn r#match`) |
| 2 | `spectral_contrast` | `double spectral_contrast(float *a, float *b, int length)` | YES (`src/spectral_contrast.rs`, `#[unsafe(no_mangle)] extern "C" fn spectral_contrast`) |

Weak/undefined entries in the C `.so` are toolchain- or libc-provided and are
not part of the library surface:

| symbol | kind | note |
|--------|------|------|
| `_ITM_deregisterTMCloneTable` | `w` (weak undef) | GCC transactional-memory stub |
| `_ITM_registerTMCloneTable`   | `w` (weak undef) | GCC transactional-memory stub |
| `__cxa_finalize@GLIBC_2.2.5`  | `w` (weak undef) | libc |
| `__gmon_start__`              | `w` (weak undef) | profiling hook |
| `memcpy@GLIBC_2.14`           | `U` (undef)      | libc, used by `preprocess` |
| `sqrt@GLIBC_2.2.5`            | `U` (undef)      | libm, used by `normalize` |

## Static (non-exported) C functions

These have internal linkage and appear in **no** dynamic symbol table, so they
are not required to be exported by the Rust `.so`. They are nonetheless fully
translated because `match` / `spectral_contrast` depend on them.

| C static symbol | file | Rust counterpart |
|-----------------|------|------------------|
| `total`         | `src/match.c`             | `match.rs::total` |
| `smoothen`      | `src/match.c`             | `match.rs::smoothen` |
| `differentiate` | `src/match.c`             | `match.rs::differentiate` |
| `preprocess`    | `src/match.c`             | `match.rs::preprocess` |
| `dot_product`   | `src/spectral_contrast.c` | `spectral_contrast.rs::dot_product` |
| `normalize`     | `src/spectral_contrast.c` | `spectral_contrast.rs::normalize` |

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only rust.so| awk '{print $3}' | sort)
<empty>
```

**Result: 0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

## ABI notes that the symbol table alone does not show

`include/match.h` contains `typedef double float_t;` and declares

```c
double spectral_contrast(float_t *a, float_t *b, int length);
```

but `src/spectral_contrast.c` **never includes `match.h`** — it includes only
`<math.h>`. Its `float_t` is therefore C99's `<math.h>` `float_t`, which on
x86-64 glibc (`FLT_EVAL_METHOD == 0`) is `float`. Confirmed from the compiled
object: `spectral_contrast`/`dot_product`/`normalize` use `movss` / `mulss` /
`cvtss2sd` / `cvtsd2ss` and a 4-byte element stride, while `match`'s helpers use
`movsd` / `addsd` / `subsd` and an 8-byte stride.

Consequences that both implementations must share:

* the exported `spectral_contrast` takes `float *`, length counted in `float`s;
* `match` hands `double`-typed VLAs to it, so only the low 4 bytes of each of
  the first `bins` `double` slots are read/written, and the normalised `float`
  results are stored back over those same bytes.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one (`--no-default-features` is also equivalent,
as there are no default features). Verified with `cargo check
--no-default-features` and `cargo test --no-default-features`.
