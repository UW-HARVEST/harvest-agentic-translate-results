# SYMBOLS.md — public symbol inventory and C↔Rust parity

## What the C build produces

`c_src/CMakeLists.txt` declares a single target:

```cmake
add_executable(driver src/main.c)
```

So the canonical C artifact is an **executable**, not a shared library. An
executable exports no dynamic symbols, which `nm -D` confirms — it lists only
*imports*:

```
$ nm -D c_src/build/driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
                 U __libc_start_main@GLIBC_2.34
                 U printf@GLIBC_2.2.5
                 U puts@GLIBC_2.2.5
```

`c_src/src/main.c` is nevertheless a single translation unit defining exactly two
**external** symbols, both with external linkage and neither `static`:

```
$ nm --defined-only c_src/build/driver | grep -wE 'driver|main'
0000000000401146 T driver
000000000040118a T main
```

To get a comparable FFI surface, `build.rs` compiles that *same, unmodified*
translation unit a second time with `-shared -fPIC` into `libc_driver.so`. That
shared object exports precisely those two symbols, and it is what the
differential tests `dlopen`.

## Symbol tables

Reference C shared object (`$OUT_DIR/libc_driver.so`, built from
`c_src/src/main.c` by `build.rs`):

| symbol | type | signature |
|--------|------|-----------|
| `driver` | `T` (defined, global) | `void driver(int x, int y)` |
| `main`   | `T` (defined, global) | `int main(void)` |

Imports (`U`/`w`) are all libc/toolchain (`printf`, `puts`, `__isoc99_scanf`,
`__cxa_finalize`, `_ITM_*`, `__gmon_start__`) and are not part of the API.

Rust shared object (`target/<profile>/examples/libdriver_ffi.so`, built from
`examples/driver_ffi.rs`, `crate-type = ["cdylib"]`):

| symbol | type | Rust item |
|--------|------|-----------|
| `driver` | `T` (defined, global) | `#[no_mangle] pub extern "C" fn driver(c_int, c_int)` |
| `main`   | `T` (defined, global) | `#[no_mangle] pub extern "C" fn main() -> c_int` |

Both delegate to the `driver` library crate (`src/lib.rs`), i.e. to exactly the
same code that backs the `driver` executable. Nothing is stubbed: `driver`
forwards to `driver::driver_impl` and `main` forwards to `driver::c_main`.

## Parity check

`tests/symbol_parity.rs::c_exports_are_all_present_in_rust` performs the diff at
test time, so it cannot silently rot:

```
comm -23 <(nm -D --defined-only libc_driver.so) <(nm -D --defined-only libdriver_ffi.so)
```

Result: **empty**. Every symbol defined by the C `.so` is defined by the Rust
`.so` under the exact same name.

- [x] 0 symbols missing from the Rust `.so`.
- [x] 0 undefined non-libc symbols in the Rust `.so` (its only undefined symbols
      are libc/`ld.so` entries, checked by
      `rust_so_has_no_undefined_non_libc_symbols`).
- [x] No stubs, no `unimplemented!()`, no fabricated exports. The whole C
      translation unit (all 2 functions, 14 statements) is translated.

## Process-level surface

`main` is also the process entry point, and that is how a real consumer invokes
this program. The executable pair (`c_driver` vs `target/<profile>/driver`) is
therefore differentially tested end to end — stdin bytes in, stdout bytes and
exit status out — by `tests/differential_process.rs`. That is the boundary where
`SIGPIPE` disposition, exit status, and lazy-versus-eager stdin consumption are
observable; the in-process FFI tests cannot see those.
