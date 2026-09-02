# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libString_Slice.so`
* Rust: `translation/target/release/libString_Slice.so`

## Public (defined, dynamic) symbols

`nm -D --defined-only` output, verbatim:

```
$ nm -D --defined-only c_src/build/libString_Slice.so
0000000000001129 T slice

$ nm -D --defined-only translation/target/release/libString_Slice.so
00000000000117a0 T slice
```

| # | C symbol | type | Rust `.so` exports it? | notes |
|---|----------|------|------------------------|-------|
| 1 | `slice`  | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn slice` in `src/lib.rs` |

`include/slicing.h` declares exactly one prototype:

```c
int slice(char *mystr, int *start_ptr, int *stop_ptr);
```

There are no namespace/renaming macros, no macro-generated symbol families, no
exported globals, and no additional translation units in `CMakeLists.txt`
(`add_library(String_Slice SHARED src/slicing.c)` — a single source file).
Therefore the complete public surface is the single symbol `slice`, and no C
module was left untranslated.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libString_Slice.so    | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libString_Slice.so | awk '{print $NF}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`.

The Rust `.so` defines no extra *public API* symbols beyond `slice`; the Rust
`.so` does reference more *undefined* symbols than the C one, but every one of
them is libc / libgcc-unwind runtime support pulled in by the Rust standard
library, not an unresolved library symbol:

| Rust undefined symbol group | origin | libc / runtime? |
|-----------------------------|--------|-----------------|
| `printf`, `puts`, `strlen`, `memcpy`, `memmove`, `memset`, `bcmp`, `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`, `__errno_location`, `gettid` | glibc | yes |
| `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `__tls_get_addr`, `__cxa_thread_atexit_impl`, `__cxa_finalize` | glibc / TLS + atexit | yes |
| `_Unwind_*` | libgcc unwinder (Rust panic/backtrace machinery) | yes (runtime) |
| `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__` | weak, standard ELF/GCC boilerplate — also present in the C `.so` | yes |

The C `.so` requires `printf`, `puts` and `strlen`. `puts` appears because GCC
rewrites the three literal `printf("Error: ...\n")` calls into `puts("Error: ...")`.
That is a byte-for-byte equivalent transformation on stdout, so the Rust side
keeping `printf` for those messages is correct (verified differentially in
`tests/differential.rs`).

**Non-libc undefined symbols in the Rust `.so`: 0.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default (empty feature set):

```
$ grep -n '^\[features\]' translation/Cargo.toml ; echo "exit=$?"
exit=1
```

`--no-default-features` and the default build are therefore the same
configuration; both are exercised (see `run_all_feature_combos.sh`).

## How to reproduce

```
./verify.sh            # builds C + Rust, diffs nm -D, runs the whole suite
                       # over every feature combo × {debug, release}
./mutation_check.sh     # negative control: injects 15 behaviour changes into
                       # src/lib.rs and asserts the suite catches each one
```

`verify.sh` exists because plain `cargo test` is **not** sufficient here:

* `cargo test` does not rebuild a `cdylib` artifact, so the suite would `dlopen`
  a stale `target/*/libString_Slice.so` and pass vacuously. `verify.sh` runs
  `cargo build` first, pins the library with `SLICE_RUST_SO`, and the harness
  additionally refuses to run against a `.so` older than `src/lib.rs`.
* The tests redirect the process-wide stdout file descriptor, so they must run
  single-threaded — otherwise libtest's own progress output lands inside a
  capture. The harness asserts `RUST_TEST_THREADS=1`.
