# zstd 1.5.7 — full Rust translation (cdylib)

A complete, function-by-function translation of the C library in `c_src/` to
Rust. The result is a `cdylib` that exports the **same 615 public symbols** as
the C build and produces **byte-identical output**.

```
cargo build --release      # -> target/release/libzstd.so
```

## Scope

Every `.c` file that `c_src/CMakeLists.txt` globs into the shared library is
translated — 39 files, ~58k lines of C:

| area | C files | Rust |
|---|---|---|
| `common/` | debug, entropy_common, error_private, fse_decompress, pool, threading, xxhash, zstd_common | `src/common/` |
| `compress/` | fse_compress, hist, huf_compress, zstd_compress, zstd_compress_literals, zstd_compress_sequences, zstd_compress_superblock, zstd_double_fast, zstd_fast, zstd_lazy, zstd_ldm, zstd_opt, zstd_preSplit, zstdmt_compress | `src/compress/` |
| `decompress/` | huf_decompress, zstd_ddict, zstd_decompress, zstd_decompress_block | `src/decompress/` |
| `dictBuilder/` | cover, divsufsort, fastcover, zdict | `src/dictbuilder/` |
| `deprecated/` | zbuff_common, zbuff_compress, zbuff_decompress | `src/deprecated/` |
| `legacy/` | zstd_v01 … zstd_v07 | `src/legacy/` |

The C headers that carry real code (`mem.h`, `bits.h`, `bitstream.h`, `fse.h`,
`huf.h`, `zstd_internal.h`, `allocations.h`, `zstd_cwksp.h`,
`zstd_compress_internal.h`, `zstd_decompress_internal.h`, `clevels.h`,
`zstd_ldm_geartab.h`, `zstd_legacy.h`, `pool.h`, `threading.h`, `zstd_deps.h`)
are translated as supporting modules.

`compress/zstd_compress.c` is 7843 lines; its translation is split into
`src/compress/zstd_compress_p1.rs` … `_p6.rs`, textually `include!`d by
`src/compress/zstd_compress.rs`, so all parts share one Rust module exactly as
they share one C translation unit.

## Build configuration reproduced

The CMake build defines `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`,
`DYNAMIC_BMI2=0`, and leaves `ZSTD_MULTITHREAD` undefined. The translation
follows exactly that configuration:

