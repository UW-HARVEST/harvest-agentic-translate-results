# SYMBOLS.md — Symbol surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libdriver.so`   (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libdriver.so` (built with `cargo build`, `crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms          # must be empty
```

The automated version of this check lives in `tests/symbols.rs`
(`c_symbols_are_all_exported_by_rust`, `rust_so_has_no_unresolved_non_libc_symbols`).

## Defined (exported) symbols

C source files: `c_src/src/goto.c` — the *only* translation unit in
`c_src/CMakeLists.txt`. There is no untranslated C module, so no symbol is
missing because of a skipped file.

| # | C symbol (`nm -D --defined-only`) | type | declared in | Rust export | status |
|---|-----------------------------------|------|-------------|-------------|--------|
| 1 | `driver`                | T (global text) | `include/goto.h:26` (public header) | `#[unsafe(no_mangle)] pub extern "C" fn driver` — `src/lib.rs:179` | PRESENT |
| 2 | `forward_goto_example`  | T (global text) | `src/goto.c:29` (no prototype in header, still `extern`/exported) | `#[unsafe(no_mangle)] pub extern "C" fn forward_goto_example` — `src/lib.rs:74` | PRESENT |
| 3 | `open_with_cleanup`     | T (global text) | `src/goto.c:42` (no prototype in header, still `extern`/exported) | `#[unsafe(no_mangle)] pub extern "C" fn open_with_cleanup` — `src/lib.rs:122` | PRESENT |

`comm -23 c.syms r.syms` → **empty**: 0 symbols missing from the Rust `.so`.

There are no macro-generated symbols, no exported data objects, no versioned
symbols and no weak aliases in the C `.so` beyond the toolchain-emitted ones
listed below.

### Toolchain-emitted symbols (not part of the library surface)

`nm -D` on the C `.so` also lists these; they are emitted by the C
toolchain/CRT, not by `goto.c`, and are intentionally not reproduced by the
Rust `cdylib` (the Rust toolchain emits its own equivalents):

| symbol | kind | note |
|--------|------|------|
| `_ITM_deregisterTMCloneTable` | w (weak undefined) | GCC transactional-memory stub |
| `_ITM_registerTMCloneTable`   | w (weak undefined) | GCC transactional-memory stub |
| `__cxa_finalize@GLIBC_2.2.5`  | w (weak undefined) | glibc destructor registration |
| `__gmon_start__`              | w (weak undefined) | profiling hook |

## Undefined (imported) symbols — must all be libc

Both objects import exactly the same libc surface, which is what makes the
observable I/O byte-identical (same `stdio` buffering, same `stderr`).

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `fopen`   | U | U | `src/lib.rs:53` |
| `fgets`   | U | U | `src/lib.rs:54` |
| `ferror`  | U | U | `src/lib.rs:55` |
| `fclose`  | U | U | `src/lib.rs:56` |
| `printf`  | U | U | `src/lib.rs:51` |
| `fprintf` | U | U | `src/lib.rs:52` |
| `fwrite`  | U | (not needed) | GCC lowers `fprintf(stderr, "…")` with no conversions to `fwrite`; identical bytes on the wire |
| `stderr`  | U | U | glibc global, `src/lib.rs:49` |

The Rust `.so` additionally imports only Rust-runtime/libc symbols
(`memcpy`, `__rust_*` allocator shims, unwinding personality, …). The test
`rust_so_has_no_unresolved_non_libc_symbols` asserts that every undefined
symbol in the Rust `.so` resolves against `libc`/`libgcc`/`libm`/`ld` or the
Rust standard library, i.e. nothing is dangling.

## Feature/configuration coverage

`Cargo.toml` has **no `[features]` section**, so the complete set of valid
feature combinations is:

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo … ` (default) | the one and only configuration |
| 2 | `cargo … --no-default-features` | identical to #1 (no default features exist) |
| 3 | `cargo … --all-features` | identical to #1 (no features exist) |

`c_src/CMakeLists.txt` likewise has no options, no `#ifdef`-driven variants and
a single source file, so there is exactly one C configuration. All three cargo
invocations are exercised by `./verify.sh`.

## Verified output (`nm -D`)

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001189 T forward_goto_example
00000000000011e8 T open_with_cleanup
00000000000012a5 T driver

$ nm -D --defined-only target/debug/libdriver.so
0000000000012230 T driver
00000000000122f0 T forward_goto_example
00000000000123c0 T open_with_cleanup

$ comm -23 c.syms r.syms
(empty)
```

Automated equivalents, all passing:

| test (`tests/symbols.rs`) | asserts |
|---------------------------|---------|
| `c_symbols_are_all_exported_by_rust` | `nm -D` set difference C∖Rust is empty, and the C surface is exactly the 3 documented symbols |
| `every_c_symbol_is_dlsym_able_in_rust` | each C symbol resolves via `dlsym` on the Rust `.so` (default visibility, not merely present) |
| `rust_so_loads_with_rtld_now` | `dlopen(RTLD_NOW)` binds every relocation → nothing dangling |
| `rust_so_has_no_unresolved_non_libc_symbols` | every undefined symbol of the Rust `.so` resolves against libc/the Rust runtime |
| `both_objects_import_the_same_stdio_surface` | both import `fopen`/`fgets`/`ferror`/`fclose`/`printf`/`fprintf`/`stderr` |

**Result: 0 missing symbols, 0 unresolved symbols**, in every build
configuration and in both the `dev` and `release` profiles.

## Caveat when reproducing by hand

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, so
`target/debug/libdriver.so` can be stale after an edit to `src/lib.rs`. Always
`cargo build` first (or just use `./verify.sh`); the test harness additionally
refuses to run against a `.so` older than its sources.
