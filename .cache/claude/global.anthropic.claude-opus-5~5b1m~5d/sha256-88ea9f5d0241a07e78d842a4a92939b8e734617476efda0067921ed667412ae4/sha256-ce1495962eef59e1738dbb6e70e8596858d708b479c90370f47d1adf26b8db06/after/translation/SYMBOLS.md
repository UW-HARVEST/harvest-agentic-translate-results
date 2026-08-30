# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` (`c_src/build/libdriver.so`)

```
000000000000115b T driver
0000000000001139 T printLine
```

## Rust `.so` (`translation/target/release/libdriver.so`)

```
0000000000011750 T driver
0000000000011800 T printLine
```

## Parity table

| # | symbol    | type | in C `.so` | in Rust `.so` | source of truth        | notes |
|---|-----------|------|-----------|---------------|------------------------|-------|
| 1 | `driver`  | `T` (text, global) | yes | yes | `c_src/src/driver.c:38`, declared in `c_src/include/driver.h:27` | `void driver(int data)` |
| 2 | `printLine` | `T` (text, global) | yes | yes | `c_src/src/driver.c:30` | `void printLine(const char *line)`; not declared in the public header but has external linkage in C, therefore it is part of the exported ABI and IS exported by the Rust `.so` too |

## Missing-symbol analysis

`diff` of the two sorted symbol-name lists is **empty**:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
# -> no output
```

* 0 symbols missing from the Rust `.so`.
* 0 symbols are stubs / `unimplemented!()`: both functions are full translations of the C
  bodies (see `translation/src/lib.rs`).
* No C source file in `c_src/` was left untranslated — the library consists of the single
  translation unit `src/driver.c` (see `c_src/CMakeLists.txt`, which lists exactly one
  source file).

## Undefined (imported) symbols

The Rust `.so` imports only libc / language-runtime symbols; `ldd -r` reports **no**
unresolved symbol for either library (checked by
`tests/phase_d_symbols.rs::d3_rust_library_has_no_undefined_non_libc_symbols`):

```sh
ldd -r c_src/build/libdriver.so                    # no "undefined symbol" lines
ldd -r translation/target/release/libdriver.so     # no "undefined symbol" lines
```

Observed difference in *imports* (not in exports, and not observable in behaviour):
LLVM rewrites `printf("%s\n", s)` into `puts(s)`, so the Rust `.so` imports `puts`
where the unoptimised C object imports `printf`. The two emit the exact same bytes on
the exact same `FILE *stdout` stream; this is verified explicitly — including with
stdout switched to **unbuffered** mode, where the choice of writer would be most
visible — by `tests/phase_b_configs.rs::c22_buffering_pipe_vs_file` (row C23 of
`CONFIGS.md`). Beyond that, the Rust `.so` imports `memset`/`strncpy` plus the usual
glibc + Rust `std` entries (`malloc`, `write`, `_Unwind_*`, `__cxa_*`, …).

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configuration is the default one (`--no-default-features` and the default build produce
identical symbol sets). Verified by script in `run_all.sh`.
