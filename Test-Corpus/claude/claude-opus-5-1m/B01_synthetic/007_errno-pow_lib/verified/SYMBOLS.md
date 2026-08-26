# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libpow.so

# Rust
cargo build --offline --no-default-features
# -> target/debug/libpow.so
```

## Translation-unit inventory (completeness check)

The whole C library is two files; both are accounted for in the Rust
translation, so there is no skipped module.

| C source file | contents | translated in |
|---|---|---|
| `c_src/include/pow.h` | declares `double my_pow(double, double)` (only decl in header) | `src/lib.rs` |
| `c_src/src/pow.c` | defines `my_pow` (only definition in the file; no `static` helpers) | `src/lib.rs` |

`CMakeLists.txt` builds exactly one target (`add_library(pow SHARED src/pow.c)`),
so `src/pow.c` is the complete implementation surface.

## Exported (defined, dynamic) symbols

| # | symbol | type | C `.so` | Rust `.so` | status |
|---|--------|------|---------|------------|--------|
| 1 | `my_pow` | `T` (global text) | yes | yes | **MATCH** |

`my_pow` is exported from Rust via
`#[unsafe(no_mangle)] pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double`.

### Symbol diff

```sh
comm -23 <(nm -D --defined-only c_src/build/libpow.so | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/debug/libpow.so | awk '{print $3}' | sort)
```

Output: **empty**. Every symbol exported by the C `.so` is exported by the Rust
`.so` under the exact same name. There are no macro-generated symbols in this
library.

## Undefined (imported) symbols

The C `.so` imports 4 real symbols. The Rust `.so` imports the **same 4**, with
byte-identical glibc version tags — this matters, because `pow` is a versioned
symbol and the two glibc versions have different `errno` semantics.

| imported symbol | C `.so` | Rust `.so` | note |
|---|---|---|---|
| `pow@GLIBC_2.29` | yes | yes | **same version tag.** The Rust code deliberately calls libm `pow` through `extern "C"` instead of `f64::powf`, because `f64::powf` never sets `errno` and would silently dead-code the entire error surface. |
| `__errno_location@GLIBC_2.2.5` | yes | yes | glibc's TLS `errno` accessor; `errno` is a macro for `*__errno_location()`. Same TLS slot is shared by both `.so`s in one process. |
| `fprintf@GLIBC_2.2.5` | yes | yes | Rust calls variadic `fprintf` so `%.2f` rendering (incl. `inf`/`nan` spellings and the 309-digit `DBL_MAX` expansion) is produced by the same libc code. |
| `stderr@GLIBC_2.2.5` | yes | yes | the same `FILE*` object, so output ordering/buffering is identical. |

The Rust `.so` additionally imports the usual Rust `std` runtime dependencies
(`_Unwind_*` from libgcc, and libc `malloc`/`memcpy`/`write`/`pthread_key_*`/…).
These are all libc/libgcc runtime symbols, not library-surface symbols, and are
expected for any `cdylib` linked against `std`.

## Completion gate item

- [x] `nm -D` shows **0 missing** symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows **0 undefined non-libc/non-libgcc** symbols in the Rust `.so`.
