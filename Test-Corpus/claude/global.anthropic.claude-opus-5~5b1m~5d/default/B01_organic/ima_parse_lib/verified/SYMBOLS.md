# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-7QqEDG.so   (name == parent dir name, see CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libima_parse_lib.so   (crate-type = ["cdylib"])
```

## C source inventory (completeness check)

The whole C library is **two files**:

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/include/lib.h` | typedefs `ima_u8/u16/u32/u64/f64_t`, `struct ima_block`, `struct ima_info`, prototype `ima_parse` | yes — `translation/src/lib.rs` |
| `c_src/src/lib.c` | typedefs `ima_s32/s64_t`; `struct caf_header`, `caf_chunk`, `caf_audio_description`, `caf_packet_table`, `caf_data`; `static` fns `ima_bswap16/32/64`, `ima_btoh16/32/64`; extern fn `ima_parse` | yes — `translation/src/lib.rs` |

No C module was skipped: there is exactly one translation unit and it is fully
translated. Every `static` helper is reproduced (as private Rust `fn`s); they
have internal linkage in C and therefore correctly do **not** appear in `nm -D`
for either library.

## `nm -D --defined-only` — C

```
000000000000122d T ima_parse
```

## `nm -D --defined-only` — Rust

```
0000000000011c90 T ima_parse
```

## Symbol parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `ima_parse` | `T` (global text) | `T` (global text) | **MATCH** |

**Missing from Rust: none.** The symbol diff is empty:

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-7QqEDG.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libima_parse_lib.so | awk '{print $NF}' | sort)
$ echo $?
0
```

## Weak / undefined symbols

C `.so` undefined+weak: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__` — all toolchain/libc artifacts.

Rust `.so` undefined: `_Unwind_*@GCC_*` (libgcc_s), and libc symbols
(`malloc`, `free`, `memcpy`, `memset`, `memmove`, `bcmp`, `calloc`, `realloc`,
`posix_memalign`, `abort`, `__errno_location`, `strlen`, `open64`, `read`,
`write`, `writev`, `close`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`,
`munmap`, `getcwd`, `getenv`, `readlink`, `realpath`, `syscall`,
`dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`,
`__cxa_thread_atexit_impl`, `gettid`).

`ldd` resolves to only `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`.

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.** ✔

## Cargo feature surface

`translation/Cargo.toml` declares **no `[features]` section**, so there is
exactly one feature combination to verify:

| # | feature combo | cargo invocation |
|---|---------------|------------------|
| 1 | default (empty) | `cargo test --release` |
| 2 | `--no-default-features` (identical to #1, no features exist) | `cargo test --release --no-default-features` |

Both (plus `--all-features`) are built and tested by `verify.sh`.

## ABI layout parity (verified against a `gcc` `sizeof`/`offsetof` probe)

| struct | C size/align | C offsets | Rust constant | ✔ |
|--------|--------------|-----------|---------------|---|
| `caf_header` | 8 / 4 | type 0, version 4, flags 6 | `SIZEOF_CAF_HEADER=8`, `CAF_HEADER_TYPE=0`, `CAF_HEADER_VERSION=4` | ✔ |
| `caf_chunk` | 16 / 8 | type 0, **size 8** (4 bytes padding) | `SIZEOF_CAF_CHUNK=16`, `CAF_CHUNK_TYPE=0`, `CAF_CHUNK_SIZE=8` | ✔ |
| `caf_audio_description` | 32 / 8 | sample_rate 0, format_id 8, format_flags 12, bytes_per_packet 16, frames_per_packet 20, channels_per_frame 24, bits_per_channel 28 | `CAF_DESC_SAMPLE_RATE=0`, `CAF_DESC_FORMAT_ID=8`, `CAF_DESC_CHANNELS_PER_FRAME=24` | ✔ |
| `caf_packet_table` | 24 / 8 | packet_count 0, frame_count 8, priming 16, remainder 20 | `CAF_PAKT_FRAME_COUNT=8` | ✔ |
| `caf_data` | 4 / 4 | edit_count 0 | `SIZEOF_CAF_DATA=4` | ✔ |
| `ima_block` | 34 / 2 | preamble 0, data 2 | `#[repr(C)] ima_block` + `const _` asserts | ✔ |
| `ima_info` | 40 / 8 | blocks 0, size 8, sample_rate 16, frame_count 24, channel_count 32 | `#[repr(C)] ima_info` + `const _` asserts | ✔ |
