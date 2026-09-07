# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only  c_src/build/libStaticAlias.so
nm -D --defined-only  translation/target/release/libStaticAlias.so
nm -D --undefined-only <both>
```

## C source inventory (completeness check)

The whole C library is two files:

| C file | functions defined | translated? |
|--------|-------------------|-------------|
| `c_src/src/staticalias.c` | `static_alias`, `driver` | yes — `translation/src/lib.rs` |
| `c_src/include/staticalias.h` | declarations only (`static_alias`, `driver`) | n/a |

`c_src/CMakeLists.txt` declares exactly one target, `add_library(StaticAlias SHARED src/staticalias.c)`.
There is **no `add_executable`**, so the project builds **no driver binary** — the
"compare binary stdout" clause of the completion gate is not applicable. (The
function *named* `driver` is a library export, exercised in Phase B via `dlopen`.)

No C source file was skipped by the translation; there is no missing module.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `static_alias` | `T` | `T` | present in both |
| 2 | `driver`       | `T` | `T` | present in both |

Symbol diff (`comm -23` of the two sorted defined-symbol lists): **empty**.
No symbol required a new `#[no_mangle]` wrapper and no C module needed
translating. Nothing is stubbed: both Rust exports contain the real translated
logic (`translation/src/lib.rs`).

Macro-generated symbols: the C source uses no symbol-generating macros, so
there are no extra macro-expanded exports to match.

## Undefined (imported) symbols

C imports: `printf@GLIBC_2.2.5` plus the standard weak CRT hooks
(`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`).

Rust imports the same `printf@GLIBC_2.2.5` — the translation deliberately calls
the platform `printf` rather than Rust's buffered `stdout`, so `driver`'s output
bytes and flush behaviour are produced by the identical glibc code path — plus
the Rust runtime's own libc/`libgcc` dependencies (`malloc`, `memcpy`, `write`,
`_Unwind_*`, `pthread_key_*`, …).

**Gate:** every Rust undefined symbol is a libc / libgcc-unwind runtime symbol.
`0` missing or undefined **non-libc** symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table** and no optional
dependencies, therefore exactly one build configuration exists
(`--no-default-features` and the default build are the same artifact). This is
verified mechanically by `check_features.sh`; Phases B–C are complete after one
pass.
