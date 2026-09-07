# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Artifacts compared:

- C:    `c_src/build/libdriver.so`        (cmake, `add_library(driver SHARED src/driver.c)`)
- Rust: `translation/target/release/libdriver.so`  (`crate-type = ["cdylib"]`)

Command used (both sides):

```
nm -D --defined-only <lib>
```

## C source surface

`c_src/` contains exactly two source files and one translation unit:

| file | contents |
|------|----------|
| `c_src/include/driver.h` | one declaration: `void driver(int x, int y);` |
| `c_src/src/driver.c`     | one definition: `driver`, calling `div()` + `printf()` |

There are no other `.c` files, no macro-generated symbol families, no global
variables, and no `static` helpers. So the complete public surface is one
symbol. Nothing in `c_src` was left untranslated.

## Dynamic-symbol table (defined / exported)

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|------------|---------------|--------|
| 1 | `driver` | `T` (global text) | yes | yes | MATCH |

Exported-symbol diff (`comm -3` on the sorted name lists): **empty**.

- Symbols in C `.so` but missing from Rust `.so`: **0**
- Extra defined/exported symbols in the Rust `.so`: **0** — `nm -D
  --defined-only` on the Rust `.so` prints exactly one line, `T driver`.

## Undefined (imported) symbols

Both libraries import only libc:

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so` | none — `U div@GLIBC_2.2.5`, `U printf@GLIBC_2.2.5`, plus the weak `__cxa_finalize` / `__gmon_start__` / `_ITM_*` toolchain hooks |
| Rust `.so` | none — `U printf@GLIBC_2.2.5` plus libc (`malloc`, `memcpy`, `write`, `open64`, …), the `_Unwind_*` GCC unwinder, and the same weak toolchain hooks. All resolve from `libc`/`libgcc_s`. |

Note: the C `.so` imports `div` from glibc rather than inlining it, so the
reference behavior is glibc's `div()`, which is a plain `numer / denom` and
`numer % denom` on `int` — i.e. a single `idiv` on x86-64. The Rust translation
emits `cdq; idiv` via inline asm for exactly this reason (see `ERRORS.md` rows
1–2).

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. `--no-default-features` and the default build
produce the identical symbol table (verified by re-running `nm -D` after
`cargo build --release --no-default-features`). There are no feature-gated code
paths to re-verify.

## Binary / driver executable

Neither build produces an executable:

- `c_src/CMakeLists.txt` has only `add_library(driver SHARED ...)`; no
  `add_executable`, and `driver.c` has no `main`.
- `translation/Cargo.toml` has only a `[lib]` target (`cdylib`); there is no
  `[[bin]]` and no `src/main.rs`.

So the "compare the two binaries' stdout" clause is not applicable. stdout is
still compared byte-for-byte, but through the shared libraries: the tests
redirect fd 1 to a temp file around each `driver()` call (see
`tests/differential.rs`).
