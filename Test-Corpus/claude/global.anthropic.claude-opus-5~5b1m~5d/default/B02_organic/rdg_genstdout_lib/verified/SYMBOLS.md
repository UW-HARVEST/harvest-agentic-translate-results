# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built shared objects:

```
c_src/build/libdriver.so            (C, ground truth)
translation/target/release/libdriver.so  (Rust)
```

## C `.so` — defined (exported) dynamic symbols

```
$ nm -D --defined-only c_src/build/libdriver.so
00000000000011c7 T FIO_createFilename_fromOutDir
0000000000001189 T extractFilename
```

Note: `extractFilename` is **not** declared in `include/lib.h`, but it is a
non-`static` definition in `src/lib.c`, so it *is* part of the exported ABI and
must be exported by the Rust `.so` as well.

## Rust `.so` — defined (exported) dynamic symbols

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011f70 T FIO_createFilename_fromOutDir
0000000000012080 T extractFilename
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `T` | `T` | ✅ present in both, exact name |
| 2 | `extractFilename`               | `T` | `T` | ✅ present in both, exact name |

**Missing from Rust: none.** No `#[no_mangle]` wrapper had to be added and no
C module was left untranslated — `src/lib.c` is the only C translation unit and
both of its external definitions are implemented in `translation/src/lib.rs`.

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
   (empty — symbol diff reaches empty)
```

## Undefined (imported) symbols

The C `.so` imports the following from libc. These are *not* required to be
exported by the Rust `.so`; they only document the libc surface the C relies on,
which the Rust translation must bind to the *same* libc so that the returned
buffer is `free()`-able by the caller.

| libc symbol | used by C for | Rust translation |
|---|---|---|
| `calloc@GLIBC_2.2.5` | allocating the result buffer | bound via `extern "C" { fn calloc(..) }` — same allocator ✅ |
| `exit@GLIBC_2.2.5` | `exit(30)` on allocation failure | bound via `extern "C" { fn exit(..) -> ! }` ✅ |
| `fprintf@GLIBC_2.2.5` | error message to `stderr` | bound via `extern "C" { fn fprintf(..) }` ✅ |
| `stderr@GLIBC_2.2.5` | stream for the error message | bound via `extern "C" { static mut stderr }` ✅ |
| `strerror@GLIBC_2.2.5` | rendering `errno` | bound via `extern "C" { fn strerror(..) }` ✅ |
| `__errno_location@GLIBC_2.2.5` | reading `errno` | bound via `extern "C" { fn __errno_location() }` ✅ |
| `memcpy@GLIBC_2.14` | copying dir + separator + filename | bound via `extern "C" { fn memcpy(..) }` — same implementation ✅ |
| `strlen@GLIBC_2.2.5` | string lengths | bound via `extern "C" { fn strlen(..) }` ✅ |
| `strrchr@GLIBC_2.2.5` | finding the last separator | bound via `extern "C" { fn strrchr(..) }` ✅ |

The three string routines were originally *re-implemented* in Rust. Phase C
proved that wrong: see the divergence log below. They are now bound to glibc,
so the Rust `.so` imports exactly the same libc surface the C `.so` does:

```
$ diff <(nm -D --undefined-only c_src/build/libdriver.so   | awk '{print $2}' | sed 's/@.*//' | sort -u) \
       <(nm -D --undefined-only translation/target/debug/libdriver.so | awk '{print $2}' | sed 's/@.*//' | sort -u)
   (only `>` lines — every libc symbol the C imports is also imported by Rust;
    the extra Rust-side imports are its std runtime: unwinding, allocator, TLS)
```

There are **no undefined non-libc symbols** in the Rust `.so`:

```
$ nm -D --undefined-only translation/target/debug/libdriver.so | awk '{print $2}' | grep -E '^(_ZN|_R|rust_)'
   (empty)
$ ldd -r translation/target/debug/libdriver.so | grep -i undefined
   (empty — fully resolvable at load time)
```

Weak/toolchain symbols in the C `.so` (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`) are emitted by
the C toolchain's CRT glue, not by the library's own source, and are excluded
from the parity requirement.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one. There is exactly one feature combination to
verify (`--no-default-features` and the default build are identical here); this
is confirmed programmatically by `check_feature_combos.sh`.

## Divergences found and fixed (the Rust was changed; the C never was)

| # | symptom | root cause | fix |
|---|---------|------------|-----|
| 1 | `extractFilename(NULL, '/')` and `FIO_createFilename_fromOutDir` with any `NULL` argument: **C died with SIGSEGV (11), Rust died with SIGABRT (6)** | the hand-written Rust `strlen` dereferenced the pointer with `*s.add(len)`. rustc's debug "null pointer dereference occurred" check fired, and a panic crossing an `extern "C"` boundary is a *non-unwinding* panic → `abort()`. The C instead faults inside glibc's `strlen`. | bound `strlen`, `strrchr` and `memcpy` to glibc — the exact routines the C source calls. Now both fault identically, under **every** build profile (the release build masked this bug entirely, which is why the debug profile is also exercised). Caught by `err_09_null_path_extract` / `err_10_null_args_create`. |

## Harness weaknesses found by mutation testing (and fixed)

The differential harness was itself validated by mutating the Rust translation
and confirming the suite fails. One mutant initially **survived**:

* `.wrapping_add(suffixLen)` → `.wrapping_add(0)` in the `calloc` size, i.e. the
  Rust under-allocated by `suffixLen` bytes. The byte comparison read past the
  short allocation and happened to find zeros, so it passed. Fixed by adding
  (a) a `malloc_usable_size()` lower-bound assertion on both returned buffers,
  and (b) heap poisoning with `0xAA` before each call, so bytes beyond a short
  allocation no longer read back as an indistinguishable `0`.

Mutants now killed (each name is the first failing test):

| mutation | detected by |
|---|---|
| `search.add(1)` → `search.add(0)` (no `+1` after separator) | 24 Phase B tests |
| invert the `outDirName[len-1] == separator` branch | 17 Phase B tests |
| drop `suffixLen` from the allocation size | 15 Phase B tests |
| halve `suffixLen` in the allocation size | 15 Phase B tests |
| drop the trailing-NUL `+1` from the allocation size | 17 Phase B tests |
| `calloc` → `malloc` (lose the zero-fill) | 17 Phase B tests |
| `strrchr` → `strchr` (first instead of last separator) | 21 Phase B tests |
| add a defensive `NULL` check to `extractFilename` | `err_09_null_path_extract` |
| `exit(30)` → `exit(31)` | `err_08_alloc_failure_exit_30` |
| drop the `fprintf` diagnostic | `err_08_alloc_failure_exit_30` |

Two mutants survive and are **provably equivalent**, not test gaps:

* `separator as c_int` → `separator as u8 as c_int` (sign- vs zero-extension).
  C7.24.5.5 specifies `strrchr` compares `c` *converted to `char`*, so only the
  low 8 bits are ever significant; both forms are identical by definition. This
  is the same reason `ERRORS.md` row 11 passes.
* Anything inside the `cfg!(windows)` branch — unreachable on this target, in
  both the C (`#if defined(_MSC_VER)...`) and the Rust.
