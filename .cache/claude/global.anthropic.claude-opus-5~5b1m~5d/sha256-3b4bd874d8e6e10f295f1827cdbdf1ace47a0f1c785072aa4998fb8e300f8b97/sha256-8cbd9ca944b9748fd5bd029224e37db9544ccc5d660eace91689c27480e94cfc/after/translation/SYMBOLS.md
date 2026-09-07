# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```sh
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build
#   -> c_src/build/libharvest-work-OcxwHm.so
cargo build --release --manifest-path translation/Cargo.toml
#   -> translation/target/release/libnext_double_lib.so
```

## C translation units covered

| C source file | Rust counterpart | status |
|---|---|---|
| `c_src/src/lib.c` | `translation/src/lib.rs` | fully translated |
| `c_src/include/lib.h` | `translation/src/lib.rs` (`cn_rnd_t`) | fully translated |

The whole C library is a single translation unit. There is no untranslated
module.

## Exported (defined) symbols

`nm -D --defined-only <so> | awk '{print $3}' | sort`

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `next_double` | yes | yes | `double next_double(cn_rnd_t *rnd)` / `pub unsafe extern "C" fn next_double(*mut cn_rnd_t) -> f64` |

### Non-exported C symbols (intentionally not in the ABI)

| C symbol | linkage | Rust counterpart | exported? |
|---|---|---|---|
| `cn_rnd_next` | `static` (internal) | private `fn cn_rnd_next` | no — correct, C does not export it either |

`cn_rnd_next` is `static` in C, so it is deliberately absent from `nm -D` on
both libraries. Exporting it from Rust would be a *parity violation*, not a fix.

## Symbol diff

```
comm -23 c_syms.txt rust_syms.txt   # in C but missing from Rust
<empty>

comm -13 c_syms.txt rust_syms.txt   # extra in Rust
<empty>
```

**Result: the symbol diff is EMPTY in both directions.**

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u libnext_double_lib.so` lists only libc / libgcc-unwind imports pulled
in by the Rust runtime (`memcpy`, `malloc`, `abort`, `_Unwind_*`,
`__cxa_finalize`, `dl_iterate_phdr`, ...). There are **0 undefined non-libc
symbols**, i.e. nothing from the translated library itself is left dangling.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table** and no optional
dependencies, therefore the only build configuration is the default one:

```
cargo test                              # == default == all features == no-default-features
```

`cargo test --no-default-features` and `cargo test --all-features` resolve to the
identical feature set. Phases B–C therefore cover the complete configuration
space; `run_verification.sh` still runs all three explicitly (x both profiles)
and fails if a `[features]` table is ever added without extending the matrix.

## Phase D result

```
== 3. symbol parity (nm -D) ==
   C exports   : 1   [next_double ]
   Rust exports: 1   [next_double ]
   missing from Rust: NONE
   extra in Rust    : NONE
   undefined non-libc symbols in Rust .so: NONE
```

No C source was left untranslated and no symbol needed to be added, so the
Phase A "translate the missing module" rule never had to be applied. Nothing is
stubbed: `next_double` is a full translation of `lib.c`, and the `static` helper
`cn_rnd_next` is correctly kept unexported to match C's internal linkage.

Reproduce everything (build both libraries, diff symbols, run all phases under
every feature combination and profile) with:

```sh
bash translation/run_verification.sh
```
