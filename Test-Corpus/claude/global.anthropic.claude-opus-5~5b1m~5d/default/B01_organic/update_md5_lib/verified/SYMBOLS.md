# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-H5hJaK.so
nm -D --defined-only translation/target/release/libupdate_md5_lib.so
```

## C source inventory (`c_src/src/lib.c`, `c_src/include/lib.h`)

The whole library is one translation unit, `src/lib.c`, with three
non-`static` functions. There are no other C source files, so no module was
skipped by the translation.

| C definition | file:line | exported |
|---|---|---|
| `void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n)` | src/lib.c:5 | yes (not `static`) |
| `void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val)` | src/lib.c:16 | yes (not `static`) |
| `tflac_u32 update_md5(tflac *t, const tflac_s32 *samples)` | src/lib.c:33 | yes (declared in lib.h:22) |

No macro-generated symbols, no global/static data objects, no `#ifdef`
feature gates exist in the C source.

## Symbol table comparison

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `tflac_pack_u64le`    | `T` | `T` | present in both |
| 2 | `tflac_md5_addsample` | `T` | `T` | present in both |
| 3 | `update_md5`          | `T` | `T` | present in both |

**Missing from Rust `.so`: none.** No `#[no_mangle]` wrapper had to be added
and no untranslated C module was found.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `memcpy`, `memset`, `__errno_location`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, …) that come from the Rust standard
library runtime. **0 missing/undefined non-libc symbols.**

## Layout / ABI parity (checked with the C compiler, `_Alignof`/`offsetof`)

| type | C size | C align | C offsets | Rust size | Rust align |
|---|---|---|---|---|---|
| `tflac_md5` | 88 | 8 | `pos`@0, `total`@8, `buffer`@16 | 88 | 8 |
| `tflac`     | 96 | 8 | `md5_ctx`@0, `cur_blocksize`@88, `channels`@92 | 96 | 8 |

Enforced in `src/lib.rs` by a `const _: ()` block of `assert!`s.

## Automated re-check

`tests/phase_d_symbols.rs` re-runs this comparison as a test (`nm -D` on both
`.so`s, C-exports ⊆ Rust-exports, no non-libc undefined symbols, the two loaded
libraries are distinct files, and the Rust `.so` contains no
`unimplemented!()`/`todo!()` panic strings), so parity is checked under every
profile and feature set. `run_all.sh` prints the same diff at the end of a run.

Static (non-dynamic) symbol dump of the C `.so` confirms there is nothing else
to translate — apart from the three `T` symbols above, it contains only CRT
glue (`_init`, `_fini`, `frame_dummy`, `register_tm_clones`, …).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and the sources
contain **no `#[cfg(feature …)]`**, so there is exactly one build
configuration. `--no-default-features` is byte-identical to the default
build; the test script still runs both to prove it.
