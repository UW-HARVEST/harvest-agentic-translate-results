# SYMBOLS.md — exported-symbol surface (Phase A / Phase D)

## Build commands

C shared object (the C sources are compiled unchanged; nothing in `c_src/` is
modified):

```sh
gcc -shared -fPIC -o <out>/libcdriver.so c_src/src/main.c
```

C executable (exactly as `c_src/CMakeLists.txt` prescribes — it declares
`add_executable(driver src/main.c)`, so the CMake project yields a program, not a
library):

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

Rust shared object with the same C ABI surface, plus the Rust executable:

```sh
cargo build --lib --bins --examples      # -> target/debug/examples/libcapi.so
                                         # -> target/debug/driver
```

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001188 T main
                 U printf@GLIBC_2.2.5
                 U puts@GLIBC_2.2.5
0000000000001139 T static_alias
                 U strtol@GLIBC_2.2.5
```

`printf`, `puts` and `strtol` are undefined libc imports; `_ITM_*`,
`__cxa_finalize` and `__gmon_start__` are weak toolchain/CRT symbols emitted by
the linker for every shared object (they are also weak-undefined in the Rust
`.so`). The **defined, non-libc surface is therefore exactly two symbols.**

## Parity table

| # | C symbol | kind | Rust `.so` (`target/debug/examples/libcapi.so`) | where implemented | status |
|---|----------|------|--------------------------------------------------|-------------------|--------|
| 1 | `static_alias` | `T` (global text) | `static_alias` (`T`) | `src/lib.rs` — `#[no_mangle] pub unsafe extern "C" fn static_alias` | present |
| 2 | `main`         | `T` (global text) | `main` (`T`)         | `examples/capi.rs` — `#[no_mangle] pub unsafe extern "C" fn main` forwarding to `driver::c_main` (`src/lib.rs`) | present |

`nm -D --defined-only` on the Rust `.so`:

```
0000000000014f90 T main
0000000000015d10 T static_alias
```

Symbol diff (C defined symbols missing from the Rust `.so`): **empty**.
No stubs, no `unimplemented!()`: both symbols are complete translations of the
corresponding C definitions.

Notes:

* `main` cannot be exported from `src/lib.rs` directly, because the `driver`
  binary target links that library and rustc emits its own `main` symbol for the
  executable (duplicate-symbol link error: `rust-lld: error: duplicate symbol:
  main`). The `#[no_mangle]` wrapper therefore lives in the separate `capi`
  cdylib target, which is what the differential tests load; the library target
  itself is an `rlib` and contributes the `#[no_mangle] static_alias` export to
  that cdylib.
* The `capi` export of `main` is `#[cfg(not(test))]`: `cargo test --all-targets`
  compiles every target in test mode, and the generated libtest entry point would
  otherwise collide with it. The `.so` the tests load is always the normal
  (non-test) build, which does export `main` — verified by `tests/symbols.rs`
  running `nm -D` on the very file that gets `dlopen`ed.
* Rust runtime/allocator symbols (`__rust_alloc`, `rust_eh_personality`, …) are
  additional exports of the Rust `.so`; extra symbols are allowed — the
  requirement is that every C symbol exists in Rust.

## Verification

`tests/symbols.rs` re-derives both symbol lists at test time with `nm -D`
(the C `.so` is compiled by the test itself) and fails if any C-defined symbol
is missing from the Rust `.so`. It additionally `dlopen`s both objects and
resolves each symbol through `libloading`, so the exports are proven to be
callable and not merely present in the symbol table.
