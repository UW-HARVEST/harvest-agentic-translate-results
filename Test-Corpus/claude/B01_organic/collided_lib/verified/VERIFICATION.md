# VERIFICATION.md — result of the C↔Rust differential verification

## How to reproduce

```sh
./run_verification.sh      # builds C .so, then per feature combo: check, build,
                           # symbol diff, and the full differential suite
./mutation_check.sh        # proves the suite detects injected translation bugs
```

## Bug found and fixed

**One real divergence was found: `c2Dot` returned a different NaN payload than
the C reference.**

`c2Dot` is `return a.x * b.x + a.y * b.y;`. On x86-64, SSE `mulss`/`addss`
propagate the **destination** operand when it is a NaN and only fall back to the
source operand otherwise. The reference C build (`c_src/CMakeLists.txt` sets no
`CMAKE_BUILD_TYPE`, so GCC runs at `-O0`) emits:

```text
mulss %xmm0,%xmm1   ; xmm1 = a.x * b.x        destination = a.x
mulss %xmm2,%xmm0   ; xmm0 = b.y * a.y        destination = b.y   <-- swapped
addss %xmm1,%xmm0   ; xmm0 = y_prod + x_prod  destination = y_prod <-- swapped
```

The original Rust translation wrote the expression in source order, so LLVM used
`a.y` as the y-product destination and `x_prod` as the sum destination. Whenever
the two products were both NaN, or `a.y` and `b.y` were NaNs with different
sign/payload, the returned bits differed:

| input | C (`-O0`) | Rust (before) |
|-------|-----------|---------------|
| `a = (0.0, 0x7FC00000)`, `b = (0.0, 0xFFC00000)` | `0xFFC00000` | `0x7FC00000` |

`c2Dot` is a public export returning `float`, so this is directly observable by
a caller — and it also feeds `c2CircletoCircle` / `c2CircletoAABB`
(there the value is only compared, so those results were already correct).

**Fix** (`src/lib.rs`): explicit `mulss` / `addss` helpers that reproduce x86 NaN
propagation and destination priority, used in the exact order the reference build
uses. Because the propagation is now written in the source rather than left to
instruction selection, the behaviour is identical in **debug and release**
builds — verified separately (see below). For non-NaN operands the helpers are a
plain `*` / `+`, and both operations are commutative for non-NaN inputs
(including `±0.0` and the invalid-operation cases `0 * Inf` and `Inf + -Inf`,
which yield the same default QNaN either way), so nothing else changed.

Related codegen quirks that were investigated and confirmed **not** observable:

* `c2CircletoCircle` computes `A.r + B.r` with `B.r` as the `addss` destination
  at `-O0`. The sum only feeds `r2*r2` and then a comparison, so the NaN payload
  cannot escape; the boolean result is identical either way.
* `c2Sub` uses `subss` with `a.x`/`a.y` as destination, which matches the
  natural Rust `a.x - b.x` (`fsub` is non-commutative, so the order is forced).
* `c2Maxv` / `c2Minv` / `c2Clampv` are pure `comiss` + register selects — they
  copy operand bits without quieting, which the Rust `if a.x > b.x {...}` form
  reproduces exactly (a NaN comparison is false, so `b` is selected).

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` lists 10 defined symbols; the
      Rust `.so` exports all 10 with identical names. Symbol diff is **empty**
      in both debug and release. No stubs: every symbol is a real translation of
      the corresponding C function. Undefined symbols in the Rust `.so` are only
      libc/unwinder imports from `libstd`, i.e. **0 missing non-libc symbols**.
- [x] **Phase B** — all **17** rows of `CONFIGS.md` (C1–C17) pass, driven with
      fixed-seed randomized inputs plus exhaustive boundary sweeps.
- [x] **Phase C** — all **13** rows of `ERRORS.md` (E1–E6, N1–N7) have a passing
      error-path differential test asserting the same exact sentinel.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]` and
      `c_src` contains no `#if`/`#ifdef` and no CMake `option()`, so there is
      exactly **one** valid configuration: `--no-default-features` (≡ default).
      `run_verification.sh` enumerates the feature set mechanically from
      `Cargo.toml` and would loop over the full power set if any existed.

## Test inventory

| file | tests | purpose |
|------|-------|---------|
| `tests/valid_paths.rs`   | 17 | Phase B, one test per `CONFIGS.md` row |
| `tests/error_paths.rs`   | 12 | Phase C, one test per `ERRORS.md` row |
| `tests/harness_sanity.rs` | 3 | proves the two `.so`s are loaded independently |
| **total** | **32** | all passing |

Measured volume: **~2.0 million** differential comparisons in
`tests/valid_paths.rs` and **~0.1 million** in `tests/error_paths.rs`.

## Properties the harness guarantees

* **Both** libraries are loaded through `libloading`/`dlopen` and every call goes
  through a `dlsym`'d symbol — the Rust crate is never called directly, so the
  `#[no_mangle] extern "C"` wrappers and the struct-passing ABI (`c2v` returned
  packed in `xmm0`, `c2Circle`/`c2AABB` split across `xmm0`/`xmm1`) are under test.
* **Bit-exact comparison.** Results are compared via `to_bits()`, never float
  `==`, so NaN-vs-NaN, differing NaN payloads and `+0.0`-vs-`-0.0` are caught.
  This is what surfaced the `c2Dot` bug.
* **No symbol interposition.** Both `.so`s export the same names, and the C
  `c2Maxv` calls `c2V` through its PLT. `harness_sanity.rs` asserts all 10
  symbols resolve to distinct addresses per library; mutating the Rust `c2V`
  breaks only the Rust results, confirming the C library keeps calling its own
  `c2V` and the differential assertions are not vacuous.
* **Staleness guard.** `cargo test` builds the `src/lib.rs` test harness but does
  **not** relink the `cdylib`, so a stale `libcollided_lib.so` would silently be
  tested. The harness compares mtimes and aborts with a rebuild hint. (This trap
  fired during verification and hid the `c2Dot` fix until it was added.)
* **Mutation-tested.** `mutation_check.sh` injects 13 plausible translation bugs
  (wrong comparison operator, `f32::min`/`max` instead of the C ternary, swapped
  `c2Clampv`/`c2Sub`/`collided` arguments, dropped negation in `c2AABBtoAABB`,
  the natural-order `c2Dot`, an over-permissive `collided` default arm) and
  requires the suite to fail for each: **13 detected, 0 undetected.**

## Release-profile check

`profile.release` sets `panic = "abort"` and optimizations, which could have
changed float instruction selection. Verified explicitly:

```sh
cargo build --release --no-default-features
RUST_SO_PATH=$PWD/target/release/libcollided_lib.so cargo test --no-default-features
```

Result: same 10 exported symbols, all 32 tests pass.
