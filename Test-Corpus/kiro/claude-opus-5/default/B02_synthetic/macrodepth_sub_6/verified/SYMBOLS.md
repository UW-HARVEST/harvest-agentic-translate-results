# SYMBOLS.md — dynamic symbol parity, C `.so` vs Rust `.so`

## How the two shared libraries are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/mdcore.c src/mdmain.c)`,
so CMake alone never emits a `.so`. The shared library used for differential
testing is built from `src/mdcore.c` with an equivalent PIC compile (this leaves
`c_src/` untouched):

```
gcc -O2 -fPIC -std=c11 -DOP=<op> -DREPEAT=<r> -shared -o libmdcore_<op>_<r>.so c_src/src/mdcore.c
cargo build --release --no-default-features --features <op>,<r>   # -> libdriver.so
```

`src/mdmain.c` is deliberately excluded from the `.so`: it contributes only
`main`, and the Rust `[lib]` target mirrors `mdcore.c` (`mdmain.c` is translated
into the `[[bin]]` target `src/main.rs`). The `main`/driver surface is verified
separately, process-to-process, in `tests/driver_cli.rs`.

Scripts: `build_all.sh` (build every configuration), `check_symbols.sh`
(the parity diff below, over all 24 configurations).

## `nm -D --defined-only` on the C `.so`

Exactly 8 dynamic symbols, identical in every one of the 24 `(OP, REPEAT)`
configurations:

| # | symbol | class | C declaration | present in Rust `.so` |
|---|--------|-------|---------------|-----------------------|
| 1 | `op_add`        | `T` (text) | `int op_add(int a, int b)`           | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn op_add` |
| 2 | `op_sub`        | `T` (text) | `int op_sub(int a, int b)`           | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn op_sub` |
| 3 | `op_mul`        | `T` (text) | `int op_mul(int a, int b)`           | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn op_mul` |
| 4 | `helper_call`   | `T` (text) | `int helper_call(int a, int b)`      | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn helper_call` |
| 5 | `helper_ptr`    | `T` (text) | `int helper_ptr(int a, int b)`       | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn helper_ptr` |
| 6 | `use_generated` | `T` (text) | `int use_generated(int n)`           | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub extern "C" fn use_generated` |
| 7 | `G_OP`          | `D` (data) | `int (*G_OP)(int,int) = OP_FN(OP)`   | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub static mut G_OP` |
| 8 | `G_OP_NAME`     | `D` (data) | `const char *G_OP_NAME = STR(OP)`    | yes — `mdcore.rs`, `#[unsafe(no_mangle)] pub static G_OP_NAME` |

Symbol *class* also matches: 1–6 are `T` in both, 7–8 are `D` in both.

## Symbols intentionally NOT exported

| C construct | why absent from `nm -D` in both |
|-------------|---------------------------------|
| `accum_<OP>` (from `DEFINE_ACCUM(OP)`, `mdcore.c:33`) | declared `static` by the macro; internal in C, so private `fn accum` in Rust |
| `op_add`/`op_sub`/`op_mul` for the *unselected* ops | all three are unconditionally defined in `mdcore.c`, so all three are exported in both — no configuration-dependent gaps |
| every `mdmacros.h` macro (`STR`, `CAT`, `OP_FN`, `STEP_*`, `INIT_*`, `REP0`–`REP7`, `CHOOSE_REP`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `DISPATCH_REP`, `DEFINE_ACCUM`, `ACCUM_FN`) | preprocessor-only; nothing reaches the object file. Translated to `cfg`-selected `const`s and `#[inline]` `fn`s in `mdmacros.rs` |
| `main` (`mdmain.c:28`) | not compiled into the `.so`; lives in the `driver` executable on both sides |

## Undefined symbols

C `.so`: `printf@GLIBC_2.2.5` (`U`), plus the usual weak ELF/glibc hooks
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`).

Rust `.so`: only libc/`GLIBC`-versioned imports (`write`, `memcpy`, pthread
primitives, …) pulled in by the statically linked `std`. **Zero unresolved
non-libc symbols** — checked by `check_symbols.sh`, which greps out
`_ZN`/`_R`-mangled Rust-runtime names and `@GLIBC`/`__`-prefixed libc names and
requires the remainder to be empty.

Note that `printf` is *not* an import of the Rust `.so`: `mdcore.rs` routes the
three `printf` call sites through `stdio::print_str`, which formats with
`format!` and writes the bytes to `std::io::stdout()`. That changes the libc
function used but not the bytes emitted; `tests/stdout_bytes.rs` asserts the
emitted bytes are identical.

## Result

```
$ ./check_symbols.sh
[add_0] symbol parity OK (8 C symbols)
...
[mul_7] symbol parity OK (8 C symbols)
EXIT=0
```

- 0 symbols missing from the Rust `.so`, in all 24 configurations.
- 0 unresolved non-libc symbols in the Rust `.so`, in all 24 configurations.
- Nothing was stubbed: every symbol is backed by a real translation of the
  corresponding `mdcore.c` definition.
