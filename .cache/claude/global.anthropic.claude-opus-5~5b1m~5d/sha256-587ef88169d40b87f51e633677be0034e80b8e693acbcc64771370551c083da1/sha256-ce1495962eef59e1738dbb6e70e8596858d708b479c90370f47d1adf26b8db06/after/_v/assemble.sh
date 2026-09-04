#!/bin/bash
# Assemble ERRORS.md / CONFIGS.md in the crate root from the per-area fragments
# produced during verification.
W=$HARVEST_WORKDIR
AREAS=$(ls $W/_v/configs/*.md 2>/dev/null | xargs -n1 basename 2>/dev/null | sed 's/\.md$//' | sort)

{
cat <<'HDR'
# CONFIGS.md — configuration-surface table (valid inputs)

Mechanically derived from the C sources in `c_src/libsodium/` (the axes each
`if`/`switch`/parameter actually makes the C branch on), one row per meaningful
combination of options × input shape that the C treats differently.

Every row is exercised by a differential test in `tests/<area>.rs` that calls
**both** the C `libsodium.so` and the Rust `liblibsodium.so` through
`libloading` and compares return values and full output buffers byte-for-byte,
over many randomized inputs with a fixed seed.

`[x]` = row passes (C output == Rust output for every randomized input).

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section**: the crate has no
optional features and no `cfg`-gated code, so there is exactly ONE build
configuration (`--no-default-features` and any `--features` combination are
identical to the default). Verified with:

```
$ grep -c '^\[features\]' translation/Cargo.toml   # -> 0
$ cargo test --release --no-default-features        # same result as default
```

The C build likewise defines no `HAVE_*` macros (see `c_src/CMakeLists.txt`), so
the portable/reference implementation is the only code path in both libraries;
`sodium_runtime_has_*()` returning identical values in both `.so`s is asserted in
`tests/sodium.rs`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
HDR
for a in $AREAS; do cat $W/_v/configs/$a.md; echo; done
} > $W/translation/CONFIGS.md

{
cat <<'HDR'
# ERRORS.md — error-surface table (rejections / failure returns)

Mechanically derived by grepping every `.c` file under `c_src/libsodium/` for
every distinct rejection site: `return -1`, `return NULL`, `goto` to an error
label, `ARGON2_*` / `ESCRYPT` error codes, `errno = ...`, explicit range/size
checks, `_MIN`/`_MAX` constant checks, `assert(...)`, `abort()` and
`sodium_misuse()`.

Each row has a differential test that constructs exactly that invalid input,
calls **both** libraries through `libloading`, and asserts they return the SAME
value (exact code/sentinel, not merely "both failed").

`[x]` = row has a passing differential test.
`[abort]` = the C path calls `sodium_misuse()`/`abort()`, which cannot be
executed in-process without killing the test binary; equivalence verified by
source inspection instead (noted per row).

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
HDR
for a in $AREAS; do cat $W/_v/errors/$a.md 2>/dev/null; echo; done
} > $W/translation/ERRORS.md

echo "areas: $(echo $AREAS | tr '\n' ' ')"
echo "CONFIGS.md rows: $(grep -c '^| ' $W/translation/CONFIGS.md)"
echo "ERRORS.md rows:  $(grep -c '^| ' $W/translation/ERRORS.md)"
echo "unchecked config rows: $(grep '^| ' $W/translation/CONFIGS.md | grep -c '\[ \]')"
echo "unchecked error rows:  $(grep '^| ' $W/translation/ERRORS.md | grep -c '\[ \]')"

# NOTE: the "## Summary" footers currently at the end of translation/{ERRORS,CONFIGS}.md
# were written by hand after the last assemble run; re-running this script drops them.
