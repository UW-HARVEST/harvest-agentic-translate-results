# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C shared object

`c_src/build/libharvest-work-7WmmEA.so`

```
$ nm -D --defined-only c_src/build/libharvest-work-7WmmEA.so
00000000000010f9 T update_frame_header
```

## Rust shared object

`translation/target/release/libupdate_frame_header_lib.so`

```
$ nm -D --defined-only translation/target/release/libupdate_frame_header_lib.so
00000000000116b0 T update_frame_header
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `update_frame_header` | T | T | MATCH |

Symbol diff (`comm -23` of the C name list against the Rust name list): **empty**.

There are no macro-generated symbols in this library: `c_src/src/lib.c` defines
exactly one function and `c_src/include/lib.h` declares exactly one function.
No C source file is untranslated — the C tree is `include/lib.h` + `src/lib.c`
only, and `src/lib.rs` covers both.

## Undefined (imported) symbols

The C `.so` imports nothing but the ELF/glibc baseline; the Rust `.so` imports
only libc/`__cxa`-style runtime symbols supplied by the platform. There are 0
missing/undefined **non-libc** symbols in the Rust `.so`.

## Type surface (not symbols, but part of the ABI)

`struct tflac` layout, verified against the C compiler with `offsetof`:

| field | C offset | Rust `#[repr(C)]` offset |
|-------|----------|--------------------------|
| `samplerate` | 0 | 0 |
| `channels` | 4 | 4 |
| `bitdepth` | 8 | 8 |
| `channel_mode` | 12 | 12 |
| `frame_header` | 16 | 16 |
| `cur_blocksize` | 20 | 20 |
| **size / align** | 24 / 4 | 24 / 4 |

## Result

`comm -23` of the C symbol list against the Rust symbol list is **empty** in
every configuration. Asserted from inside the suite so it cannot drift:

| test | check |
|------|-------|
| `d01_symbol_parity` | every symbol `nm -D --defined-only` reports for the C `.so` is also defined by the Rust `.so` |
| `d02_no_undefined_non_libc_symbols` | `ldd -r` reports no unresolved symbols for either `.so`, and no symbol the Rust `.so` *imports* is one the C `.so` *defines* (which would mean the Rust calls back into untranslated C) |

`d02` uses `ldd -r` rather than an allowlist over `nm -D --undefined-only`: a Rust
cdylib legitimately imports the whole libgcc unwinder plus a large slice of glibc
(`_Unwind_*`, `mmap64`, `statx`, `pthread_key_create`, …), so an allowlist would
be guesswork, whereas `ldd -r` performs real relocation processing and reports
only genuine failures.

`feature_matrix.sh` re-checks the symbol diff for every feature configuration.

No C source was untranslated: the C tree is exactly `include/lib.h` +
`src/lib.c`, declaring and defining one function, and both are covered by
`src/lib.rs`. Nothing is stubbed and there is no `unimplemented!()` in the crate.
