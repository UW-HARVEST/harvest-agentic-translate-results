# Differential verification harness

Reproduces the whole verification from scratch.  Nothing in `c_src/` is
modified; C build products go to `c_build/`.

## 1. Build the C reference (48 configurations)

```sh
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      ./build_c.sh $b $t $s
    done
  done
done
```

`build_c.sh` puts the declaration-only OpenSSL shim in
`translation/c_compat/openssl/` on the include path, because this host has
`libcrypto.so.3` but no `openssl-devel`; `c_src/app/src/rng.c` would otherwise
not compile and the `sphincs_core_det` / `driver` targets would be missing.

## 2. Compile-check every Cargo feature combination (120)

```sh
translation/check_all.sh
```

## 3. Exported-symbol parity (48 configurations)

```sh
./symsweep.sh          # sweep; writes /tmp/symsweep.txt
./symdiff.sh blake simple 128f   # one configuration, verbose
```

## 4. `driver` stdout byte-for-byte (48 configurations)

```sh
./driversweep.sh       # writes /tmp/driversweep.txt
```

## 5. Differential tests (96 configurations)

```sh
./run_backend_tests.sh blake &
./run_backend_tests.sh haraka &
./run_backend_tests.sh sha2 &
./run_backend_tests.sh shake &
wait
cat /tmp/difftests/summary_*.txt
```

Each backend gets its own `CARGO_TARGET_DIR` so the four can run in parallel.
Every configuration runs `cargo build --release` (to produce the Rust `.so` that
the tests `dlopen`) followed by `cargo test --release -- --test-threads=1`.
Single-threading is required: the tests share the process-global `DRBG_ctx`.

To run one configuration by hand:

```sh
cd translation
cargo build --release --no-default-features --features "sha2,robust,192f"
cargo test  --release --no-default-features --features "sha2,robust,192f" -- --test-threads=1
```

Environment overrides understood by `tests/common/mod.rs`:

* `SPHINCS_RUST_SO` — path to the Rust `cdylib`
  (default `translation/target/release/libsphincsplus.so`)
* `SPHINCS_C_DIR` — the `c_build/<backend>_<thash>_<secpar>` directory

## Artifacts

* `translation/SYMBOLS.md` — symbol inventory and parity result
* `translation/ERRORS.md` — error-surface table (22 rows)
* `translation/CONFIGS.md` — configuration-surface table (98 rows)
