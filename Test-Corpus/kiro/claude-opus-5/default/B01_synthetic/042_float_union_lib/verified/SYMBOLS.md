# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C source inventory

The whole library is two files:

| C file | translated in Rust? |
|--------|---------------------|
| `c_src/include/driver.h` (declares `void driver(double f);`) | yes |
| `c_src/src/driver.c` (defines `driver`, plus file-local `raw_double_t` union) | yes — `translation/src/lib.rs` |

`raw_double_t` is a file-local `typedef union` with no linkage, so it produces no
symbol. There is no module/file in `c_src/` that lacks a Rust counterpart, so no
Phase A "translate the missing C source" work was required.

## `nm -D --defined-only c_src/build/libdriver.so`

```
0000000000001109 T driver
```

That is the complete exported surface of the C `.so`; the only other entries in
plain `nm -D` are the toolchain-injected weak symbols and the imported libc
`printf`:

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001109 T driver
                 U printf@GLIBC_2.2.5
```

## `nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000012dc0 T driver
```

## Parity table

| # | symbol | exported by C `.so` | exported by Rust `.so` | status |
|---|--------|---------------------|------------------------|--------|
| 1 | `driver` | T (global text) | T (global text) | MATCH — `#[unsafe(no_mangle)] pub extern "C" fn driver(f: c_double)` |

Weak toolchain symbols (`_ITM_registerTMCloneTable`,
`_ITM_deregisterTMCloneTable`, `__cxa_finalize`, `__gmon_start__`) are emitted by
the linker/CRT, not by the translated source, and are present in both objects as
weak/undefined. They are not part of the API surface.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only translation/target/.../libdriver.so | awk '{print $NF}' | sort -u)
(no output)
```

**Missing symbols: 0.**

## Undefined (imported) symbols in the Rust `.so`

Every `U`/`w` entry in `nm -D -u` on the Rust `.so` resolves to glibc or to the
GCC unwinder that `libstd` links against:

`_Unwind_*@GCC_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `fegetround`, `free`, `fstat64`, `fwrite`, `getcwd`,
`getenv`, `gettid`, `localeconv`, `lseek64`, `malloc`, `memcpy`, `memmove`,
`memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `statx`, `stdout`, `strlen`, `syscall`, `write`,
`writev`.

**Non-libc / non-runtime undefined symbols: 0.** Four of these are deliberate
imports the translation needs in order to behave like the C:

| import | why |
|--------|-----|
| `stdout`, `fwrite` | output goes through the very same glibc `FILE` object that C `printf` would have used, so buffering and interleaving with other C output are identical |
| `localeconv` | glibc's `%a` and `%.4f` use the `LC_NUMERIC` decimal point as the radix character (see `CONFIGS.md` row 34) |
| `fegetround` | glibc's `%.4f` rounds according to the current FP rounding direction (see `CONFIGS.md` row 35) |

`scripts/verify.sh` re-derives this list mechanically and fails if any
undefined symbol outside the allowlist appears.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one (there is nothing for
`--no-default-features --features <combo>` to select). `cargo metadata` confirms
the feature map is empty. Phase D's "repeat B–C for every feature combination"
therefore reduces to the single default combination, which is verified. This is
re-checked mechanically by `scripts/check_features.sh`.

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc / non-runtime symbols in the Rust `.so`.
