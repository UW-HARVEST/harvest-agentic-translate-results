# SYMBOLS.md — dynamic symbol surface (Phase A.1)

## How the two shared objects are produced

`c_src/CMakeLists.txt` declares a single target, the **executable** `driver`,
built from `src/main.c` + `src/sillymain.c`:

```cmake
add_executable(driver
    src/main.c
    src/sillymain.c)
```

There is no library target, so the C shared object is produced out-of-tree from
exactly those two translation units (nothing inside `c_src/` is modified):

```sh
gcc -shared -fPIC -o target/c_build/libcdriver.so c_src/src/sillymain.c c_src/src/main.c
```

The CMake executable is also built, out-of-source, to confirm the sanctioned
build still works and to compare whole-process behaviour:

```sh
cmake -S c_src -B target/cmake_build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build target/cmake_build
```

The Rust side gains a `[lib]` target with `crate-type = ["cdylib", "rlib"]`
(`src/lib.rs`), which re-exports the translation under the C names with C
linkage. `src/main.rs` remains the translation of `main.c` for the `driver`
binary.

Build both, then diff:

```sh
cargo build --offline                       # -> target/debug/libdriver.so
nm -D --defined-only target/c_build/libcdriver.so | awk '{print $3}' | sort > c.txt
nm -D --defined-only target/debug/libdriver.so    | awk '{print $3}' | sort > rust.txt
comm -23 c.txt rust.txt                     # symbols in C but missing in Rust
```

## Defined dynamic symbols of the C `.so` (`nm -D --defined-only`)

| # | symbol | type | C definition | exported by Rust `.so` | Rust definition |
|---|--------|------|--------------|------------------------|-----------------|
| 1 | `helloworld` | `T` (global text) | `c_src/src/sillymain.c:28`, declared `c_src/src/sillymain.h:27` | yes (`T helloworld`) | `#[no_mangle] pub extern "C" fn helloworld()` in `src/lib.rs`, body in `src/sillymain.rs` |
| 2 | `main`       | `T` (global text) | `c_src/src/main.c:26` | yes (`T main`) | `#[no_mangle] pub extern "C" fn main()` in `src/lib.rs` |

That is the complete list — the C `.so` defines no other global symbols, no
data symbols, no weak aliases and no macro-generated names (the only
preprocessor construct in the sources is the `SILLYMAIN_H_` include guard).

## Symbol diff result

```
$ comm -23 c.txt rust.txt
(empty)
```

**0 symbols missing from the Rust `.so`.** Nothing was stubbed: both exports
call the real translated body in `src/sillymain.rs`. No C translation unit was
skipped — `main.c` and `sillymain.c` are the only ones and both are translated
(`src/main.rs` + `src/lib.rs::main`, `src/sillymain.rs`).

## Undefined symbols of the Rust `.so` (`nm -D -u`)

All undefined entries are libc / libgcc-unwinder imports, i.e. 0 missing
non-libc symbols:

* glibc: `printf` is reached through `puts`-free direct call
  (`printf@GLIBC_2.2.5`), plus the usual `malloc`, `free`, `calloc`,
  `realloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`,
  `strlen`, `write`, `writev`, `read`, `close`, `open64`, `lseek64`,
  `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `getcwd`, `getenv`,
  `readlink`, `realpath`, `abort`, `syscall`, `__errno_location`,
  `dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`,
  `__tls_get_addr`, `__cxa_thread_atexit_impl`, `gettid`
* libgcc_s unwinder: `_Unwind_*`
* standard weak ELF hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__cxa_finalize`, `__gmon_start__`

The C `.so` imports `puts@GLIBC_2.2.5` (gcc rewrites the `printf` of a
literal ending in `\n` into `puts`); the Rust `.so` imports `printf`. Both
write the identical byte sequence `48 65 6c 6c 6f 20 57 6f 72 6c 64 21 0a`
to the same `FILE *stdout`, so this import-name difference is not observable —
and it is verified byte-for-byte by the Phase B tests.

## Parity is re-checked mechanically

`./verify.sh` regenerates both symbol lists and fails if `comm -23` is non-empty,
for the **debug** and the **release** cdylib (the release profile uses
`panic = "abort"`, a different codegen configuration):

```
PASS target/debug/libdriver.so   exports all 2 C symbols: helloworld main
PASS target/debug/libdriver.so   has no undefined non-libc symbols
PASS target/release/libdriver.so exports all 2 C symbols: helloworld main
PASS target/release/libdriver.so has no undefined non-libc symbols
```

`tests/phase_c_errors.rs::err15_absent_symbols_are_absent_in_both` closes the
other direction: names the C library does *not* define (`helloworld_`,
`_helloworld`, `HelloWorld`, `hello_world`, `sillymain`, `driver_main`) must fail
`dlsym` on the Rust library too, so parity cannot be faked with extra
look-alike exports.

## Note on the `main` export

`src/lib.rs` exports `main` under `#[cfg(not(test))]`. The `cdylib` that the
differential tests `dlopen` is built without `cfg(test)` and therefore always
exports it; the guard only keeps the symbol out of the *unit-test* harness
binary, where it would collide with the entry point libtest generates. The
`nm -D` output above is taken from the real `cdylib`, and
`tests/phase_b_valid.rs` rows 13–16 call the exported `main` through `dlsym`.
