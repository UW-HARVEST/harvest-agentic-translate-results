# SYMBOLS.md — symbol surface parity (Phase A / Phase D)

## Shape of the project

`c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver src/main.c)`)
from a single translation unit, `c_src/src/main.c` (51 lines).  There are no
headers, no `#ifdef`s and no CMake options, so there is exactly one C build
configuration.

Because the task requires comparing the two implementations *through the FFI
boundary*, the same translation unit is additionally compiled as a shared
object (the C sources are **not** modified; only the compile command differs,
and the output goes to `target/cdiff/`):

```sh
gcc -shared -fPIC -o target/cdiff/libc_driver.so c_src/src/main.c
```

On the Rust side the translation lives in `src/lib.rs`, which exports
`print_foo` and `driver` with `#[no_mangle] extern "C"`.  The matching shared
object is the `[[example]] name = "cdylib"` target (`examples/cdylib.rs`,
`crate-type = ["cdylib"]`), built by `cargo test` / `cargo build --example
cdylib` to `target/<profile>/examples/libcdylib.so`; it links the library (so
`driver` and `print_foo` come straight from it) and adds the `main` symbol,
which forwards to the same `driver::c_main()` that the `driver` executable
(`src/main.rs`) calls.  The executable and the shared object therefore run
bit-identical code paths.

(`main` cannot be exported by the library itself: a `#[no_mangle] fn main` in
the `rlib` would collide with the entry point rustc generates for the
executable target.)

## `nm -D --defined-only` on the C shared object

```
0000000000001195 T driver
000000000000120b T main
0000000000001139 T print_foo
```

(The cmake-built *executable* `c_src/build/driver` is linked non-PIE and has an
empty dynamic symbol table — `nm -D` prints nothing for it — so the shared
object above is the reference surface.)

## Parity table

| # | C symbol | type | C `.so` | Rust `.so` (`target/<profile>/examples/libcdylib.so`) | defined in | status |
|---|----------|------|---------|------------------------------------------------------|-----------|--------|
| 1 | `driver`    | `T` (global text) | yes | yes (`#[no_mangle] pub extern "C" fn driver`)           | `src/lib.rs` | OK |
| 2 | `print_foo` | `T` (global text) | yes | yes (`#[no_mangle] pub unsafe extern "C" fn print_foo`) | `src/lib.rs` | OK |
| 3 | `main`      | `T` (global text) | yes | yes (`#[no_mangle] pub extern "C" fn main`)             | `examples/cdylib.rs` → `driver::c_main()` | OK |

Verified mechanically (both profiles):

```
$ nm -D --defined-only target/cdiff/libc_driver.so       | grep -E ' T '
0000000000001195 T driver
000000000000120b T main
0000000000001139 T print_foo
$ nm -D --defined-only target/release/examples/libcdylib.so | grep -E ' T (driver|main|print_foo)$'
0000000000013f20 T driver
00000000000131d0 T main
0000000000014070 T print_foo
```

and asserted by `tests/phase_d_symbols.rs::sym_c_exports_are_all_present_in_rust`
(set difference must be empty) plus `tests/ffi_inproc.rs::sym_exports_present`
(`dlsym` on both objects).

Missing symbols: **0**.  Nothing was stubbed: every export is the real
translation of the corresponding C function (`src/lib.rs`).

The Rust `.so` additionally exports the usual Rust/`std` runtime symbols
(`_ZN…`, `rust_eh_personality`, `__rust_*`, …).  Extra exports are not a
correctness problem; the requirement is that no C symbol is missing.

### Undefined (imported) symbols

Both objects import only libc / C-runtime symbols.  The C object imports
exactly `printf@GLIBC_2.2.5`, `__isoc99_scanf@GLIBC_2.7` plus the four weak
crt symbols (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).  The Rust object
imports 50 symbols, all of them from glibc (`memcpy`, `read`, `write`,
`malloc`, `pthread_key_*`, `open64`, …), from `libgcc_s`' unwinder
(`_Unwind_*`) or the same weak crt symbols.  There are **0 undefined non-libc symbols** in the Rust object:

```sh
nm -D --undefined-only target/release/examples/libcdylib.so  # libc / libgcc only
ldd target/release/examples/libcdylib.so                     # no missing dependency
```

`tests/phase_d_symbols.rs::sym_rust_so_has_no_unresolved_symbols` additionally
`dlopen`s the Rust object with `RTLD_NOW`, which resolves *every* relocation
eagerly and therefore fails if anything is unresolved.

## C ABI notes that the exported wrappers must honour

* `foo_t` layout (verified against gcc/x86-64 with `offsetof`/`_Alignof` and a
  byte dump): `sizeof == 8`, `_Alignof == 4`, `offsetof(z) == 4`; the three
  bit-fields all live in **byte 0**: `x` = bits 0..1, `y` = bits 2..4,
  `b` = bit 5.  Bits 6..7 and bytes 1..3 are padding that the C code never
  reads (`print_foo` masks) and only partially writes.
* `void driver(unsigned int, unsigned int, bool, int)` — the third argument is
  a C `_Bool`, passed in the low byte of `%edx`.  gcc stores it into the
  one-bit bit-field with `and $0x1`, i.e. **only bit 0 of the byte matters**
  (`driver(…, 2, …)` prints `0`).  The Rust export therefore takes `u8` (same
  ABI as `_Bool`) instead of `bool`, so that every byte a C caller can pass is
  reproduced exactly instead of being Rust UB.
* `print_foo` dereferences its argument unconditionally — no null check.
