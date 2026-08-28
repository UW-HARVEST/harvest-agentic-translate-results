# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

## How the artifacts were produced

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-KtEZ0h.so   (name comes from the parent dir name,
#                                             see cmake_path(...) in CMakeLists.txt)

# Rust
cd translation && cargo build --release --offline
# -> translation/target/release/libupdate_frame_header_lib.so
```

## C source inventory (completeness check)

The whole library is two files; nothing was skipped by the translation step.

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/include/lib.h` | `translation/src/lib.rs` (`tflac_u8`, `tflac_u32`, `struct tflac`) | complete |
| `c_src/src/lib.c` | `translation/src/lib.rs` (`enum TFLAC_CHANNEL_MODE`, `update_frame_header`) | complete |

`add_library(... SHARED src/lib.c)` in `CMakeLists.txt` confirms `src/lib.c` is the
only translation unit, so there is no un-translated module.

## Defined dynamic symbols

`nm -D --defined-only <so>`:

| # | symbol | C `.so` | Rust `.so` | type | notes |
|---|--------|---------|------------|------|-------|
| 1 | `update_frame_header` | `T` (0x10f9) | `T` (0x11c60) | func | `void update_frame_header(tflac *t)`; exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

**Symbol diff (C minus Rust): EMPTY.** No symbol is missing from the Rust `.so`,
so no `#[no_mangle]` wrapper had to be added and no C module had to be translated.

### Symbols intentionally NOT exported

These exist in the C source but have no linkage, so they must not (and do not)
appear in `nm -D` for either library:

| C entity | reason |
|----------|--------|
| `enum TFLAC_CHANNEL_MODE` + its 5 enumerators | file-local enum type/constants in `src/lib.c`; no object emitted |
| `typedef uint8_t tflac_u8` / `typedef uint32_t tflac_u32` | typedefs |
| `struct tflac` / `typedef struct tflac tflac` | type only; the caller owns the storage |

## Undefined dynamic symbols

`nm -D --undefined-only <so>`:

* C: `_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable` (w),
  `__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w) — all weak/libc.
* Rust: the same four, plus **libc/libgcc-unwinder imports pulled in by `std`**
  (`_Unwind_*@GCC_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`,
  `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
  `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
  `munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`,
  `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`,
  `syscall`, `write`, `writev`).

**0 missing / undefined non-libc symbols in the Rust `.so`.** Every undefined
entry above resolves from `libc.so.6` / `libgcc_s.so.1`, which is why
`Library::new()` on the Rust `.so` succeeds in the tests.

## ABI of the shared type

`struct tflac` is passed by pointer, so its layout is part of the ABI. Measured
with `offsetof`/`sizeof` on the C side (gcc 11.5, x86-64):

| field | C offset | Rust `#[repr(C)]` offset |
|-------|----------|-------------------------|
| `samplerate` (`u32`) | 0 | 0 |
| `channels` (`u32`) | 4 | 4 |
| `bitdepth` (`u32`) | 8 | 8 |
| `channel_mode` (`u8`) | 12 | 12 |
| *(padding)* | 13..15 | 13..15 |
| `frame_header` (`u32`) | 16 | 16 |
| `cur_blocksize` (`u32`) | 20 | 20 |
| `sizeof` / `align` | 24 / 4 | 24 / 4 |

The differential tests do not use a Rust `struct` at all: they build the 24-byte
record byte-by-byte at these offsets inside a guarded buffer and compare all
bytes (including the 3 padding bytes) afterwards, which also proves neither
implementation writes outside the record.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is also equivalent
here — there are no optional deps and no `cfg(feature = ...)` in `src/lib.rs`).
The tests are nevertheless run under `--no-default-features` as well to prove it.

## Completion gate (Phase D)

Run `./verify.sh` from the crate root to re-check everything below.

- [x] `SYMBOLS.md`: `nm -D` diff (C minus Rust) is EMPTY for both the debug and
      the release cdylib; 0 undefined non-libc symbols in the Rust `.so`.
      Enforced by `tests/phase_d_symbols.rs` (3 tests) and by `verify.sh`.
- [x] Phase B: all 33 rows of `CONFIGS.md` pass — `tests/phase_b_configs.rs`
      (33 tests, ~30 million differential comparisons per run).
- [x] Phase C: all 21 rows of `ERRORS.md` have a passing differential test —
      `tests/phase_c_errors.rs` (23 tests, including the NULL-pointer
      subprocess test and the out-of-range-enum cross product).
- [x] All of the above hold for EVERY feature combination (`<default>`,
      `--no-default-features`, `--all-features` — the crate declares no
      `[features]`, so these are all the configurations that exist) AND for both
      Rust build profiles (debug and release cdylib), i.e. 6 runs of the full
      suite.

### Harness self-check (mutation testing)

To prove the suite is not vacuously passing, 11 deliberate bugs were injected
into `src/lib.rs` one at a time and the suite re-run:

| injected bug | detected? |
|--------------|-----------|
| `bitdepth 20` code `5` → `4` | yes |
| `samplerate < 65536` → `<= 65536` | yes |
| `channels.wrapping_sub(1)` → `saturating_sub(1)` | yes |
| `cur_blocksize <= 256` → `< 256` | **equivalent mutant** (see below) |
| `channel_mode % 4` → `% 5` | yes |
| `samplerate / 1000 < 256` → `<= 256` | yes |
| sync word `0xFFF8` → `0xFFF9` | yes |
| `cur_blocksize 32768` code `0xF` → `0xE` | yes |
| `MID_SIDE` code `0x0A` → `0x0B` | yes |
| `samplerate % 10` → `% 100` | yes |
| drop `#[unsafe(no_mangle)]` | yes (`sym_01` reports the missing symbol) |
| `bitdepth` default arm sets bits instead of none | yes (`err_18`, `err_19`, …) |

`cur_blocksize <= 256` → `< 256` is a genuinely **equivalent** mutant, not a
coverage gap: the ternary at `lib.c:55` is only reached when the value matched no
`case`, and `256` *is* a case label (`lib.c:29`), so the two predicates can only
disagree on a value that never reaches them.
