# SYMBOLS.md — dynamic symbol parity between the C `.so` and the Rust `.so`

Derived mechanically from:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

Reproduce with `./check_symbols.sh` in the crate root (exit 0 == parity).

## Exported (defined) dynamic symbols

The whole library is `c_src/src/driver.c` (58 lines, 4 functions). There is no
second translation unit, no macro-generated symbol, and no namespace-renaming
macro in `driver.h`, so source-level names are also linker names.

| # | symbol | type | C `.so` | Rust `.so` | C signature | source |
|---|--------|------|---------|------------|-------------|--------|
| 1 | `printIntPtrLine` | `T` (global text) | yes | yes | `void printIntPtrLine(const int *intNumber)` | `driver.c:28` |
| 2 | `bad` | `T` (global text) | yes | yes | `void bad(void)` | `driver.c:33` |
| 3 | `good` | `T` (global text) | yes | yes | `void good(void)` | `driver.c:39` |
| 4 | `driver` | `T` (global text) | yes | yes | `void driver(int useGood)` | `driver.c:48` |

Only `driver` is declared in the public header `include/driver.h`. The other
three have external linkage in the C (no `static`), so they are part of the
exported ABI and are treated as public entry points here.

**Missing from Rust: none.** `nm -D --defined-only` on the two objects yields the
same 4-name set. No `#[no_mangle]` wrapper had to be added and no C module was
untranslated, so neither Phase A remediation rule applied.

## Undefined (imported) dynamic symbols

| object | non-libc undefined symbols | notes |
|--------|---------------------------|-------|
| C `.so` | none | imports `printf@GLIBC_2.2.5`; weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` |
| Rust `.so` | none | imports `printf@GLIBC_2.2.5` plus glibc allocator/IO/TLS entry points pulled in by `libstd`, and `_Unwind_*` from `libgcc_s` |

`_Unwind_*` come from `libgcc_s.so.1`, which is part of the platform C/C++
runtime and is listed as a `NEEDED` dependency, so it resolves at load time. The
completion-gate criterion "0 missing/undefined **non-libc** symbols" is met:
every undefined symbol in the Rust object is satisfied by `libc.so.6`,
`libgcc_s.so.1`, or `ld-linux-x86-64.so.2`, all of which are recorded as
`NEEDED`. Verified by `ldd -r` reporting no unresolved symbols.

## Dynamic-linking flags (not a symbol, but ABI-observable here)

| object | `DT_FLAGS` | binding |
|--------|-----------|---------|
| C `.so` (as built by `c_src/CMakeLists.txt`) | *(absent)* | lazy PLT binding |
| Rust `.so`, default rustc flags | `BIND_NOW`, `NOW` | eager binding |
| Rust `.so`, with `translation/.cargo/config.toml` | *(absent)* | lazy PLT binding |

This matters for this specific library rather than being cosmetic: `bad()` reads
an uninitialised stack slot, and on the C library the first call through a PLT
slot runs `_dl_runtime_resolve`, whose own stack usage lands in that slot.
`translation/.cargo/config.toml` therefore passes `-Wl,-z,lazy` so the Rust
object is lazily bound too. See `CONFIGS.md` row 12 and the header comment in
`src/lib.rs`.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one build configuration (empty default feature set). The Phase D
requirement to repeat Phases B–C for every feature combination is satisfied by
the single combination; `./check_features.sh` enumerates the feature table
mechanically (`awk` over `[features]`), finds it empty, and runs the whole suite
for that one configuration under **both** the `debug` and the `release` profile —
four runs in total (`--no-default-features` and the default set are the same
build here, and both are exercised).

Verified output of `./check_features.sh`:

```text
features declared in Cargo.toml: 0 (none)
configurations to verify: 2
  profile=debug   features='--no-default-features'  -> 18 + 14 + 5 tests passed
  profile=debug   features='<default>'              -> 18 + 14 + 5 tests passed
  profile=release features='--no-default-features'  -> 18 + 14 + 5 tests passed
  profile=release features='<default>'              -> 18 + 14 + 5 tests passed
ALL CONFIGURATIONS PASSED
```

## A build-system hazard worth recording

Because the crate is `crate-type = ["cdylib"]` and the tests load the library
with `dlopen` rather than linking it, **`cargo test` does not rebuild
`libdriver.so`**. This was confirmed directly: touching `src/lib.rs` and running
`cargo test` left the `.so` mtime unchanged, so an entire test run can pass
against a library from an earlier edit. Adding `"rlib"` to `crate-type` does not
fix it either.

`tests/common/mod.rs::rust_so_path` therefore compares the `.so`'s mtime against
the newest of `src/**`, `Cargo.toml` and `.cargo/config.toml`, rebuilds if the
`.so` is behind, and aborts with an explicit message if it still cannot get a
fresh one. If the crate ever gains features, it refuses to guess which ones to
build with and tells the caller to build explicitly instead.
