# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts compared:

* C:    `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C `.so` dynamic symbols (`nm -D --defined-only`)

| # | symbol | C type | source of the symbol | exported by Rust `.so`? |
|---|--------|--------|----------------------|-------------------------|
| 1 | `driver`    | `T` (global text) | `c_src/src/driver.c:38`, declared in `c_src/include/driver.h:27` | YES |
| 2 | `printLine` | `T` (global text) | `c_src/src/driver.c:30` — non-`static`, so it has external linkage and is part of the library ABI even though it is absent from the public header | YES |

No macro-generated symbols exist: `c_src/src/driver.c` contains no
function-defining macros, and `c_src/include/driver.h` defines only the
`DRIVER_H_` include guard.

## Rust `.so` dynamic symbols (`nm -D --defined-only`)

| # | symbol | Rust definition |
|---|--------|-----------------|
| 1 | `driver`    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(data: c_int)` |
| 2 | `printLine` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine(line: *const c_char)` |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0.** No implementation is absent, so
neither the "add a `#[no_mangle]` wrapper" rule nor the "translate a skipped
module" rule applies. `c_src` consists of exactly one translation unit
(`src/driver.c`) plus one header, both fully translated in
`translation/src/lib.rs`; no C module was skipped.

## Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol. Undefined symbols of each
library, filtered to non-libc:

| library | undefined non-libc symbols |
|---------|----------------------------|
| C    | none (imports only `printf`, `memset`, `strncpy` and the glibc/ld startup symbols) |
| Rust | none (imports only `printf`, `memset`, `strncpy` and the glibc/ld startup symbols) |

The Rust translation deliberately calls libc `memset`/`strncpy`/`printf`
directly rather than reimplementing them, so the observable behaviour
(including stdout buffering and the out-of-bounds accesses the C performs) is
identical.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Verification result

Measured on both Rust artifacts:

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so             | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/debug/libdriver.so | awk '{print $3}' | sort)
$ diff <(nm -D --defined-only c_src/build/libdriver.so               | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Both diffs are empty. Symbol set on both sides: `driver`, `printLine`.

Undefined-symbol check: after removing the `@GLIBC_*`-versioned glibc imports
and the reserved `_`-prefixed ld/glibc names, **zero** undefined symbols remain
in either Rust artifact, so nothing is left unresolved.

Automated by `translation/verify.sh` steps 6 and 7, and asserted from inside the
suite by the test `sym_01_exported_symbol_parity`, which resolves every C-exported
name through `dlsym` on the Rust `.so`, and `sym_02_rust_exports_are_callable_via_dlsym`,
which calls both through the `.so` boundary.
