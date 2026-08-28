# SYMBOLS.md — public ABI surface parity

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-qlgOWs.so
#    (CMakeLists.txt derives the project name from the PARENT directory name,
#     so the .so basename tracks the checkout directory. Tests glob for it.)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libtfm_lib.so
```

## C source inventory (completeness check)

The entire library is two files:

| C file | lines | functions defined |
|--------|-------|-------------------|
| `c_src/include/lib.h` | 1 | (declaration only) `tfm` |
| `c_src/src/lib.c` | 32 | `tfm` |

There is exactly one translation unit and exactly one function. No C module was
skipped by the translation — `translation/src/lib.rs` covers 100% of `c_src`.

## `nm -D --defined-only` on the C `.so`

```
0000000000001109 T tfm
```

Exactly **one** exported (globally defined, dynamic) symbol: `tfm`.
`lib.h` contains no namespacing/renaming macros, so the linker name is the
plain source name.

## `nm -D --defined-only` on the Rust `.so`

```
0000000000011cb0 T tfm
```

## Symbol parity table

| # | C symbol | type | exported by Rust `.so`? | Rust definition site |
|---|----------|------|-------------------------|----------------------|
| 1 | `tfm`    | `T` (global text) | **yes**, exact name | `#[unsafe(no_mangle)] pub unsafe extern "C" fn tfm` — `src/lib.rs` |

**Missing symbols: 0.** No `#[no_mangle]` wrapper needed to be added and no C
module needed to be translated.

## Undefined (imported) symbols

C `.so` imports, ignoring weak/`_ITM_`/`__gmon_start__` glue:

```
U sqrtf@GLIBC_2.2.5
```

Rust `.so` imports: only libc (`malloc`, `memcpy`, `open64`, `read`, …),
`_Unwind_*` from libgcc, and the same weak glue. All are libc/runtime, none is
a symbol that the C library defines. `sqrtf` is not imported because Rust
lowers `f32::sqrt` to the `sqrtss` instruction inline (bit-identical to glibc's
`sqrtf`, which is correctly rounded per IEEE-754 and also just `sqrtss` on
x86-64).

> Note: the *arithmetic* Rust `.so` symbol count is intentionally larger than
> the C one only in the *undefined* column. The **defined** column is what the
> parity requirement is about, and it matches exactly: `{tfm}` == `{tfm}`.

## Verification

Automated by `tests/symbols.rs::c_and_rust_export_identical_symbol_sets`, which
shells out to `nm -D --defined-only` on both objects, filters to global text and
data symbols, and asserts set equality (plus asserts that every Rust *undefined*
symbol is a known libc/unwind import).

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the complete set
of feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | default (empty) | `cargo test` |
| 2 | no-default-features (identical to #1, no features exist) | `cargo test --no-default-features` |

`src/lib.rs` contains no `#[cfg(...)]` attributes at all, so #1 and #2 compile
byte-identical code. Both are still run by `run_all.sh` for completeness, and
both the `debug` and `release` Rust `.so` are exercised (the FP helpers are
`#[inline(always)]`/`#[cold]`, so optimization level could in principle change
codegen; it is verified not to change results).

---

## Verification result

| gate | result |
|------|--------|
| `nm -D` missing/undefined non-libc symbols in Rust | **0** |
| symbol diff `C \ Rust` | **empty** (`{tfm}` == `{tfm}`) |
| C modules never translated | **none** — `c_src` is one 32-line file, fully covered |
| stubs / `unimplemented!()` added to fake a symbol | **none** |

Enforced continuously by `tests/symbols.rs`:

| test | what it locks down |
|------|--------------------|
| `both_shared_objects_exist` | both objects were actually built |
| `c_exports_exactly_tfm` | the C ABI is still exactly `{tfm}` — a new C symbol makes this fail loudly rather than letting the parity check compare two equally-incomplete sets |
| `rust_exports_every_c_symbol` | `C \ Rust` is empty |
| `rust_has_no_unresolved_non_libc_symbols` | no dangling reference to untranslated C |
| `tfm_is_loadable_from_both` | the symbol is `dlsym`-able and callable in both |
| `shared_objects_are_not_stale` | **`.so` mtime ≥ `src/lib.rs` mtime.** `cargo test` builds only the *test* targets; since the tests `dlopen` the cdylib instead of linking it, cargo has no reason to rebuild it, so a stale `.so` would be silently "verified". This actually happened during this work and briefly masked the E6/E7 divergence below — hence the guard. |
