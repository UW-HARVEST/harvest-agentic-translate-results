# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared libraries.

* C: `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## Defined (exported) symbols

`nm -D --defined-only`

| # | symbol | C `.so` | Rust `.so` | signature (from `c_src/include/driver.h` / `c_src/src/driver.c`) |
|---|--------|---------|------------|------------------------------------------------------------------|
| 1 | `driver`    | `T` | `T` | `void driver(int data)` — declared in the public header |
| 2 | `printLine` | `T` | `T` | `void printLine(const char *line)` — not in the header, but non-`static` in `driver.c`, therefore an exported public symbol |

**Symbol diff (C exported − Rust exported): EMPTY.** No symbol needed to be added
and no C source file was left untranslated (`driver.c` is the only C source in
`CMakeLists.txt`, and both of its non-`static` functions are translated and
exported with `#[unsafe(no_mangle)] pub unsafe extern "C"`).

## Undefined (imported) symbols

`nm -D --undefined-only`

C imports: `memset`, `puts`, `strncpy` (+ weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`).

Note: the C compiler rewrites `printf("%s\n", line)` into `puts(line)`, which is
why `printf` does not appear. The Rust build performs the *same* rewrite, so
`puts` is imported by both — the emitted byte stream and stdio buffering
behaviour are therefore identical.

Rust imports: the same libc entry points (`memset`, `puts`, `strncpy`) plus the
Rust runtime's standard libc/unwinder set (`malloc`, `free`, `memcpy`,
`_Unwind_*`, `dl_iterate_phdr`, …).

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.** Every undefined
symbol in the Rust library resolves against glibc / libgcc, exactly as for the C
library; there are no unresolved references to code that was not translated.
