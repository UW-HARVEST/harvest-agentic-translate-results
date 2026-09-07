# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

```
C:    nm -D --defined-only c_src/build/libdriver.so
Rust: nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` dynamic symbol table (defined, global)

| # | symbol | C type | present in Rust `.so`? | Rust definition |
|---|--------|--------|------------------------|-----------------|
| 1 | `printLine` | `T` (`void printLine(const char*)`) | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `bad`       | `T` (`void bad(void)`)              | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 3 | `good`      | `T` (`void good(void)`)             | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 4 | `driver`    | `T` (`void driver(int)`)            | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

Missing symbols: **none**. `comm -23` of the two sorted symbol-name lists is empty.

## Deliberately NOT exported (must stay local in both)

These are `static` in the C translation unit (`nm` shows lowercase `t`, they are
absent from `nm -D`), so they are private in Rust as well. Exporting them would
be an ABI *divergence*, not a fix.

| symbol | C linkage | Rust |
|--------|-----------|------|
| `helperBad`    | `t` (file-local) | private `fn helperBad()` |
| `helperGood1`  | `t` (file-local) | private `fn helperGood1()` |

## Undefined (imported) symbols

The C `.so` imports `printf` / `puts` from libc. The Rust `.so` imports `puts`
(LLVM lowers `printf("%s\n", p)` to `puts(p)`, exactly as GCC does at `-O2`;
byte output is identical) plus `memcpy`/`malloc`/unwinder/`std`-runtime libc
symbols. `nm -D -u` on the Rust `.so` shows **0 undefined non-libc,
non-libgcc_s symbols** — every remaining `U` entry resolves from
`libc.so.6` / `libgcc_s.so.1` (glibc + `_Unwind_*`), which are present on the
target platform and are loaded automatically.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one (`--no-default-features` is
equivalent to the default here). Verified by
`grep -n '^\[features\]' Cargo.toml` → no match. Phase D's "every feature
combination" therefore reduces to the single default combination, and the test
suite is additionally run with `--no-default-features` to prove that.

## Binary executable

Neither project builds a driver binary: `c_src/CMakeLists.txt` contains only
`add_library(driver SHARED src/driver.c)` (no `add_executable`), there is no
`main()` in `c_src/src/driver.c`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` (no `[[bin]]`, no `src/main.rs`). The
"compare binary stdout" gate is therefore not applicable; stdout is instead
compared per-call through the FFI boundary by capturing fd 1 (see
`tests/differential.rs`).
