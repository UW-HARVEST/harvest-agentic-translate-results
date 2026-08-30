# VERIFICATION.md — completion gate

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `../c_src`. Both are loaded as shared objects via `libloading`
and driven only through their exported `extern "C"` symbols, so the
`#[no_mangle]` wrappers are part of what is under test — no Rust function is
ever called directly.

## How to reproduce

```bash
# 1. Build the C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Build the Rust cdylib and run every suite
cd ../../translation && cargo build && cargo test

# 3. Or run the whole feature x profile matrix in one go
./scripts/check_features.sh
```

(Add `--offline` to the cargo commands if the crates registry is unreachable;
`libloading` and its two transitive deps are the only dev-dependencies.)

## Artifacts

| file | phase | contents |
|---|---|---|
| `SYMBOLS.md`  | A / D | every `nm -D` export of the C `.so`, mapped to the Rust `.so` |
| `ERRORS.md`   | A / C | the error-surface table: every rejection the C performs |
| `CONFIGS.md`  | A / B | the configuration-surface table: option × input-shape combinations |
| `FEATURES.md` | D     | the feature/profile matrix and the mutation-sensitivity check |
| `tests/common/mod.rs` | — | the fd-1 capture harness, the seeded PRNG, the sequential runner |
| `tests/configs.rs`    | B | 22 cases, one per `CONFIGS.md` row |
| `tests/errors.rs`     | C | 12 cases, one per `ERRORS.md` row |
| `tests/symbols.rs`    | D | 5 cases, the symbol-parity gate |
| `tests/optlevels.rs`  | D | 6 cases, C rebuilt at -O0/-O1/-O2/-O3/-Os |
| `tests/smoke.rs`      | — | 6 harness self-checks, incl. a negative control |
| `scripts/check_features.sh` | D | enumerates features from `cargo metadata` and loops |

## Gate

- [x] **`SYMBOLS.md`: 0 missing symbols.** `nm -D --defined-only` on the C `.so`
      yields exactly `{bad, driver, good, printLine}`; the Rust `.so` exports the
      identical set, and every undefined symbol it imports is libc or
      libgcc-unwind. Asserted at test time by
      `symbols.rs::phase_d_symbol_parity_c_minus_rust_is_empty`,
      `rust_so_exports_no_extra_api_symbols` and
      `rust_so_has_no_unresolved_non_libc_symbols`. No module of the C source was
      left untranslated: `c_src/CMakeLists.txt` compiles one translation unit and
      all six of its functions (including the two `static` helpers) are present.
- [x] **Phase B: every `CONFIGS.md` row passes across randomized inputs.** 22/22
      rows green; the randomized rows run 200–2000 seeded inputs each.
- [x] **Phase C: every `ERRORS.md` row has a passing error-path test.** 3/3 real
      rejection branches (E1–E3) plus 11/11 mandated generic FFI-boundary rows
      (G1–G11), including out-of-range "enum" ints across the boundary, null
      pointers, zero/oversized lengths, and one-past-range values.
- [x] **All of the above hold under every feature combination.** `Cargo.toml`
      declares no `[features]`, so there are 2 combinations (default ≡
      `--no-default-features`); crossed with the `dev` and `release` profiles
      that is 4 runs × 51 cases = 204 passing cases. See `FEATURES.md`.

## Result

**No divergence found.** The Rust translation is byte-for-byte equivalent to the
C library across every configuration, input shape and error path enumerated in
`CONFIGS.md` and `ERRORS.md`, and remains equivalent when the C is rebuilt at any
optimization level. `src/lib.rs` required no behavioural change.

The one substantive finding was in the *test harness*, not the translation:
because the library's only observable output is what it writes to file
descriptor 1, and the harness redirects fd 1 process-wide, libtest's parallel
progress lines were landing inside the captured bytes and causing spurious
mismatches. The differential suites therefore use `harness = false` with a
sequential runner. See `FEATURES.md` for details.

## The one thing to know about this library

`c_src/src/driver.c` is a CWE-562 ("Return of Stack Variable Address")
demonstration. `helperBad()` returns the address of its automatic
`char charString[] = "helperBad string"`, which is undefined behaviour. The
reference build resolves that UB by emitting `mov $0x0,%eax`, so `helperBad`
returns **NULL**, `printLine`'s `if (line != NULL)` guard rejects it, and
`bad()` — and therefore `driver(0)` — **prints nothing at all**. The translation
reproduces that exactly rather than "fixing" the defect; `ERRORS.md` row E2 and
the `optlevels` suite pin it down, and the mutation check in `FEATURES.md`
confirms that "fixing" it is caught by 17 cases.
