# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-X1RjLL.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libcrc16_lib.so
```

## C `.so` exported (defined) dynamic symbols

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `crc16` | `T` (text/global function) | YES — `T crc16` |

That is the complete list. `nm -D --defined-only` on
`libharvest-work-X1RjLL.so` yields exactly one line.

### Why there is only one symbol

`c_src/include/lib.h` defines `tflac_crc16_tables` as
`static const tflac_u16 tflac_crc16_tables[8][256]` — `static` gives it
**internal linkage**, so it is deliberately not part of the ABI and cannot
appear in `nm -D`. `c_src/src/lib.c` re-declares the same `static` array and
defines the single external function `crc16`. `tflac_u8` / `tflac_u16` /
`tflac_u32` are `typedef`s and emit no symbols.

Consequently there is **no missing/untranslated C module**: `src/lib.c` is the
only translation unit in `CMakeLists.txt`, and the whole of it (the 8x256
table plus `crc16`) is present in the Rust crate
(`translation/src/tables.rs`, `translation/src/lib.rs`).

## Rust `.so` exported (defined) dynamic symbols

| # | symbol | type |
|---|--------|------|
| 1 | `crc16` | `T` |

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libharvest-work-X1RjLL.so | awk '{print $NF}' | sort) \
         <(nm -D --defined-only translation/target/release/libcrc16_lib.so | awk '{print $NF}' | sort)
```

Result: **EMPTY** — 0 C symbols missing from the Rust `.so`.

## Undefined symbols in the Rust `.so`

All undefined entries are libc / libgcc-unwind imports pulled in by the Rust
`std` runtime, not unresolved project symbols:

`_Unwind_*` (GCC unwinder), `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`,
`calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
`gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`,
`munmap`, `open64`, `posix_memalign`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`,
`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`.

**Non-libc undefined symbols: 0.**

## Table-data parity (checked mechanically, not by eye)

The 8x256 `tflac_crc16_tables` constant is the bulk of the C source, so its
transcription was verified by extraction + `diff` rather than inspection:

```sh
awk '/tflac_crc16_tables\[8\]\[256\]/,/^};/' c_src/include/lib.h \
  | grep -oE '0x[0-9a-fA-F]{4}' | tr 'A-F' 'a-f' > /tmp/c_tables.txt
grep -oE '0x[0-9a-fA-F]{4}' translation/src/tables.rs \
  | tr 'A-F' 'a-f' > /tmp/rust_tables.txt
wc -l /tmp/c_tables.txt /tmp/rust_tables.txt   # 2048 each (8 * 256)
diff /tmp/c_tables.txt /tmp/rust_tables.txt    # no output
```

Result: 2048 values on each side, **identical in value and order**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default (empty) feature set. `cargo check
--no-default-features` and the default build are the same compilation. See
`CONFIGS.md` for the runtime configuration surface, which is where this
library's actual variability lives.

## Gate status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
