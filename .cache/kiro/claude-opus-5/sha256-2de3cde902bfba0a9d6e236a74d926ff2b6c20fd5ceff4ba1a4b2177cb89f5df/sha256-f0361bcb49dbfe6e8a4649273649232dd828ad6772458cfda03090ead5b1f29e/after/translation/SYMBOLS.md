# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-9AcdNV.so
nm -D --defined-only translation/target/release/libget_predict_func_lib.so
```

## C `.so` dynamic (exported) symbols

```
00000000000024c8 T get_predict_func
```

That is the complete `nm -D --defined-only` output for the C library. Every
other function in `c_src/src/lib.c` is declared `static`, so it has local
(`t`) linkage and is *not* part of the ABI:

| C symbol | linkage | exported? |
|---|---|---|
| `get_predict_func` | `T` (global text) | YES |
| `BTAC1C2_GetPredictFunc` | `t` (local) | no — `static` |
| `BTAC1C2_PredictSample` | `t` (local) | no — `static` |
| `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` (12 fns) | `t` (local) | no — `static` |

CRT/linker-synthesised locals (`_init`, `_fini`, `frame_dummy`,
`register_tm_clones`, `deregister_tm_clones`, `__do_global_dtors_aux`,
`__dso_handle`, `_DYNAMIC`, `_GLOBAL_OFFSET_TABLE_`, `__FRAME_END__`,
`__GNU_EH_FRAME_HDR`, `__TMC_END__`, `completed.0`, and the
`*_init_array_entry` / `*_fini_array_entry` data symbols) are toolchain
artifacts, not library API, and are excluded.

There are no macro-generated symbol families in this source: `lib.c` contains
no function-defining macros, so the symbol list above is exhaustive.

## Rust `.so` dynamic (exported) symbols

```
00000000000120c0 T get_predict_func
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `get_predict_func` | yes (`T`) | yes (`T`, via `#[unsafe(no_mangle)] pub extern "C"`) | MATCH |

## Diff

```
$ comm -3 <(c symbols) <(rust symbols)
(empty)
```

- Symbols exported by C but missing from Rust: **0**
- No symbol required translating a previously-skipped C module: all 15 C
  functions (`get_predict_func`, `BTAC1C2_GetPredictFunc`,
  `BTAC1C2_PredictSample`, `BTAC1C2_PredictSample_Pfn0..11`) are present in
  `translation/src/lib.rs`; the 14 `static` ones are private in Rust too,
  matching their C linkage.
- Undefined non-libc symbols in the Rust `.so`: **0** (verified with
  `nm -D --undefined-only`; all remaining undefined entries resolve to
  `libc`/`libgcc`/`ld-linux` runtime symbols).

STATUS: **PASS** — symbol diff is empty.
