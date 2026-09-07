# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/src/driver.c`, 37 lines — the whole library)

| C symbol     | linkage  | in `nm -D` of C `.so`? | Rust counterpart                 | exported by Rust `.so`? |
|--------------|----------|------------------------|----------------------------------|-------------------------|
| `driver`     | external | yes (`T driver`)       | `pub extern "C" fn driver`       | yes (`T driver`)        |
| `print_hex`  | `static` | no (internal)          | private `unsafe fn print_hex`    | no (correct — matches C)|

`c_src/include/driver.h` declares exactly one entry point: `void driver(int x);`.
There are no other `.c` files, no macro-generated symbols, no global data,
no constructors/destructors. Nothing in the C tree is untranslated.

## Exported (defined) dynamic symbols

C `.so`:

```
0000000000001173 T driver
```

Rust `.so`:

```
0000000000011730 T driver
```

**Diff (C-exported symbols missing from Rust): EMPTY.**

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

## Undefined (imported) symbols

C `.so` imports: `printf`, `putchar` (the compiler lowers `printf("\n")` to
`putchar`), plus the usual weak CRT symbols
(`_ITM_*TMCloneTable`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports the same `printf`/`putchar` plus glibc/`libgcc` symbols
pulled in by the Rust standard library and its unwinder
(`memcpy`, `malloc`, `free`, `_Unwind_*`, `pthread_key_*`, …).

**0 missing / unresolvable non-libc symbols in the Rust `.so`** — every
undefined symbol is provided by glibc or libgcc_s, both of which are already
dependencies of any process that loads the library. Verified by loading the
Rust `.so` with `dlopen` (via `libloading`) in `tests/differential.rs`, which
fails at load time if any symbol is unresolvable.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the complete set of
feature combinations is the single default (empty) combination. `cargo test`,
`cargo test --no-default-features` and `cargo test --all-features` are therefore
the same build. Verified by `scripts/check_features.sh`.

## Binary / driver executable

Neither build produces an executable: `c_src/CMakeLists.txt` contains only
`add_library(driver SHARED src/driver.c)`, and `translation/Cargo.toml` declares
only `[lib] crate-type = ["cdylib"]` with no `src/main.rs` or `src/bin/`.
The "compare binary stdout" gate is therefore N/A; stdout is instead compared
byte-for-byte through the FFI boundary by capturing fd 1 around each call.

## Completion gate (Phase D) — all verified

- [x] `nm -D` symbol diff C→Rust is **empty**; 0 missing/unresolvable non-libc
      symbols. No module of the C source was left untranslated (the C library is
      one 37-line file), so nothing needed stubbing.
- [x] Phase B: all 19 `CONFIGS.md` rows pass, including the seeded randomized
      rows (≈13 500 differential calls per run).
- [x] Binary/stdout gate: N/A — neither build produces an executable. Stdout is
      compared byte-for-byte through fd 1 around each FFI call instead.
- [x] Phase C: all reachable `ERRORS.md` rows (E1–E7, E9) have passing
      differential tests; E8 is unreachable through the public ABI by
      inspection.
- [x] All of the above hold under **every** configuration:
      `{default, --no-default-features, --all-features} × {dev, release}` = 6
      builds, driven by `scripts/run_all.sh`. 29/29 cases pass in each.

### Anti-vacuity evidence

Passing tests are only meaningful if they can fail. `scripts/mutation_check.sh`
injects seven plausible translation bugs into `src/lib.rs` and requires the
suite to fail for each, then requires it to pass again once restored. All seven
are caught:

| mutation | caught |
|----------|--------|
| `unsigned char` → `i8` promotion (sign extension) | yes |
| length `sizeof(int) - 1` | yes |
| length `sizeof(int) + 1` | yes |
| `x.swap_bytes()` (byte order) | yes |
| `%02x` → `%02X` (case) | yes |
| `%02x` → `%x` (zero padding) | yes |
| dropped trailing `printf("\n")` | yes |

### Harness pitfall found and fixed

`cargo test` does **not** rebuild `cdylib` artifacts — it only rebuilds the test
binaries. An early version of this suite loaded a stale
`target/debug/libdriver.so` and reported 29/29 passing against a deliberately
broken library. `tests/differential.rs` now refuses to run if any file under
`src/` is newer than the loaded `.so` (`assert_rust_so_is_fresh`), and
`scripts/run_all.sh` always runs `cargo build` before `cargo test`.

### Reproducing

```
cd translation && ./scripts/run_all.sh        # all 6 configurations + symbol diff
cd translation && ./scripts/mutation_check.sh # proves the suite is not vacuous
```
