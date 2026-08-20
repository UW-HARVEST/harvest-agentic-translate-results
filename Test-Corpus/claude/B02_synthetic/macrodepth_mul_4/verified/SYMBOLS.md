# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D` on the C shared library and the Rust `cdylib`.

## How the two `.so` files are produced

`c_src/CMakeLists.txt` builds an **executable** (`driver`) out of two translation
units:

| C file | contents | Rust counterpart |
|--------|----------|------------------|
| `c_src/src/mdcore.c`  | the whole library surface (`op_*`, `helper_*`, `use_generated`, `G_OP`, `G_OP_NAME`, `static accum_<OP>`) | `src/mdcore.rs` (compiled into the `cdylib` `libdriver.so`) |
| `c_src/src/mdmain.c`  | `main` only | `src/main.rs` (the `driver` binary) |
| `c_src/src/mdmacros.h`| preprocessor-only: `OP` / `REPEAT` selection, `STEP_*`, `INIT_*`, `REP0..REP7`, `DISPATCH_REP`, `DEFINE_ACCUM` | `src/mdmacros.rs` |

So the *library* half is exactly `mdcore.c`, and the comparable shared objects are

* C:    `gcc -fPIC -shared -DOP=<op> -DREPEAT=<n> -o libcdriver.so c_src/src/mdcore.c`
* Rust: `cargo build --no-default-features --features <op>,<n>` → `libdriver.so`

Both are produced for all 24 configurations by `scripts/build_artifacts.sh` into
`artifacts/<op>_<repeat>/`. `scripts/symdiff.sh` performs the `nm -D` diff.
The CMake-built executable is also kept (`artifacts/<cfg>/cbin/driver`) and is
diffed against the Rust executable (`artifacts/<cfg>/rbin/driver`).

No C source file was left untranslated: `mdcore.c` → `mdcore.rs`,
`mdmain.c` → `main.rs`, `mdmacros.h` → `mdmacros.rs`. There are 3 C files and 3
matching Rust modules (plus `lib.rs`, the crate root that has no C counterpart).

## Defined dynamic symbols (`nm -D --defined-only`)

Identical in all 24 `(OP, REPEAT)` configurations — neither `OP` nor `REPEAT`
adds or removes an exported symbol.

| # | symbol | C type/section | Rust type/section | present in Rust `.so` |
|---|--------|----------------|-------------------|-----------------------|
| 1 | `op_add`        | `T` (`.text`) `int(int,int)` | `T` `#[no_mangle] extern "C" fn` | yes |
| 2 | `op_sub`        | `T` (`.text`) `int(int,int)` | `T` `#[no_mangle] extern "C" fn` | yes |
| 3 | `op_mul`        | `T` (`.text`) `int(int,int)` | `T` `#[no_mangle] extern "C" fn` | yes |
| 4 | `helper_call`   | `T` (`.text`) `int(int,int)` | `T` `#[no_mangle] extern "C" fn` | yes |
| 5 | `helper_ptr`    | `T` (`.text`) `int(int,int)` | `T` `#[no_mangle] extern "C" fn` | yes |
| 6 | `use_generated` | `T` (`.text`) `int(int)`     | `T` `#[no_mangle] extern "C" fn` | yes |
| 7 | `G_OP`          | `D` `.data`, `OBJECT`, size 8 | `D` `.data`, `OBJECT`, size 8 (`#[link_section = ".data"]`) | yes |
| 8 | `G_OP_NAME`     | `D` `.data`, `OBJECT`, size 8 | `D` `.data`, `OBJECT`, size 8 (`#[link_section = ".data"]`) | yes |

`readelf -sW` agrees on name, `OBJECT`/`FUNC`-vs-`T`/`D` class, `GLOBAL` binding
and, for the two data objects, on size (8) — see `scripts/symdiff.sh`, which
compares the `nm -D` *type letter* as well as the name.

### Deliberately **not** exported (matches C)

| C entity | why it is not a dynamic symbol |
|----------|-------------------------------|
| `accum_<OP>` (from `DEFINE_ACCUM(OP)`) | declared `static` in `mdcore.c` → local symbol. Rust: private `fn accum_op` in `mdcore.rs`. |
| `STEP_*`, `INIT_*`, `REP0..REP7`, `DISPATCH_REP`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `CHOOSE_REP`, `OP_FN`, `ACCUM_FN`, `STR`, `CAT` | preprocessor macros — they have no symbol at all. Rust: `pub(crate)`-level `const fn`s in `mdmacros.rs`, inlined, no `#[no_mangle]`. |
| `main` | lives in `mdmain.c`, which is part of the *executable*, not the library half. The Rust `cdylib` likewise contains no `main`. |

### Section-placement fidelity note

`int (*G_OP)(int,int)` and `const char *G_OP_NAME` are **non-const** C globals, so
gcc emits them into the writable `.data` section. A plain Rust `static` would be
emitted into `.data.rel.ro`, which RELRO turns read-only after relocation — a
consumer that assigns to `G_OP` (legal in C) would then trap. `mdcore.rs`
therefore pins both objects with `#[unsafe(link_section = ".data")]`, and
`tests/differential.rs::b14_g_op_writable_then_call_through` writes through both
libraries' `G_OP` to prove the parity.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only libdriver.so`, after filtering `@GLIBC*`/`@GCC*` imports
and the standard weak ELF hooks (`_ITM_registerTMCloneTable`,
`_ITM_deregisterTMCloneTable`, `__gmon_start__`, `__cxa_finalize`), is **empty**
for every one of the 24 configurations.

For reference, the C `.so` imports `printf@GLIBC_2.2.5` plus the same weak hooks.

## Result

```
$ bash scripts/symdiff.sh
OK   add_0  (8 symbols, exact name+type match, 0 undefined non-libc)
...
OK   sub_7  (8 symbols, exact name+type match, 0 undefined non-libc)
---
SYMBOL PARITY OK FOR ALL CONFIGS
```

- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name, `nm` type letter and (for data) size.
- [x] 0 missing symbols, 0 undefined non-libc symbols, in all 24 configurations.
- [x] No stubs / `unimplemented!()`: all 8 symbols are real translations of
      `mdcore.c`.
