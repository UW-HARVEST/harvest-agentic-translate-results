# SYMBOLS.md — Phase A: exported-symbol parity

## How the two shared objects are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c)`, so the
project ships a program rather than a library.  For differential testing the
same single translation unit is additionally compiled as a shared object with
the *same* flags CMake uses (`CMakeFiles/driver.dir/flags.make` shows
`C_FLAGS = -fPIE`, i.e. **no** `-O` flag → `-O0`, no `-D` defines):

```
cd translated_rust/c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> ./driver (exe)
gcc -fPIC -shared -o libcdriver.so ../src/main.c                   # -> ./libcdriver.so
```

The Rust side declares both a `[[bin]]` (`target/debug/driver`) and a
`[lib] crate-type = ["cdylib"]` (`target/debug/libdriver.so`).  `cdylib` is the
*only* crate type, so the integration tests cannot link the crate directly —
they are forced to `dlopen` the `.so` via `libloading`, which is what exercises
the `#[no_mangle]` export wrappers.

## `nm -D --defined-only` comparison

C `.so` (`c_src/build/libcdriver.so`) — 3 defined dynamic symbols:

| # | symbol      | type | C declaration                                                                          | exported by Rust `.so`? |
|---|-------------|------|----------------------------------------------------------------------------------------|-------------------------|
| 1 | `fma_array` | `T`  | `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`   | yes (`src/lib.rs`)      |
| 2 | `driver`    | `T`  | `void driver(int *out, int len)`                                                        | yes (`src/lib.rs`)      |
| 3 | `main`      | `T`  | `int main(void)`                                                                        | yes (`src/lib.rs`)      |

Rust `.so` (`target/debug/libdriver.so`) — 3 defined dynamic symbols:
`driver`, `fma_array`, `main`.

**Symbol diff (C-defined minus Rust-defined): EMPTY.**  Nothing was stubbed:
each export forwards to a real translation of the corresponding C function in
`src/imp.rs` (`fma_array_raw`, `driver_raw`, `c_main`).

`main` is exported behind `#[cfg(not(test))]` because the `cfg(test)` build of
the library target gets libtest's own `main`, and two `main` symbols in one
object collide at link time.  The shipped (non-test) `cdylib` always has it, as
`nm -D` above shows.

## Undefined symbols

* C `.so` imports: `__isoc99_scanf`, `printf` (+ the usual weak
  `_ITM_*` / `__gmon_start__` / `__cxa_finalize` stubs).
* Rust `.so` imports: 50 symbols, **all** of which are `@GLIBC_*` / `@GCC_*`
  versioned libc/libgcc_s entries or the weak `_ITM_*` / `__gmon_start__`
  markers.  Filtering those out leaves an empty list:

  ```
  nm -D -u target/debug/libdriver.so | awk '{print $NF}' \
      | grep -v -E '@GLIBC|@GCC|_ITM_|__gmon_start__'      # -> no output
  ```

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.**

Verified mechanically by `tests/symbols.rs::c_so_symbols_are_all_exported_by_rust_so`
(runs `nm -D` on both objects and asserts the set difference is empty) and by
`tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`.
`tests/symbols.rs::both_so_expose_the_same_callable_entry_points` additionally
`dlsym`s all three names out of both objects, so the check is not just textual.

The release artefact exports the same three names:

```
$ nm -D --defined-only target/release/libdriver.so
0000000000013110 T driver
0000000000013120 T fma_array
00000000000132a0 T main
```

## Test-time artefact layout

| artefact | path |
|----------|------|
| C program | `c_src/build/driver` |
| C shared object | `c_src/build/libcdriver.so` |
| Rust program | `target/debug/driver` |
| Rust shared object | `target/debug/libdriver.so` |
| dlopen harness used by the tests | `target/debug/examples/so_runner` |

`cargo test` does **not** build a `cdylib`, so `./run_all.sh` runs
`cargo build --lib --bins --examples` before every `cargo test` invocation; the
test harness hard-fails with the exact command to run if an artefact is missing.
The suite can be re-pointed at other artefacts with `DIFF_C_SO`, `DIFF_RUST_SO`,
`DIFF_C_EXE` and `DIFF_RUST_EXE` (used by `run_all.sh` phase 5 to re-run
everything against the release Rust build and against a `-O2` build of the C).
