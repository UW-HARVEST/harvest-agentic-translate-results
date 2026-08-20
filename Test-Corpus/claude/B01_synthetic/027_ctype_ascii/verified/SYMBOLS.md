# SYMBOLS.md — dynamic-symbol surface (Phase A / Phase D)

## Build commands

C shared object (the CMake project only declares `add_executable`, so the same
single translation unit is additionally compiled as a shared object without
touching `c_src/`):

```
gcc -shared -fPIC -O2 -o target/c/libdriver_c.so c_src/src/main.c
```

C executable (exactly as `c_src/CMakeLists.txt` declares it):

```
cd c_src && mkdir -p build && cd build && \
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> c_src/build/driver
```

Rust shared object + executable:

```
cargo build --offline        # -> target/debug/libctype_driver.so, target/debug/driver
```

## Defined (exported) symbols

`nm -D --defined-only target/c/libdriver_c.so`:

| # | C symbol | type | C declaration | exported by Rust `.so`? | Rust definition |
|---|----------|------|---------------|-------------------------|-----------------|
| 1 | `driver` | `T` (text, global) | `void driver(char c)` | YES — `nm -D` shows `T driver` | `#[no_mangle] pub extern "C" fn driver(c: c_char)` in `src/lib.rs` |
| 2 | `main`   | `T` (text, global) | `int main()`           | YES — `nm -D` shows `T main`   | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

The C translation unit (`c_src/src/main.c`, 51 lines) defines no other
functions, no global variables, no `static` data and no macros that generate
symbols, so the exported surface is exactly these two symbols. Nothing in the C
source was left untranslated: `driver`, `main`, and the `getchar`/ctype/`printf`
behaviour they rely on are all implemented in `src/ctype.rs` + `src/tables.rs`
(no stubs, no `unimplemented!()`).

`main` is `#[cfg(not(test))]` in `src/lib.rs` only so that the unit-test
harness's own generated `main` does not collide; the shipped `cdylib` always
exports it (verified by `nm -D` in `tests/symbols.rs`).

The Rust binary target (`src/main.rs`) intentionally includes `ctype`/`tables`
by module declaration instead of linking the library crate, because linking the
`rlib` would pull in its `#[no_mangle] main` and collide with the binary's own
`main` symbol.

## Undefined (imported) symbols of the C `.so`

These are libc imports, not part of the exported surface; the Rust `.so`
implements the same behaviour with Rust `std` + the transcribed glibc "C"
locale tables, so it does not need to import them:

| C undefined symbol | why the Rust `.so` does not need it |
|--------------------|--------------------------------------|
| `__ctype_b_loc@GLIBC_2.3` | `tables::CTYPE_CLASS` is the glibc "C" locale class table |
| `__ctype_tolower_loc@GLIBC_2.3` | `tables::CTYPE_TOLOWER` |
| `__ctype_toupper_loc@GLIBC_2.3` | `tables::CTYPE_TOUPPER` |
| `getc@GLIBC_2.2.5` (`getchar`) | `ctype::getchar` (buffered `std::io::stdin`) |
| `printf@GLIBC_2.2.5` | `ctype::write_line` / `write_char_line` + `std::io::stdout` |
| `setlocale@GLIBC_2.2.5` | no-op: the tables already encode the `"C"` locale that `setlocale(LC_ALL, "C")` selects |
| `stdin@GLIBC_2.2.5` | `std::io::stdin()` |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*` (weak) | toolchain/linker artefacts |

## Result

`tests/symbols.rs::c_defined_symbols_are_all_exported_by_rust` runs `nm -D` on
both shared objects and asserts the set difference is empty:

```
C defined:    driver, main
Rust defined: driver, main
missing:      (none)
```

`tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols` additionally
asserts that every undefined symbol of the Rust `.so` resolves at load time
(`ldd -r` reports no "undefined symbol"), i.e. 0 missing/undefined non-libc
symbols.
