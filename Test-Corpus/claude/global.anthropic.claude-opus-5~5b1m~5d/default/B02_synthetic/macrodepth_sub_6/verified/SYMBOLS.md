# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

C shared object (`mdcore.c` is the only translation unit that produces library
symbols; `mdmain.c` only contains `main`, which is reproduced by the Rust
`[[bin]]` target, not the `cdylib`):

```sh
gcc -O2 -fPIC -shared -DOP=<op> -DREPEAT=<n> -o cbuild/libcdriver_<op>_<n>.so c_src/src/mdcore.c
nm -D --defined-only cbuild/libcdriver_add_5.so
```

Rust shared object:

```sh
cd translation && cargo build --release --no-default-features --features <combo>
nm -D --defined-only target/release/libdriver.so
```

The automated comparison lives in `tests/symbols.rs`
(`symbol_parity_c_vs_rust`), which shells out to `nm -D` on both objects and
asserts the C-exported set is a subset of the Rust-exported set.

## Symbol table (default configuration: `OP=add`, `REPEAT=5`)

| # | C symbol | nm type | kind | in Rust `.so`? | Rust definition |
|---|----------|---------|------|----------------|-----------------|
| 1 | `op_add`        | `T` | function | yes | `mdcore::op_add` (`#[no_mangle] extern "C"`) |
| 2 | `op_sub`        | `T` | function | yes | `mdcore::op_sub` (`#[no_mangle] extern "C"`) |
| 3 | `op_mul`        | `T` | function | yes | `mdcore::op_mul` (`#[no_mangle] extern "C"`) |
| 4 | `helper_call`   | `T` | function | yes | `mdcore::helper_call` (`#[no_mangle] extern "C"`) |
| 5 | `helper_ptr`    | `T` | function | yes | `mdcore::helper_ptr` (`#[no_mangle] extern "C"`) |
| 6 | `use_generated` | `T` | function | yes | `mdcore::use_generated` (`#[no_mangle] extern "C"`) |
| 7 | `G_OP`          | `D` | writable data (`int (*)(int,int)`) | yes | `mdcore::G_OP` (`#[no_mangle] static`) |
| 8 | `G_OP_NAME`     | `D` | writable data (`const char *`)     | yes | `mdcore::G_OP_NAME` (`#[no_mangle] static`, `repr(transparent)` newtype over `*const c_char`) |

Result: **8 / 8 C symbols exported by the Rust `.so`; 0 missing.**

## Deliberately-unexported symbols (present in C, but static there too)

| C entity | why no linker symbol |
|----------|----------------------|
| `accum_add` / `accum_sub` / `accum_mul` (from `DEFINE_ACCUM(OP)`) | `DEFINE_ACCUM` declares the generated function `static`, so it is file-local in C. Confirmed absent from `nm -D` on the C `.so`. Mirrored by the private Rust `fn accum`. |
| `main` (`mdmain.c`) | Not part of the shared library; lives in the `driver` executable / Rust `[[bin]]`. Differentially tested via `tests/driver_bin.rs`. |
| All `mdmacros.h` macros (`STEP_*`, `REP0..REP7`, `DISPATCH_REP`, `OP_FN`, `INIT_FOR`, `STR`, `CAT`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `CHOOSE_REP`, `ACCUM_FN`) | Preprocessor-only; they emit no symbols. Their *effects* are what the differential tests check. |

Note: `DO_LOOP` / `FOR_EACH` are defined in `mdmacros.h` but never used by any
C source, so they contribute no code to either artifact.

## Extra symbols in the Rust `.so`

The Rust `cdylib` additionally exports Rust-runtime symbols
(`rust_eh_personality`, `_ZN*` mangled std items, `__rust_*` allocator shims,
etc.). These are an implementation detail of the Rust runtime and are *not* a
parity violation: parity requires C ⊆ Rust, which holds. The test filters the
comparison to the C symbol set.

## Parity under every feature combination

The symbol *set* is configuration-independent in both languages: `OP`/`REPEAT`
only change the *bodies* of `helper_call` / `helper_ptr` / `use_generated` and
the *values* of `G_OP` / `G_OP_NAME`. `op_add`, `op_sub` and `op_mul` are
always all three compiled, regardless of which one `OP` selects. `tests/symbols.rs`
is therefore valid for every combination and is run for all of them by
`../run_all.sh`.

## Undefined-symbol check

```sh
nm -D -u --format=posix target/debug/libdriver.so
```

Rust `.so` undefined imports: `abort`, `bcmp`, `calloc`, `close`, `lseek64`,
`malloc`, `mmap64`, `munmap`, `realloc`, `realpath`, `stat64`, `statx` plus the
usual `__*` / `_ITM_*` / `_Unwind_*` linker-and-ABI entries — **all** from
`libc.so.6` / `ld-linux`. **0 missing/undefined non-libc symbols.**
(C `.so` for comparison imports `printf`, `__cxa_finalize`, `__gmon_start__`,
`_ITM_*`.)

## Symbol *placement* parity (a real bug found and fixed)

`nm` type letters alone are not enough: `G_OP` and `G_OP_NAME` are **non-`const`**
C globals, so gcc emits them into `.data`, which stays writable at run time.
The original Rust translation used plain `static`s; rustc emits a `static` whose
initializer needs a relocation into `.data.rel.ro`, which full RELRO maps
**read-only** after loading. An external consumer assigning to the exported
`G_OP` therefore worked against the C `.so` but raised `SIGSEGV` against the Rust
`.so` (reproduced by `tests/errors.rs::err_13_*`).

Fixed by declaring both as `static mut`, which rustc places in `.data`:

```sh
readelf -SW <so> | grep -E '\.data '     # section index of .data
readelf -sW <so> | grep -E ' G_OP$| G_OP_NAME$'
```

| object | `G_OP` / `G_OP_NAME` section |
|--------|------------------------------|
| C `.so` (gcc `-O2 -fPIC -shared`) | `.data` |
| Rust `.so` **before** fix | `.data.rel.ro` (read-only ⇒ divergent) |
| Rust `.so` **after** fix (debug and release) | `.data` ✅ |

## Automation

* `../build_c.sh` — builds the C `.so` + executable for all 36 configurations
  (plus a no-`-D` "defaults" pair) into `../cbuild/`; also runs the documented
  CMake build.
* `../symdiff.sh` — `nm -D` C-vs-Rust diff for all 36 combinations.
  Latest run: **TOTAL MISSING SYMBOLS: 0**.
* `../check_combos.sh` — `cargo check` for all 36 valid combinations plus 5
  conflicting ones: **41/41 ok**.
* `../run_all.sh` — the whole Phase B/C/D test suite per combination:
  **36/36 PASS**.
