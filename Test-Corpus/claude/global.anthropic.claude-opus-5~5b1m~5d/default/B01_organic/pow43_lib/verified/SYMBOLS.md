# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libharvest-work-6Ga2gD.so
#      (CMake derives the project name from the parent directory of c_src,
#       so the file name tracks the checkout directory name.)

# Rust
cd translation && cargo build --release
#   -> translation/target/release/libpow43_lib.so
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libharvest-work-6Ga2gD.so`

```
00000000000010f9 T pow43
```

Total: **1** exported symbol.

`g_pow43` is `static const`, so it is a *local* symbol (`nm` type `r`, not in
`.dynsym`) and is deliberately **not** part of the public surface.

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libpow43_lib.so`

```
0000000000011a60 T pow43
```

Total: **1** exported symbol.

## Symbol parity table

| # | symbol | C `.so` | Rust `.so` | signature | status |
|---|--------|---------|------------|-----------|--------|
| 1 | `pow43` | `T` (global text) | `T` (global text) | `float pow43(int x)` / `extern "C" fn(c_int) -> f32` | **MATCH** |

### Diff

```
$ diff <(nm -D --defined-only <c.so>  | awk '{print $3}' | sort) \
       <(nm -D --defined-only <rs.so> | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: none. Extra in Rust: none. The symbol diff is empty.**

No symbol required translating a skipped module: the C library is a single
translation unit (`c_src/src/lib.c`, 49 lines) with a single public function,
and that function is fully translated in `translation/src/lib.rs` — no stubs,
no `unimplemented!()`, no `todo!()`.

```
$ grep -nE 'unimplemented!|todo!|panic!\("stub' translation/src/lib.rs
(no matches)
```

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/libpow43_lib.so` lists only
libc / libgcc-unwind / weak ELF housekeeping imports pulled in by the Rust
runtime, and **no** unresolved project symbols:

* `_Unwind_*@GCC_*` (13) — panic unwinder
* `__errno_location`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
  `__tls_get_addr`, `__gmon_start__`, `_ITM_*` — runtime/TLS/weak stubs
* `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`,
  `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`,
  `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
  `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
  `write`, `writev` — glibc

**0 missing / undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no
`cfg(feature = ...)` appears anywhere in `src/`, so there is exactly one
build configuration (the default). `--no-default-features` is therefore
equivalent to the default build; both are exercised in
`tests/feature_combos.rs` / by the `run_all.sh` loop.

There is no `[[bin]]` target in `Cargo.toml` and no `add_executable` in
`c_src/CMakeLists.txt` (and no `main(` in the C sources), so the
"compare binary stdout" requirement is **not applicable** to this project.
