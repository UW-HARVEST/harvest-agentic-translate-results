# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects. No symbol on this page
was chosen by judgement; the lists are the raw dynamic symbol tables.

Artifacts compared:

- C:    `c_src/build/libhello.so`            (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust: `translation/target/release/libhello.so` (`cargo build --release`, `crate-type = ["cdylib"]`)

Commands used:

```sh
nm -D --defined-only   c_src/build/libhello.so
nm -D --defined-only   translation/target/release/libhello.so
nm -D --undefined-only c_src/build/libhello.so
nm -D --undefined-only translation/target/release/libhello.so
ldd -r <each .so>                      # confirm nothing is left unresolved
```

## 1. Defined (exported) dynamic symbols

The C library is built from exactly one translation unit (`c_src/src/hello.c`,
31 lines) and its public header declares exactly one function
(`c_src/include/hello.h:27` → `int helloworld();`). There are no
namespace-renaming or symbol-generating macros in the header, so the exported
name is the plain identifier.

| # | C symbol | C type | exported by Rust `.so`? | Rust type | status |
|---|----------|--------|-------------------------|-----------|--------|
| 1 | `helloworld` | `T` (global text) | yes | `T` (global text) | MATCH |

Rust side, for completeness — the Rust `.so` exports **no extra** public symbols
beyond the one required (a `cdylib` hides all non-`#[no_mangle]` Rust items):

| # | Rust symbol | type | present in C `.so`? |
|---|-------------|------|---------------------|
| 1 | `helloworld` | `T` | yes |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libhello.so       | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/.../libhello.so   | awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0.** No `#[no_mangle]` wrapper had to be
added and no untranslated C module was found: `src/hello.c` is the only source
file listed in `c_src/CMakeLists.txt`'s `add_library(hello SHARED src/hello.c)`,
and it is fully translated in `translation/src/lib.rs`. Nothing is stubbed and
there is no `unimplemented!()`/`todo!()` anywhere in the crate.

## 2. Undefined (imported) symbols

These are *not* a parity requirement — they are the libc/runtime imports each
toolchain happens to need — but they are recorded because one of them is
behaviourally significant.

C `.so` imports:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U puts@GLIBC_2.2.5
```

Rust `.so` imports: the same four weak toolchain symbols, `U puts@GLIBC_2.2.5`,
plus the Rust `std` runtime's own libc/unwinder imports
(`_Unwind_*@GCC_*` from `libgcc_s`, `malloc`/`free`/`realloc`/`calloc`/
`posix_memalign`, `memcpy`/`memmove`/`memset`/`bcmp`/`strlen`,
`open64`/`read`/`write`/`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`/
`readlink`/`realpath`/`getcwd`, `mmap64`/`munmap`, `abort`, `syscall`,
`getenv`, `__errno_location`, `dl_iterate_phdr`, `gettid`,
`pthread_key_create`/`pthread_key_delete`/`pthread_setspecific`,
`__cxa_thread_atexit_impl`, `__tls_get_addr`).

Every one of those is a **libc / libgcc runtime** symbol. `ldd -r` on both
objects reports no undefined and no missing symbols, so:

> **0 missing/undefined non-libc symbols in the Rust `.so`.**

### Note: `printf` → `puts`

Worth recording because it is the one place the two objects had to agree at the
ABI level. The C source calls `printf("Hello World!\n")`; GCC rewrites a
format-string-free `printf` ending in `\n` into `puts("Hello World!")`, which
is why the C `.so` imports `puts` and not `printf`:

```
0000000000001109 <helloworld>:
    110d: lea 0xeec(%rip),%rax   # -> "Hello World!"
    1117: call 1030 <puts@plt>
    111c: mov $0x0,%eax
    1122: ret
```

`translation/src/lib.rs` deliberately calls the libc `printf` (rather than
`std::io::stdout`) so the bytes land in the *same libc `stdout` FILE stream and
buffer* the C used; LLVM applies the identical `printf`→`puts` rewrite, so the
Rust `.so` also imports `puts`. This matters for observable behaviour, not just
symbol names: it makes stdout buffering and interleaving with other C-side
stdio writes identical, which rows C5–C8 of `CONFIGS.md` verify.

## 3. Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, no `optional`
dependencies, and `grep -rn feature translation/src/` finds no `cfg(feature)`.
There is therefore exactly **one** build configuration; `--no-default-features`
and the default build are the same object. Recorded here so the Phase D
"every feature combination" gate is discharged explicitly rather than skipped.

## 4. Binary / driver executables

Neither project builds one: `c_src/CMakeLists.txt` has no `add_executable`
(only `add_library(hello SHARED ...)`), and `translation/Cargo.toml` has no
`[[bin]]`, no `src/main.rs` and no `src/bin/`. The "compare C and Rust binary
stdout" gate is therefore **N/A** — there is no driver to run. Library stdout is
still compared byte-for-byte at the fd level in Phase B.
