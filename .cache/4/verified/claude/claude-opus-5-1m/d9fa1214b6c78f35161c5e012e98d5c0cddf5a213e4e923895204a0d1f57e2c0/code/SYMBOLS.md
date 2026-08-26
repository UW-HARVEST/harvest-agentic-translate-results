# SYMBOLS.md — Phase A / Phase D: public symbol surface

## How this was produced

The C project (`c_src/CMakeLists.txt`) contains exactly one translation unit,
`c_src/src/main.c`, and builds it as an executable:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

`find c_src -name '*.c' -o -name '*.h'` returns that single file, so the whole C
surface is the two functions it defines.

To get a comparable shared object — without modifying anything in `c_src/` — the
same translation unit is compiled with `-shared -fPIC`:

```sh
cc -shared -fPIC -o libdriver_c.so c_src/src/main.c
```

The Rust crate was given a `[lib]` target with `crate-type = ["cdylib"]`
(`src/lib.rs`) whose only purpose is to re-export the identical C ABI surface.
`src/imp.rs` holds the translation and is shared verbatim between the cdylib and
the `driver` binary (`src/main.rs` pulls it in with `#[path = "imp.rs"] mod imp;`),
so the shared object and the executable can never drift apart.

## `nm -D --defined-only` on the C `.so`

```
0000000000001129 T driver
0000000000001174 T main
```

Undefined / imported symbols in the C `.so` (libc, not part of the surface):
`printf@GLIBC_2.2.5`, `__isoc99_scanf@GLIBC_2.7`, plus the weak toolchain
symbols `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.

## `nm -D --defined-only` on the Rust `.so`

```
00000000000174a0 T driver
00000000000174c0 T main
```

## Parity table

| # | C symbol | C type | signature (from `c_src/src/main.c`) | exported by Rust `.so` | Rust definition | status |
|---|----------|--------|------------------------------------|------------------------|-----------------|--------|
| 1 | `driver` | `T` (global text) | `void driver(int x)` | yes, `T` | `#[no_mangle] pub extern "C" fn driver(x: c_int)` in `src/lib.rs` | ✅ match |
| 2 | `main`   | `T` (global text) | `int main(void)` (K&R empty parameter list) | yes, `T` | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` | ✅ match |

There are no macro-generated symbols, no global/static data objects, no weak
aliases and no versioned symbols in the C source, so the table above is the
complete surface.

## Missing-symbol analysis

* Symbols exported by the C `.so` but **missing from the Rust `.so`: none.**
* No C source file was left untranslated: `c_src/src/main.c` is the only C file
  in the project, and both of its functions (`driver`, `main`) are translated in
  `src/imp.rs` and exported from `src/lib.rs`.
* No stubs, `todo!()` or `unimplemented!()` were introduced.
* Undefined symbols in the Rust `.so` are libc / libgcc-unwind imports only
  (`printf`-family, `write`, `mmap`, `pthread_*`, `_Unwind_*`, …); there are no
  undefined non-libc symbols. `check_symbols.sh` verifies this mechanically.

The Rust `.so` additionally exports the usual Rust runtime symbols
(`rust_eh_personality`, `__rust_*` allocator shims, `_ITM_*` weak stubs, …).
Extra symbols are harmless — the requirement is that every C symbol is present
in Rust under the exact same name, which holds.

## Notes on how `main` can be exported at all

A Rust `cdylib` that defines `#[no_mangle] extern "C" fn main` cannot also host
libtest's generated entry point, so `Cargo.toml` sets `test = false` /
`doctest = false` on `[lib]` and the lib is built as `cdylib` only (no `rlib`).
Consequently **all** testing happens from `tests/`, which `dlopen`s the two
shared objects — never linking the crate's Rust code into the test binary. That
is exactly the property the task asks for.

## Verification

```sh
./check_symbols.sh                                  # debug profile
PROFILE_DIR=target/release CARGO_EXTRA=--release ./check_symbols.sh
```

Both print `PASS: every C symbol is exported by the Rust .so.` and an empty
missing-symbol list. `run_difftests.sh` runs this for every feature combination.
