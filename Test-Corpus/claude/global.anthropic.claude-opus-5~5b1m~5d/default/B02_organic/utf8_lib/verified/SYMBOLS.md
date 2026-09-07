# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only c_src/build/libdriver.so`
Compared against: `nm -D --defined-only translation/target/release/libdriver.so`

## Dynamic symbols DEFINED (exported) by the C `.so`

| # | symbol | type | C declaration | present in Rust `.so`? |
|---|--------|------|---------------|------------------------|
| 1 | `w_utf8_drop`   | `T` (global text) | `const char * w_utf8_drop(const char * string)` — non-`static`, defined in `src/lib.c:39`, not in `include/lib.h` (still externally linkable) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn w_utf8_drop` |
| 2 | `w_utf8_filter` | `T` (global text) | `char * w_utf8_filter(const char * string, bool replacement)` — declared in `include/lib.h:3`, defined in `src/lib.c:59` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn w_utf8_filter` |

There are no other non-`static` functions, no global variables, no macro-generated
exports, and no additional translation units (`CMakeLists.txt` lists exactly one
source file, `src/lib.c`). The `valid_1` .. `valid_4` and `REPLACEMENT_INC`
identifiers are preprocessor macros and therefore produce no symbols.

## Missing-symbol analysis

**MISSING FROM RUST: none.** The symbol diff is empty in both directions:

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $2,$3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $2,$3}' | sort)
(no output)
```

No module/file of the C source was skipped, so no additional translation work was
required, and no stub/`unimplemented!()` was introduced.

## Dynamic symbols UNDEFINED (imported)

The C `.so` imports: `__assert_fail`, `malloc`, `realloc`, `strdup`, `strlen`,
`memcpy` (plus the toolchain's `__cxa_finalize`, `__gmon_start__`,
`_ITM_*registerTMCloneTable` weak stubs).

The Rust `.so` imports the same six libc functions (it calls the real `malloc` /
`realloc` / `strdup` / `strlen` / `memcpy` / `__assert_fail` via `extern "C"`, so
the returned buffer is `free()`-able by the caller exactly as with the C library),
plus the Rust standard library's own libc/unwinder dependencies
(`_Unwind_*`, `abort`, `free`, `memset`, `mmap64`, `pthread_key_*`, `write`, ...).

**0 missing / 0 unresolvable non-libc undefined symbols in the Rust `.so`.**
Every undefined symbol in the Rust `.so` is provided by `libc.so.6`,
`libgcc_s.so.1` or is a weak toolchain stub, verified by the tests loading the
Rust `.so` with `libloading` (`dlopen` succeeds, which requires full relocation
of all non-lazy symbols).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one
(`--no-default-features` is equivalent to the default). Phase D's
"repeat for every feature combination" therefore collapses to the single
default configuration, which is confirmed programmatically by
`tests/check_features.sh`.
