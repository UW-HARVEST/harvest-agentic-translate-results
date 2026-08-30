# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | source | present in Rust `.so`? |
|---|--------|------|--------|------------------------|
| 1 | `driver` | `T` (global text) | `c_src/src/driver.c:28`, declared `c_src/include/driver.h:27` | YES — `translation/src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn driver` |

The C library declares exactly one public entry point in its public header
(`void driver(int x);`) and defines exactly one non-static function. There are
no macro-generated symbols, no exported globals/data symbols, no static
initializers, and no additional translation units in `CMakeLists.txt`
(`add_library(driver SHARED src/driver.c)` — a single source file).

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

| # | symbol | type | note |
|---|--------|------|------|
| 1 | `driver` | `T` (global text) | matches C exactly |

## Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

* Symbols in C but missing from Rust: **0** — diff is EMPTY.
* No stubs, no `unimplemented!()`, no faked exports: the single symbol is a
  real translation of the single C function.
* No C source file was left untranslated (`driver.c` is the only source).

## Undefined (imported) symbols

The C `.so` imports `printf@GLIBC_2.2.5` plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` glibc/ITM markers.

The Rust `.so` imports the same `printf@GLIBC_2.2.5` (it deliberately calls the
platform `printf` so formatting/stream/buffering behaviour is byte-identical),
plus the Rust standard-library runtime's own libc/`_Unwind_*` imports
(`malloc`, `memcpy`, `write`, `dl_iterate_phdr`, …).

* Undefined **non-libc / non-runtime** symbols in the Rust `.so`: **0**.
  Every unresolved symbol is provided by glibc or libgcc, exactly as for the C
  `.so`. Nothing is left dangling.

## Verification status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.

## Harness sensitivity (mutation check)

To prove the differential suite is not vacuously passing, four mutations were
injected into `src/lib.rs`, rebuilt, and the suite re-run; each was **detected**
(tests failed), and the original file was restored afterwards:

| mutation | detected? |
|----------|-----------|
| `j = j.wrapping_add(3)` instead of `+2` | yes |
| `while i <= x` instead of `while i < x` | yes |
| format string `"%d  %d\n"` (two spaces) | yes |
| `#[unsafe(no_mangle)]` removed (export lost) | yes |

Run everything with `bash translation/verify.sh`.
