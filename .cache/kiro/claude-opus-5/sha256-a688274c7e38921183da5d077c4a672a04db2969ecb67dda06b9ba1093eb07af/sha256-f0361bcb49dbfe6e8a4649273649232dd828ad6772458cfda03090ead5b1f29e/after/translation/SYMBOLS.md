# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D c_src/build/libdriver.so
nm -D translation/target/release/libdriver.so
```

## C `.so` dynamic symbol table (verbatim)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
000000000000122b T driver
0000000000001129 T fma_array
                 U memcpy@GLIBC_2.14
                 U printf@GLIBC_2.2.5
```

## Defined (`T`) public symbols exported by C

| # | symbol | C declaration | exported by Rust `.so`? |
|---|--------|---------------|-------------------------|
| 1 | `driver`    | `void driver(const int *data, int len);` (`include/driver.h`) | YES (`T driver`) |
| 2 | `fma_array` | `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);` (`src/driver.c`, no header decl but external linkage) | YES (`T fma_array`) |

`inner` (`src/driver.c`) is declared `static` → internal linkage → not in the
dynamic symbol table of either library. Correctly translated as a private Rust
`fn inner`. It is **not** a missing symbol.

There are no macro-generated symbols, no error enums, no global data objects,
and no additional translation units in `c_src` (`CMakeLists.txt` compiles
exactly one source file: `src/driver.c`).

## Symbol diff

```
comm -23 <(C defined T symbols) <(Rust defined T symbols)   ->  (empty)
```

* Missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
  (all `U` entries resolve to glibc or to `libgcc_s`/`_Unwind_*` unwinder
  symbols pulled in by the Rust runtime; none are unresolved project symbols —
  the library loads and both symbols resolve at runtime, proven by the
  `libloading` tests.)

**Phase A / Phase D symbol gate: PASS (diff is empty).**

No Rust source had to be added: the whole of `driver.c` was translated.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default (empty) feature set. There is
consequently exactly one feature combination to verify:

| # | combination | command |
|---|-------------|---------|
| 1 | default (= no features; identical to `--no-default-features`) | `cargo test --release` / `cargo test --release --no-default-features` |
