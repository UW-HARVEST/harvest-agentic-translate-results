# SYMBOLS.md — dynamic-symbol parity, C `.so` vs Rust `.so`

## How the two `.so`s are produced

`c_src/CMakeLists.txt` declares `add_executable(driver src/mdcore.c src/mdmain.c)`,
so cmake alone yields no shared object. `mdmain.c` holds `main`; `mdcore.c` is the
library half and is exactly what `translation/src/lib.rs` (`crate-type = ["cdylib"]`)
mirrors. `build_c.sh` therefore compiles the reference `.so` from `mdcore.c` only,
with the same `-DOP=`/`-DREPEAT=` flags cmake would pass, and additionally builds the
cmake-equivalent `driver` executable for each configuration:

```
gcc -O2 -fPIC -shared -DOP=<op> -DREPEAT=<r> -Ic_src/src \
    -o cbuild/libcdriver_<op>_<r>.so c_src/src/mdcore.c
gcc -O2               -DOP=<op> -DREPEAT=<r> -Ic_src/src \
    -o cbuild/exe_<op>_<r>/driver c_src/src/mdcore.c c_src/src/mdmain.c
```

Nothing under `c_src/` is modified; every artifact lands in `cbuild/`.

Rust side: `cargo test` does not build a `cdylib`, so `translation/build_so.sh` runs
`cargo build` for a feature set and stamps a per-configuration copy
(`target/<profile>/libdriver_<op>_<repeat>.so`); the test harness is pointed at it
through `$MD_RUST_SO`. The stamped name exists because
`target/<profile>/libdriver.so` is a single path shared by every feature set, so an
artifact left there by a previous build would otherwise be loaded silently.

## Defined dynamic symbols

`nm -D --defined-only` on the C `.so`. The set is identical for all 24
`OP × REPEAT` configurations (the macro dispatch is entirely preprocessor-level and
leaves no per-configuration symbols behind).

| # | C symbol | `nm` type | C declaration | Rust `.so` | Rust definition |
|---|----------|-----------|---------------|------------|-----------------|
| 1 | `op_add`       | `T` (text) | `int op_add(int a,int b)` — `mdcore.c:28` | present `T` | `mdcore.rs` `#[unsafe(no_mangle)] pub extern "C" fn op_add` |
| 2 | `op_sub`       | `T` (text) | `int op_sub(int a,int b)` — `mdcore.c:29` | present `T` | `mdcore.rs` `op_sub` |
| 3 | `op_mul`       | `T` (text) | `int op_mul(int a,int b)` — `mdcore.c:30` | present `T` | `mdcore.rs` `op_mul` |
| 4 | `G_OP`         | `D` (data) | `int (*G_OP)(int,int) = OP_FN(OP);` — `mdcore.c:36` | present `D` | `mdcore.rs` `pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int` |
| 5 | `G_OP_NAME`    | `D` (data) | `const char *G_OP_NAME = STR(OP);` — `mdcore.c:37` | present `D` | `mdcore.rs` `pub static G_OP_NAME: CStrPtr` (`repr(transparent)` over `*const c_char`) |
| 6 | `helper_call`  | `T` (text) | `int helper_call(int,int)` — `mdcore.c:39` | present `T` | `mdcore.rs` `helper_call` |
| 7 | `helper_ptr`   | `T` (text) | `int helper_ptr(int,int)` — `mdcore.c:47` | present `T` | `mdcore.rs` `helper_ptr` |
| 8 | `use_generated`| `T` (text) | `int use_generated(int)` — `mdcore.c:54` | present `T` | `mdcore.rs` `use_generated` |

**Symbol diff: empty.** 8 exported by C, 8 exported by Rust, same names, same
`T`/`D` classification. Verified by `tests/phase_d_symbols.rs`, which shells out to
`nm -D --defined-only` on both objects and asserts set equality, and by
`sweep_so.sh` across all 24 configurations.

## Symbols deliberately NOT exported

| C entity | why it is not a dynamic symbol | Rust counterpart |
|----------|-------------------------------|------------------|
| `accum_<OP>` | `DEFINE_ACCUM` (`mdmacros.h:95`) emits `static int CAT(accum_, op)(int n)`; internal linkage. At `-O2` gcc inlines it into `use_generated` and emits no symbol at all. | private `fn accum` in `mdcore.rs` |
| `main` | lives in `mdmain.c`, which is the executable half, not the library. | `src/main.rs` (`[[bin]] driver`) |
| `STR`, `CAT`, `OP_FN`, `STEP_add/sub/mul`, `INIT_add/sub/mul`, `REP0`..`REP7`, `CHOOSE_REP`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `DISPATCH_REP`, `DEFINE_ACCUM`, `ACCUM_FN` | preprocessor macros; nothing reaches the object file. | `const`/`fn`/`#[cfg]` items in `mdmacros.rs`, all crate-private to the `cdylib` |
| `atoi` | libc, consumed by `mdmain.c` only. | `src/cstdlib.rs::atoi`, used by the `driver` binary only |
| `printf` | libc import (`U printf@GLIBC_2.2.5`). | `src/stdio.rs` over `std::io::stdout()` |

No symbol in this translation is a stub: every exported symbol runs the translated
body of the corresponding C function.

## Undefined symbols

C `.so` imports: `printf@GLIBC_2.2.5` plus the four weak toolchain hooks
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`).

Rust `.so` imports: the same weak hooks, plus libc (`malloc`, `free`, `memcpy`,
`write`, `__errno_location`, `pthread_*`, …) and the `libgcc` unwinder
(`_Unwind_*@GCC_*`) that `std` needs. **0 undefined non-libc / non-toolchain
symbols** — nothing that would need another translation unit to resolve.

## Verification

`nm -D --defined-only` diff, run directly from the shell by `../check_symbols.sh`
for each of the 24 `OP × REPEAT` configurations:

```
ok  add/0  8 symbols identical
...
ok  mul/7  8 symbols identical
SYMBOL PARITY: empty diff for all 24 configurations
```

Inside the test suite, `tests/phase_d_symbols.rs` asserts the same thing plus:

- `sym_01_defined_symbol_sets_are_identical` — set difference in **both**
  directions is empty, and equals the eight expected names.
- `sym_02_symbol_kinds_match` — the `nm` type letters agree (`D` for the two data
  slots, `T` for the six functions), so a caller reading `G_OP`/`G_OP_NAME` through
  `dlsym` sees the same shape.
- `sym_03_no_unresolved_non_libc_symbols` — `ldd -r` reports no undefined symbol
  for either object, and every remaining import is either version-tagged
  (`@GLIBC_*` / `@GCC_*`) or one of the weak toolchain hooks.
- `sym_04_every_symbol_is_reachable_not_a_stub` — each of the eight symbols is
  `dlsym`'d and driven, and its result compared against the C, so a symbol added
  only to satisfy `nm` would fail.

`../mutation_check.sh` confirms these are not vacuous: renaming the exported
symbol for `use_generated`, `helper_ptr` or `G_OP_NAME` (via `#[export_name]`, so
the crate still compiles) is caught by `sym_01` in every case.

No C source was found to be missing from the translation: `mdcore.c`, `mdmain.c`
and `mdmacros.h` are the entire library, and `src/{mdcore,mdmacros,stdio,cstdlib}.rs`
plus `src/main.rs` cover all three.
