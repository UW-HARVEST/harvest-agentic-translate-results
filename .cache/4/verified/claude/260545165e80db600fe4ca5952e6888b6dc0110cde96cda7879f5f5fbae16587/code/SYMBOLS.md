# SYMBOLS.md — public symbol surface

## Build configurations (Phase A)

`Cargo.toml` has **no `[features]` section**, so there is exactly one feature
combination to verify:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo check/test --no-default-features` | == default; no `[features]`, no optional deps, no `#[cfg(feature)]` in `src/` |

`c_src/CMakeLists.txt` has **no `option()` / `if()` / `add_definitions`** either:
a single target (`add_executable(driver src/main.c src/lib.c)`), no compile-time
switches, and no `#ifdef` in the C sources other than the `STRCPY_FUN_H_` include
guard. So the C side has exactly one configuration too.

`probe/check_features.sh` derives the list from `Cargo.toml` mechanically and
runs `cargo check --no-default-features [--features …] --all-targets` for every
combination:

```
$ ./probe/check_features.sh
Cargo.toml declares no [features]; the only configuration is the default
=== cargo check --no-default-features ===
    Finished `dev` profile
all configurations check out
```

## C shared library

`c_src` builds an *executable*; the library half of it (`src/lib.c`) is also
built as a shared object for the differential tests (this does not modify
`c_src/`; `tests/common/mod.rs` does it automatically into `target/diff/`):

```
gcc -shared -fPIC -o target/diff/libcstrfun.so c_src/src/lib.c
```

`nm -D --defined-only target/diff/libcstrfun.so`:

```
0000000000001159 T process_strings
```

All five helpers in `lib.c` (`validate_token`, `parse_command`,
`compare_prefix`, `find_delimiter`, `match_pattern`) are `static`, so they are
not part of the dynamic symbol table and cannot be reached from outside except
through `process_strings`.

## Rust shared library

`Cargo.toml` declares `crate-type = ["lib", "cdylib"]`; the `#[no_mangle]`
wrapper lives in `src/ffi.rs` and forwards to the translated
`strcpy_fun::process_strings` with a `RawMem` view (real pointers).

`nm -D --defined-only target/release/libdriver.so`:

```
0000000000011ec0 T process_strings
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | signature |
|---|--------|---------|------------|-----------|
| 1 | `process_strings` | T | T | `int (char *, size_t, const char *, size_t, int, uint32_t)` |

**Missing symbols: none.** The `nm -D` diff between the two libraries is empty:

```
$ diff <(nm -D --defined-only target/diff/libcstrfun.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort)
$ echo $?
0
```

Undefined (imported) symbols of the Rust `.so` are libc / loader runtime symbols
only (`memcpy`, `__libc_start_main`, `pthread_*`, `dl*`, …); no non-libc symbol
is left undefined. The C `.so` imports `strcmp`, `strncmp`, `strlen`, `strncpy`,
`strncat` and `snprintf`, all of which the Rust translation reimplements in
`src/cstr.rs` (byte-at-a-time, so that an out-of-bounds read happens at exactly
the same byte as in the C version).

## Executable surface

`c_src/src/main.c` is a `main()` that reads a fixed token stream from stdin and
prints the `process_strings` result, so its whole observable surface is
stdin → (stdout, stderr, exit status). It is translated by `src/main.rs`
(+ `src/scanf.rs` for the `scanf` conversions and `src/mem.rs` /
`src/frame_junk.rs` for the `main` stack frame that the C code reads past the end
of its buffers into). It is verified with

* `tests/exe_diff.rs` — the two real binaries, on inputs whose result does not
  depend on uninitialised memory,
* `tests/exe_frame.rs` — the overread cases, with the C program's uninitialised
  frame bytes forced to the modelled snapshot by `probe/inject_frame.c`.
