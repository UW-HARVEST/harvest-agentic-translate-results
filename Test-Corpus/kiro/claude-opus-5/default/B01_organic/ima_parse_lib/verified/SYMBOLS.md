# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D --defined-only c_src/build/libharvest-work-7soRSc.so
nm -D --defined-only translation/target/release/libima_parse_lib.so
```

## C source surface

The C library is a single translation unit, `c_src/src/lib.c`, with a single
public header, `c_src/include/lib.h`.

`include/lib.h` declares exactly one function:

| declaration | linkage |
|---|---|
| `int ima_parse(struct ima_info *info, const void *data);` | external |

Everything else in `src/lib.c` has **internal** linkage (`static`) and is
therefore not part of the ABI:

| C entity | kind | in Rust |
|---|---|---|
| `ima_bswap16` | `static` function | `fn ima_bswap16` (private) |
| `ima_bswap32` | `static` function | `fn ima_bswap32` (private) |
| `ima_bswap64` | `static` function | `fn ima_bswap64` (private) |
| `ima_btoh16` | `static` function | `fn ima_btoh16` (private) |
| `ima_btoh32` | `static` function | `fn ima_btoh32` (private) |
| `ima_btoh64` | `static` function | `fn ima_btoh64` (private) |
| `struct caf_header` | private type | `#[repr(C)] struct caf_header` |
| `struct caf_chunk` | private type | `#[repr(C)] struct caf_chunk` |
| `struct caf_audio_description` | private type | `#[repr(C)] struct caf_audio_description` |
| `struct caf_packet_table` | private type | `#[repr(C)] struct caf_packet_table` |
| `struct caf_data` | private type | `#[repr(C)] struct caf_data` |
| `struct ima_block` | public type (header) | `#[repr(C)] pub struct ima_block` |
| `struct ima_info` | public type (header) | `#[repr(C)] pub struct ima_info` |

No macro-generated symbol names, no `#ifdef`-gated extra entry points, no
compile-time feature flags exist in the C source. `grep -n 'ifdef\|ifndef\|
#if\|define' c_src/src/lib.c c_src/include/lib.h` yields only the `#include`
lines and the inline `(32)` array bound.

## `nm -D --defined-only` diff

| symbol | C `.so` | Rust `.so` | status |
|---|---|---|---|
| `ima_parse` | `T` (0x122d) | `T` (0x116f0) | **present in both** |

Symbols exported by C but missing from Rust: **0**
Symbols exported by Rust but not by C: **0** (no extra `T`/`D`/`B` entries)

No implementation was absent, so no C module had to be translated and no
`#[no_mangle]` wrapper had to be added.

## Undefined (imported) symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
entries (`memcpy`, `malloc`, `_Unwind_*`, `__errno_location`, `dl_iterate_phdr`,
…) that come from the Rust standard library's panic/backtrace machinery.

**0 missing or undefined non-libc symbols.**

## Automated re-check

`translation/check_symbols.sh` re-derives this diff and exits non-zero if it is
not empty.

## Completion gate (Phase D)

| gate | result |
|---|---|
| `nm -D`: 0 symbols missing from the Rust `.so` | **PASS** (`ima_parse` in both; diff empty in both directions) |
| `nm -D`: 0 undefined non-libc symbols in the Rust `.so` | **PASS** (all imports are libc / libgcc-unwind) |
| No module of C source left untranslated | **PASS** (`src/lib.c` is the only translation unit; every `static` helper and every `struct` is present in `src/lib.rs`) |
| No stub / `unimplemented!()` used to fake a symbol | **PASS** (`grep -n 'unimplemented\|todo!\|panic!' src/lib.rs` → no matches) |
| Every `CONFIGS.md` row passes across randomized inputs | **PASS** (40 tests) |
| Every `ERRORS.md` row has a passing differential test | **PASS** (27 tests) |
| Holds under every cargo feature combination | **PASS** (no `[features]` declared ⇒ `{default}` ≡ `{--no-default-features}`; both run) |
| Holds in both the `dev` and `release` profiles | **PASS** |

Reproduce with `./run_tests.sh` (builds the C `.so`, enumerates feature
combinations from `Cargo.toml`, `cargo check`s each, builds the cdylib and runs
the full differential suite in both profiles, then re-checks the symbol diff).
