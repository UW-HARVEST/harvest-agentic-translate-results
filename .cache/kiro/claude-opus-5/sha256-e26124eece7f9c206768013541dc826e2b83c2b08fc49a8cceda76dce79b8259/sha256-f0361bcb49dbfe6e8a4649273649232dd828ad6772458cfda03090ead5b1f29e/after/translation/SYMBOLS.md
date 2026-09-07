# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-odDx5S.so` (from `c_src/CMakeLists.txt`,
  single translation unit `src/lib.c`)
* Rust `.so`: `translation/target/release/libdoubleneg_lib.so`
  (`[lib] name = "doubleneg_lib"`, `crate-type = ["cdylib"]`)

`c_src/include/lib.h` declares only `doubleneg`, but no function in `src/lib.c`
is `static`, so all six symbols are part of the public ABI. There are no
namespacing/renaming macros in the header, so link names equal source names.

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | Rust implementation | status |
|---|--------|---------|------------|---------------------|--------|
| 1 | `calculate_with_doubles` | T | T | `src/doubles.rs` | present |
| 2 | `convert_double_to_int`  | T | T | `src/conv.rs`    | present |
| 3 | `create_numeric_buffer`  | T | T | `src/buffer.rs`  | present |
| 4 | `doubleneg`              | T | T | `src/doubleneg.rs` | present |
| 5 | `find_value_in_buffer`   | T | T | `src/buffer.rs`  | present |
| 6 | `process_negation`       | T | T | `src/negation.rs` | present |

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
(empty)
```

**0 symbols missing from the Rust `.so`.** No module of `src/lib.c` was skipped;
every non-`static` C function has a real translation (no stubs, no
`unimplemented!()`).

## Undefined (imported) non-libc symbols

Both libraries import only libc/libm entry points, and the Rust translation
deliberately binds the *same* ones (`src/ffi.rs`) so formatting and
floating-point results are bit-identical:

| symbol | C `.so` | Rust `.so` | provider |
|--------|---------|------------|----------|
| `printf` | U | U | libc |
| `memchr` | U | U | libc |
| `pow`    | U | U | libm (C links `m`; Rust resolves via libm loaded in-process) |

There are **0 undefined non-libc symbols** in the Rust `.so`.

## Reproducing this verification

```bash
# 1. build the C shared library (ground truth)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build the Rust cdylib — `cargo test` does NOT build a cdylib, so this
#    step is required before the differential tests can dlopen it
cd translation && cargo build --release

# 3. every phase, every feature combination, plus the nm -D symbol diff
./verify_all.sh
```

`verify_all.sh` enumerates the `[features]` powerset from `Cargo.toml` (currently
empty, so it runs `default` and `--no-default-features`), and for each one runs
`cargo check`, `cargo build --release`, the `nm -D` symbol diff, and `cargo test`.

The test harness (`tests/common/mod.rs`) refuses to run against a `.so` older
than any file in `src/`, so a stale artifact cannot be mistaken for a pass.

## Verification checklist

- [x] `nm -D` symbol sets are identical (6 = 6, diff empty).
- [x] Every C symbol has a real Rust implementation (not a stub).
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] Cargo.toml declares **no `[features]`**, so the default build is the only
      feature combination; `--no-default-features` is equivalent.
- [x] `CMakeLists.txt` builds **only** `add_library(... SHARED)` — there is no
      binary/driver executable, so the stdout-comparison gate is satisfied by
      the in-process stdout capture of `doubleneg` in Phase B.
