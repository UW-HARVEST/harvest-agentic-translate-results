# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

## Build configurations

`Cargo.toml` contains **no `[features]` section** and no optional dependencies,
therefore the complete set of valid feature combinations is exactly one: the
empty set (`--no-default-features` is identical to the default build).

`c_src/CMakeLists.txt` defines no `option()`, no `target_compile_definitions`,
and no conditional sources — it unconditionally compiles `src/lib.c` into one
shared library. There are no `#if`/`#ifdef` branches anywhere in `c_src`
(verified by grep). So there is exactly **one** build configuration on both
sides.

| build matrix | command |
|---|---|
| default (== only combo) | `cargo check --no-default-features` / `cargo test --no-default-features` |

## Shared objects compared

* C:    `c_src/build/libtranslated_rust.so` (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `target/debug/libcrc16_lib.so` (`crate-type = ["cdylib"]`)

## `nm -D --defined-only` — exported (dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `crc16` | `T` (`00000000000010f9`) | `T` (`00000000000135c0`) | the entire public ABI; `#[unsafe(no_mangle)] pub unsafe extern "C" fn crc16` |

**Symbol diff: EMPTY.** Every symbol exported by the C `.so` is exported by the
Rust `.so` under the exact same name. The Rust `.so` exports no extra
non-libc/non-runtime symbols either.

### Why only one symbol

`tflac_crc16_tables` is declared `static const` in `c_src/include/lib.h`, so it
has internal linkage and is deliberately **not** exported by the C `.so`
(confirmed: `nm -D` shows no `tflac_crc16_tables`). The Rust side mirrors this
with a private `mod tables;` / `pub(crate)`-scoped `TFLAC_CRC16_TABLES` static,
which likewise is not exported. The `tflac_u8` / `tflac_u16` / `tflac_u32`
typedefs are compile-time-only and produce no symbols in either language.

No C source file was left untranslated: `c_src` contains exactly one
translation unit (`src/lib.c`, 20 lines) and one header (`include/lib.h`, 282
lines of which 279 are the table data). Both are fully translated into
`src/lib.rs` + `src/tables.rs`. No stubs and no `unimplemented!()` exist in the
Rust tree.

## Table-data parity

The 8×256 = **2048** `tflac_crc16_tables` entries were compared mechanically by
parsing the hex literals out of `c_src/include/lib.h` and `src/tables.rs`:

```
C count: 2048  Rust count: 2048  identical: True
```

## Undefined symbols

The C `.so` imports only weak CRT hooks (`__cxa_finalize`, `__gmon_start__`,
`_ITM_*`). The Rust `.so` additionally imports libc (`malloc`, `memcpy`,
`abort`, …) and `_Unwind_*` from libgcc — all of which come from the Rust
standard library / panic runtime, not from untranslated C code. **0 missing or
undefined non-libc symbols.**

## Release artifact parity

The `release` profile sets `panic = "abort"`, which changes codegen. The release
cdylib was verified separately (`CRC16_RUST_SO=target/release/libcrc16_lib.so
cargo test`): identical single exported symbol `crc16`, and all 35 differential
tests pass against it.

## Cross-optimization matrix

The C reference was additionally rebuilt at `-O0`, `-O2` and `-O3` (into
out-of-tree build dirs; `c_src/` itself was not modified) and the full suite run
against each, plus the optimized-Rust × `-O2`-C pairing:

| C build | Rust build | tests passed | failures |
|---|---|---|---|
| default (cmake, no `-O`) | debug | 35 | 0 |
| `-O0` | debug | 35 | 0 |
| `-O2` | debug | 35 | 0 |
| `-O3` | debug | 35 | 0 |
| default | release (`panic=abort`) | 35 | 0 |
| `-O2` | release | 35 | 0 |

## ⚠ Harness correctness note (important)

`cargo test` **does not build the `cdylib`**. Because `crate-type =
["cdylib"]` and an integration test cannot link a cdylib, Cargo never emits
`libcrc16_lib.so` during `cargo test` — so a naive `cargo test` silently
`dlopen`s a **stale** `.so` from a previous `cargo build` and every differential
assertion passes vacuously. This was observed in practice during this
verification (a deliberately broken Rust build kept "passing").

Two mitigations are in place:

1. `run_tests.sh` always runs `cargo build` before `cargo test`.
2. `tests/common/mod.rs::assert_fresh` compares the `.so` mtime against the
   newest file in `src/` (and `c_src/`) and panics with `STALE ARTIFACT: …` if
   the library is older than the source. This turns silent vacuity into a loud
   failure.

Always invoke the suite via `./run_tests.sh`.