* `XXH_NAMESPACE=ZSTD_` → the xxhash exports are named `ZSTD_XXH32`,
  `ZSTD_XXH64_digest`, `ZSTD_XXH_versionNumber`, … (this is the "macro renames
  the linker symbol" case).
* `DYNAMIC_BMI2=0` / `STATIC_BMI2=0` → every `_bmi2` dispatcher takes the
  `_default` path; `bmi2`/`flags` parameters are kept in the signatures.
* No `ZSTD_MULTITHREAD` → `POOL_*` are the synchronous stubs, `threading.c`
  contributes only `g_ZSTD_threading_useless_symbol`, `ZSTDMT_createCCtx_advanced`
  returns NULL, and every `#ifdef ZSTD_MULTITHREAD` block is omitted.
* `ZSTD_LEGACY_SUPPORT=5` → `zstd_legacy.h`'s dispatch covers v05/v06/v07 only,
  while `zstd_v01.c`…`zstd_v04.c` are still compiled and still export their
  symbols.
* `ZSTD_TRACE == 1` (gcc has weak symbols) → `ZSTD_CCtx`/`ZSTD_DCtx` keep their
  `traceCtx` field (it changes `sizeof`), but the weak `ZSTD_trace_*` hooks are
  undefined/NULL so every trace body reduces to `traceCtx = 0` / nothing.
* `DEBUGLEVEL=0` → `assert()`, `DEBUGLOG`, `RAWLOG` and the static asserts are
  compiled out.
* `ZSTD_ARCH_X86_SSE2` **is** defined on x86-64 gcc, so `zstd_lazy.c`'s
  `ZSTD_row_getSSEMask` is translated with `core::arch::x86_64` intrinsics.
  `ZSTD_ARCH_X86_AVX2` is **not** defined (no `-mavx2`), so the scalar variants
  of `convertSequences_noRepcodes` and `ZSTD_get1BlockSummary` are used.
* No sanitizers, no `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`, no
  `ZSTD_STRIP_ERROR_STRINGS`, no `HUF_FORCE_DECOMPRESS_X1/X2`.

## Translation approach

Transliteration, not redesign: raw pointers, C control flow, C arithmetic, C
order of error checks. `#[repr(C)]` on every shared struct. libc's
`malloc`/`calloc`/`free` (so allocation behaviour matches) and libc's
`qsort`/`qsort_r` (so sort tie-breaking — which is observable in dictionary
output — matches). Bugs and quirks in the original are preserved deliberately,
e.g. `ZSTD_ldm_gear_reset` not writing back its rolling hash,
`HUF_fillDTableX4` being called with `rankStart0`, and the 1.3.4/1.4.0 decoder
work-arounds in the super-block encoder.

## Verification

### 1. Struct layout equality

`layoutcheck/csizes.c` prints `sizeof`/`alignof`/`offsetof` for 35 shared
structs from the real C headers; the Rust side prints the same list.
`layoutcheck/c_sizes.txt` and `layoutcheck/rust_sizes.txt` are identical —
including `ZSTD_CCtx` (5280), `ZSTD_DCtx` (95992), `ZSTD_CDict` (6080),
`ZSTD_DDict` (27352) and every field offset checked. This is what caught the
missing `traceCtx` field.

### 2. Exported-symbol equality

```
./verify.sh
    C   exports: 615
    Rust exports: 615
    MISSING in Rust: 0
    EXTRA in Rust:   0
    TOTAL MISSING SYMBOLS: 0     (per-C-file breakdown, all 39 files OK)
```

### 3. Differential execution against the C library

`difftest/driver.c` `dlopen()`s a given `libzstd.so` and drives a large slice of
the API, printing a deterministic transcript (sizes, return codes, error
strings, and FNV digests of every output buffer). Running it against the C
`.so` and the Rust `.so` and diffing proves byte-identical behaviour:

```
./difftest/run.sh
    C driver exit=0  (11749 lines)
    Rust driver exit=0  (11749 lines)
    === TRANSCRIPTS IDENTICAL (11749 lines) ===
```

What the transcript covers:

* one-shot compression of 5 corpora × 19 sizes (0 B … 400 KB) × every level
  −5…22, each round-tripped, plus `ZSTD_compressCCtx`/`ZSTD_decompressDCtx`
  (2285 compressed digests);
* 27 advanced `ZSTD_c_*` parameters × several values × several levels via
  `ZSTD_compress2` (340 cases) and a `ZSTD_CCtx_getParameter` /
  `ZSTD_cParam_getBounds` / `ZSTD_dParam_getBounds` sweep;
* 6 MB inputs with long-range repeats, with and without LDM, at 4 levels, and
  with `windowLog` 10/15/20 to force the extDict / window-sliding paths;
* streaming compression and decompression with 5 chunk sizes, `compressStream2`
  with `flush`/`end` directives, and 6 MB streamed through 9973/7919-byte chunks;
* dictionaries: `ZDICT_trainFromBuffer`, `ZDICT_trainFromBuffer_cover` (d=6,8),
  `ZDICT_trainFromBuffer_fastCover` (accel 1,2,5), then `ZSTD_CDict`/`ZSTD_DDict`
  round-trips over 6 payload sizes × 4 levels, plus dictionary streaming;
* the sequence APIs (`ZSTD_generateSequences`, `ZSTD_mergeBlockDelimiters`,
  `ZSTD_compressSequences`), the block API (`ZSTD_compressBlock` /
  `ZSTD_decompressBlock` / `ZSTD_insertBlock`), `ZSTD_copyCCtx`,
  `ZSTD_CCtx_refPrefix` / `ZSTD_DCtx_refPrefix`, skippable frames
  (`ZSTD_writeSkippableFrame` / `ZSTD_readSkippableFrame` / `ZSTD_isSkippableFrame`),
  multi-frame concatenation, and 240 `ZSTD_getFrameHeader` cases including every
  partial-input length and the magicless format;
* static (in-place) contexts: `ZSTD_initStaticCCtx`/`DCtx`/`CStream`/`DStream`/
  `CDict`/`DDict`, including the too-small-workspace rejections;
* 1200 corruption/truncation cases (random truncation + random bit flips) fed to
  both `ZSTD_decompress` and `ZSTD_decompressStream`, plus 150 more against a
  dictionary frame — every error code and every partial output digest compared;
* `ZSTD_XXH32`/`ZSTD_XXH64` over 51 lengths, `HIST_count`/`HIST_add`,
  `HUF_cardinality`/`HUF_minTableLog`, `FSE_optimalTableLog`, `ZSTD_cycleLog`,
  `divsufsort` on two 5000-byte corpora, all 82 `ZSTD_getErrorString` codes, the
  `ZBUFF_*` deprecated surface, and the legacy `ZSTD/ZBUFF/FSE v0x` entry points.

Note: feeding *random* bytes to the `legacy/` decoders is undefined behaviour in
the original C (the reference build segfaults), so those cases are excluded —
only well-formed-input behaviour is compared there. Likewise the corruption fuzz
leaves the 4 magic bytes intact, because flipping them re-routes the data into
those same unhardened legacy decoders.

### 4. No reliance on undefined behaviour

The same transcript was produced by three different Rust builds, all identical
to the C output:

| build | result |
|---|---|
| `opt-level=2` (the shipped profile) | identical |
| `opt-level=0` | identical |
| `overflow-checks=on` | identical, **and no panic** |

The overflow-checks build is the interesting one: it completes the entire suite
without a single arithmetic-overflow panic, which means every place the C relies
on wrapping is spelled out explicitly with `wrapping_*` in the Rust. (This audit
is what found `ZSTD_highbit32(0)`, reached from
`FSE_optimalTableLog_internal(…, srcSize == 1, …)`, where the C computes
`31 - 32` in `unsigned` and wraps.)

`--force-warn unreachable_patterns` was also used across the modules to rule out
the silent-miscompile hazard where a `match` arm naming an out-of-scope constant
becomes a catch-all binding instead of a constant pattern.

## Repository layout

```
Cargo.toml            crate-type = ["cdylib"], overflow-checks off (as C)
src/                  the translation
CONTRACT.md           the translation rules and the shared-module vocabulary
check.sh              type-check individual modules in isolation
genlib.sh             regenerate src/lib.rs from the files present
fixups.sh             mechanical `/*!` -> `/*` comment fixups
verify.sh             C vs Rust exported-symbol diff + per-file coverage
difftest/             differential driver + run.sh + recorded transcripts
layoutcheck/          struct-layout equality evidence
cbuild/               the reference C build (cmake) used by the comparisons
```
