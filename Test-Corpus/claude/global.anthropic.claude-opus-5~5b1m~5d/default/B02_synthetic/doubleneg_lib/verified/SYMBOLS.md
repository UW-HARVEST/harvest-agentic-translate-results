# SYMBOLS.md — Phase A symbol surface

C library: `c_src/build/libharvest-work-qXV8vG.so` (built from the single TU `c_src/src/lib.c`).
Rust library: `translation/target/release/libdoubleneg_lib.so` (`crate-type = ["cdylib"]`).

## `nm -D --defined-only` on the C `.so`

| # | symbol | type | C signature (`c_src/src/lib.c`) | exported by Rust `.so`? | Rust definition |
|---|--------|------|---------------------------------|--------------------------|-----------------|
| 1 | `convert_double_to_int`  | T | `int convert_double_to_int(double)`                        | YES | `src/cvt.rs`      |
| 2 | `find_value_in_buffer`   | T | `int find_value_in_buffer(const char*, size_t, int)`       | YES | `src/buffer.rs`   |
| 3 | `process_negation`       | T | `int process_negation(int)`                                | YES | `src/negation.rs` |
| 4 | `create_numeric_buffer`  | T | `void create_numeric_buffer(char*, int, int)`              | YES | `src/buffer.rs`   |
| 5 | `calculate_with_doubles` | T | `double calculate_with_doubles(int, int, int)`             | YES | `src/dmath.rs`    |
| 6 | `doubleneg`              | T | `int doubleneg(int, int, int, int)`                        | YES | `src/doubleneg.rs`|

There are no macro-generated / namespaced / versioned symbols: `c_src/include/lib.h`
contains exactly one declaration (`doubleneg`) and no renaming macros, and `lib.c`
declares no `static` helpers that would be hidden. All six `extern` functions in the
TU are visible.

## Symbol diff

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: NONE.** No stubs were required; every symbol has a real
translated body.

## Undefined (imported) symbols

The C `.so` imports `memchr`, `pow`, `printf`, `puts` (GCC rewrites the
newline-terminated `printf("...\n")` calls into `puts`) plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` stubs.

The Rust `.so` imports `pow`, `printf`, `puts`, `memcpy`, `memset`, `strlen`, … —
all of them libc / libgcc-unwind symbols resolved from `libc.so.6`, `libm.so.6`
and `libgcc_s.so.1` (confirmed by `ldd`). **0 missing/undefined non-libc symbols.**

`memchr` is absent from the Rust imports because `src/buffer.rs` implements the
scan in Rust; this is behaviourally identical (byte-wise first-match search) and
is exercised by the Phase B/C differential tests.
