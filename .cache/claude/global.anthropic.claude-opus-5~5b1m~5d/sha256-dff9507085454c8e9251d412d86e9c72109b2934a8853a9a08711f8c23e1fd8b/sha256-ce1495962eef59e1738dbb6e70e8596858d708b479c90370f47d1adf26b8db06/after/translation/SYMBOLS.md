# SYMBOLS.md — public symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` — `c_src/build/libdriver.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001149 T driver
```

| # | symbol | type | exported by Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub extern "C" fn driver(c: c_char)` in `src/lib.rs` |

## Rust `.so` — `translation/target/release/libdriver.so`

```
$ nm -D --defined-only translation/target/release/libdriver.so
00000000000129e0 T driver
```

The Rust `cdylib` additionally defines the usual Rust/`compiler_builtins`
housekeeping symbols only when they are needed; none of them are part of the C
surface and none of the C symbols are missing.

## Undefined (imported) symbols

The C object imports `printf`, `putchar`/`puts` (compiler-substituted), and
`setlocale` from libc. The Rust object imports the same `printf` and
`setlocale` from libc on purpose, so the two share one `stdout` `FILE` and
therefore the identical buffering behaviour.

```
$ nm -D --undefined-only c_src/build/libdriver.so   | grep -v '@GLIBC\|__cxa\|_ITM_\|__gmon'
$ nm -D --undefined-only translation/target/release/libdriver.so | grep -v '@GLIBC\|__cxa\|_ITM_\|__gmon'
```

Both reduce to the empty set of non-libc undefined symbols.

## Verdict

- C symbols: 1 (`driver`).
- Missing from Rust: **0**.
- Non-libc undefined in Rust: **0**.
- No whole C module was skipped: `c_src` contains exactly one translation unit
  (`src/driver.c`) with exactly one non-static function.

Symbol diff is EMPTY. ✅
