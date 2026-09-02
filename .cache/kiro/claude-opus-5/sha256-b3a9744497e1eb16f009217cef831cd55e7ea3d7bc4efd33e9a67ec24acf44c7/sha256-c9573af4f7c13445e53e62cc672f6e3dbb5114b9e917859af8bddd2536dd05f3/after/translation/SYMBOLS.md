# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C   : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## C `.so` exported (defined) symbols

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|--------------------------|
| 1 | `parse_number` | `T` (text/global func) | YES |

The C translation unit is a single file (`c_src/src/lib.c`) with exactly one
non-`static` definition. `can_access_at_index` and `buffer_at_offset` are
preprocessor macros, so they generate no symbols. `include/lib.h` declares no
other functions and defines no macro that expands to a definition, so there are
no macro-generated symbols to mirror.

## Rust `.so` exported (defined) symbols relevant to the C surface

| # | symbol | type | present in C? |
|---|--------|------|---------------|
| 1 | `parse_number` | `T` | YES |

Verified: the Rust `.so` dynamic symbol table contains **exactly one** defined
global, `parse_number` — no extra exports, no missing exports. The Rust runtime
helpers are all hidden/local in a `cdylib`.

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011980 T parse_number
```

## Diff

```
symbols in C .so but NOT in Rust .so :  (none)
```

Command used to verify (see `verify_symbols.sh`):

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort -u)
```

## Undefined (imported) symbols in the Rust `.so`

All 50 undefined symbols resolve to glibc or `libgcc_s` (`strtod@GLIBC_2.2.5`,
`malloc`, `free`, `memcpy`, `realloc`, `posix_memalign`, `__errno_location`,
`_Unwind_*@GCC_*`, `_ITM_*`, `__gmon_start__`, …). There are no unresolved
non-libc symbols, and `ldd -r` reports none. Verified with:

```sh
nm -D --undefined-only translation/target/release/libdriver.so
ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one. `--no-default-features` is equivalent to
the default build. Phase D's "every feature combination" requirement is
satisfied by the single (default) configuration; this is asserted mechanically by
`verify_symbols.sh`, which fails if a `[features]` section ever appears.
