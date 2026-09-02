# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-ZcvML8.so   (name derives from the parent dir)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libflip_horizontal_lib.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C file | translated in Rust? | where |
|--------|--------------------|-------|
| `c_src/src/lib.c` | yes | `translation/src/lib.rs` |

Public header inventory:

| C header | declarations | translated? |
|----------|--------------|-------------|
| `c_src/include/lib.h` | `cp_pixel_t`, `cp_image_t`, `flip_horizontal` | yes (all three) |

No module/file was skipped: the library is a single `.c` file with a single
`.h`, and there are no `#ifdef` blocks, no renaming/namespace macros and no
macro-generated symbol families, so linker names equal source-level names.

## Defined dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `flip_horizontal` | `T` | `T` | ✅ present in both |

Symbol diff (`comm -23` of the two sorted symbol lists) is **EMPTY**.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
glibc-TLS imports pulled in by the Rust runtime (`malloc`, `memcpy`, `abort`,
`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `__errno_location`, …).

**0 missing / undefined non-libc symbols.** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
complete set of feature combinations is the single default (empty) one:

```sh
cargo test                          # default == only combination
cargo test --no-default-features    # identical (no default features exist)
```

Both are exercised, under both the `dev` and `release` profiles, by
`scripts/verify_all.sh` (4 configurations total). `verify_all.sh` also
enumerates the `[features]` powerset automatically, so it stays correct if
features are ever added.

## How to run the verification

```sh
cd translation
./scripts/verify_all.sh          # C build + all 4 configs + symbol diff
./scripts/verify_all.sh --slow   # ...also the #[ignore]d long-running rows
./scripts/mutation_check.sh      # anti-vacuity: 8 mutations must all be caught
```

### Why a script instead of bare `cargo test`

**`cargo test` does not emit `cdylib` artifacts.** The differential tests load
the Rust `.so` through `libloading`, so it must be produced by an explicit
`cargo build --lib` first. This is not a nicety — during bring-up the harness
had a "search nearby target dirs" fallback, silently picked up a `.so` left
behind by an earlier `cargo build --release`, and **four semantic mutations of
`src/lib.rs` went completely undetected** while the suite reported all green.

`tests/common/mod.rs` now resolves the Rust `.so` strictly from the running test
binary's own profile directory, with **no cross-profile fallback and no
auto-rebuild**, and asserts the `.so` is newer than every `.rs` source. A
missing or stale artifact is a hard test failure with a message naming the
`cargo build` command to run. The C `.so` gets the same staleness guard against
`c_src/src/lib.c`, `c_src/include/lib.h` and `c_src/CMakeLists.txt`.

### Anti-vacuity evidence

`scripts/mutation_check.sh` breaks `src/lib.rs` in 8 semantically
non-equivalent ways and requires the suite to catch every one (it restores the
file afterwards and verifies the md5). Current result: **8/8 detected.**

| mutation | caught by |
|----------|-----------|
| swap drops the alpha channel | `phase_b_configs` (17 rows fail) |
| inner loop stops one column early | `phase_b_configs` (16 rows fail) |
| mirrored row index `h-i-1` -> `h-i` | `phase_b_configs` (writes past the guard band) |
| second row pointer duplicates the first | `phase_b_configs` (17 rows fail) |
| adds a NULL-`img` guard C does not have | `phase_c_errors` (`err_01`) |
| adds a NULL-`pix` guard C does not have | `phase_c_errors` (`err_02`) |
| negative height stops being a silent no-op | `phase_c_errors` (8 rows fail) |
| negative width stops being a silent no-op | `phase_c_errors` |

Note that mutations which *look* wrong but are provably equivalent were
discarded rather than counted: `flips = (h+1)/2` only adds a self-swap of the
middle row (`i == h-i-1`), and an early `return` for `w < 0` matches what the C
already does, so neither is a real divergence.
