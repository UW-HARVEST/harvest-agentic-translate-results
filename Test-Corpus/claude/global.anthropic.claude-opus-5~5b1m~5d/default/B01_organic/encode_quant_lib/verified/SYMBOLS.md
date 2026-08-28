# SYMBOLS.md — Phase A symbol map

Mechanically derived from `nm -D` on both shared objects.

## Build commands used

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-s8YUb3.so   (name comes from the parent dir name,
#    see cmake_path(GET parent FILENAME project_name) in CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libencode_quant_lib.so  ([lib] name = "encode_quant_lib",
#    crate-type = ["cdylib"])
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated in Rust? | Rust location |
|---|---|---|
| `c_src/src/lib.c` | yes | `translation/src/lib.rs` |

Public header `c_src/include/lib.h` declares exactly one prototype (the whole
file is 1 line, 78 bytes):

```c
int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);
```

There are **no** other `.c` / `.h` files, no macro-generated symbol families,
no `#ifdef`-gated alternate entry points (`grep -cE '^#if' c_src/src/lib.c` = 0),
and no `enum`/`struct`/`typedef` declarations in either file. So no C module was
skipped by the translation step.

## Exported (defined) dynamic symbols

`nm -D --defined-only <so> | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort`

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `encode_quant` | `T` (present) | `T` (present) | `#[unsafe(no_mangle)] pub extern "C" fn` in `src/lib.rs` |

**Total C exported symbols: 1. Total present in Rust: 1.**

## Symbol diff

```
$ comm -23 c_syms.txt r_syms.txt      # exported by C but missing from Rust
(empty)
```

**0 symbols missing from the Rust `.so`.** No `#[no_mangle]` wrapper had to be
added and no C module had to be translated — the single C translation unit is
fully covered. Nothing is stubbed or `unimplemented!()`.

## Undefined symbols (imports)

| library | undefined symbols |
|---|---|
| C `.so` | `_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable` (w), `__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w) |
| Rust `.so` | the same 4 weak toolchain symbols, plus only libc/`libgcc` unwinder imports pulled in by `std`: `__errno_location`, `__tls_get_addr`, `__cxa_thread_atexit_impl`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`, `_Unwind_*@GCC_*` |

**0 missing/undefined non-libc symbols in the Rust `.so`.** Every `U`/`w` entry
above resolves against `libc.so.6` / `libgcc_s.so.1` at load time; `libloading`
opens the object successfully in the Phase B/C/D tests, which is the runtime
proof that nothing is unresolved.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default (empty) feature set. `--no-default-features` and
the default build are therefore the same code. Verified by
`tests/phase_d_symbols.rs::features_declared_in_cargo_toml` and by
`scripts/check_all_features.sh`.

## Automated re-check

`tests/phase_d_symbols.rs` re-runs this whole comparison at test time (shelling
out to `nm -D`) so the parity claim above cannot silently rot.
