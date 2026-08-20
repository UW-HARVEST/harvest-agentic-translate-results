# SYMBOLS.md — Symbol surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libbuffapp_lib.so`
  (`[lib] crate-type = ["cdylib"], name = "buffapp_lib"`)

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → the only valid feature
  combination is the empty/default one. `cargo check --no-default-features`
  and `cargo check` are therefore the complete matrix (verified, both clean).
* `c_src/src/lib.c` / `c_src/include/lib.h` contain **no** `#if` / `#ifdef` /
  `#ifndef` / `#define` conditionals (`grep` returns nothing).
* `c_src/CMakeLists.txt` declares **no** `option()`, `add_definitions()`, or
  `target_compile_definitions()` — a single unconditional `add_library(... SHARED src/lib.c)`.

⇒ Exactly **one** build configuration exists. See `CONFIGS.md` for the runtime
configuration surface (which is where all the real variability lives).

## Defined dynamic symbols exported by the C `.so`

| # | symbol | type | C declaration | exported by Rust `.so`? |
|---|--------|------|---------------|-------------------------|
| 1 | `append_to_buffer` | `T` | `int append_to_buffer(StringBuffer *buffer, const char *str)` | ✅ yes |
| 2 | `buffapp`          | `T` | `int buffapp(int, int, int, int)` (the only symbol in `lib.h`) | ✅ yes |
| 3 | `create_buffer`    | `T` | `StringBuffer* create_buffer(int initial_capacity)` | ✅ yes |
| 4 | `destroy_buffer`   | `T` | `void destroy_buffer(StringBuffer *buffer)` | ✅ yes |
| 5 | `get_operation_name` | `T` | `const char* get_operation_name(int op_code)` | ✅ yes |
| 6 | `perform_operation` | `T` | `int perform_operation(int a, int b, const char *operation)` | ✅ yes |

No macro-generated symbols exist in this translation unit (no `#define` at all),
so the list above is complete.

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/debug/libbuffapp_lib.so    | awk '{print $NF}' | sort)
<empty>
```

**C-exported symbols missing from the Rust `.so`: 0.**
Nothing was stubbed; every symbol has a real translated body in `src/lib.rs`
(`create_buffer`, `append_to_buffer`, `destroy_buffer`, `get_operation_name`,
`perform_operation`, `buffapp`) — i.e. the whole of `lib.c` is translated, and
no `unimplemented!()`/`todo!()` appears anywhere in `src/`.

The Rust `.so` exports **exactly** these 6 names and nothing else
(`nm -D --defined-only … | grep -cE '_ZN|_R'` → `0`; total defined count `6`).

## Undefined (imported) symbols

C `.so` imports: `free`, `malloc`, `printf`, `realloc`, `sprintf`, `strcmp`,
`strcpy`, `strlen` (all `@GLIBC_2.2.5`) plus the usual weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__`.

The Rust `.so` imports **the same eight glibc functions** — `src/lib.rs`
deliberately binds `malloc`/`realloc`/`free`/`strlen`/`strcpy`/`strcmp`/
`sprintf`/`printf` via `unsafe extern "C"` instead of using Rust's allocator or
`println!`, so that

* buffers can be created in one library and destroyed in the other, and
* `buffapp`'s output goes through the *same* `stdout` `FILE` object, making the
  captured bytes directly comparable.

Everything else the Rust `.so` imports is libc / `libgcc` unwinder runtime
(`_Unwind_*`, `memcpy`, `mmap64`, `pthread_key_create`, `dl_iterate_phdr`,
`abort`, …) pulled in by `std`.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.**
`ldd`/`dlopen` of the Rust `.so` resolves with no missing symbols (the test
harness `dlopen`s it successfully, which is a stronger check than `nm`).

## Phase D — completion gate (re-verified)

`run_verification.sh` performs the symbol diff for every build configuration
and profile. Latest run:

```
==== Phase A: enumerating feature combinations ====
Cargo.toml declares no [features] -> the only combination is the default (empty) one.
combination count: 1

==== combo=<no-default-features> profile=dev : Phase D symbol parity ====
symbol diff empty: all 6 C symbols exported by target/debug/libbuffapp_lib.so
==== combo=<no-default-features> profile=dev : Phase B + Phase C differential tests ====
result: ok. 98 passed; 0 failed; 0 filtered out
result: ok. 19 passed; 0 failed; 0 filtered out

==== combo=<no-default-features> profile=release : Phase D symbol parity ====
symbol diff empty: all 6 C symbols exported by target/release/libbuffapp_lib.so
==== combo=<no-default-features> profile=release : Phase B + Phase C differential tests ====
result: ok. 98 passed; 0 failed; 0 filtered out
result: ok. 19 passed; 0 failed; 0 filtered out

==== RESULT ====
ALL CONFIGURATIONS PASSED
```

Checklist:

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and **0** undefined
      non-libc symbols in the Rust `.so` (both profiles).
- [x] Phase B: **all 97** rows of `CONFIGS.md` pass across randomized inputs
      (98 test cases; 1 000 000-call soak run clean).
- [x] Phase C: **all 12 differentiable** rows of `ERRORS.md` have a passing
      error-path differential test that asserts the *exact* sentinel
      (`NULL` / `-1` / `0` / `"unknown"`); the 7 non-differentiable rows
      (process-fatal C UB or unreachable allocator failure) are documented with
      the reason and verified by source inspection.
- [x] Holds under **every** build configuration: the single feature combination
      (`--no-default-features`, identical to default) in both `dev` and
      `release` profiles, and against gcc `-O0`/`-O2`/`-O3` and clang `-O2`
      builds of the C reference.
