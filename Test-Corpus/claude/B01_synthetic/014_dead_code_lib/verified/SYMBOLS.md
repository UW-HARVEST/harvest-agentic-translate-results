# SYMBOLS.md — Phase A: dynamic-symbol parity

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd translated_rust/c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> c_src/build/libdriver.so
cargo build                                                          # -> target/debug/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/driver.c` | `src/lib.rs` | fully translated |
| `c_src/include/driver.h` | (declaration only: `void driver(void)`) | n/a |

All 6 C functions (`printLine`, `helperBad`, `bad`, `helperGood`, `good`,
`driver`) are present in `src/lib.rs`. The two `static` ones are non-exported
Rust `fn`s (`helper_bad`, `helper_good`), matching their C linkage
(`nm` shows them as local `t`, not dynamic).
No C module was skipped, so no symbol required a new translation or a stub.

## Exported (defined) dynamic symbols

`nm -D --defined-only` — function symbols:

| # | symbol | C `.so` | Rust `.so` | note |
|---|--------|---------|-----------|------|
| 1 | `printLine` | T | T | `void printLine(const char *)` |
| 2 | `bad`       | T | T | `void bad(void)` |
| 3 | `good`      | T | T | `void good(void)` |
| 4 | `driver`    | T | T | `void driver(void)` |

Deliberately NOT exported by either object (C `static` / Rust private):

| symbol | C `.so` | Rust `.so` |
|--------|---------|-----------|
| `helperBad` / `helper_bad`   | local (`t`), not in `nm -D` | local, not in `nm -D` |
| `helperGood` / `helper_good` | local (`t`), not in `nm -D` | local, not in `nm -D` |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust `.so`: 0.** **Extra in Rust `.so`: 0.**

## Undefined symbols

All undefined symbols in the Rust `.so` are libc / libgcc-unwind imports
(`printf`, `memcpy`, `malloc`, `_Unwind_*`, `__errno_location`, …) supplied by
the Rust standard library and the dynamic loader — **0 missing/undefined
non-libc symbols**.

The C `.so` imports `puts@GLIBC_2.2.5` rather than `printf`: GCC strength-reduces
`printf("%s\n", line)` to `puts(line)`. That is a pure libc-call-shape
difference; the emitted bytes on `stdout` are identical (`line` followed by
`'\n'`), which Phase B verifies byte-for-byte, so the Rust side keeps `printf`.

Verified for the `debug` profile **and** the `release` profile
(`panic = "abort"`, optimized) — optimization does not drop or add any exported
symbol. `tests/symbol_parity.rs` re-derives this diff from `nm -D` on every run
(cases `d1`–`d4`), so it cannot silently rot.

**Freshness caveat:** `cargo test` does *not* rebuild a `cdylib` lib target, so
`nm` / `dlopen` on `target/<profile>/libdriver.so` can easily inspect a stale
artifact. The harness therefore rebuilds the cdylib and asserts it is newer than
`src/` before loading it (see `VERIFICATION.md`).

## SONAME note (affects the test harness, not parity)

The C object carries `SONAME libdriver.so` (CMake default); the Rust `cdylib`
carries none, and both files are named `libdriver.so`. The differential tests
therefore copy each object to a distinct path/filename before `dlopen`, and
assert the resolved symbol addresses differ, so the loader can never dedupe the
two libraries into one.
