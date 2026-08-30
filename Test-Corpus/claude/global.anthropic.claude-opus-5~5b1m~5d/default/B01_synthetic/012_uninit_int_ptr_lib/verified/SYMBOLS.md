# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D` on the C shared library
`c_src/build/libdriver.so`, compared against the Rust cdylib
`translation/target/release/libdriver.so`.

## Regeneration command

```sh
nm -D --defined-only c_src/build/libdriver.so            | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms    # MUST be empty
```

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C declaration | notes |
|---|--------|---------|------------|---------------|-------|
| 1 | `printIntPtrLine` | T | T | `void printIntPtrLine(const int *intNumber)` | Lowest-level entry point. Not declared in `driver.h`, but has external linkage in `driver.c`, so it IS part of the ABI surface and must be tested directly. |
| 2 | `bad`             | T | T | `void bad(void)`               | CWE-457: reads an uninitialised `int *` and passes it to `printIntPtrLine`. Undefined behaviour by construction. |
| 3 | `good`            | T | T | `void good(void)`              | Initialises `int data = 5`, takes its address, prints `5\n`. |
| 4 | `driver`          | T | T | `void driver(int useGood)`     | The only symbol declared in `driver.h`. Dispatches to `good()` when `useGood` is non-zero, else `bad()`. |

**Missing from Rust `.so`: none.** No `#[no_mangle]` wrapper had to be added and
no C module was left untranslated: `c_src/src/driver.c` is the only translation
unit in `CMakeLists.txt`, and all four of its external functions are exported by
the Rust crate via `#[unsafe(no_mangle)] pub unsafe extern "C"`.

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak, non-libc-startup symbol:
`printf@GLIBC_2.2.5`.

The Rust `.so` imports `printf@GLIBC_2.2.5` as well — the translation
deliberately calls the platform libc `printf` rather than Rust's `println!`, so
stdout buffering, `%d` formatting and flush ordering are byte-identical to C.

The remaining Rust imports (`_Unwind_*`, `malloc`, `free`, `memcpy`,
`dl_iterate_phdr`, `pthread_key_*`, `write`, ...) come from the Rust standard
library / panic-unwind runtime that is statically linked into every cdylib.
They are all libc / libgcc_s symbols, so the "0 missing/undefined non-libc
symbols" gate holds.

## Verdict

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] `comm -23 c.syms r.syms` is empty.
- [x] No extra *public API* symbols are exported by Rust (no leaked Rust
      mangled `_ZN...` symbols in the dynamic table).
- [x] 0 missing / 0 undefined non-libc symbols.
