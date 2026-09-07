# SYMBOLS.md — exported-symbol surface (Phase A / Phase D)

Derived mechanically from `nm -D` on the C shared library built from the
UNMODIFIED `c_src/src/mdcore.c` (`./build_c.sh`, which compiles the library for
every `-DOP=<op> -DREPEAT=<n>` configuration into `cbuild/`), compared against
the Rust `cdylib` (`./build_rust.sh` → `rustbuild/`).

C `.so` (`gcc -DOP=add -DREPEAT=5 -fPIC -shared -O2 c_src/src/mdcore.c`):

```
$ nm -D --defined-only cbuild/libcdriver_add_5.so
0000000000004020 D G_OP
0000000000004018 D G_OP_NAME
0000000000001150 T helper_call
0000000000001180 T helper_ptr
0000000000001120 T op_add
0000000000001140 T op_mul
0000000000001130 T op_sub
00000000000011a0 T use_generated
```

Rust `cdylib` (`cargo build --release --no-default-features --features add,5`):

```
$ nm -D --defined-only rustbuild/libdriver_add_5.so | grep -v ' [tbrd] '
000000000004f7a8 D G_OP
000000000004f7b0 D G_OP_NAME
00000000000124a0 T helper_call
0000000000012530 T helper_ptr
00000000000125b0 T op_add
00000000000125c0 T op_mul
00000000000125d0 T op_sub
00000000000125e0 T use_generated
```

## Symbol table

| # | symbol | nm type (C / Rust) | C source | Rust source | present in Rust `.so` |
|---|--------|--------------------|----------|-------------|-----------------------|
| 1 | `op_add`        | `T` / `T` | `mdcore.c:28` | `mdcore.rs` `#[no_mangle] extern "C" fn op_add`       | yes |
| 2 | `op_sub`        | `T` / `T` | `mdcore.c:29` | `mdcore.rs` `#[no_mangle] extern "C" fn op_sub`       | yes |
| 3 | `op_mul`        | `T` / `T` | `mdcore.c:30` | `mdcore.rs` `#[no_mangle] extern "C" fn op_mul`       | yes |
| 4 | `G_OP`          | `D` / `D` | `mdcore.c:36` | `mdcore.rs` `#[no_mangle] static mut G_OP`            | yes |
| 5 | `G_OP_NAME`     | `D` / `D` | `mdcore.c:37` | `mdcore.rs` `#[no_mangle] static mut G_OP_NAME`       | yes |
| 6 | `helper_call`   | `T` / `T` | `mdcore.c:39` | `mdcore.rs` `#[no_mangle] extern "C" fn helper_call`  | yes |
| 7 | `helper_ptr`    | `T` / `T` | `mdcore.c:47` | `mdcore.rs` `#[no_mangle] extern "C" fn helper_ptr`   | yes |
| 8 | `use_generated` | `T` / `T` | `mdcore.c:54` | `mdcore.rs` `#[no_mangle] extern "C" fn use_generated`| yes |

### Deliberately NOT exported (matches C)

| C entity | why not exported |
|----------|------------------|
| `accum_add` / `accum_sub` / `accum_mul` (`DEFINE_ACCUM(OP)`, `mdmacros.h:95`) | `static int accum_<OP>(int)` — internal linkage in C, absent from `nm -D`. Translated as the private `fn accum` in `mdcore.rs`. |
| `main` (`mdmain.c:28`) | Lives in `mdmain.c`, which is linked into the `driver` *executable*, not the shared library. Translated as `src/main.rs` and shipped as the `driver` bin target. |
| `STR/CAT/OP_FN/STEP_*/INIT_*/REP0..REP7/CHOOSE_REP/FOR_EACH/DO_LOOP/RUN_LOOP/DISPATCH_REP/DEFINE_ACCUM/ACCUM_FN` (`mdmacros.h`) | Preprocessor macros — no symbols. Translated into `mdconfig.rs` (`step`, `op_fn`, `INIT`, `REPEAT`, `run_loop`, `dispatch_rep`) + Cargo features. |
| `DO_LOOP` / `FOR_EACH` | Defined in `mdmacros.h` but never expanded by any translation unit; produce no symbol and no code. Represented by `mdconfig::run_loop`'s loop form. |

### Fixes made during Phase A/D

* `G_OP` and `G_OP_NAME` were `pub static` in Rust, which placed them in
  `.data.rel.ro`; full RELRO makes that page read-only at load time, so a store
  through the exported symbol (legal in C, both globals are mutable objects)
  would have faulted. Changed to `#[no_mangle] pub static mut`, which puts them
  in `.data` exactly like the C objects (verified with `readelf -S`/`readelf -s`).

### Verification

`./symdiff.sh` compares `nm -D --defined-only` for the C and Rust `.so` for all
24 `(OP, REPEAT)` configurations and also lists any non-libc undefined symbol in
the Rust `.so`:

```
$ ./symdiff.sh
symbol parity OK for all 24 configs (0 missing, 0 unexpected undefined)
```

Object sizes/types also agree (`readelf -s`): `G_OP` and `G_OP_NAME` are both
8-byte `OBJECT GLOBAL` in `.data` in both libraries.
