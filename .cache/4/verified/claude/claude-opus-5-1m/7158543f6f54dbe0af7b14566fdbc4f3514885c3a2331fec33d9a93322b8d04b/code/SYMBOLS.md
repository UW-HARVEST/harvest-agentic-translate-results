# SYMBOLS.md — exported symbol parity (Phase A / Phase D)

## How this table was produced

The C project is an **executable** (`c_src/CMakeLists.txt`:
`add_executable(driver src/main.c)`), so it was additionally compiled as a
shared object in order to compare dynamic symbol tables:

```sh
# C shared object (same flags as the CMake default configuration: no -O, no -D)
gcc -fPIC -shared -o cbuild/libcdriver.so c_src/src/main.c
nm -D --defined-only cbuild/libcdriver.so

# Rust shared object (crate-type = ["cdylib"], src/ffi.rs)
cargo build --release
nm -D --defined-only target/release/libdriver.so
```

`c_src/src/main.c` is the *only* C translation unit in the project
(`find c_src -type f` → `CMakeLists.txt`, `src/main.c`), so the two symbols
below are the complete public surface — no C module was skipped by the
translation.

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C signature | Rust export |
|---|--------|---------|------------|-------------|-------------|
| 1 | `driver` | `T driver` | `T driver` | `void driver(int x)` | `#[no_mangle] pub extern "C" fn driver(x: c_int)` → `prog::driver` |
| 2 | `main`   | `T main`   | `T main`   | `int main(void)`      | `#[no_mangle] pub extern "C" fn main() -> c_int` → `prog::main`, returns `0` |

Both sides export **exactly** these two symbols and nothing else:

```text
$ nm -D --defined-only cbuild/libcdriver.so
0000000000001129 T driver
000000000000115e T main

$ nm -D --defined-only target/release/libdriver.so
0000000000012f10 T driver
0000000000012f20 T main
```

`nm -D --defined-only <c.so> | awk '{print $3}' | sort` diffed against the same
command on the Rust `.so` yields an **empty diff** (asserted by the test
`symbol_parity::c_defined_symbols_are_all_exported_by_rust`).

There are no macro-generated symbols in the C source (`grep -nE '#if|#ifdef|#define' c_src/src/main.c`
→ no matches), so no additional names can appear under any configuration.

## Undefined (imported) symbols

The C `.so` imports only libc symbols; the Rust `.so` is statically linked
against Rust `std` and therefore imports a different (also libc-only) set.
This is not a completeness failure — it is the expected difference between a
libc-hosted object and a Rust `std` object.

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|------------|------|
| `__isoc99_scanf@GLIBC_2.7` | `U` | – | replaced by the hand-written `scanf_d` reader in `src/main.rs` |
| `printf@GLIBC_2.2.5` | `U` | – | replaced by `write!(stdout, "{}\n", y)` |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*` | `w` (weak) | `w` (weak) | toolchain boilerplate, not API |
| `memcpy`, `write`, `read`, `pthread_*`, `malloc`, … | – | `U` | Rust `std` runtime, all libc |

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.**

## Executable symbol check

For completeness, the two *executables* also agree on the only symbol a loader
cares about:

| symbol | `c_src/build/driver` | `target/release/driver` |
|--------|----------------------|-------------------------|
| `main` | present (`T`/`t`)    | present (`T`)           |
