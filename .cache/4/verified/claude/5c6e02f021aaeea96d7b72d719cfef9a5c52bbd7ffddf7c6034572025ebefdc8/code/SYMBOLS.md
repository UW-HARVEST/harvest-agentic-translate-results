# SYMBOLS.md — dynamic symbol surface (Phase A / Phase D)

## How the two shared objects are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c)`, so a
shared library is produced from the *unmodified* C source (nothing under
`c_src/` is edited) with the same default flags CMake uses (no `-O`):

```sh
gcc -shared -fPIC -o c_build/libc_driver.so c_src/src/main.c
```

The Rust crate declares `crate-type = ["cdylib", "rlib"]`, producing
`target/<profile>/libdriver.so`. All `#[no_mangle] extern "C"` wrappers live in
`src/lib.rs`; the translation itself lives in `src/translated.rs` and is shared
verbatim with the `driver` binary (pulled in via `#[path]`, so the binary never
links the library's `main` wrapper and no duplicate `main` symbol is emitted).

## Defined (exported) symbols

`nm -D --defined-only` on both objects:

| # | C symbol (`libc_driver.so`) | type | Rust symbol (`libdriver.so`) | type | status |
|---|-----------------------------|------|------------------------------|------|--------|
| 1 | `bad`                       | `T`  | `bad`                        | `T`  | ✅ present |
| 2 | `good`                      | `T`  | `good`                       | `T`  | ✅ present |
| 3 | `main`                      | `T`  | `main`                       | `T`  | ✅ present |
| 4 | `printIntLine`              | `T`  | `printIntLine`               | `T`  | ✅ present |
| 5 | `printLine`                 | `T`  | `printLine`                  | `T`  | ✅ present |

C `.so` exports 5 symbols; Rust `.so` exports the same 5 names.
**Missing from Rust: 0.** No stubs, no `unimplemented!()` — every symbol is
backed by the translated implementation of the corresponding C function.

`c_src/src/main.c` is the *only* C translation unit (`c_src/` contains just
`CMakeLists.txt` and `src/main.c`), and all five of its functions
(`printLine`, `printIntLine`, `bad`, `good`, `main`) are translated, so there is
no un-translated C module.

## Undefined / imported symbols

| object | undefined symbols |
|--------|-------------------|
| `libc_driver.so` | `__isoc99_scanf@GLIBC_2.7`, `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5` (+ weak `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`) |
| `libdriver.so`   | libc/libgcc imports only (`@GLIBC_*` / `@GCC_*`) plus the same three *weak* toolchain symbols `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__` |

**Non-libc undefined symbols in the Rust `.so`: 0.** The three remaining
weak entries are the standard GCC/glibc transactional-memory and profiling
hooks that are intentionally left undefined in *both* objects (they are `w`,
not `U`), exactly as in the C `.so`.

Note: GCC rewrites `printf("%s\n", line)` inside `printLine` into `puts(line)`.
That is a byte-for-byte equivalent transformation (string followed by one
newline), which is what the Rust `printLine` wrapper emits.

## Verification command

`tests/symbols.rs::c_and_rust_shared_objects_export_identical_symbol_sets`
re-derives both symbol lists with `nm -D --defined-only` at test time and
asserts the C set is a subset of the Rust set (and, for this library, equal),
so the check cannot silently rot.
