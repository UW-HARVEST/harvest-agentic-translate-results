# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C:    c_src/build/libdriver.so          (cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON)
Rust: translation/target/release/libdriver.so   (cargo build --release)
```

## C source inventory (`c_src/src/driver.c`)

Every function definition in the single C translation unit, with its linkage:

| C function | linkage | exported? | Rust counterpart |
|---|---|---|---|
| `void printLine(const char *line)` | external | yes | `printLine` (`#[unsafe(no_mangle)] pub unsafe extern "C"`) |
| `static void helperBad(void)` | **internal (`static`)** | no | `helperBad` (private `fn`, `#[allow(dead_code)]`) |
| `void bad(void)` | external | yes | `bad` (`#[unsafe(no_mangle)] pub extern "C"`) |
| `static void helperGood(void)` | **internal (`static`)** | no | `helperGood` (private `fn`) |
| `void good(void)` | external | yes | `good` (`#[unsafe(no_mangle)] pub extern "C"`) |
| `void driver(void)` | external | yes | `driver` (`#[unsafe(no_mangle)] pub extern "C"`) |

There is no other C source file, no header-defined `static inline`, no macro that
generates symbols, and no `#ifdef`-gated code. `include/driver.h` declares only
`void driver(void)`. The remaining three externals (`printLine`, `bad`, `good`)
are not declared in the public header but *are* in the C `.so`'s dynamic symbol
table, so they are part of the verified surface.

## Defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `bad`       | `T` | `T` | MATCH |
| 2 | `driver`    | `T` | `T` | MATCH |
| 3 | `good`      | `T` | `T` | MATCH |
| 4 | `printLine` | `T` | `T` | MATCH |

**C defined symbols: 4. Rust defined symbols: 4. Missing from Rust: 0. Extra in Rust: 0.**

Negative parity (must *not* be exported, because the C marks them `static`):

| symbol | C `.so` | Rust `.so` | status |
|---|---|---|---|
| `helperBad`  | absent | absent | MATCH |
| `helperGood` | absent | absent | MATCH |

## Undefined / imported symbols

The C `.so` imports exactly one non-weak libc symbol:

```
U puts@GLIBC_2.2.5
```

`printf("%s\n", line)` is rewritten by the compiler into `puts(line)`. The Rust
`.so` declares `printf` but LLVM applies the identical transformation, so it too
imports `puts@GLIBC_2.2.5` and never imports `printf`. The Rust `.so`
additionally imports libc/`libgcc` symbols pulled in by `std` (`malloc`, `memcpy`,
`_Unwind_*`, …); all are libc / unwinder symbols, not untranslated library
symbols.

**Non-libc undefined symbols in the Rust `.so`: 0.**

## Completion checklist

- [x] `nm -D` shows 0 missing non-libc symbols in the Rust `.so`.
- [x] `nm -D` shows 0 symbols exported by C but absent in Rust.
- [x] `static` C helpers remain unexported in Rust.
- [x] No C source file was left untranslated (the project has exactly one `.c`).
