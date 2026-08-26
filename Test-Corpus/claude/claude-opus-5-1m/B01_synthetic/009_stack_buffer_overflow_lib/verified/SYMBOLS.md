# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically:

```sh
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cargo build --offline                       # target/debug/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

The whole C library is a single translation unit (`c_src/src/driver.c`,
114 lines) with one public header (`c_src/include/driver.h`).  There are no
other C sources, so no module was skipped by the translation.

## Defined (exported) dynamic symbols

| # | C symbol (`nm -D`) | C binding | Rust `.so` exports it | Rust item |
|---|--------------------|-----------|-----------------------|-----------|
| 1 | `printLine`    | `T` (global text) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `printIntLine` | `T` (global text) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printIntLine` |
| 3 | `bad`          | `T` (global text) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 4 | `good`         | `T` (global text) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 5 | `driver`       | `T` (global text) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Missing from the Rust `.so`: none.** The symbol diff is empty
(`tests/symbols.rs::c_and_rust_export_the_same_symbols` enforces this
automatically, so it is re-checked on every `cargo test`).

## Deliberately *not* exported

`driver.c` declares two `static` functions, which therefore have local binding
and are absent from `nm -D` on the C `.so`:

| C symbol | C binding | Rust counterpart |
|----------|-----------|------------------|
| `goodG2B` | `t` (local, `static void goodG2B()`) | private `fn good_g2b()` |
| `goodB2G` | `t` (local, `static void goodB2G(int)`) | private `fn good_b2g(c_int)` |

Exporting either of these from Rust would be a *surplus* symbol, not a parity
fix; they are exercised through `good()` / `driver()` instead.

## Undefined (imported) symbols

| C `.so` imports | Rust `.so` imports | note |
|-----------------|--------------------|------|
| `printf@GLIBC_2.2.5` | `printf@GLIBC_2.2.5` | Rust calls libc `printf` directly so the emitted bytes and the stdout buffering behaviour are identical. |
| `puts@GLIBC_2.2.5` | (not needed) | gcc rewrites `printf("%s\n", s)` into `puts(s)`; byte-identical output, so this is an implementation detail, not an API symbol. |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` (all weak) | same class of weak/libc symbols | toolchain-generated, not API. |

0 missing / undefined **non-libc** symbols in the Rust `.so`
(`nm -D -u target/debug/libdriver.so` lists only libc/`ld` runtime symbols;
asserted by `tests/symbols.rs::rust_so_has_no_unexpected_undefined_symbols`).

## Build configurations

`Cargo.toml` has **no `[features]` table** and `src/` contains **no
`#[cfg(feature = …)]`**; `c_src/CMakeLists.txt` defines no options, no
`add_definitions`, and `driver.c`/`driver.h` contain no `#ifdef` other than the
header include guard.  Therefore there is exactly **one** valid feature
combination — the empty one — and

```sh
cargo check --no-default-features   # ==  cargo check  ==  cargo check --all-features
```

are all the same build.  Both cargo profiles (`dev` and `release`, the latter
with `panic = "abort"`) are nevertheless verified by
`./run_all_configs.sh`.
