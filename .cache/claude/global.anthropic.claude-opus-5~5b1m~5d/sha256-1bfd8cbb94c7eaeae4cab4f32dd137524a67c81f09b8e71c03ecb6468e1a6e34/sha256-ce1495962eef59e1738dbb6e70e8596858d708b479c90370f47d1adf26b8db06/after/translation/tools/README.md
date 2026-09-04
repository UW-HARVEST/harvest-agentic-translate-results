# Artifact generators

These scripts regenerate the Phase A artifacts mechanically from the C source
and from the two built shared libraries. Run them from the **repository root**
(the directory that contains both `c_src/` and `translation/`):

```sh
# 1. build both libraries
(cd c_src && mkdir -p build && cd build \
   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . -j"$(nproc)")
(cd translation && cargo build --release --offline)

# 2. regenerate the artifacts
python3 translation/tools/extract2.py     # -> wk/errrows.json  (raw rejection sites)
python3 translation/tools/gen_errors.py    # -> translation/ERRORS.md
python3 translation/tools/gen_symbols.py   # -> translation/SYMBOLS.md
```

* `extract2.py` walks every `c_src/src/**.{c,h}` and extracts every
  `RETURN_ERROR(...)`, `RETURN_ERROR_IF(...)`, `return ERROR(...)`,
  `return NULL;`, `return -1;` and `... = ERROR(...)` site, joining
  continuation lines, and resolves the error code from the macro arguments.
* `gen_errors.py` turns those rows into `ERRORS.md`, assigns each row the test
  file that covers it, and ticks the row iff that test file is listed in
  `passing_tests.txt`.
* `gen_symbols.py` diffs `nm -D --defined-only` between the two `.so`s and
  writes `SYMBOLS.md`.

`CONFIGS.md` is maintained by hand from the public headers (it enumerates the
*valid* configuration surface, which cannot be derived from `return` statements).

Phase D is driven by `translation/run_all_features.sh`, which extracts the
`[features]` table from `Cargo.toml` and runs check/build/symbol-diff/test for
every combination.
