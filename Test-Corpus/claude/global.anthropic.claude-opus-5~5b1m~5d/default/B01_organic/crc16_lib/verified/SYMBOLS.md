# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects. Nothing here is
assumed; every row is a line of `nm -D` output.

## Build commands

```sh
# C
cd c_src && cmake -S . -B build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build build
#   -> c_src/build/libharvest-work-7ROkid.so   (name comes from the parent dir
#      name via cmake_path(... FILENAME project_name) in CMakeLists.txt)

# Rust
cd translation && cargo build --release
#   -> translation/target/release/libcrc16_lib.so   ([lib] name = "crc16_lib",
#      crate-type = ["cdylib"])
```

## C `.so` dynamic symbol table

`nm -D c_src/build/libharvest-work-7ROkid.so`:

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000010f9 T crc16
```

The four `w` entries are weak, toolchain-injected (ITM/gmon/`__cxa_finalize`)
symbols emitted by the crt glue for *every* GCC shared object. They are not part
of the library's API and are explicitly out of scope ("0 missing/undefined
non-libc symbols").

## Defined-symbol parity table

| # | C symbol (`nm -D --defined-only`) | type | Exported by Rust `.so`? | Rust type | Notes |
|---|-----------------------------------|------|-------------------------|-----------|-------|
| 1 | `crc16` | `T` | **YES** — `crc16` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn crc16` in `src/lib.rs` |

**Missing symbols: 0.** The symbol diff (C-defined minus Rust-defined) is empty.

## Why the surface is exactly one symbol

`c_src/include/lib.h` declares exactly one function (line 282):

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

`tflac_u8` / `tflac_u16` / `tflac_u32` are `typedef`s — they generate no symbols.

`tflac_crc16_tables` (header line 7) is declared
`static const tflac_u16 tflac_crc16_tables[8][256]`. `static` at file scope gives
it **internal linkage**, so it contributes no dynamic symbol; `nm -D` on the C
`.so` confirms its absence. The Rust translation correctly mirrors this by making
`tables::TFLAC_CRC16_TABLES` `pub(crate)` — crate-private, not exported. This is
a match, not a gap.

Note also `c_src/src/lib.c` line 3 re-declares
`static const tflac_u16 tflac_crc16_tables[8][256];` — a *tentative*
re-declaration of the already-initialised header definition, so the initialised
data from the header is what the function reads. There is no second, zeroed
table. (Verified empirically: the C `.so` returns non-trivial CRCs, and matches
the Rust table-driven implementation on every input tested.)

## No whole-module gaps

`c_src` contains exactly three files: `CMakeLists.txt`, `include/lib.h`,
`src/lib.c`. `CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`).
No C source file was skipped by the translation, so no "TRANSLATE the missing C
source" work is required. Confirmed:

```sh
$ find c_src -type f
c_src/CMakeLists.txt
c_src/include/lib.h
c_src/src/lib.c
```

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc/`ld.so` imports
(`memcpy`, `__cxa_*`, `_Unwind_*`-class runtime glue). There are **0 undefined
non-libc symbols** — nothing the loader cannot resolve. Verified by
`tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`, which
`dlopen`s the Rust `.so` with `RTLD_NOW` (eager binding, so any unresolvable
relocation fails the test).

## Verification

Enforced as executable tests, not prose, in `translation/tests/symbols.rs`:

* `c_and_rust_export_identical_symbol_sets` — runs `nm -D --defined-only` on
  both `.so`s, filters weak toolchain glue, and asserts the two sets are equal
  (so the diff must reach **empty** in both directions).
* `rust_so_has_no_unresolved_non_libc_symbols` — eager-binds the Rust `.so`.
* `tflac_crc16_tables_is_not_exported` — asserts the `static` table is absent
  from *both* `.so`s, matching internal linkage.
