# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source surface

The whole C library is two files:

| file | lines | contents |
|------|-------|----------|
| `c_src/include/driver.h` | 28 | declares `void driver(const char *in);` only |
| `c_src/src/driver.c`     | 63 | defines `fma_array`, `call_fma`, `driver` |

There are no macros that generate symbol names, no `#ifdef`-gated
definitions, no additional translation units in `CMakeLists.txt`
(`add_library(driver SHARED src/driver.c)`), and no `static` functions.
Therefore the complete public symbol set is the three function definitions
in `driver.c`. `fma_array` and `call_fma` are *not* declared in the public
header but they have external linkage, so they are exported and are part of
the ABI surface an external caller can reach — the tests exercise them
directly via `dlsym`/`libloading`.

## Exported symbol table

| # | symbol | C `.so` | Rust `.so` | Rust definition site | status |
|---|--------|---------|------------|----------------------|--------|
| 1 | `fma_array` | T (defined) | T (defined) | `src/driver.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn fma_array` | MATCH |
| 2 | `call_fma`  | T (defined) | T (defined) | `src/driver.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn call_fma`  | MATCH |
| 3 | `driver`    | T (defined) | T (defined) | `src/driver.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver`    | MATCH |

Missing from Rust: **none**. No wrapper had to be added and no C module was
left untranslated — `driver.c` is the only C translation unit and all three
of its functions are present in `src/driver.rs`.

## Signature parity

| symbol | C declaration | Rust `extern "C"` signature |
|--------|---------------|------------------------------|
| `fma_array` | `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)` | `(*mut c_int, *const c_int, *const c_int, *const c_int, c_int) -> ()` |
| `call_fma`  | `int call_fma(const int *data, int len)` | `(*const c_int, c_int) -> c_int` |
| `driver`    | `void driver(const char *in)` | `(*const c_char) -> ()` |

## Undefined (imported) symbols

The Rust `.so` must not depend on anything outside libc, and ideally should
reach libc through the *same* entry points the C object does.

`driver.c` uses exactly two libc facilities. One of them needed a fix:

| source-level call | C `.so` imports | Rust `.so` originally imported | now |
|---|---|---|---|
| `printf("%d\n", ...)` | `printf@GLIBC_2.2.5` | `printf@GLIBC_2.2.5` | unchanged |
| `sscanf(in, "%d%zn", ...)` | `__isoc99_sscanf@GLIBC_2.7` | `sscanf@GLIBC_2.2.5` | `__isoc99_sscanf@GLIBC_2.7` |

glibc's `<stdio.h>` redirects the source-level name `sscanf` to
`__isoc99_sscanf` for C99 and later, so the compiled C object never calls the
symbol literally spelled `sscanf`. The two are genuinely different functions at
run time (distinct `dlsym` addresses); they differ in whether `%a` is the C99
float conversion or the older GNU allocate-the-string extension. A direct
differential probe over ~30 inputs found no divergence for the fixed `"%d%zn"`
format actually used, so this was not an observable bug — but `src/cstdio.rs`
now binds `__isoc99_sscanf` on `target_env = "gnu"` anyway, which removes the
risk class instead of depending on two implementations continuing to agree.
`d3_rust_imports_the_same_libc_entry_points_as_c` asserts this and fails if the
legacy name creeps back in.

Beyond those two, the Rust `.so`'s undefined list contains only the Rust/libc
runtime (`malloc`, `memcpy`, `_Unwind_*`, ...). There are no non-libc undefined
symbols, verified the robust way rather than by grep: `dlopen(..., RTLD_NOW)`
resolves every relocation eagerly and therefore fails outright if anything is
unresolvable (`d2_rust_so_has_no_unresolved_symbols`).

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one, and `--no-default-features` is equivalent to
it. `check_feature_combos.sh` derives the combination list from `Cargo.toml`
rather than hardcoding it, so a feature added later is picked up automatically.

## Verification checklist

- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name (3/3) — `d1_c_symbols_are_all_exported_by_rust`.
- [x] The Rust `.so` exports no extra public non-libc symbols beyond those 3
      (asserted, so the ABI cannot silently widen).
- [x] The C `.so`'s exported set is exactly the three functions defined in
      `driver.c`, so the mapping is re-derived on every run rather than trusted.
- [x] `nm -D` shows 0 missing symbols; `dlopen(RTLD_NOW)` shows 0 unresolved.
- [x] The Rust `.so` imports the same libc entry points as the C `.so`.
