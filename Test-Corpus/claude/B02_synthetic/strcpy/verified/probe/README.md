# probe/ — verification tooling

Nothing in here is part of the shipped crate (`cargo build --release` only
compiles `src/`); these are the tools used to verify the translation against the
C code and to calibrate the model of the C program's stack frame.

| file | purpose |
|------|---------|
| `check_features.sh` | derives every build-time configuration from `Cargo.toml` and runs `cargo check --no-default-features [--features …] --all-targets` for each (there is exactly one: the crate declares no `[features]`) |
| `dump_frame.c` | ptrace helper: stops the C driver at the entry of `process_strings` and dumps `main`'s stack frame (`ref_buffer`, `input_buffer` and the locals above them). Used to produce `src/frame_junk.rs` and, from `tests/exe_frame.rs`, to re-check that snapshot against reality |
| `build_table.py` | turns 24 `dump_frame` captures into `src/frame_junk.rs` (per byte majority vote, so the zero pattern the C code branches on is reproduced) |
| `probe_zeros.py` | derives, purely from the C program's *observable output*, which bytes of the two buffers are zero — an independent cross-check of the frame model (2041 of 2046 offsets agreed; the 5 that did not are ASLR dependent pointer bytes) |
| `verify_table.py` | compares `src/frame_junk.rs` against the `probe_zeros.py` measurement |
| `inject_frame.c` | ptrace helper: overwrites the uninitialised part of the C driver's frame with the snapshot the Rust translation uses, so the executable level differential tests are deterministic and environment independent. Used by `tests/exe_frame.rs` |
| `fuzz_diff.py`, `fuzz_diff2.py` | randomised differential fuzzing of the two real executables (stdin → stdout/stderr/exit status) |
| `fuzz_inject.py` | the same, with the C driver's frame controlled by `inject_frame` |
| `fuzz_stable.py` | runs the C program several times per input and only reports a divergence when the C result is *stable*: this separates real bugs from the C program's own ASLR dependent nondeterminism |

## Reproducing the calibration

```sh
# build the C driver exactly as CMakeLists.txt does
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

gcc -O2 -o probe/dump_frame probe/dump_frame.c
BP=$(nm c_src/build/driver | awk '$3=="process_strings"{print $1}')
printf '9 0 0 0\n' > $TMPDIR/pristine.txt
mkdir -p $TMPDIR/dumps
for i in $(seq 1 24); do
    ./probe/dump_frame c_src/build/driver $BP $TMPDIR/pristine.txt 6144 2>/dev/null \
        | grep -v '^#' > $TMPDIR/dumps/d$i.txt
done
python3 probe/build_table.py > src/frame_junk.rs
python3 probe/verify_table.py src/frame_junk.rs 1023
```

## Fuzzing

```sh
cargo build --release
python3 probe/fuzz_diff2.py  c_src/build/driver target/release/driver 500 1   # real binaries
python3 probe/fuzz_inject.py c_src/build/driver target/release/driver 500 1   # controlled frame
python3 probe/fuzz_stable.py c_src/build/driver target/release/driver 400 1 5 # ignore C nondeterminism
```
