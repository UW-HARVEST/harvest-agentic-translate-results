# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C library

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001109 T tool_basename
```

## Rust library

```
$ nm -D --defined-only translation/target/release/libdriver.so
00000000000116b0 T tool_basename
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `tool_basename` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn tool_basename(*mut c_char) -> *mut c_char` |

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only translation/target/.../libdriver.so | awk '{print $2, $3}' | sort)
(empty)
```

**Missing symbols: 0.** No C source file was left untranslated: the whole C
library is `c_src/src/lib.c` (22 lines, one function) plus the single-line
header `c_src/include/lib.h`.

## Undefined (imported) symbols

The Rust `.so` imports only libc/`std` runtime symbols (`memchr`, `strlen`,
unwind/personality, `pthread`/`dl` stubs). The C `.so` imports `strrchr`.
No non-libc undefined symbols in either.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one (`--no-default-features` is
equivalent). Verified by:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # no match
```
