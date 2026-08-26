# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Mechanically derived from `nm -D` on both shared objects.

## How the artifacts were produced

```sh
# C (default configuration, exactly as CMakeLists.txt specifies)
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libdriver.so

# Rust
cargo build --no-default-features
#   -> target/debug/libdriver.so

nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## Build-time configuration surface

`Cargo.toml` has **no `[features]` section**, so there is exactly **one** valid
feature combination: the empty/default set. `cargo check`/`cargo test` with
`--no-default-features` is therefore the complete Phase-D feature matrix.

`c_src/CMakeLists.txt` defines a single target (`driver`, SHARED, from
`src/driver.c`) with no `option()`, no `target_compile_definitions`, and no
conditional sources. `driver.c`/`driver.h` contain **no `#ifdef`** other than
the `DRIVER_H_` include guard. So the C side likewise has one configuration.

`CMAKE_BUILD_TYPE` is empty in the generated cache, which means the default C
build is **`-O0`** (no optimization flags). This matters for `bad()` — see
`ERRORS.md` §UB.

## Exported (defined) dynamic symbols

The C translation unit has four non-`static` functions; all four are exported.
Only `driver` is declared in the public header, but the `.so` exports all four,
so the Rust translation reproduces all four with identical linkage names.

| # | C symbol | C signature | in C `.so` | in Rust `.so` | status |
|---|----------|-------------|------------|---------------|--------|
| 1 | `printLine` | `void printLine(const char *line)` | `T` | `T` | ✅ match |
| 2 | `bad`       | `void bad(void)`                    | `T` | `T` | ✅ match |
| 3 | `good`      | `void good(void)`                   | `T` | `T` | ✅ match |
| 4 | `driver`    | `void driver(int useGood)`          | `T` | `T` | ✅ match |

**Missing from Rust `.so`: none.** No `#[no_mangle]` wrapper had to be added and
no C source was left untranslated — `c_src/src/driver.c` is the only translation
unit in the project and all four of its external functions are implemented in
`src/lib.rs`. There are no macro-generated symbols in this library.

## Undefined / imported symbols

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|------------|------|
| `puts@GLIBC_2.2.5`   | `U` | — | gcc rewrites `printf("%s\n", s)` into `puts(s)` even at `-O0` |
| `printf@GLIBC_2.2.5` | — | `U` | Rust calls `printf("%s\n", s)` directly |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | `w` | (weak/toolchain) | standard glibc/CRT weak refs, not part of the API |

`puts(s)` and `printf("%s\n", s)` emit byte-identical output (the argument is
never interpreted as a format string in either case, so `%` inside `line` is
safe in both), write to the same process-wide `stdout` `FILE*`, and share the
same buffering. The import-name difference is therefore not an observable
behavioural difference. This is confirmed by the differential tests, which
include payloads full of `printf` conversion specifiers (`CONFIGS.md` row 4).

Rust additionally imports the usual `std`/`libc` runtime symbols (`memcpy`,
`__rust_*`, pthread/dl symbols, unwinder, etc.). These are implementation
detail of the Rust standard library, not part of the translated API surface,
and there are **0 missing/undefined non-libc symbols** — verified by
`tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`, which resolves
every `U` symbol in the Rust `.so` against the process's loaded libraries.

## Verdict

`nm -D` symbol diff (C-exported minus Rust-exported) is **empty**. Enforced
automatically by `tests/symbols.rs::c_and_rust_export_identical_symbol_sets`.
