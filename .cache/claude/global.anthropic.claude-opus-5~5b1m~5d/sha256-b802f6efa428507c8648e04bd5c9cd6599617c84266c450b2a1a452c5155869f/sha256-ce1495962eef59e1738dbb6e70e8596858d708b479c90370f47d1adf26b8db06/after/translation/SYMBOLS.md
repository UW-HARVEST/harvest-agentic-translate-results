# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C `.so` (`c_src/build/libdriver.so`)

```
$ nm -D --defined-only c_src/build/libdriver.so
000000000000122b T driver
0000000000001129 T fma_array
```

Note: `inner` is `static` in `src/driver.c` and therefore **not** an exported
symbol (confirmed: it does not appear in `nm -D`). It must NOT be exported from
the Rust `.so` either.

## Rust `.so` (`translation/target/release/libdriver.so`)

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep ' T '
0000000000011760 T driver
0000000000011910 T fma_array
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|---------------|--------|
| 1 | `fma_array` | `T` (func) | yes | yes | OK |
| 2 | `driver`    | `T` (func) | yes | yes | OK |

**Missing from Rust: none.**
**Extra non-libc undefined symbols in Rust: none** (only libc/`ld` runtime
imports such as `printf`, `memcpy`, `malloc`, unwinding stubs).

### C declarations (ground truth)

```c
/* include/driver.h */
void driver(const int *data, int len);

/* src/driver.c — defined but not declared in the public header;
   still an exported symbol with external linkage. */
void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);

/* src/driver.c — static, NOT exported */
static void inner(int *out, int len);
```

### Whole-module completeness check

`c_src` contains exactly one translation unit (`src/driver.c`) and one public
header (`include/driver.h`). All three functions defined in that TU
(`fma_array`, `inner`, `driver`) have counterparts in `translation/src/lib.rs`
(`fma_array`, `inner` as a private `unsafe fn`, `driver`). No C module was
skipped; no symbol required stubbing.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only configuration is the default (empty) feature set.
`--no-default-features` is therefore equivalent to the default build. Both are
still exercised by `run_all.sh` for completeness.

## Verification result

* `nm -D` symbol diff (C minus Rust) is **EMPTY** for both the debug and the
  release Rust `.so`. Verified by `run_all.sh` step 5 and by the in-suite tests
  `d1_rust_so_exports_every_c_symbol` / `d2_static_c_function_is_not_exported_by_either`.
* `ldd -r` reports **no** unresolved symbols in either `.so`; every undefined
  symbol in the Rust `.so` is a versioned glibc/libgcc import
  (`printf@GLIBC_2.2.5`, `memcpy@GLIBC_2.14`, `_Unwind_Resume`, ...).
  Verified by `d3_rust_so_has_no_unresolved_non_libc_symbols`.
* Both symbols are additionally proven to be `dlsym`-able and live by
  `d4_every_c_symbol_is_dlsym_able_from_rust_so` — the whole suite only ever
  reaches the Rust code through `dlopen` + `dlsym` on `libdriver.so`, never by
  calling Rust functions directly, so the `#[unsafe(no_mangle)] extern "C"`
  wrappers are themselves under test.
* No C module was missing, so no new translation was required in this phase.

### Build caveat found during verification

The crate declares `crate-type = ["cdylib"]` only. `cargo test` compiles
`src/lib.rs` into a (empty) unit-test binary but does **not** emit
`target/<profile>/libdriver.so`. Running `cargo test` alone therefore silently
tests whatever `.so` was last built — a mutation of `src/lib.rs` passed the
entire suite this way. The harness now refuses to run if the `.so` is older than
any file in `src/` (`assert_so_fresh`), and `run_all.sh` always runs
`cargo build` before `cargo test`.
