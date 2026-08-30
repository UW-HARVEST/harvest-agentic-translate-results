# Verification harness

Scripts that drive the C-vs-Rust differential verification. All of them iterate
the **48 build configurations** (4 hash backends × 2 `thash` variants × 6
security parameters) and can be narrowed with the `BACKENDS`, `SECPARS` and
`THASHES` environment variables, e.g.

```sh
BACKENDS=blake SECPARS=128f THASHES=simple ./verif/run_tests.sh
```

| script | what it does |
|---|---|
| `build_c_all.sh` | `cmake` + build `c_src` into `c_src/build-<backend>-<secpar>-<thash>/` for all 48 configurations: `libsphincs_core.so`, `libsphincs_core_det.so`, `lib<backend>.so` and `driver`. Locates an OpenSSL for `rng.c` automatically. Never modifies `c_src`. |
| `cargo_all.sh <subcmd>` | runs `cargo <subcmd> --no-default-features --features <backend>,<thash>,<secpar>` for all 48 combinations (e.g. `./verif/cargo_all.sh check`). |
| `symbols_all.sh` | builds each Rust cdylib and diffs `nm -D --defined-only` against the union of the three C `.so` files. Raw lists land in `verif/symbols/`. |
| `run_tests.sh [cargo test args]` | builds the cdylib (`cargo test` alone does **not** refresh it) and then runs the `tests/diff_*.rs` differential suite for each configuration. |
| `driver_all.sh` | runs the C KAT driver and the Rust `driver` binary for each configuration and compares their transcript digests. |

Order for a full run:

```sh
./verif/build_c_all.sh
./verif/cargo_all.sh check
./verif/symbols_all.sh
./verif/run_tests.sh
./verif/driver_all.sh
```

Other files: `build_c_all.log` (C build log), `symbols/` (`nm` output per
configuration), `osslink/` (a `libcrypto.so` development symlink, needed because
`c_src/app/CMakeLists.txt` links the driver with a bare `-lcrypto` and this host
only ships `libcrypto.so.3`), `rng/` (a standalone `rng.c` build used while
bringing the OpenSSL dependency up).
