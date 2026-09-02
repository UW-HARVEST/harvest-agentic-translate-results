# Verification aids

Development tooling used to verify `translation/` against `c_src/`.  Not part
of `cargo test`; see `translation/ERRORS.md` for the findings.

A full run of the program takes ~8 min (C) / ~5 min (Rust), so the numeric and
parsing layers are checked at reduced scale here instead.  Both oracle scripts
regenerate their harness from the *real* `translation/src/main.rs`, so they
cannot drift from the shipped code.

| File | Purpose |
|---|---|
| `check_oracle.sh` + `oracle.c` | glibc `rand()` stream (20 seeds) and the 100-step arithmetic kernel (28 hostile `int32` values) |
| `check_str.sh` + `oracle_str.c` + `gen_str_cases.py` | `strtoul(arg,&endptr,10)` value / `endptr` / `ERANGE` / accept-reject decision over 113 byte-exact inputs |
| `run_pairs.sh` | 14 full-runtime C-vs-Rust comparisons, in parallel |
| `sweep.sh` | 20 further full-runtime seed comparisons |
| `probe.c`, `signs.c` | cycle structure of the arithmetic map (why `xor_result` is always non-negative) |
| `sp.c`, `sp.rs`, `sp_fixed.rs`, `plumb.rs` | minimal stand-ins that isolated the `SIGPIPE` divergence and validated the `pipe2(O_CLOEXEC)` test plumbing |
| `cargo_test_gate.log` | final `cargo test --release` run: 61 passed, 0 failed, 0 ignored |
