# SYMBOLS.md — Phase A symbol surface

## Source of truth

The C library is a single translation unit (`c_src/src/lib.c`) with a single
public declaration in `c_src/include/lib.h`:

```c
int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);
```

There are no namespacing/renaming preprocessor macros, no `#ifdef` feature
gates, no additional `.c` files in `CMakeLists.txt`, and no macro-generated
symbol families. Therefore the expected dynamic-symbol surface is exactly one
symbol.

## `nm -D --defined-only` on the C `.so`

Built via:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

Artifact: `c_src/build/libharvest-work-hoSYXc.so` (the CMake project name is
derived from the parent directory name, so the `.so` file name is
environment-dependent; the tests locate it with a glob).

```
00000000000010f9 T wcscat
```

## `nm -D --defined-only` on the Rust `.so`

Built via `cd translation && cargo build --release`.
Artifact: `translation/target/release/libwcscat_lib.so`.

```
0000000000011690 T wcscat
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `wcscat` | `T` (defined text) | YES — `T wcscat` | `#[no_mangle] pub unsafe extern "C" fn wcscat` in `src/lib.rs` |

### Symbols exported by Rust but not by C

None. The Rust `cdylib` exports no extra public symbols (no `rust_eh_personality`,
no `__rust_*` allocator shims appear in `nm -D --defined-only`), so the surface
is an exact one-to-one match.

### Undefined / imported non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only glibc and libgcc-unwind
imports pulled in by the Rust standard library:

- loader / TM stubs (weak): `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__gmon_start__`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `gettid`, `statx`
- libgcc unwinder: `_Unwind_Backtrace`, `_Unwind_Resume`, `_Unwind_Get*`,
  `_Unwind_Set*`
- glibc: `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
  `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`,
  `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
  `posix_memalign`, `pthread_key_{create,delete}`, `pthread_setspecific`,
  `read`, `readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`,
  `write`, `writev`

Every entry is libc / compiler-runtime. There are **0 missing or undefined
non-libc symbols** — in particular nothing from the translated library itself is
left undefined.

(The C `.so` imports only the four weak loader stubs, because it does not link
the Rust standard library. This difference is inherent to the language runtime
and does not affect the exported API surface.)

## Verdict

- Missing symbols: **0**
- Stubbed / `unimplemented!()` symbols: **0** (the single symbol is a full
  translation of the C body, not a stub)
- Untranslated C modules: **0** (`src/lib.c` is the only C source)

Symbol parity: **PASS**.

## `wchar_t` ABI note

The C build target is Linux/x86-64 glibc, where `sizeof(wchar_t) == 4` and
`wchar_t` is **signed** (verified by compiling a probe: prints `4 1`). The Rust
translation maps `wchar_t` to `i32` on non-Windows, which matches. This matters
for the comparison `*ptr != 0` and for values with the high bit set, which the
differential tests exercise explicitly.

## Automated re-check

`./symbol_parity.sh` recomputes the diff from `nm -D` on both objects and fails
if anything in C is missing from Rust. Latest run:

```
=== C defined dynamic symbols (1) ===   wcscat
=== Rust defined dynamic symbols (1) === wcscat
=== MISSING from Rust -- MUST be empty ===   (empty)
RESULT: PASS -- 0 missing symbols
```

Because `wcscat` is *also* a glibc symbol with a different (2-argument)
signature, `tests/phase_d_provenance.rs` additionally proves that neither
`dlsym` resolved to glibc's definition: the two addresses differ, each lies
inside the expected object per `/proc/self/maps`, and both return exactly `22`
for `numElem == 0` (which glibc's `wcscat` cannot do). Without this check the
entire differential suite could have been silently vacuous.
