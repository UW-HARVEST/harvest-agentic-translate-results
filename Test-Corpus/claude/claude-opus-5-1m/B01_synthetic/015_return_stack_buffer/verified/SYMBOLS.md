# SYMBOLS.md — Phase A: exported-symbol surface

## How the two shared objects are produced

`c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`, i.e. the C
project has exactly **one** translation unit, `c_src/src/main.c`. The same TU is
compiled into a shared object for differential testing:

```sh
gcc -shared -fPIC -o cbuild/libcdriver.so c_src/src/main.c
```

The Rust side produces its shared object from `[lib] crate-type = ["cdylib"]`:

```sh
cargo build            # -> target/debug/libdriver.so
```

## `nm -D` on the C shared object

Raw output (`nm -D cbuild/libcdriver.so`):

```text
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
0000000000001196 T bad
00000000000011bc T good
00000000000011d5 T main
0000000000001149 T printLine
                 U puts@GLIBC_2.2.5
```

## `nm -D` on the Rust shared object

```text
0000000000016560 T bad
0000000000016580 T good
00000000000165a0 T main
00000000000165d0 T printLine
```

## Parity table

Every symbol the C `.so` **defines** (`T`), and its status in the Rust `.so`:

| # | C symbol | C type | Rust `.so` | Rust definition site | Notes |
|---|----------|--------|-----------|----------------------|-------|
| 1 | `printLine` | `T` (global text) | present, `T` | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn printLine(*const c_char)` | `void printLine(const char *line)` |
| 2 | `bad`       | `T` (global text) | present, `T` | `src/lib.rs` `#[no_mangle] pub extern "C" fn bad()` | `void bad(void)` |
| 3 | `good`      | `T` (global text) | present, `T` | `src/lib.rs` `#[no_mangle] pub extern "C" fn good()` | `void good(void)` |
| 4 | `main`      | `T` (global text) | present, `T` | `src/lib.rs` `#[no_mangle] pub extern "C" fn main() -> c_int` | `int main(void)`; `#[cfg(not(test))]` only because libtest emits its own `main` entry symbol — the shipped `cdylib` is never built with `cfg(test)` |

**Missing symbols: 0.** Verified mechanically by
`tests/symbol_parity.rs::c_defined_symbols_all_exported_by_rust`, which shells
out to `nm -D` on both objects and diffs the `T`/`W`/`D`/`B` (defined) global
sets.

## Symbols intentionally *not* exported

| C symbol | Reason |
|----------|--------|
| `helperBad`    | `static char *helperBad()` — internal linkage in C (`nm` shows it as `t`, not `T`), so it is not part of the ABI. Translated as the private `prog::helper_bad`. |
| `helperGood1`  | `static char *helperGood1()` — same, translated as private `prog::helper_good1`. |

Both helpers **are** translated (no code was skipped); they are simply private,
matching their C linkage.

## Undefined (imported) symbols

The C object imports `__isoc99_scanf` and `puts` from glibc, plus the usual
weak ELF/`__cxa_finalize` hooks. The Rust object imports a larger but purely
libc/`libgcc_s` set (`malloc`, `memcpy`, `write`, `read`, `_Unwind_*`,
`pthread_*`, …) because Rust's `std` is statically linked into the `cdylib`.

**Undefined non-libc / non-unwinder symbols in the Rust `.so`: 0** — i.e. the
Rust object has no dangling references to untranslated code. Verified by
`tests/symbol_parity.rs::rust_so_has_no_unresolved_non_libc_symbols`, which
`dlopen()`s the object with `RTLD_NOW` (an eager-binding load fails outright on
any unresolvable symbol).

## Note on the executable link

`c_src/build/driver` (the CMake executable) exports the same four application
symbols plus the standard CRT entry points `_start`, `_init`, `_fini`,
`_dl_relocate_static_pie`, which are supplied by the C runtime rather than by
`main.c`. The Rust `driver` binary gets its equivalents from the Rust start-up
shim. These are not part of the translated surface.
