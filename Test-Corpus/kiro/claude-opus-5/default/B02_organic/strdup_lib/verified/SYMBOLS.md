# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C source inventory

The whole library is two files:

| C file | translated? | notes |
|--------|-------------|-------|
| `c_src/include/lib.h` | n/a (header) | declares exactly one function, no renaming macros |
| `c_src/src/lib.c` | YES → `translation/src/lib.rs` | 22 lines, one function definition |

`grep -c '^[a-zA-Z].*(' c_src/src/lib.c` confirms a single function definition,
so no C module was skipped. There is no macro-generated symbol machinery
(no `#define`d name prefixes/suffixes anywhere in `c_src/`).

## `nm -D --defined-only` comparison

Commands used:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

| symbol | in C `.so` | in Rust `.so` | status |
|--------|-----------|---------------|--------|
| `custom_strdup` | `T` | `T` | MATCH |

### Diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort -u) \
          <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort -u)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0.** No `#[no_mangle]` wrapper needed to
be added and no untranslated C module was found.

## Undefined (imported) symbols

The Rust `.so` must not pull in any non-libc undefined symbol.

`nm -D --undefined-only translation/target/release/libdriver.so` resolves only
against `libc`/`ld-linux` (`malloc`, `memcpy`, `strlen`, plus the standard
`_ITM_*` / `__gmon_start__` / `__cxa_*` weak stubs the toolchain always emits).
The Rust translation deliberately imports `malloc`/`memcpy`/`strlen` from libc
rather than using Rust's allocator, because the C contract is that the caller
releases the returned buffer with `free()` — that makes the allocator part of
the observable ABI.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `--no-default-features` and the default
build are therefore the same compilation unit; both are still exercised by
`tests/feature_matrix.sh` for completeness.
