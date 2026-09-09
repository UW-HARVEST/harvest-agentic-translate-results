# Differential test suite (C `.so` vs Rust `.so`)

Every test loads **both** shared objects with `libloading` and calls them only through their
exported symbols — the Rust crate is never linked or called directly, so the
`#[no_mangle] extern "C"` export wrappers are part of what is tested:

| | path |
|---|---|
| C reference | `c_src/build/libpcre2.so` |
| Rust build | `translation/target/release/libpcre2.so` |

Build both, then run everything (the crates.io network is blocked, so `--offline` is required):

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . -j8
cd translation && cargo build --release --offline
cd translation && cargo test --offline --release
```

or drive the whole thing (build, symbol diff, all feature combinations) with `_v/verify.sh`.

## Files

| file | tests | covers |
|------|-------|--------|
| `common/mod.rs` | — | the dual-library loader, all `PCRE2_*` constants, `compile_probe` (errorcode + erroroffset + 25 `pattern_info` scalars + name table + start bitmap + the byte-exact serialized compiled block), `read_match` (rc + full ovector + startchar + mark), the fixed-seed `Rng`, and the zeroing allocator used where the C leaves fields indeterminate |
| `smoke.rs` | 3 | both `.so`s load and resolve, and a first end-to-end compile/match/leaf comparison |
| `configs_tables_leaf.rs` | 55 | `CONFIGS.md` C001–C055 — exported data tables and the `PRIV()` leaf functions |
| `configs_study_context.rs` | 25 | C056–C077 — `_pcre2_study`, `_pcre2_auto_possessify` (through compile and by direct FFI), the context family, every validating setter, `pcre2_config`, `pcre2_maketables` |
| `configs_compile.rs` | 48 | C078–C125 — the whole `pcre2_compile` option/extra-option/limit/shape surface plus `code_copy*` and the match-data constructors |
| `configs_match.rs` | 32 | C126–C157 — `pcre2_match`: validation, start optimizations, all three limits, offset limit, partial matching, newline conventions, callouts, and one row per opcode family |
| `configs_dfa.rs` | 14 | C158–C169 — `pcre2_dfa_match` (workspace thresholds, restart, shortest, multi-match ovector, unsupported items) and `pcre2_next_match` |
| `configs_subst.rs` | 17 | C170–C189 — `pcre2_substitute` (all option bits, replacement syntax, both callouts, overflow bookkeeping), all 11 `pcre2_substring_*`, serialize/deserialize incl. cross-library decode |
| `configs_info_convert.rs` | 13 | C190–C202 — every `pcre2_pattern_info` key, `pcre2_callout_enumerate`, every `pcre2_pattern_convert` mode, the free-function/memctl contract, the JIT stubs |
| `errors_compile.rs` | 3 | `ERRORS.md` part 1 — all 121 compile error codes (102 reachable), `get_error_message` for every code and buffer size, every option bit |
| `errors_runtime.rs` | 62 | `ERRORS.md` part 2 — all 267 reachable runtime/API rejections plus ~2500 generic-boundary cases and 6300 random ones |
| `fuzz_crosscheck.rs` | 4 | cross-cutting: 800 000 fixed-seed cases pushed through the whole pipeline at once |
| `regressions.rs` | 2 | the divergences found and fixed during verification (see below) |

Total: **278 tests, 0 failures**, ~30 M differential comparisons per run of the suite
(plus the ≈220 M-comparison extended mode of `configs_compile.rs` via `FUZZ_N`).

## Divergences found and fixed

1. **`GET_UCD` panicked where the C reads on.** `PRIV(extuni)` (`\X`) reaches `GET_UCD` with a
   code point above `MAX_UTF_CODE_POINT` (`GETCHARLEN` on a 5/6-byte UTF-8 lead byte), and the
   8-bit build of `GET_UCD` has no guard (`c_src/src/pcre2_internal.h:2123-2131`). The C reads
   `PRIV(ucd_stage1)` out of bounds and returns; the Rust used bounds-checked slice indexing
   and, with `panic = "abort"`, killed the process. Reproducer: pattern `b"\\X"` + `PCRE2_UTF`,
   subject `b"a\xf8\x88\x80\x80\x80"`, `PCRE2_NO_UTF_CHECK` → C `rc=1`, Rust SIGABRT.
   Fixed in `src/macros.rs` by using unchecked pointer arithmetic with C's `(int)`
   division/modulo semantics. Pinned by `regressions.rs::extuni_high_codepoint`.
2. **The same class in 14 more places** — `UCD_CATEGORY!` and every
   `_pcre2_ucp_gentype_8[UCD_CHARTYPE(...)]` / `_pcre2_ucp_gbtable_8[UCD_GRAPHBREAK(...)]` site
   in `src/match_engine.rs`, `src/compile.rs`, `src/substitute.rs`: the byte read out of an
   out-of-range `ucd_record` can be anything 0..255, far past those 30- and 15-entry tables.
   All converted to unchecked pointer arithmetic. Three of them were demonstrated to abort
   before the fix and to return the C's value after it.

No other divergence was found: every compared return code, output argument, buffer, compiled
pattern image, ovector, callout-block field and data-table byte is identical.

## Mutation testing (evidence the suite is not vacuous)

Three deliberate defects were injected into the Rust source, one at a time, rebuilt, and the
suite re-run; each was caught, and reverted afterwards:

| injected defect | caught by |
|---|---|
| `pcre2_config(PCRE2_CONFIG_PARENSLIMIT)` returns `PARENS_NEST_LIMIT + 1` (`src/config.rs:143`) | `configs_study_context::c075_config_with_buffer` |
| UCP word-boundary test drops the `ucp_Pc` category (`src/match_engine.rs`, `\b` under `PCRE2_UCP`) | `configs_dfa::c168_c169_random_sweep` |
| `re->max_lookbehind` off by one when > 2 (`src/compile_main.rs:743`) | 10+ tests in `configs_compile.rs` (`c078`, `c079`, `c082`, `c088`, `c101`, `c102`, `c104`, `c105`, `c108`, `c123`, …) |

Individual test authors additionally ran their own negative controls (e.g. XOR-ing
`PCRE2_NOTBOL` into only the Rust call made 26 of the 32 `configs_match.rs` tests report a
divergence; a one-byte output-length change was caught by the poison-tail buffer comparison in
`configs_subst.rs`).

## Binary / driver comparison

Neither project builds an executable: `c_src/CMakeLists.txt` has only `add_library(pcre2 SHARED …)`
and no `add_executable`, and `translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`
with no `[[bin]]` and no `src/bin/`. So there is no pair of binaries whose stdout could be diffed.

The equivalent check was done instead: the two pre-existing C driver programs (`_gen/harness.c`,
a curated API walk, and `_gen/fuzz.c`, a randomized one) were each compiled TWICE — once linked
against `c_src/build/libpcre2.so` and once against `translation/target/release/libpcre2.so` — and
their stdout compared byte-for-byte:

```
harness: 2 576 785 lines — IDENTICAL
fuzz:   20 031 354 lines — IDENTICAL
```

Both were re-run after the source fixes described above and remained identical.
