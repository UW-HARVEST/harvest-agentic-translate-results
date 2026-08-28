# SYMBOLS.md — public-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
C   .so: c_src/build/libharvest-work-QX18IK.so   (name = parent dir of c_src, per CMakeLists.txt)
Rust.so: translation/target/{debug,release}/libhsl_to_rgb_lib.so
```

## Raw `nm -D` of the C `.so`

```
                 w _ITM_deregisterTMCloneTable      <- weak, toolchain-generated
                 w _ITM_registerTMCloneTable        <- weak, toolchain-generated
                 w __cxa_finalize@GLIBC_2.2.5       <- weak, libc
                 w __gmon_start__                   <- weak, toolchain-generated
                 U fmodf@GLIBC_2.2.5                <- undefined, libm import
0000000000001109 T hsl_to_rgb                       <- the ONLY defined public symbol
```

`nm -D --defined-only` on the C `.so` yields exactly one line:

```
0000000000001109 T hsl_to_rgb
```

## Defined-symbol table (the parity requirement)

| # | C symbol (`nm -D --defined-only`) | C type | Rust `.so` exports it? | Rust source |
|---|-----------------------------------|--------|------------------------|-------------|
| 1 | `hsl_to_rgb`                      | `T` (global text) | YES — `T hsl_to_rgb` | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn hsl_to_rgb` |

### Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only rust.so | awk '{print $NF}' | sort -u)
<empty>
```

**0 symbols missing from the Rust `.so`.** No C translation unit was skipped:
`c_src` contains exactly one `.c` file (`src/lib.c`, 48 lines) with exactly one
function definition, and `include/lib.h` declares exactly that one function.
There is therefore no un-translated module and no symbol that needed a stub.

## Undefined (imported) symbols

`nm -D` on the Rust `.so` lists a larger set of `U` entries than the C `.so`
(`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …). These are all libc /
libgcc-unwind imports pulled in by the Rust standard library that is statically
linked into the `cdylib`; **none is a non-libc symbol and none is unresolved at
load time** (both `.so`s load successfully under `libloading`, which is what the
integration tests do). Requirement "0 missing/undefined non-libc symbols" holds.

### One notable difference in how `fmodf` is resolved

* The C `.so` imports `fmodf@GLIBC_2.2.5` (dynamic, resolved to glibc's libm).
* The Rust `.so` has **no** dynamic `fmodf` import; the `extern "C" { fn fmodf }`
  declaration binds to the *local* (hidden) `fmodf` that `compiler_builtins`
  statically provides:

  ```
  000000000004b380 t _ZN17compiler_builtins4math9libm_math4fmod5fmodf17h…E
  000000000004b370 t fmodf
  ```

  This is not a symbol-parity violation (nothing is missing), but it means the
  two libraries execute *different* `fmodf` implementations. Behavioural
  equivalence of those two implementations — including NaN sign/payload
  propagation and the ±Inf / ±0 / subnormal cases — is therefore not free and is
  verified explicitly by the differential tests (see `CONFIGS.md` rows F1–F6 and
  `ERRORS.md` rows E7–E10).

## Verification commands

```
nm -D --defined-only ../c_src/build/lib*.so
nm -D --defined-only target/release/libhsl_to_rgb_lib.so
```

Both must print `hsl_to_rgb` and nothing else. `tests/symbols.rs` asserts this
at test time by shelling out to `nm`, so the parity check is part of the suite
rather than a one-off manual observation.

## Addendum — import changes made during Phase C

Reproducing the C's signalling `comiss` behaviour (see `ERRORS.md` row E21)
required the Rust to call `feraiseexcept`, so the Rust `.so` now has one extra
dynamic import:

```
U feraiseexcept@GLIBC_2.2.5
```

This is a libc/libm symbol, it resolves at load time, and it does not affect the
*exported* surface — the defined-symbol diff is still empty. Both objects are now
opened by the harness with **`RTLD_NOW`**, which forces the dynamic loader to
resolve every undefined symbol eagerly; a successful `dlopen` is therefore a
proof of "0 unresolved symbols" that is stronger than reading `nm` output.
`tests/symbols.rs` asserts:

1. `no_unresolved_symbols_under_rtld_now` — both `.so`s open with `RTLD_NOW`.
2. `every_c_export_is_also_a_rust_export` — the defined-symbol diff is empty.
3. `c_exports_exactly_the_documented_surface` — the C exports only `hsl_to_rgb`,
   so this file cannot silently go stale.
4. `rust_imports_are_all_platform_runtime_symbols` — every Rust import is
   versioned `GLIBC_*`/`GCC_*` or is a known unversioned toolchain hook.
5. `import_inventory_is_as_documented` — pins the `fmodf` situation (dynamic in
   the C, statically provided by `compiler_builtins` in the Rust) so that a
   toolchain change which alters it forces this document to be revisited.
