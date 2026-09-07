# SYMBOLS.md — exported-symbol surface

Derived mechanically from `nm -D` on the C shared object built from
`c_src/src/mdcore.c` (the only C translation unit without `main`):

```
gcc -shared -fPIC -DOP=add -DREPEAT=5 -o libcdriver.so c_src/src/mdcore.c
nm -D libcdriver.so
```

and on the Rust `cdylib`:

```
cd translation && cargo build --release   # target/release/libdriver.so
nm -D target/release/libdriver.so
```

## Defined (non-`U`) symbols

| # | symbol | C type | Rust type | in C `.so` | in Rust `.so` | notes |
|---|--------|--------|-----------|------------|---------------|-------|
| 1 | `op_add`        | `T` (func) | `T` | yes | yes | `int op_add(int,int)` |
| 2 | `op_sub`        | `T` (func) | `T` | yes | yes | `int op_sub(int,int)` |
| 3 | `op_mul`        | `T` (func) | `T` | yes | yes | `int op_mul(int,int)` |
| 4 | `helper_call`   | `T` (func) | `T` | yes | yes | `int helper_call(int,int)` |
| 5 | `helper_ptr`    | `T` (func) | `T` | yes | yes | `int helper_ptr(int,int)` |
| 6 | `use_generated` | `T` (func) | `T` | yes | yes | `int use_generated(int)` |
| 7 | `G_OP`          | `D` (data) | `D` | yes | yes | `int (*G_OP)(int,int)` — writable data in both |
| 8 | `G_OP_NAME`     | `D` (data) | `D` | yes | yes | `const char *G_OP_NAME` — writable pointer to r/o literal |

## Symbols intentionally NOT exported

| symbol | why |
|--------|-----|
| `accum_add` / `accum_sub` / `accum_mul` | `DEFINE_ACCUM(op)` emits `static int accum_<op>(int n)`. `static` ⇒ internal linkage ⇒ no dynamic symbol in the C `.so` (confirmed: absent from `nm -D`). The Rust side mirrors this with a private `fn accum`. Reachable only through `use_generated`. |
| `main` | lives in `mdmain.c`, which is compiled into the `driver` executable, not the shared object. |

## Toolchain-injected / weak-undefined symbols (not part of the API)

Present in one or both `.so`s and excluded from parity comparison because they
are emitted by the C runtime / Rust std, not by the translated source:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__` (both), plus Rust-std-only
weak undefined `__cxa_thread_atexit_impl@GLIBC_2.18`, `gettid@GLIBC_2.30`,
`statx@GLIBC_2.28`. All are weak (`w`) or `U`.

## Result

`nm -D` diff of defined symbols, C `.so` vs Rust `.so`: **empty** — 0 symbols
missing from Rust, and no non-libc undefined symbols in the Rust `.so`.
Verified for **all 24** `OP` × `REPEAT` configurations by
`tests/symbol_parity.rs` / `scripts/check_symbols.sh`; the exported symbol set
is configuration-independent in both languages (`G_OP`/`G_OP_NAME` only change
their *initial value*, never their name).
