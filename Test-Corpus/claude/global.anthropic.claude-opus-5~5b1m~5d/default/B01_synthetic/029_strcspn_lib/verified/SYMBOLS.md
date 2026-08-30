# SYMBOLS.md — Phase A: Exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The entire C library is three files; there are no untranslated modules.

| C file | contents | translated in Rust? |
|--------|----------|---------------------|
| `c_src/CMakeLists.txt` | build script, `add_library(driver SHARED src/driver.c)` | n/a (build only) |
| `c_src/include/driver.h` | one declaration: `void driver(const char *s1, const char *s2);` | yes |
| `c_src/src/driver.c` | one definition: `driver` = `printf("%zu\n", strcspn(s1, s2))` | yes (`src/lib.rs`) |

There are no other translation units, no static/internal helpers, no macro-generated
symbol families, no global data, and no constructors/destructors in the C library.

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001119 T driver
                 U printf@GLIBC_2.2.5
                 U strcspn@GLIBC_2.2.5
```

## `nm -D` on the Rust `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 U _Unwind_Backtrace@GCC_3.3
                 U _Unwind_GetDataRelBase@GCC_3.0
                 U _Unwind_GetIP@GCC_3.0
                 U _Unwind_GetIPInfo@GCC_4.2.0
                 U _Unwind_GetLanguageSpecificData@GCC_3.0
                 U _Unwind_GetRegionStart@GCC_3.0
                 U _Unwind_GetTextRelBase@GCC_3.0
                 U _Unwind_Resume@GCC_3.0
                 U _Unwind_SetGR@GCC_3.0
                 U _Unwind_SetIP@GCC_3.0
                 w __cxa_finalize@GLIBC_2.2.5
                 w __cxa_thread_atexit_impl@GLIBC_2.18
                 U __errno_location@GLIBC_2.2.5
                 w __gmon_start__
                 U __tls_get_addr@GLIBC_2.3
                 U abort@GLIBC_2.2.5
                 U bcmp@GLIBC_2.2.5
                 U calloc@GLIBC_2.2.5
                 U close@GLIBC_2.2.5
                 U dl_iterate_phdr@GLIBC_2.2.5
00000000000116d0 T driver
                 U free@GLIBC_2.2.5
                 U fstat64@GLIBC_2.33
                 U getcwd@GLIBC_2.2.5
                 U getenv@GLIBC_2.2.5
                 w gettid@GLIBC_2.30
                 U lseek64@GLIBC_2.2.5
                 U malloc@GLIBC_2.2.5
                 U memcpy@GLIBC_2.14
                 U memmove@GLIBC_2.2.5
                 U memset@GLIBC_2.2.5
                 U mmap64@GLIBC_2.2.5
                 U munmap@GLIBC_2.2.5
                 U open64@GLIBC_2.2.5
                 U posix_memalign@GLIBC_2.2.5
                 U printf@GLIBC_2.2.5
                 U pthread_key_create@GLIBC_2.34
                 U pthread_key_delete@GLIBC_2.34
                 U pthread_setspecific@GLIBC_2.34
                 U read@GLIBC_2.2.5
                 U readlink@GLIBC_2.2.5
                 U realloc@GLIBC_2.2.5
                 U realpath@GLIBC_2.3
                 U stat64@GLIBC_2.33
                 w statx@GLIBC_2.28
                 U strlen@GLIBC_2.2.5
                 U syscall@GLIBC_2.2.5
                 U write@GLIBC_2.2.5
                 U writev@GLIBC_2.2.5
```

## Parity table

Only *defined, globally exported* symbols (`nm -D` type `T`/`D`/`B`/`R`) form the ABI
contract. Weak (`w`) entries are toolchain/glibc boilerplate emitted into every ELF
shared object, and `U` entries are imports, not exports.

| # | C symbol | `nm` type in C | present in Rust `.so`? | Rust `nm` type | notes |
|---|----------|----------------|------------------------|----------------|-------|
| 1 | `driver` | `T` (defined, global) | **yes** | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` in `src/lib.rs` |

### Weak toolchain boilerplate (not part of the ABI, listed for completeness)

| C weak symbol | present in Rust `.so`? |
|---------------|------------------------|
| `_ITM_deregisterTMCloneTable` | yes |
| `_ITM_registerTMCloneTable` | yes |
| `__cxa_finalize@GLIBC_2.2.5` | yes |
| `__gmon_start__` | yes |

### C imports (`U`) and how Rust satisfies them

| C import | Rust equivalent |
|----------|-----------------|
| `printf@GLIBC_2.2.5` | imported (`U printf@GLIBC_2.2.5`) — the Rust code calls the *platform* `printf`, so formatting and stdio buffering are identical |
| `strcspn@GLIBC_2.2.5` | imported (`U strcspn@GLIBC_2.2.5`) — bound to the same libc function the C calls. A hand-written Rust reimplementation was tried first and diverged twice (access order, and `SIGABRT` vs `SIGSEGV` in debug builds); see ERRORS.md. Delegating makes the two `.so`s' imports match as well as their behaviour. |

Note: the `nm -D` dump above was taken from the earlier revision that reimplemented
`strcspn`; the current Rust `.so` additionally imports `strcspn@GLIBC_2.2.5`, which is
exactly what the C `.so` imports. The *exported* surface is unchanged (`T driver`), and
`tests/symbol_parity.rs` re-derives both sets at test time rather than trusting this
transcript.

## Missing / undefined non-libc symbols in the Rust `.so`

**None.**

* Missing exports: 0 — every `T` symbol of the C `.so` (`driver`) is exported by the
  Rust `.so` under the exact same name.
* Extra exports in Rust: 0 (only `driver`).
* Undefined (`U`/`w`) symbols in the Rust `.so` that are *not* libc/toolchain runtime:
  0. All `U` entries resolve to glibc (`GLIBC_*` versioned) or to libgcc's unwinder
  (`_Unwind_*@GCC_*`), which is the standard Rust panic-runtime dependency and is
  present on any system that can load a C++/Rust shared object. Verified with
  `ldd -r translation/target/release/libdriver.so` reporting no unresolved symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only build
configuration is the default one. `--no-default-features` and every possible
`--features <combo>` are all the empty set and produce a bit-identical library. The
Phase D "every feature combination" requirement is satisfied by the single default
configuration; `tests/feature_matrix.rs` and `check_features.sh` assert that no
features exist so this claim cannot silently rot if a feature is added later.
