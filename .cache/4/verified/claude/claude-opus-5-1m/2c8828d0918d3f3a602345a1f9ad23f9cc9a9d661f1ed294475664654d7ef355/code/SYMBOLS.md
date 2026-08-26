# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build --release
# -> target/release/libunderhanded_c_nuke_lib.so
```

## Defined (exported) dynamic symbols

`nm -D --defined-only <so> | sort`

| # | C symbol (`libtranslated_rust.so`) | present in Rust `.so`? | Rust source |
|---|------------------------------------|------------------------|-------------|
| 1 | `T match`                          | YES — `T match`        | `src/match_.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn r#match`) |
| 2 | `T spectral_contrast`              | YES — `T spectral_contrast` | `src/spectral_contrast.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn spectral_contrast`) |

**Symbol diff (C exported minus Rust exported): EMPTY.**

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only target/release/libunderhanded_c_nuke_lib.so | awk '{print $2, $3}' | sort)
(no output)
```

Both libraries export exactly `T match` and `T spectral_contrast`.

### Why there are only two symbols

Every other function in the two translation units is `static`, so it has
internal linkage and is deliberately *not* exported. These are translated as
private Rust `fn`s and must NOT be exported (exporting them would be a symbol
surface mismatch in the other direction):

| C static function | file | Rust counterpart |
|---|---|---|
| `static double total(float_t*, int)`                  | `match.c` | `match_::total` |
| `static void smoothen(float_t*, int)`                 | `match.c` | `match_::smoothen` |
| `static void differentiate(float_t*, int)`            | `match.c` | `match_::differentiate` |
| `static void preprocess(float_t*, float_t*, int)`     | `match.c` | `match_::preprocess` |
| `static double dot_product(float_t*, float_t*, int)`  | `spectral_contrast.c` | `spectral_contrast::dot_product` |
| `static void normalize(float_t*, int)`                | `spectral_contrast.c` | `spectral_contrast::normalize` |

No C source file was skipped: `CMakeLists.txt` compiles exactly
`src/match.c` + `src/spectral_contrast.c`, and both are translated
(`src/match_.rs`, `src/spectral_contrast.rs`). No stubs, no `unimplemented!()`.

## Undefined (imported) symbols

`nm -D --undefined-only <so>`

C: `memcpy@GLIBC_2.14`, `sqrt@GLIBC_2.2.5`, plus the usual weak
`_ITM_*`/`__cxa_finalize`/`__gmon_start__`.

Rust: `memcpy`, `memmove`, `memset`, `bcmp`, `malloc`, `calloc`, `realloc`,
`free`, `posix_memalign`, `abort`, `__errno_location`, `getenv`, `getcwd`,
`open64`/`read`/`write`/`close`/`lseek64`/`stat64`/`fstat64`/`readlink`/
`realpath`/`mmap64`/`munmap`/`writev`/`syscall`, `pthread_key_*`,
`pthread_setspecific`, `__tls_get_addr`, `dl_iterate_phdr`, `strlen`, and the
`_Unwind_*` family from `libgcc_s.so.1` (panic machinery + std's default
panic-hook backtrace support).

`NEEDED`: `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`.

**0 missing/undefined non-libc symbols** — every Rust import is satisfied by
libc (`libc.so.6`), the dynamic loader, or the compiler unwinder
(`libgcc_s.so.1`). Rust resolves `sqrt` to the inline `sqrtsd` instruction
instead of the libm call, which is bit-identical (both are IEEE-754
correctly-rounded square root; glibc's x86-64 `sqrt` is itself `sqrtsd`).

## ABI note that drives the whole translation

`c_src/include/match.h` does `typedef double float_t;`, but
`c_src/src/spectral_contrast.c` **does not include `match.h`** — it only
includes `<math.h>`, which supplies C99's own `float_t`. On this target:

```
$ ./flt      # printf("%d %zu", __FLT_EVAL_METHOD__, sizeof(float_t))
FLT_EVAL_METHOD=0
sizeof(float_t)=4 sizeof(double_t)=8
```

So `float_t` is `double` (8 bytes) inside `match.c` and `float` (4 bytes)
inside `spectral_contrast.c`. `match()` therefore builds two `double` VLAs and
passes them to `spectral_contrast()`, which reinterprets that same memory as
`float`. Confirmed in the generated code:

```
dot_product:  movss / movss / mulss / cvtss2sd / addsd
normalize:    movss / cvtss2sd / divsd / cvtsd2ss / movss
```

i.e. the product is rounded to *single* precision before being widened and
accumulated into a `double`, and the quotient is narrowed back to `float` on
store. The Rust translation reproduces this exactly (`*mut f32` in
`spectral_contrast.rs`, `t.base_ptr() as *mut f32` at the call site in
`match_.rs`). This is a bug in the original C, but it is *observable*
behaviour, so it is replicated rather than "fixed".
