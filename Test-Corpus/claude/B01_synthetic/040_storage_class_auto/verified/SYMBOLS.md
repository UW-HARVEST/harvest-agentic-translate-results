# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Build configurations

`c_src/CMakeLists.txt` declares exactly one target and no options:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

* No `option()`, no `add_compile_definitions`, no `#ifdef` in `c_src/src/main.c`
  → **one** C configuration (default flags, no `-O`, no `-D`).
* `Cargo.toml` has **no `[features]` section** → the only valid feature
  combination is the empty/default one. Enumerated combos:

  | # | cargo invocation | notes |
  |---|------------------|-------|
  | 1 | `cargo check/test --no-default-features` | the only combination |
  | 2 | `cargo check/test` (default features = none) | identical to #1 |
  | 3 | `cargo check/test --all-features` | identical to #1 (no features exist) |

## Shared libraries under comparison

`CMakeLists.txt` builds an *executable*, so for symbol-level and FFI-level
differential testing the same single translation unit is additionally compiled
as a shared object with the same (default) flags:

```
gcc -fPIC -shared -o c_src/build/libcdriver.so c_src/src/main.c
```

Rust side (`Cargo.toml`): `[lib] crate-type = ["cdylib"]` →
`target/debug/libdriver.so`. `src/main.rs` (bin `driver`) and `src/lib.rs`
(cdylib) share the single translation module `src/driver_impl.rs`, so the binary
and the shared library cannot drift apart.

## `nm -D --defined-only` comparison

C — `c_src/build/libcdriver.so`:

| symbol | type | C declaration |
|--------|------|---------------|
| `driver` | `T` (global text) | `void driver(int x)` |
| `main`   | `T` (global text) | `int main()` |

Rust — `target/debug/libdriver.so`:

| symbol | type | Rust definition |
|--------|------|-----------------|
| `driver` | `T` | `#[no_mangle] pub extern "C" fn driver(x: c_int)` (`src/lib.rs`) |
| `main`   | `T` | `#[no_mangle] pub extern "C" fn main() -> c_int` (`src/lib.rs`) |

**Symbol diff (C exports not exported by Rust): EMPTY.**

`tests/symbols.rs::c_symbols_are_all_exported_by_rust` re-derives both lists
with `nm -D --defined-only` at test time and asserts the diff is empty, so the
table above cannot go stale. It also asserts both symbols are dynamically
loadable with `dlsym` (via `libloading`).

### Undefined (imported) symbols

`nm -D -u c_src/build/libcdriver.so`:

| symbol | kind |
|--------|------|
| `printf@GLIBC_2.2.5` | libc |
| `__isoc99_scanf@GLIBC_2.7` | libc |
| `__cxa_finalize@GLIBC_2.2.5` | libc (weak) |
| `__gmon_start__`, `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable` | weak toolchain stubs |

All are libc / toolchain symbols. The Rust `.so` imports only libc + `ld.so`
symbols as well (`read`, `write`, `memcpy`, `malloc`, `signal`, …); it has
**0 undefined non-libc symbols**, i.e. nothing from the C library is left
untranslated and referenced.

### Translation completeness

`c_src/` contains exactly one C file (`src/main.c`, 36 lines) with exactly two
functions, `driver` and `main`. Both are translated in
`src/driver_impl.rs` (`driver_impl::driver`, `driver_impl::run`) and both are
exported with their C names from `src/lib.rs`. No C source file, function, or
symbol is missing, stubbed, or `unimplemented!()`.

## Final verification (Phase D)

```text
$ nm -D --defined-only c_src/build/libcdriver.so   |  $ nm -D --defined-only target/so-build/debug/libdriver.so
driver                                             |  driver
main                                               |  main
```

* C symbols missing from Rust: **none** (`tests/symbols.rs::c_symbols_are_all_exported_by_rust`).
* Non-libc undefined symbols in either `.so`: **none**, and both libraries
  `dlopen` successfully, which proves every non-weak import resolves
  (`tests/symbols.rs::rust_so_has_no_non_libc_undefined_symbols`).
* Both exports are reachable through `dlsym`
  (`tests/symbols.rs::both_exports_are_dlsym_reachable`).
* All three tests pass under every feature combination enumerated above
  (`./run_all_configs.sh`).
