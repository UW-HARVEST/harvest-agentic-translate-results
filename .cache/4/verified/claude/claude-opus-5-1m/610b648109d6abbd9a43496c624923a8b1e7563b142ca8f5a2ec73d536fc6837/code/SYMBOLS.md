# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

The C project's `CMakeLists.txt` declares only `add_executable(driver ...)`, so it
has no `.so` rule of its own. The library surface of the project is exactly the
translation unit `src/mdcore.c` (the set of entities declared `extern` in the
public header `src/mdmacros.h`); `src/mdmain.c` contains only `main`. Two C
artifacts are therefore built, with the same `-DOP=/-DREPEAT=` flags CMake uses
(`CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}"`, no optimisation flags):

```sh
# library surface, compared symbol-for-symbol against the Rust cdylib
gcc -shared -fPIC -DOP=<op> -DREPEAT=<n> -o libcdriver_<op>_<n>.so c_src/src/mdcore.c
# executable, compared stdout/stderr/exit-status against the Rust `driver` bin
gcc         -fPIC -DOP=<op> -DREPEAT=<n> -o cdriver_<op>_<n>     c_src/src/mdcore.c c_src/src/mdmain.c
```

`tests/common/mod.rs::c_lib_path()` / `c_exe_path()` generate these on demand,
and `tests/symbols.rs` asserts the `nm -D` parity automatically for the
configuration under test. The CMake build itself is also exercised for the
default configuration and asserted to be byte-identical to the plain-`gcc`
executable, confirming the flags above faithfully reproduce the CMake build.

## Symbol table

`nm -D --defined-only` on the C library `.so` yields 8 dynamic symbols. All 8 are
exported by the Rust `cdylib` with the exact same name **and** the same `nm` type
letter (`T` = text/function, `D` = initialised writable data).

| # | symbol | `nm` type (C) | `nm` type (Rust) | kind | in Rust `.so`? |
|---|--------|---------------|------------------|------|----------------|
| 1 | `op_add`       | `T` | `T` | `int op_add(int,int)`      | yes |
| 2 | `op_sub`       | `T` | `T` | `int op_sub(int,int)`      | yes |
| 3 | `op_mul`       | `T` | `T` | `int op_mul(int,int)`      | yes |
| 4 | `helper_call`  | `T` | `T` | `int helper_call(int,int)` | yes |
| 5 | `helper_ptr`   | `T` | `T` | `int helper_ptr(int,int)`  | yes |
| 6 | `use_generated`| `T` | `T` | `int use_generated(int)`   | yes |
| 7 | `G_OP`         | `D` | `D` | `int (*G_OP)(int,int)`     | yes |
| 8 | `G_OP_NAME`    | `D` | `D` | `const char *G_OP_NAME`    | yes |

**Symbol diff: EMPTY.** No symbol exported by the C `.so` is missing from the
Rust `.so`, under every one of the 24 `OP` × `REPEAT` configurations
(`tests/symbols.rs` re-checks this per configuration; `check_all.sh` loops over
all of them).

### Deliberately-not-exported symbols

* `accum_add` / `accum_sub` / `accum_mul` — `DEFINE_ACCUM(op)` expands to
  `static int accum_<op>(int n)`. Being `static` it has internal linkage and does
  **not** appear in `nm -D` for the C `.so`; the Rust counterpart
  (`mdcore::accum_op`) is likewise a private `fn`. Verified absent from both.
* `main` — lives in `mdmain.c`, which is linked into the *executable*, not the
  library. It is absent from the C library `.so` and correspondingly absent from
  the Rust `cdylib` (Rust's `main` is in the `driver` binary). Behavioural parity
  for `main` is covered by `tests/driver_bin.rs`, which diffs the two
  executables' stdout, stderr and exit status.
* Rust-internal symbols (`_ZN*`, `__rust_*`, `rust_*`, `DW.ref.*`,
  `_ZNSt*`, allocator shims, and the standard-library personality/panic
  machinery). These are additions required by the Rust runtime, not omissions;
  the parity requirement is one-directional (every C symbol must exist in Rust).

## Storage-class fidelity (fixed defect)

`G_OP` and `G_OP_NAME` are **non-`const` globals** in C — `mdmacros.h` declares
them `extern int (*G_OP)(int,int);` and `extern const char *G_OP_NAME;` (the
`const` qualifies the *characters*, not the pointer). Both therefore live in a
writable `.data` section and a consumer of the shared library may reassign them.

The initial translation used immutable Rust `static`s, which rustc placed in
`.data.rel.ro`; RELRO maps that segment read-only after relocation, so a store
through either symbol **segfaulted** against the Rust `.so` while succeeding
against the C `.so`. They are now `static mut`, which restores `.data`
placement. `tests/globals.rs::g_op_is_writable_like_c` pins this behaviour, and
`tests/symbols.rs` additionally asserts the ELF section of both symbols is
writable in each library.
