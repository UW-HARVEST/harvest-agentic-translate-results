# verify/ — auxiliary verification scripts

The authoritative verification is the Rust integration-test suite
(`cargo test`, see `tests/`), which loads **both** shared objects through
`libloading` and compares them. These scripts are supporting tools used to build
and audit that suite; they are not required for `cargo test` to pass.

Both `.so` files are produced by `cargo test` itself:

* C reference: `target/c_build/libtranslated_rust.so`
  (equivalently `cmake -S c_src -B target/c_build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build target/c_build`)
* Rust: `target/cdylib_build/debug/libpinflate_lib.so` (or `cargo build` →
  `target/debug/libpinflate_lib.so`)

Run everything with `ulimit -c 0`: hostile input legitimately aborts both
libraries, and core dumps make the sweeps very slow.

| script | purpose |
|---|---|
| `feature_matrix.sh` | **Phase A/D.** Reads `[features]` out of `Cargo.toml` and the configuration knobs out of `c_src/CMakeLists.txt`, enumerates the powerset, and runs `cargo check` + `cargo test` for every combination. Fails loudly if a feature is ever added without extending the matrix. |
| `mutation_check.sh` | **Suite sensitivity.** Injects 17 one-line divergences into `src/lib.rs` and requires the suite to fail for each. Currently 17/17 detected, 0 blind spots. This is what proves the differential tests are not vacuous. |
| `validonly.py` | 3 900 real zlib-produced streams (levels 0/1/2/6/9 × 5 strategies × 13 payloads up to 70 000 B × 4 input × 3 output alignments). |
| `probe_fork.py` | Fork-isolated randomized fuzz (mutated / truncated / random / maximal-dynamic-header inputs), comparing return value, `cp_error_reason`, output digest, and fatal signal. `SIGALRM` bounds the non-terminating cases. |
| `find_asserts.py` | Classifies C aborts by `lib.c:<line>` to find a witness input for each live `assert()`. Produced the witnesses used by `tests/phase_c_errors.rs`. |
| `hunt.py` | Narrower, shape-directed version of the same hunt. |
| `construct_e7.py` | Simulates the C bit reader over a *request schedule* to construct an input that reaches `cp_ptr`'s `assert(!(s->bits_left & 7))` — the reader's `count == -consumed (mod 8)` invariant only breaks via `cp_peak_bits`' final-word branch, which this enumerates. |
| `difftest.py`, `fuzzdiff.py`, `crashcheck.py`, `oob_check.py`, `parity.py`, `dump_case.py` | Earlier-generation sweeps kept for reproducibility. |

Typical session:

```sh
cd translated_rust
ulimit -c 0
cargo test                                    # the real suite
bash verify/feature_matrix.sh                 # every build configuration
bash verify/mutation_check.sh                 # prove the suite has teeth
python3 verify/validonly.py  target/c_build/libtranslated_rust.so target/debug/libpinflate_lib.so
python3 verify/probe_fork.py target/c_build/libtranslated_rust.so target/debug/libpinflate_lib.so 7 400
```
