# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Derived mechanically:

```
# C
cd c_src/build && nm -D --defined-only libdriver.so
# Rust
cargo build --no-default-features && nm -D --defined-only target/debug/libdriver.so
```

## C source inventory (every function definition in `c_src/`)

```
$ grep -nE '^[a-z].*\(' c_src/src/driver.c
29: void printLine (const char * line)
37: void printIntLine (int intNumber)
42: void bad()
50: void good()
58: void driver()
```

5 function definitions, 5 exported symbols — no `static` functions, no macro-generated
symbols, no data symbols, no additional translation units (`CMakeLists.txt` compiles
exactly `src/driver.c`). So the whole library was translated; nothing is absent.

## Symbol table

| # | symbol | in C `.so` | in Rust `.so` | Rust definition | status |
|---|--------|-----------|---------------|-----------------|--------|
| 1 | `printLine`    | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine`    | OK |
| 2 | `printIntLine` | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn printIntLine` | OK |
| 3 | `bad`          | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad`          | OK |
| 4 | `good`         | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn good`         | OK |
| 5 | `driver`       | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver`       | OK |

Note: only `driver` is declared in the public header `include/driver.h`; the other four
have external linkage in the C translation unit and are therefore exported by the C
`.so` too, so they are part of the ABI surface a real consumer can call and are
verified as such.

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Missing in Rust: 0. Extra in Rust: 0.** Enforced by the automated test
`tests/differential.rs::symbol_parity_c_vs_rust`, which re-runs this diff at test time.

## Undefined (imported) non-libc symbols in the Rust `.so`

```
$ nm -D --undefined-only c_src/build/libdriver.so      # 6 imports
_ITM_deregisterTMCloneTable  _ITM_registerTMCloneTable  __cxa_finalize
__gmon_start__               printf@GLIBC_2.2.5         puts@GLIBC_2.2.5

$ nm -D --undefined-only target/debug/libdriver.so     # 51 imports
_ITM_*  __gmon_start__  __cxa_finalize  __cxa_thread_atexit_impl  __errno_location
__tls_get_addr  _Unwind_*  abort  bcmp  calloc  close  dl_iterate_phdr  free
fstat64  getcwd  getenv  gettid  lseek64  malloc  memcpy  memmove  memset  mmap64
munmap  open64  posix_memalign  printf  pthread_key_*  pthread_setspecific  read
readlink  realloc  realpath  stat64  statx  strlen  syscall  write  writev
```

Every Rust import is a libc / libgcc-unwind / loader symbol (the extras beyond the
C set are the Rust runtime's allocator, TLS and panic-unwind machinery, all
satisfied by `libc.so.6` / `libgcc_s.so.1`). **0 missing/undefined non-libc
symbols.**

Note: the C compiler rewrote `printf("%s\n", line)` into a `puts(line)` call, so the
C `.so` imports `puts` while the Rust `.so` imports only `printf`. `puts(s)` writes
`s` followed by `'\n'`, i.e. exactly what `printf("%s\n", s)` writes, so the emitted
bytes are identical — confirmed empirically for the whole byte domain
(`CONFIGS.md` rows 2–10, `ERRORS.md` rows 2–8).
