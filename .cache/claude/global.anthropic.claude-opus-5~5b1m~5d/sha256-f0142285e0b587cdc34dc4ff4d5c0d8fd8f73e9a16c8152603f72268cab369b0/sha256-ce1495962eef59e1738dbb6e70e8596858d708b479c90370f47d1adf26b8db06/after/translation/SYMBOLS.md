# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects via
[`../scripts/symdiff.sh`](../scripts/symdiff.sh). Regenerate with:

```sh
./scripts/symdiff.sh                      # default: release Rust .so
./scripts/symdiff.sh translation/target/debug/libldexp_q2_lib.so
```

## Translation-unit inventory (completeness check)

Before comparing symbols, confirm no C source file was skipped by the
translation. `c_src/CMakeLists.txt` compiles exactly one translation unit:

```cmake
add_library(${project_name} SHARED
    src/lib.c)
```

| C source file | lines | translated to | status |
|---|---|---|---|
| `c_src/src/lib.c`      | 12 | `translation/src/lib.rs` | translated |
| `c_src/include/lib.h`  | 1  | (declaration only, no code) | n/a |

`find c_src -name '*.c'` returns exactly `c_src/src/lib.c`. There is **no
untranslated module** — the whole library is one 12-line function, so the
"whole file missing" failure mode from Phase A does not apply here.

## `nm -D --defined-only` comparison

C `.so` = `c_src/build/libharvest-work-X5tAvE.so`
(the CMake target name is derived from the parent directory name, so the file
name varies by checkout; the script globs `c_src/build/lib*.so`.)

Rust `.so` = `translation/target/{release,debug}/libldexp_q2_lib.so`

| # | symbol | type | in C `.so` | in Rust `.so` | notes |
|---|--------|------|-----------|--------------|-------|
| 1 | `ldexp_q2` | `T` (global text) | yes | yes | `#[unsafe(no_mangle)] pub extern "C" fn ldexp_q2(f32, c_int) -> f32` |

**Count: C defines 1 dynamic symbol; Rust defines 1. Missing from Rust: 0.**

No macro-generated symbols exist: `grep -c define c_src/src/lib.c c_src/include/lib.h`
is 0, so there is no macro expansion producing extra exported names.

### Weak / undefined symbols (informational — not required for parity)

`nm -D` on the C `.so` also lists the standard glibc-emitted weak undefined
entries, which are toolchain artifacts and not part of the library API:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The Rust `.so` has **0 undefined non-libc symbols** (checked by `nm -D -u`
filtered against `@GLIBC`, `_ITM_*`, `__gmon_start__`, `__cxa_*`, `_Unwind_*`,
`__tls_get_addr`).

### `static const float g_expfrac[4]` is intentionally NOT a symbol

In the C source `g_expfrac` is a function-local `static const`, so it has
internal linkage and appears only as a local symbol (`nm` shows
`g_expfrac.0` in the non-dynamic table at `.rodata`/`0x2000`). It is **not**
exported via `nm -D`, so the Rust side correctly represents it as a private
`const G_EXPFRAC: [f32; 4]` and must not export it.

## Phase D completion gate

- [x] `nm -D` shows **0 symbols missing** from the Rust `.so`.
- [x] `nm -D` shows **0 undefined non-libc symbols** in the Rust `.so`.
- [x] Verified for the `release` profile `.so`.
- [x] Verified for the `debug` profile `.so`.
- [x] Verified under every feature combination (see `CONFIGS.md` §Features:
      the crate declares no `[features]`, so the default set is the only one).

## Harness soundness note (important)

An earlier version of this harness produced **false passes**. `crate-type` was
`["cdylib"]` only, so the integration tests had no Rust-level dependency on the
lib target and `cargo test` never rebuilt the `.so`. The tests happily loaded a
`.so` from a previous build, meaning every differential assertion compared the C
library against a stale artifact. This was caught by mutation testing
(`scripts/mutation_check.sh`): **all 10 injected bugs escaped detection.**

Two fixes, both required:

1. `crate-type = ["cdylib", "rlib"]` — the `rlib` gives the integration tests a
   real dependency on the lib target, so `cargo test` rebuilds the `cdylib`.
   This does not change the `cdylib`'s contents or exported ABI (verified: the
   `.so` still exports exactly `ldexp_q2` and nothing else).
2. `assert_not_stale()` in `tests/common/mod.rs` — fails loudly if the loaded
   `.so` is older than any `src/**.rs`, so this failure mode can never again be
   silent. Verified to fire by pointing `RUST_SO_PATH` at a back-dated `.so`.

After the fixes, 12 of 12 non-equivalent mutations are caught.
