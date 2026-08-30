# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on the C `.so` and the Rust `cdylib`.

## How the artifacts are produced

The C project (`c_src/CMakeLists.txt`) only declares an `add_executable(driver ...)`
target, so there is no `.so` target to reuse. `build_c_so.sh` (repo root) compiles
the *same* translation unit that carries the library surface — `c_src/src/mdcore.c` —
with the *same* flags CMake would use (`-DOP=${OP} -DREPEAT=${REPEAT}`) plus
`-fPIC -shared`, producing `cbuild/libcdriver_<op>_<repeat>.so` for all 24
`(OP, REPEAT)` configurations. `c_src/` itself is never modified.
`mdmain.c` is *not* part of the `.so` (it only holds `main`); it is additionally
linked into `cbuild/cdriver_<op>_<repeat>` for the end-to-end stdout comparison.

Rust side: `cargo build --release` → `translation/target/release/libdriver.so`.

Regenerate + diff with:

```sh
./build_c_so.sh
cd translation && cargo build --release
diff <(nm -D --defined-only ../cbuild/libcdriver_add_5.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/release/libdriver.so   | awk '{print $3}' | sort)
```

`translation/tests/symbols.rs::symbol_parity_all_configs` performs exactly this
diff, for every one of the 24 configurations, as a test.

## The complete C dynamic-symbol surface

`nm -D --defined-only cbuild/libcdriver_add_5.so` (identical symbol *set* for all
24 configurations — the macro selection changes the *bodies* and the initialisers
of the globals, never the names):

| # | symbol | `nm` type | C declaration | exported by Rust `.so` | Rust definition |
|---|--------|-----------|---------------|------------------------|-----------------|
| 1 | `op_add`        | `T` (text, `.text`) | `int op_add(int a, int b)` | yes | `mdcore.rs::op_add` |
| 2 | `op_sub`        | `T` (text, `.text`) | `int op_sub(int a, int b)` | yes | `mdcore.rs::op_sub` |
| 3 | `op_mul`        | `T` (text, `.text`) | `int op_mul(int a, int b)` | yes | `mdcore.rs::op_mul` |
| 4 | `helper_call`   | `T` (text, `.text`) | `int helper_call(int a, int b)` | yes | `mdcore.rs::helper_call` |
| 5 | `helper_ptr`    | `T` (text, `.text`) | `int helper_ptr(int a, int b)` | yes | `mdcore.rs::helper_ptr` |
| 6 | `use_generated` | `T` (text, `.text`) | `int use_generated(int n)` | yes | `mdcore.rs::use_generated` |
| 7 | `G_OP`          | `D` (data, `.data`, 8 B) | `int (*G_OP)(int,int) = OP_FN(OP);` | yes | `mdcore.rs::G_OP` (`static mut OpFn`) |
| 8 | `G_OP_NAME`     | `D` (data, `.data`, 8 B) | `const char *G_OP_NAME = STR(OP);` | yes | `mdcore.rs::G_OP_NAME` (`static mut *const c_char`) |

### Section placement matters for the two data symbols

Name parity alone is not enough for `G_OP` / `G_OP_NAME`. In C both are *mutable*
objects (only `G_OP_NAME`'s pointee is `const`), so they land in the writable
`.data` section and a consumer that `dlopen`s the library may legally store
through the `dlsym` address. An immutable Rust `static` holding a relocated
function pointer is instead emitted into `.data.rel.ro`, which RELRO makes
read-only after loading — such a store would `SIGSEGV` where the C library
succeeds. Both are therefore `static mut`, and `readelf -SW` confirms the fix:

```
C    : G_OP, G_OP_NAME -> section 23 = .data          PROGBITS ... WA
Rust : G_OP, G_OP_NAME -> section 27 = .data          PROGBITS ... WA   (was .data.rel.ro)
```

`ERRORS.md` rows 15–17 / `tests/globals.rs` assert the store succeeds in both.

**Symbol diff: EMPTY.** 8 C symbols, 8 Rust symbols, exact name match, in all 24
configurations, in both the dev and release profiles. No symbol required a new
translation — `mdcore.c` and `mdmacros.h` were both already translated
(`src/mdcore.rs`, `src/mdmacros.rs`), and `mdmain.c` is translated as
`src/main.rs`. Nothing is stubbed (`grep -rE 'unimplemented!|todo!' src/` is
empty).

## Symbols deliberately NOT exported

| C entity | why absent from `nm -D` in **both** C and Rust |
|----------|-----------------------------------------------|
| `accum_<OP>` (`DEFINE_ACCUM`) | `DEFINE_ACCUM` expands to `static int accum_<op>(int n)`. `static` ⇒ internal linkage ⇒ no dynamic symbol. In C it appears only as a local symbol (often inlined away at `-O2`); the Rust counterpart is the private `fn accum`. It is reachable only through `use_generated`, which is how the tests drive it. |
| `main` | lives in `mdmain.c`, which is not part of the `.so`. |
| `STR`/`CAT`/`OP_FN`/`STEP_*`/`INIT_*`/`REP0..7`/`CHOOSE_REP`/`FOR_EACH`/`DO_LOOP`/`RUN_LOOP`/`DISPATCH_REP`/`ACCUM_FN` | preprocessor macros — no linkage at all. Their Rust counterparts (`OP_FN`, `step`, `INIT_FOR`, `REPEAT`, `OP_NAME`, `op_fn`) are `const`/`#[inline] fn`, likewise unexported. |

Note that `FOR_EACH` / `DO_LOOP` are **dead macros**: `RUN_LOOP` is defined as
`CHOOSE_REP(n)(op, acc)` and never expands `DO_LOOP`, so the runtime `for` loop
form is never instantiated anywhere in the C program. Nothing in Rust needs to
correspond to them.

## Undefined (imported) symbols

`nm -D cbuild/libcdriver_add_5.so` additionally lists only libc/toolchain
imports: `printf@GLIBC_2.2.5` (`U`) and the weak `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.
The Rust `.so` imports only libc/`std` symbols (its own `printf`-equivalent
formatting is statically linked in). **0 missing/undefined non-libc symbols on
the Rust side** — verified by
`translation/tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`,
which asserts the `.so` `dlopen`s with `RTLD_NOW` (an eager-binding load fails
outright on any unresolvable symbol).
