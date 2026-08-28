# Verification report

The Rust crate in this directory is a translation of `../c_src`. It was verified
**differentially**: both implementations are built as shared objects, both are
loaded with `libloading`, and every comparison goes through the exported C
symbols — the Rust crate is never linked or called directly, so the
`#[no_mangle]` / `extern "C"` wrappers are themselves under test.

## Reproduce

```bash
# 1. build the C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. run the whole gate (symbols, all rows, all feature combos, mutation check)
cd translation && bash scripts/verify_gate.sh
```

Individual pieces:

```bash
cargo test --offline                       # all 74 tests
bash   scripts/check_features.sh           # every feature combo, debug + release
python3 scripts/mutation_check.py          # harness self-validation
SOAK_ITERS=500000 cargo test --offline --release --test soak -- --nocapture
```

`libloading` is vendored into `vendor/` and `.cargo/config.toml` pins
`net.offline = true`, so the suite builds with no network and no pre-populated
registry cache.

## Completion gate

| gate item | status | evidence |
|-----------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | `diff` of the two defined-symbol sets is empty for both the debug and release `.so`; `tests/phase_d_symbols.rs` enforces it, and additionally asserts the Rust `.so` does not *import* any of the three names (i.e. it is not a thunk over the C library) |
| Phase B: EVERY row in `CONFIGS.md` passes across randomized inputs | **PASS** | 35 rows ↔ 35 `#[test]` fns in `tests/phase_b_configs.rs`, cross-checked by `scripts/verify_gate.sh` |
| Phase C: EVERY row in `ERRORS.md` has a passing error-path differential test | **PASS** | 30 rows ↔ 30 `#[test]` fns in `tests/phase_c_errors.rs`, cross-checked by `scripts/verify_gate.sh` |
| All of the above under EVERY feature combination | **PASS** | `Cargo.toml` declares no `[features]`, so there is one configuration; `scripts/check_features.sh` derives the list from `Cargo.toml` and runs the suite in **debug and release** (release differs materially: `panic = "abort"`, `overflow-checks = false`) |

## What is compared

For `parse_uname_string`, every call compares:

* all **9** `os_data` pointer fields — NULL-ness *and* the full string bytes;
* whether each field was **written at all**, distinguished from "written as
  NULL" by pre-filling `os_data` with non-null sentinels (the C never zeroes
  `osd`, so an untouched field must keep the caller's value verbatim);
* the **caller's `uname` buffer** after the call, including 16 guard bytes on
  each side, so the C's out-of-bounds `*(p + strlen(p) - 1) = '\0'` write on an
  empty remainder (`lib.c:72`) is captured rather than ignored.

For `w_regexec`: the return value, plus the **entire** `regmatch_t` array
pre-filled with a sentinel (so "slot not written" is distinguishable from
"slot written as `{-1,-1}`"), plus — for the `regcomp`-failure path — the
**byte-exact `stderr` diagnostic**.

For `get_os_arch`: the returned string (or NULL) and the input buffer.

Allocation sizes are compared too, in `tests/phase_d_alloc_sizes.rs`. The C
documents that every OUT pointer must be freed by the caller, so the number of
bytes asked of `malloc` is observable. Because glibc may hand back a chunk larger
than the request when it reuses a free chunk, `malloc_usable_size` depends on
heap history and cannot be compared in-process; the test therefore measures each
side in its **own subprocess** which performs an identical allocation sequence
(both `.so`s are dlopened in both children; only the final function pointer
differs), keeping the two heap histories in lockstep.

## Harness self-validation

Matching symbols and green tests prove nothing unless the tests can actually
fail. `scripts/mutation_check.py` injects 37 deliberate bugs into
`src/lib.rs`, rebuilds the cdylib and requires the suite to fail:

```
caught=37 survived=0 skipped=0
```

35 mutants are caught. **2 are recorded as provably equivalent** and are
*required to survive* — if either were ever caught, the equivalence argument
would be wrong and the suite over-constrained:

| mutant | why no test can catch it |
|--------|--------------------------|
| `EQUIV:dup_match_snprintf_size_plus_two` | `snprintf(dst, match_size + 2, "%.*s", match_size, …)`. The `%.*s` precision caps the conversion at `match_size` bytes and `snprintf` appends exactly one NUL, so at most `match_size + 1` bytes are ever written regardless of the size argument. The `malloc` size is a separate expression and stays `match_size + 1`, so no overflow is introduced either. |
| `EQUIV:matches_array_not_zeroed` | The scratch `regmatch_t match[2]` (`lib.c:61`) is only *read* after `w_regexec` returned non-zero, and glibc's `regexec` fills every `nmatch` slot on a match, so the initial value is dead — in the C too. |

Both claims are *proven against the real C library* by
`tests/equivalence_proofs.rs`, which shows that for the three patterns the parser
uses, a match always overwrites both `pmatch` slots, and that the extracted
fields are exactly the captured group for match sizes 1..140.

## Soak

`tests/soak.rs` was additionally run at `SOAK_ITERS=500000` (≈1.5 M randomized
differential comparisons across all three entry points, in release mode) with
zero divergences. Three input generators are used: uniform bytes over eight
alphabets, separator-token splicing, and small random edits of well-formed
uname strings.

## Notable C behaviours the translation reproduces (do not "fix" these)

1. **`os_name` is the text *after* `" ["`, not before** (`lib.c:101`) — the host
   prefix is discarded on the POSIX path.
2. **Trailing byte is chopped unconditionally** (`lib.c:72,106,113,131`), on the
   assumption that it is `]` or `)`. On an empty string this writes one byte
   *before* the buffer. Reproduced with wrapping address arithmetic so the
   computed address matches C in every build profile.
3. **`lib.c:106` chops before `lib.c:109` searches for `" ("`** — so
   `"host [Ubuntu: 22.04 ("` yields **no** codename, while
   `"host [Ubuntu: 22.04 ()"` yields the empty codename. `"…(jammy)"` (no
   closing `]`) yields the truncated `"jamm"`.
4. **`os_arch` is only computed on the POSIX path** (`lib.c:142` sits inside the
   `else`), and only over the `uname` prefix, which was already truncated at
   `" ["`.
5. **`ARCHS` array order decides, not position in the string** — `"aarch64 x86_64"`
   and `"x86_64 aarch64"` both yield `x86_64`.
6. **`osd` is never zeroed** — fields the code does not reach keep whatever the
   caller had. `os_uname` is never assigned at all.
7. **`os_build` only exists on the Windows path**; the POSIX path never sets it.
8. **`w_regexec` collapses every failure to `0`** — bad pattern and no-match are
   indistinguishable to the caller (only the `stderr` line differs).
9. Neither `uname` nor `os_header` is null-checked; `strstr(NULL, …)` faults. Both
   libraries call the same libc `strstr`, so this is identical by construction —
   asserted out-of-process by `e29_null_uname_both_fault`.
