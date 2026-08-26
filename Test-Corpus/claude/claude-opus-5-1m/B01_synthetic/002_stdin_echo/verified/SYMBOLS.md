# SYMBOLS.md — Phase A: public symbol surface

## What the C build produces

`c_src/CMakeLists.txt` declares an **executable**, not a library:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver
    src/main.c)
```

There are no `option()`s, no `add_definitions()`, no `target_compile_definitions()`
and no `if()` blocks, so there is exactly **one** build configuration.
`c_src/src/main.c` is the only translation unit and it defines exactly one
function, `main`.

### `nm -D` on the C executable

```
$ nm -D c_src/build/driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
                 U __libc_start_main@GLIBC_2.34
                 U fgets@GLIBC_2.2.5
                 U fputs@GLIBC_2.2.5
0000000000404030 B stdin@GLIBC_2.2.5
0000000000404020 B stdout@GLIBC_2.2.5
```

Everything there is either a weak toolchain stub (`w`), an *imported* libc symbol
(`U`), or a libc copy relocation (`stdin`/`stdout`, which are glibc's objects, not
the program's). A non-`-rdynamic` executable exports none of its own functions,
so the number of **user-defined symbols exported by the C build is zero**.

### `nm -D` on the same C source built as a shared object

To get a loadable comparison target, the *same, unmodified* source is also
compiled as a shared object (nothing in `c_src/` is edited; the file is only
read):

```
$ gcc -shared -fPIC -o libcdriver.so c_src/src/main.c
$ nm -D --defined-only libcdriver.so | grep -vE ' [wWvV] '
0000000000001119 T main
```

So the complete public symbol surface of this library is the single symbol
**`main`**.

## Symbol parity table

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `main` | `T` (defined) | `T` (defined) | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

```
$ nm -D --defined-only target/debug/libdriver.so | grep -w main
00000000000182d0 T main
```

Nothing had to be stubbed and no C source was left untranslated: `main` is the
whole library, and its body is translated in `src/echo.rs` (`echo::run()`), which
both the `cdylib` (`src/lib.rs`) and the `driver` binary (`src/main.rs`) use.

- Missing symbols in the Rust `.so`: **0**
- Undefined non-libc symbols in the Rust `.so`: **0**
  (`nm -D --undefined-only target/debug/libdriver.so` lists only `GLIBC_*`
  imports.)

## Rust target layout

| target | kind | artifact | purpose |
|--------|------|----------|---------|
| `driver` | `bin` | `target/debug/driver` | the translated program, the counterpart of the C `add_executable(driver ...)` |
| `driver` | `cdylib` | `target/debug/libdriver.so` | exports `main` so the translation can be dlopen()ed and diffed against the C `.so` through the FFI boundary (`tests/differential_so.rs`) |

## Feature combinations

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` has no
build options, so the set of valid feature combinations is the single empty
combination. It is still verified explicitly:

```
cargo check --no-default-features            # no features
cargo check --all-features                   # identical (there are none)
cargo build --release                        # panic = "abort" profile
```

See `CONFIGS.md` for the *runtime* configuration surface, which is where all of
this program's variability actually lives.

## Completion gate (Phase D)

Reproduce with `./scripts/verify_all.sh`.

- [x] **`SYMBOLS.md`**: `nm -D` shows **0** missing symbols in the Rust `.so`
      (the C `.so` exports exactly `{main}`; the Rust `.so` exports it too, with
      0 extras) and **0** unresolved/undefined non-libc symbols — checked both by
      `ldd -r` and by a subset check of all 51 imports against the export tables
      of `libc.so.6`, `libgcc_s.so.1` and `ld-linux-x86-64.so.2`.
      Tests: `c_so_exports_only_main`, `rust_so_exports_every_c_symbol`,
      `rust_so_has_no_unresolved_symbols`, `c_executable_exports_no_user_symbols`.
- [x] **Phase B**: all 35 `CONFIGS.md` rows pass, with randomized inputs
      (fixed seed) on the rows that take them — 46 tests in
      `tests/differential_cli.rs` plus 173 input shapes compared through
      `dlopen` in `tests/differential_so.rs`.
- [x] **Phase C**: all 15 `ERRORS.md` rows have a passing differential test that
      asserts the *same* outcome, not merely "both failed": identical exit code
      for rows 1–11 and 13–15, and identical *kill signal* (`SIGPIPE`, 13) for
      row 12.
- [x] **Every configuration**: `Cargo.toml` declares no `[features]` and
      `c_src/CMakeLists.txt` no options, so there is one feature combination;
      it is verified explicitly with `--no-default-features` and
      `--all-features`, and the whole Phase B + C + D suite is additionally
      repeated under the `release` profile (`panic = "abort"` + optimisation),
      which is the other genuinely distinct build configuration.

Totals: **52 tests, 0 failures**, stable across three consecutive runs.
