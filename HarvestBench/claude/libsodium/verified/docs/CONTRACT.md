# Shared sub-agent contract for differential testing

Working dir: `/tmp/harvest-work-P2znNt/translated_rust`

## Goal
Write Rust integration tests in `tests/` that load BOTH the C `.so` and the
Rust `.so` via `libloading` and compare outputs byte-for-byte through the FFI
boundary. The C code is ground truth; fix ONLY the Rust source (`src/`) on any
divergence — NEVER edit `c_src/` and NEVER edit the C behavior.

## Harness (already exists — DO NOT rewrite)
- `tests/common/mod.rs` exposes:
  - `common::libs()` -> `&'static Libs { c: Library, rust: Library }`
    (already calls `sodium_init()` on both).
  - `common::Rng` — deterministic xorshift PRNG: `Rng::new(seed)`, `.fill(&mut buf)`,
    `.vec(len)`, `.range(n)`, `.next_u64()`.
  - `sympair!(l, b"symbol_name", unsafe extern "C" fn(...) -> ...)` macro returns
    `(c_symbol, rust_symbol)` — call both and compare.
- Each test file MUST start with:
  ```rust
  #[macro_use]
  mod common;   // provides sympair! via #[macro_export] + common::*
  use common::libs;
  ```
  (Actually `sympair!` is `#[macro_export]` so it's crate-global; just `mod common;` + `use common::{libs, Rng};`.)

## Rules
1. NEVER call Rust functions directly. Always go through `sympair!` loaded symbols.
2. Get exact C signatures from the headers in
   `c_src/libsodium/include/sodium/*.h` and the C source in
   `c_src/libsodium/**/*.c`.
3. Phase B (valid path): for each meaningful configuration/input-shape, run MANY
   randomized inputs (fixed seed) and assert BOTH return code AND output buffers
   match byte-for-byte.
4. Phase C (error path): construct each invalid input the C rejects (null ptrs,
   bad lengths, out-of-range values, tampered ciphertext/tag, wrong sizes) and
   assert BOTH return the SAME error code / sentinel.
5. Use the lowest-level entry points too, not just convenience wrappers
   (e.g. `*_detached`, `*_init/_update/_final` streaming APIs, `*_ref10` internals
   if exported).
6. Build/run with: `timeout 600 cargo test --release --test <FILE_STEM> 2>&1 | tail -30`
   (release keeps the loaded cdylib `target/release/liblibsodium.so` fresh).
   The C .so is at `c_src/build/libsodium.so` (already built).
7. If you change `src/`, re-run to confirm all tests pass. Fix root causes.

## Deliverables per agent
- `tests/<FILE_STEM>.rs` with passing Phase B + Phase C tests.
- `docs/<FILE_STEM>_ERRORS.md`: rows `| # | function | trigger | expected C result |`
  for every distinct rejection in your family's C source.
- `docs/<FILE_STEM>_CONFIGS.md`: rows `| # | entry point(s) | configuration (options+shape) | [x] |`
  for every meaningful config combination, checked off once its test passes.
- Report: number of tests, pass/fail, any src/ files changed, any divergences found+fixed.
