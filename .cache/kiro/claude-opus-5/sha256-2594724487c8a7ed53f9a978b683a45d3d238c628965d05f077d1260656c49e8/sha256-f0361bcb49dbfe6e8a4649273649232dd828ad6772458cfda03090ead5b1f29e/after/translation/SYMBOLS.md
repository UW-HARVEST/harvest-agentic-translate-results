# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-qKupep.so`
* Rust `.so`: `translation/target/release/libhdr_bitrate_lib.so`

## C `.so` exported (defined, dynamic) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-qKupep.so
00000000000010f9 T hdr_bitrate
```

The C library defines exactly **one** dynamic symbol. `c_src/include/lib.h`
declares exactly one function and no renaming macros, so the linker name equals
the source name. `halfrate.0` is a function-local `static const` and is a
`LOCAL` symbol in `.symtab` only — it is not exported, so it is not part of the
ABI surface.

## Parity table

| # | C symbol | type | present in Rust `.so`? | Rust item |
|---|----------|------|------------------------|-----------|
| 1 | `hdr_bitrate` | `T` (global text) | YES — `T hdr_bitrate` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hdr_bitrate` in `src/lib.rs` |

## Symbol diff

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $3}' | sort | grep -vE '^(_ZN|rust_|__rust|_ITM_|__cxa|_Unwind)')
(empty)
```

**Missing symbols: 0.** No module of the C source was left untranslated
(`c_src/src/lib.c` is 14 lines and is the only C translation unit in
`CMakeLists.txt`). No stubs or `unimplemented!()` were added.

## Undefined (imported) symbols

The Rust `.so` imports only libc/runtime symbols
(`__libc_start_main`-class, unwinder, `memcpy`-class). Because
`panic = "abort"` is set for the release profile, no Rust-specific unresolved
symbols remain. 0 missing/undefined non-libc symbols.
