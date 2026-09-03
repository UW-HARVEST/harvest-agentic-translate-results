# SYMBOLS.md — Public symbol surface (C `.so` vs Rust `.so`)

## How the artifacts are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/mdcore.c src/mdmain.c)`,
so the upstream build produces **no** shared library. The library half of the
program is `src/mdcore.c` (`src/mdmain.c` only contains `main`), so the C `.so`
used for differential testing is that translation unit compiled standalone:

```sh
gcc -O2 -fPIC -shared -DOP=<op> -DREPEAT=<n> -o libcmd_<op>_<n>.so c_src/src/mdcore.c
```

The Rust counterpart is the `cdylib` `libdriver.so`:

```sh
cargo build --no-default-features --features <op>,<n>
```

`c_src` is never modified; the `.so` is produced out-of-tree.

## `nm -D --defined-only` on the C `.so` (OP=add, REPEAT=5)

| # | symbol | type | C declaration | Rust export site | present in Rust `.so` |
|---|--------|------|---------------|------------------|-----------------------|
| 1 | `op_add`       | `T` (text)   | `int op_add(int,int)`            | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn op_add`       | yes |
| 2 | `op_sub`       | `T` (text)   | `int op_sub(int,int)`            | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn op_sub`       | yes |
| 3 | `op_mul`       | `T` (text)   | `int op_mul(int,int)`            | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn op_mul`       | yes |
| 4 | `helper_call`  | `T` (text)   | `int helper_call(int,int)`       | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn helper_call`  | yes |
| 5 | `helper_ptr`   | `T` (text)   | `int helper_ptr(int,int)`        | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn helper_ptr`   | yes |
| 6 | `use_generated`| `T` (text)   | `int use_generated(int)`         | `mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn use_generated`| yes |
| 7 | `G_OP`         | `D` (data)   | `int (*G_OP)(int,int)`           | `mdcore.rs` `#[unsafe(no_mangle)] static G_OP: extern "C" fn(c_int,c_int)->c_int` | yes |
| 8 | `G_OP_NAME`    | `D` (data)   | `const char *G_OP_NAME`          | `mdcore.rs` `#[unsafe(no_mangle)] static G_OP_NAME: CStrPtr`  | yes |

Both `G_OP` and `G_OP_NAME` are pointer-sized mutable-data objects in C
(`D`, i.e. initialised `.data`). The Rust side exports them as pointer-sized
`static`s in `.data`, so a consumer that does `dlsym("G_OP")` and dereferences
the slot as `int(*)(int,int)` observes the identical representation.

## Symbols deliberately NOT exported

| C entity | why it is not a dynamic symbol |
|----------|-------------------------------|
| `accum_<OP>` (from `DEFINE_ACCUM(OP)`) | declared `static int` in `mdcore.c`; file-local (`t` in `nm`, absent from `nm -D`). Mirrored by the private `mdcore::accum` in Rust. |
| `main` | lives in `mdmain.c`, which is *not* part of the library translation unit. Mirrored by `src/main.rs` (the `driver` binary) and verified by whole-program differential tests. |
| all `mdmacros.h` macros (`STEP_*`, `REP0..REP7`, `DISPATCH_REP`, `OP_FN`, `INIT_FOR`, `RUN_LOOP`, `STR`, `CAT`, …) | preprocessor-only; they emit no symbols. Their expansion is resolved at build time in `src/mdconfig.rs` from Cargo features. |

## Symbol diff

Verified mechanically by `scripts/diff_symbols.sh`, which runs
`nm -D --defined-only` over both objects for **all 24** canonical
`(OP, REPEAT)` configurations and diffs the sorted name lists.

Result: **the diff is empty for every configuration** — 0 symbols exported by
the C `.so` are missing from the Rust `.so`, and the Rust `.so` exports no
extra non-libc/non-runtime symbols of its own. No stubs, no
`unimplemented!()`: every symbol is a real translation of the corresponding C
definition.

Undefined (`U`) symbols in the Rust `.so` are limited to libc/`ld.so`
imports (`memcpy`, `write`, `__errno_location`, `pthread_*`, …), i.e. exactly
the class of imports the C `.so` also has (`puts`/`printf`).

## Note on `src/mdmacros.rs`

This file was found **orphaned**: no `mod mdmacros;` declaration existed in
either `src/lib.rs` or `src/main.rs`, so it was never compiled, and its
feature-resolution cascade *contradicted* the live `src/mdconfig.rs`
(`mul > sub > add` and highest-`REPEAT`-wins, versus `add > sub > mul` and
lowest-`REPEAT`-wins). It emitted no symbols either way, but it was a latent
trap. It is now aligned with `mdconfig.rs` and declared in `lib.rs`, and a unit
test (`mdmacros::tests::agrees_with_mdconfig`) fails the build if the two ever
drift again. This adds no dynamic symbols, so symbol parity is unaffected.
