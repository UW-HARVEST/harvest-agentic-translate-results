# SYMBOLS.md — Phase A symbol surface

## How the two shared objects are produced

The C project (`c_src/CMakeLists.txt`) builds an **executable** (`add_executable(driver src/main.c)`),
so there is no `.so` target in the C build system. For differential testing the very same
translation unit is additionally compiled as a shared object (no files under `c_src/` are
modified — the output goes elsewhere):

```sh
# executable, exactly as documented
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> c_src/build/driver

# shared library for the FFI differential tests
gcc -shared -fPIC -o c_src/build/libcdriver.so c_src/src/main.c          # -> libcdriver.so
```

The Rust side gained a `[lib] crate-type = ["cdylib"]` target (`src/lib.rs`) whose only content
is the `#[no_mangle] extern "C"` export wrappers. The logic lives in `src/core_impl.rs`, which is
included by **both** `src/main.rs` (the `driver` executable) and `src/lib.rs` (the cdylib) through
`#[path = "core_impl.rs"] mod core_impl;`, so the executable and the exported symbols can never
drift apart.

```sh
cargo build          # -> target/debug/driver  and  target/debug/libdriver.so
```

## Symbol parity table (`nm -D --defined-only`)

| # | symbol | C signature (from `c_src/src/main.c`) | C `.so` | Rust `.so` | Rust definition |
|---|--------|--------------------------------------|---------|------------|-----------------|
| 1 | `foo`    | `int foo(const char *in, char c)` | `T` | `T` | `src/lib.rs::foo` -> `core_impl::foo_impl` |
| 2 | `driver` | `void driver(const char *in)`     | `T` | `T` | `src/lib.rs::driver` -> `core_impl::driver_impl` |
| 3 | `main`   | `int main()`                      | `T` | `T` | `src/lib.rs::main` -> `core_impl::main_impl` |

Raw output:

```
$ nm -D --defined-only c_src/build/libcdriver.so
00000000000011e0 T driver
00000000000011b0 T foo
0000000000001090 T main

$ nm -D --defined-only target/debug/libdriver.so
00000000000160f0 T driver
0000000000016110 T foo
0000000000016140 T main
```

**Symbol diff (C exported, missing from Rust): EMPTY.** 3 of 3 C symbols are exported by the
Rust cdylib under the exact same names. No stubs are involved — every export forwards to a full
translation of the corresponding C function.

Weak/compiler-generated entries that `nm -D` also lists for both objects
(`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__`,
`__cxa_finalize`) are toolchain artifacts, not API, and are present in both.

## Undefined symbols in the Rust `.so`

`nm -D -u target/debug/libdriver.so` lists only libc / libgcc-unwind imports
(`printf`-free: the Rust build imports `write`, `read`, `malloc`, `free`, `memcpy`,
`__errno_location`, `signal`, `_Unwind_*`, …). There are **0 undefined non-libc symbols**,
i.e. nothing in the Rust object refers to an untranslated C helper.

The C `.so` imports `fread`, `printf`, `stdin`, `strchr` — all four are reimplemented in Rust
(`core_impl::c_strchr` replicates `strchr`; stdin reading via `std::io::Stdin::read`;
formatting via `write!`), except `signal`, which is deliberately called through libc to
restore the C-default SIGPIPE disposition (see `ERRORS.md` row 12).

## Feature / configuration matrix

`translated_rust/Cargo.toml` declares **no `[features]` section**, and `c_src` contains **no**
`#ifdef` / `#if` / CMake option that changes the compiled code
(`grep -rn '#ifdef\|#ifndef\|#if ' c_src/src/` → no matches; `CMakeLists.txt` has no `option()`).

Therefore the complete set of valid build-time feature combinations is exactly one: the
default/empty set. Both of the following were run and finish clean:

```sh
cargo check --offline --all-targets                        # default features
cargo check --offline --no-default-features --all-targets  # the only other spelling of the same combo
```

Consequently "repeat Phases B–C for every feature combination" collapses to the single
combination, which is what the test suite in `tests/` exercises (it is additionally run with
`--no-default-features` by `run_all_checks.sh`).

## Verification results (final)

```
$ diff <(nm -D --defined-only c_src/build/libcdriver.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort)
                                    # (no output) -> IDENTICAL SYMBOL SETS

$ ldd -r c_src/build/libcdriver.so   | grep -i 'undefined\|not found'   # nothing
$ ldd -r target/release/libdriver.so | grep -i 'undefined\|not found'   # nothing
```

* symbol parity holds for `target/debug/libdriver.so` **and** `target/release/libdriver.so`
  (checked by `run_all_checks.sh`, step 4);
* the weak toolchain entries `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`,
  `__gmon_start__` and `__cxa_finalize` appear in both objects;
* `ldd -r` reports no unresolved symbol in either object;
* `tests/differential.rs` — 28 tests, all green in the `dev` and `release` profiles and with
  `--no-default-features` (9 consecutive full runs, no flakes).
