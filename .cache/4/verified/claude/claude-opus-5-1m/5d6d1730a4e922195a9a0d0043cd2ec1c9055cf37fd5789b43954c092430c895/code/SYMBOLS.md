# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

`c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver src/main.c)`),
so the primary artefact pair is `c_src/build/driver` vs. `target/<profile>/driver`.
The same translation unit also compiles cleanly as a shared object, which is how
the differential tests drive it through the C ABI:

```
gcc -shared -fPIC -O2 -o target/cdiff/libcdriver.so c_src/src/main.c
cargo build                       # -> target/debug/libdriver.so   ([lib] crate-type = ["cdylib", "rlib"])
```

## C source inventory

| C file | functions defined | translated in |
|--------|-------------------|---------------|
| `c_src/src/main.c` | `driver`, `main` | `src/imp.rs` (logic) + `src/lib.rs` (C ABI wrappers) + `src/main.rs` (executable entry) |

`c_src/src/main.c` is the only C source file in the project (`find c_src -name '*.c'`
→ 1 file, 41 lines, 0 headers). Nothing was skipped: both functions it defines are
translated, and there is no additional C module to translate.

## `nm -D --defined-only` — C shared object

| symbol | type | signature |
|--------|------|-----------|
| `driver` | `T` (text, global) | `void driver(double f)` |
| `main`   | `T` (text, global) | `int main(void)` |

The `raw_double_t` union is a file-local type and the `printf`/`scanf` references
are *undefined* imports satisfied by libc, so they are not part of the exported
surface.

## `nm -D --defined-only` — Rust shared object

| symbol | type | provided by |
|--------|------|-------------|
| `driver` | `T` | `#[no_mangle] pub extern "C" fn driver(f: f64)` in `src/lib.rs` |
| `main`   | `T` | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

Rust exports **no** extra symbols: `nm -D --defined-only target/debug/libdriver.so`
prints exactly two lines (std is statically linked and internal, and the crate
declares no other `#[no_mangle]` item).

## Diff

```
$ diff <(nm -D --defined-only target/cdiff/libcdriver.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort)
$ echo $?
0
```

**Result: the symbol diff is empty — 0 symbols missing from the Rust `.so`.**
This is asserted automatically by `tests/symbols.rs::c_and_rust_export_the_same_symbols`,
which also asserts that the undefined (imported) symbols of the Rust `.so` are all
libc/loader symbols, i.e. there is no unresolved non-libc dependency.

### Undefined (imported) symbols

| `.so` | undefined symbols |
|-------|-------------------|
| C     | `printf@GLIBC_2.2.5`, `__isoc99_scanf@GLIBC_2.7`, plus the weak `__cxa_finalize`, `_ITM_*`, `__gmon_start__` |
| Rust  | libc only (`read`, `write`, `memcpy`, `malloc`, `pthread_key_*`, `dl_iterate_phdr`, `__errno_location`, …) plus the libgcc unwinder (`_Unwind_*@GCC_*`) and the same weak loader symbols |

No **non-libc / non-runtime** symbol is left undefined in the Rust `.so`, i.e. no
part of the translation was left as an external reference to the C code.

## Notes on the `main` symbol

Exporting `main` from a `cdylib` collides with the `main` symbol that `rustc`
synthesises for a binary target, so `src/main.rs` pulls the implementation in with
`#[path = "imp.rs"] mod imp;` instead of linking the `driver` library crate. Both
artefacts are therefore compiled from the same `src/imp.rs` source, and the
executable and the shared object are both covered by the differential tests
(`tests/cli_diff.rs` drives the executables, `tests/ffi_*.rs` drives the `.so`s).

## Verification summary

```
$ nm -D --defined-only target/debug/libcdriver.so | awk '{print $NF}' | sort
driver
main
$ nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort
driver
main
$ diff logs/symbols-c.txt logs/symbols-rust.txt && echo "0 missing"
0 missing
```

| check | result |
|-------|--------|
| symbols exported by the C `.so` but missing from the Rust `.so` | **0** |
| symbols exported by the Rust `.so` but not by the C `.so` | **0** |
| non-libc / non-runtime undefined symbols in the Rust `.so` | **0** |
| C source files not translated | **0** (1 of 1 translated) |
| C functions not translated | **0** (2 of 2: `driver`, `main`) |

Reproduce with `./run_diff_tests.sh` (writes `logs/symbols-c.txt`,
`logs/symbols-rust.txt`) or with `cargo test --test symbols`, which asserts all of
the above programmatically.
