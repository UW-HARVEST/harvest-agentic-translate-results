# SYMBOLS.md — exported-symbol parity

## Build shape of the C reference (measured, not assumed)

`c_src/CMakeLists.txt` contains exactly:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

`add_executable` — **not** `add_library(... SHARED)`. There is therefore **no C
`.so` to load**, and no library API surface. Measured:

```
$ file c_src/build/driver
ELF 64-bit LSB executable, x86-64, dynamically linked, not stripped

$ nm -D c_src/build/driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
                 U __libc_start_main@GLIBC_2.34
                 U printf@GLIBC_2.2.5
```

**The C binary exports ZERO defined dynamic symbols.** Every line above is
either an *undefined* import (`U`, resolved from libc) or a *weak* unused stub
(`w`). It is a non-PIE executable with no dynamic symbol table entries of its
own.

## Symbol-parity table

| # | C symbol | `nm -D` class | Rust must export? | Status |
|---|----------|---------------|-------------------|--------|
| 1 | `printf` | `U` (libc import) | No — libc, not part of the translated surface | n/a |
| 2 | `__isoc99_scanf` | `U` (libc import) | No — libc | n/a |
| 3 | `__libc_start_main` | `U` (libc import) | No — C runtime | n/a |
| 4 | `_ITM_deregisterTMCloneTable` | `w` (weak, unused) | No — toolchain stub | n/a |
| 5 | `_ITM_registerTMCloneTable` | `w` (weak, unused) | No — toolchain stub | n/a |
| 6 | `__gmon_start__` | `w` (weak, unused) | No — toolchain stub | n/a |

**Missing/undefined non-libc symbols in Rust: 0** — the required set is empty,
because the C reference defines and exports no dynamic symbols.

For completeness, the C *static* (non-dynamic) text symbols are `main`,
`printIntPtrLine`, `bad`, `good`. All four are translated in `src/main.rs`
(`main`, `print_int_ptr_line`, `bad`, `good`). They are internal to the program
in both languages: `nm -D` shows none of them are dynamically exported by the C
build, so an external caller cannot reach them in either implementation, and
adding `#[no_mangle]` wrappers to Rust would *create* an API the C reference
does not have.

## Why `libloading` is not used to load symbols

`libloading` is added to `[dev-dependencies]` as instructed, but there is no
`.so` for it to open on **either** side, and no exported symbol for it to
resolve. Loading the C executable as a library and calling `printIntPtrLine`
would not be a valid differential test regardless, because `bad()`'s observable
value comes from a *stale stack slot* whose contents depend on the frame layout
left behind by `main`'s `scanf` call (see `CONFIGS.md`); invoking these
functions out of process context would measure the harness, not the translation.

The **complete** observable surface of this program is therefore the process
boundary:

```
stdin bytes  ->  (stdout bytes, stderr bytes, exit status)
```

`tests/differential.rs` exercises exactly that surface, running the real C
binary and the real Rust binary as external processes — which, for an
executable, is precisely "calling it as an external caller would."
