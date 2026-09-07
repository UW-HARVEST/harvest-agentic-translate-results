# Verification report

C-to-Rust translation of the `colourblind` library. The C in `c_src/` is the
ground truth; `translation/` must be byte-identical to it.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | `nm -D` parity table, C `.so` vs Rust `.so` |
| `ERRORS.md`  | error/rejection-surface table (12 rows), all checked |
| `CONFIGS.md` | configuration-surface table (20 rows), all checked, + findings |
| `tests/common/mod.rs` | `libloading` harness: loads BOTH `.so`s, never calls Rust directly |
| `tests/valid_paths.rs` | Phase B — 20 tests, one per `CONFIGS.md` row |
| `tests/error_paths.rs` | Phase C — 17 tests, one per `ERRORS.md` row + generic FFI boundaries |
| `check_symbols.sh` | Phase D — symbol diff, exits non-zero if non-empty |
| `check_features.sh` | Phase D — extracts features from `Cargo.toml`, runs the suite under every combination |

## Reproduce

```bash
# 1. Ground-truth C shared library
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Differential suite (the harness rebuilds + freshness-checks the Rust cdylib)
cd ../../translation
cargo test --release --no-fail-fast     # 36 tests
cargo test --no-fail-fast               # again under debug-assertions

# 3. Phase D
./check_symbols.sh
./check_features.sh

# 4. Optional soak (multiplies every randomized row)
CB_SOAK=200 cargo test --release --no-fail-fast
```

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff is empty. The C `.so` defines exactly one
      symbol, `colourblind`; the Rust `.so` defines it under the same name.
      `ldd -r` reports no unresolved non-libc symbols. `Protanopia`,
      `Deuteranopia` and `Tritanopia` are `static` in C and are correctly not
      exported by either object. `src/lib.c` is the only translation unit in
      `CMakeLists.txt`, so no module was skipped — nothing needed stubbing or
      re-translating.
- [x] **Phase B** — all 20 `CONFIGS.md` rows pass, each over many randomized
      inputs from a fixed-seed SplitMix64 PRNG, compared with `to_bits()` so NaN
      payloads and the sign of zero are part of the comparison.
- [x] **Binary/stdout comparison** — not applicable, and verified as such:
      `CMakeLists.txt` declares only `add_library(... SHARED)` (no
      `add_executable`), `Cargo.toml` declares only `crate-type = ["cdylib"]`
      (no `[[bin]]`, no `src/main.rs`). `c_src/build` contains no executable.
      Neither project builds a driver, so there is no stdout to diff.
- [x] **Phase C** — all 12 `ERRORS.md` rows have a passing differential test,
      plus the generic boundaries: null pointers (both with valid and invalid
      modes, the valid case probed out-of-process), out-of-range enum values
      across a dense −600..600 band, every power of two ±1, `INT_MIN`/`INT_MAX`,
      and 20 000 random `int` discriminants. `boundary_valid_enum_values_do_write`
      guards against the "both did nothing" tests passing vacuously.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]`
      table, so `default` and `--no-default-features` are the only combinations
      that exist; `check_features.sh` derives this from the manifest rather than
      hardcoding it and passes both. The suite additionally passes under both
      the `release` and `debug` profiles — which is what exposed the one real
      bug (see below), since `debug` enables Rust's misaligned-pointer check.

## Bugs found

Two, both fixed and re-verified:

1. **The harness itself was blind.** `cargo test` does not rebuild a `cdylib`,
   so the tests were loading a stale Rust `.so`; a gross injected mutation still
   passed 36/36. The harness now rebuilds and freshness-asserts the artifact
   before loading it. After the fix the same mutation fails 18 of 36 tests.
2. **Misaligned pointer dereference in the Rust** (`CONFIGS.md` C16 /
   `ERRORS.md` E10). The C's `movss` accepts unaligned `float*`; the Rust's plain
   `*p` is UB there and aborted the process under debug assertions. Replaced
   with `read_unaligned`/`write_unaligned`, which is identical codegen on the
   aligned path.

The rest of the translation is bit-exact, including the parts most likely to be
wrong: the SSE NaN-forwarding rules (`dst` before `src`, quieting of signalling
NaNs, x86 "real indefinite" `0xFFC00000` for `inf-inf` and `0*inf`), the
per-expression operand roles, the sign of zero, and GCC's constant pooling of
literals that round to the same `f32`.
