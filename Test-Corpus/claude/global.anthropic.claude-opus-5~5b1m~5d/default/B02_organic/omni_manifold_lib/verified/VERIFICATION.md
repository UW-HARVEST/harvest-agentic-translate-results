# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C, the
ground truth) for a reduced `cute_c2`-style 2D collision library.

Every comparison loads **both** shared objects with `libloading` and calls only
their exported symbols — the Rust functions are never called directly, so the
`#[no_mangle] extern "C"` wrappers and the SysV struct-passing classification are
under test too.

## How to reproduce

```sh
./translation/run_tests.sh            # builds both libs, diffs nm -D, runs everything
```

or manually:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && cargo build --release && cargo test --release
```

`cargo build` before `cargo test` is **required**: the crate's only lib target is
a `cdylib`, which `cargo test` does not rebuild, so the tests would otherwise
load a stale `.so`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D --defined-only` reports **46** symbols for the C
      `.so` and **46** for the Rust `.so`; `comm -23` is empty. `nm -D -u` on the
      Rust `.so` lists only libc / libgcc-unwind / rust-std imports, no
      unresolved project symbols.
- [x] **Phase B** — all **57** rows of `CONFIGS.md` pass across randomised
      inputs (fixed-seed xorshift64\*), compared **bit-for-bit** via
      `f32::to_bits` so NaN payloads and the sign of zero are part of the
      contract. 38 tests in `tests/phase_b_low.rs`, `tests/phase_b_gjk.rs`,
      `tests/phase_b_manifold.rs`.
- [x] **Binary executable** — none exists. `Cargo.toml` declares only
      `[lib] crate-type = ["cdylib"]`; `c_src/CMakeLists.txt` only
      `add_library(... SHARED ...)`. No stdout comparison applies.
- [x] **Phase C** — all **70** rows of `ERRORS.md` have a passing error-path
      differential test (19 tests in `tests/phase_c_errors.rs`), plus the generic
      FFI boundaries: null pointers on every pointer parameter, zero / negative /
      oversized counts, and **out-of-range enum ints** (`C2_TYPE_POLY`, `-1`,
      `4`, `5`, `99`, `-12345`, `INT_MIN`, `INT_MAX`, `0x7fff0000`) pushed
      through `c2MakeProxy`, `ptr_from_parts`, `c2Collide`, `c2GJK` and
      `omni_manifold`.
- [x] **Every configuration** — the crate declares no cargo features, so the
      matrix is `{default} == {--no-default-features}`; `run_tests.sh` iterates it
      generically and *additionally* re-runs the entire suite against the
      **dev-profile** (unoptimised) cdylib via `RUST_SO=`, because `opt-level`
      changes how the NaN-propagation helpers in `src/lib.rs` compile. Both
      profiles pass. The suite is also stable under `--test-threads=1` and the
      default parallel runner across repeated runs.

## Defect found and fixed in the Rust translation

**Signalling-NaN quieting.** `src/lib.rs` models x86 SSE NaN-operand selection
with `x86_mul` / `x86_add` / `x86_sub` helpers that forward whichever operand is
NaN. They forwarded the operand *verbatim*, but SSE arithmetic also **quiets** a
propagated signalling NaN by setting the mantissa MSB. Passing an sNaN such as
`0xffabcdef` through `c2Mulvs`, `c2Dot`, `c2Det2`, `c2Mulrv`, `c2MulrvT`,
`c2PlaneAt`, `c2Intersect`, … therefore produced `0xffabcdef` in Rust where the C
build produced `0xffebcdef`.

Fix: a `quiet()` helper (`bits | 0x0040_0000`) applied to the forwarded operand
in all three helpers. `src/lib.rs:167-207`.

This is exactly the class of bug that only bit-exact comparison over
non-finite inputs can find; five of the seventeen Phase B low-level tests
failed on it before the fix.

## Pre-existing undefined behaviour in the C source (not a translation defect)

`c2MakeProxy` has **no `C2_TYPE_POLY` case**, so `c2GJK` reads a completely
uninitialised `c2Proxy` local whenever `typeB == C2_TYPE_POLY`. That is on the
path of three *public* entry points (`omni_manifold` / `c2Collide` for
AABB↔CAPSULE, and `c2AABBtoCapsuleManifold` / `c2CapsuletoPolyManifold`).

Measured on the built C `.so`:

* two calls with byte-identical arguments returned two **different** manifolds;
* with hostile stack residue `pB.count` becomes a large positive integer and
  `c2Support` walks off the top of the stack — the C library **SIGSEGVs** on
  ordinary in-contract input.

The Rust translation uses `std::mem::zeroed()` for that local, which is what the
C library does whenever the stack region happens to be zero. To make the
comparison well defined the harness pins that memory four ways (see
`ERRORS.md`, row 53, for the full write-up):

1. `with_clean_stack()` — 16 KiB of **volatile** zero writes immediately below
   the frame that performs the FFI call. (`black_box` on a raw pointer does not
   clobber memory; a `write_bytes` + `black_box` version was silently optimised
   into a no-op — verified by reading the stack back.)
2. `fresh()` — one newly spawned thread per test with a unique, increasing stack
   size so glibc cannot return a cached dirty stack, plus a 1 MiB scrub at
   thread entry.
3. `dlopen` with **`RTLD_NOW`** instead of the default `RTLD_LAZY`. Under lazy
   binding the first call to each of the C library's own PLT entries runs
   `_dl_runtime_resolve_xsavec`, which spills the whole vector register file a
   few hundred bytes down the stack — landing precisely on `pB`. This made the
   first AABB-vs-capsule call of a process behave differently from every later
   one and was the last source of irreproducibility.
4. Lazy failure messages — `check(..., || format!(...))`. Eagerly formatting a
   context string every iteration left ~100 bytes of `core::fmt` debris in the
   same region.

With those in place the C library is deterministic on this path and matches the
Rust translation bit-for-bit over the tens of thousands of randomised inputs in
`CONFIGS.md` rows 44–57 and `ERRORS.md` rows 15–25, 53, 67–68.

`ptr_from_parts` has a second, milder instance: it has no `default:` and no
trailing `return`, so for `C2_TYPE_POLY` and out-of-range tags it falls off the
end and its return value is indeterminate by definition. The Rust translation
returns an explicit `NULL` sentinel; the value is never dereferenced by
`c2Collide`, whose `switch` has no case for those tags either, so the
externally-visible behaviour (`m->count == 0`) is identical and is asserted.

## Test inventory

| file | tests | covers |
|------|-------|--------|
| `tests/common/mod.rs` | – | loader (`RTLD_NOW`), ABI mirrors, bit-exact comparators, xorshift64\* PRNG, stack-pinning helpers, optional `SEGV_REPORT=1` / `CHECK_SCRUB=1` diagnostics |
| `tests/phase_b_low.rs` | 17 | `CONFIGS.md` rows 1–24 (vector maths, transforms, proxies, simplex primitives) |
| `tests/phase_b_gjk.rs` | 8 | rows 25–37 (`c2GJK`: every type pair × `use_radius` × transform × out-pointer × cache state) |
| `tests/phase_b_manifold.rs` | 13 | rows 38–57 (all six manifold generators, `c2Collide`, `omni_manifold`, separation sweeps, degenerate shapes) |
| `tests/phase_c_errors.rs` | 19 | all 70 `ERRORS.md` rows + generic FFI boundaries |
| **total** | **57** | |
