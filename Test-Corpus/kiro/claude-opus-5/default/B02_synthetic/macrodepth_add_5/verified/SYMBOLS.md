# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on the C shared library and the
Rust `cdylib`, for **every** configuration.

## How the two `.so` files are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/mdcore.c src/mdmain.c)`,
so the CMake build emits no `.so`. `c_src/` must not be modified, so the shared
library is produced from the *same* translation unit that the Rust `cdylib`
covers — `src/mdcore.c` — into a build directory outside `c_src/`:

```sh
# ../cbuild/lib/libmd_<op>_<repeat>.so   (24 configurations)
gcc -O2 -fPIC -shared -DOP=$OP -DREPEAT=$R -Ic_src/src \
    -o cbuild/lib/libmd_${OP}_${R}.so c_src/src/mdcore.c
# ../cbuild/exe/driver_<op>_<repeat>     (whole program, mdcore.c + mdmain.c)
gcc -O2 -DOP=$OP -DREPEAT=$R -Ic_src/src \
    -o cbuild/exe/driver_${OP}_${R} c_src/src/mdcore.c c_src/src/mdmain.c
```

`mdmain.c` is deliberately **not** in the `.so`: its only external definition is
`main`, which is the program entry point, not part of the library surface. It is
covered separately, end-to-end, by `tests/driver_parity.rs` against the C
executables (see `CONFIGS.md` rows C-01..C-24).

Rust side: `cargo build --release --no-default-features --features <op>,repeat_<n>`
→ `target/release/libdriver.so`.

## Symbol table (C `.so`, all 24 configurations — identical in every one)

| # | symbol | `nm` type | ELF type / size | C declaration | exported by Rust `.so`? |
|---|--------|-----------|-----------------|---------------|-------------------------|
| 1 | `op_add`       | `T` | FUNC          | `int op_add(int a, int b)`          | yes — `mdcore.rs` `#[unsafe(no_mangle)] pub extern "C" fn op_add` |
| 2 | `op_sub`       | `T` | FUNC          | `int op_sub(int a, int b)`          | yes — `mdcore.rs` `op_sub` |
| 3 | `op_mul`       | `T` | FUNC          | `int op_mul(int a, int b)`          | yes — `mdcore.rs` `op_mul` |
| 4 | `helper_call`  | `T` | FUNC          | `int helper_call(int a, int b)`     | yes — `mdcore.rs` `helper_call` |
| 5 | `helper_ptr`   | `T` | FUNC          | `int helper_ptr(int a, int b)`      | yes — `mdcore.rs` `helper_ptr` |
| 6 | `use_generated`| `T` | FUNC          | `int use_generated(int n)`          | yes — `mdcore.rs` `use_generated` |
| 7 | `G_OP`         | `D` | OBJECT, 8 B   | `int (*G_OP)(int,int) = OP_FN(OP);` | yes — `pub static G_OP: OpFn` |
| 8 | `G_OP_NAME`    | `D` | OBJECT, 8 B   | `const char *G_OP_NAME = STR(OP);`  | yes — `pub static G_OP_NAME: CStrPtr` (`repr(transparent)` over `*const c_char`) |

`readelf -sW` confirms both objects are `OBJECT GLOBAL DEFAULT`, size 8, in a
writable `.data` section (`nm` type `D`) on **both** sides, so the layout a C
consumer sees is identical.

### Symbols intentionally NOT exported

| C construct | why absent from `nm -D` on the C `.so` | Rust equivalent |
|---|---|---|
| `accum_add` / `accum_sub` / `accum_mul` (`DEFINE_ACCUM(OP)`) | declared `static int` inside the macro body → internal linkage | `mdmacros::accum` (private to the crate, reached through `use_generated`) |
| `STEP_*`, `INIT_*`, `REP0..REP7`, `DISPATCH_REP`, `OP_FN`, `ACCUM_FN`, `STR`, `CAT`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `CHOOSE_REP` | preprocessor macros — no symbol is ever emitted | `mdmacros::{step, INIT, OP_FN, OP_NAME, run_loop, do_loop, accum}` |
| `main` (`mdmain.c`) | not compiled into the `.so` (see above) | `src/main.rs` `fn main`, built as the `driver` binary |

No symbol required a new `#[no_mangle]` wrapper, and no C source file was found
untranslated: `c_src/src/` contains exactly `mdcore.c`, `mdmacros.h` and
`mdmain.c`, which map onto `src/mdcore.rs`, `src/mdmacros.rs` and `src/main.rs`.

## Verification

`./symbol_parity.sh` rebuilds the Rust `cdylib` for each of the 24
`OP × REPEAT` configurations and, for each, asserts:

1. `comm -23 <C symbols> <Rust symbols>` is empty (nothing missing in Rust);
2. every undefined symbol in the Rust `.so` is platform-provided
   (`@GLIBC`, `@GCC_` unwinder, weak loader hooks);
3. `ldd -r` reports no `undefined symbol`.

Result:

```
$ ./symbol_parity.sh
SYMBOL PARITY OK for all 24 configurations
```

Rust-only symbols (`_init`, `_fini`, `__bss_start`, `_edata`, `_end`,
`rust_eh_personality`, …) are filtered out; the required direction is
C ⊆ Rust, and that set difference is **empty**.
