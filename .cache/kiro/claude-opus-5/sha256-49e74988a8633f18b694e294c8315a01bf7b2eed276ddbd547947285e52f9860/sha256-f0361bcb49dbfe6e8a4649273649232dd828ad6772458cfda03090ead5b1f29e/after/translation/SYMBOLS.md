# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` — `c_src/build/libdriver.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001173 T driver
```

## Rust `.so` — `translation/target/release/libdriver.so`

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011730 T driver
```

## Parity table

| # | C symbol | type | present in Rust `.so` | notes |
|---|----------|------|-----------------------|-------|
| 1 | `driver` | `T` (global text) | YES — exact name | `void driver(int)`, declared in `include/driver.h`. Exported from Rust via `#[unsafe(no_mangle)] pub extern "C" fn driver(floors: c_int)`. |

### Symbols intentionally NOT exported

| C symbol | reason |
|----------|--------|
| `print_hex` | declared `static void print_hex(unsigned char *p, int len)` in `src/driver.c` → internal linkage, absent from `nm -D` on the C `.so`. Kept private (`fn print_hex`) in Rust, so parity holds. |

### Undefined (imported) symbols

The Rust `.so` imports only toolchain/libc symbols. It deliberately imports
libc's `printf`/`putchar` rather than using `println!`, so its output shares the
C runtime's `stdout` buffer and lands in the same stream in the same order as the
C implementation:

```
$ nm -D --undefined-only translation/target/release/libdriver.so | awk '{print $2}' | sort -u
```

All 51 entries are glibc (`printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`,
`malloc`, `memcpy`, `write`, …), libgcc unwinder (`_Unwind_*@GCC_*`), or the
standard weak toolchain hooks (`__gmon_start__`, `_ITM_*TMCloneTable`,
`__cxa_finalize`).

Non-libc / non-toolchain undefined symbols: **0**.

## Result

Symbol diff (C exports − Rust exports) is **EMPTY**, verified under every
feature combination by `run_differential.sh`:

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
  symbol sets identical
```

No missing implementation and no untranslated C module: `c_src` contains exactly
one translation unit (`src/driver.c`) and one public header
(`include/driver.h`), both fully translated in `translation/src/lib.rs`. Nothing
was stubbed.

## Build-system hazard found during verification

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact — only
`cargo build` does. A differential test that `dlopen`s
`target/release/libdriver.so` therefore silently exercises a **stale** `.so`
after a source edit, and passes regardless of what the Rust source says. This
was observed: three injected bugs all "passed" until the `.so` was rebuilt.

Two mitigations are in place:

* `tests/differential.rs::assert_artifacts_fresh` aborts the run if either
  `.so` is older than any of its sources.
* `run_differential.sh` always builds the C library and `cargo build --release`
  before `cargo test --release`, for every feature combination.
