> **Superseded.** This directory is the earlier C-based differential harness. It
> was written against a C build with `NDEBUG` defined, which is *not* what
> `c_src/CMakeLists.txt` produces: with no `CMAKE_BUILD_TYPE` the reference
> library is compiled at `-O0` with live `assert()`s, so many malformed inputs
> abort with `SIGABRT` instead of returning an error. The authoritative
> verification is now the Rust harness in `../tests/`, which `dlopen`s both
> `.so`s and compares exit status as well as output; see `../README.md`,
> `../SYMBOLS.md`, `../ERRORS.md` and `../CONFIGS.md`.

# Differential verification tooling

These are the scripts used to verify the translation against the C library.
They are not part of the crate build (`cargo build`/`cargo test` ignore this
directory).

```sh
# 1. build the C reference (Release => -O3 -DNDEBUG, i.e. asserts off)
cmake -S ../../c_src -B /tmp/cref -DCMAKE_BUILD_TYPE=Release && cmake --build /tmp/cref
# 2. build the Rust library
(cd .. && cargo build --release)
# 3. generate the corpora
python3 gen.py                 # 895 crafted PNGs + 24 raw DEFLATE streams
python3 fuzz.py 1234 2500      # mutated PNGs
python3 inffuzz.py 99 3000     # mutated/random DEFLATE streams
# 4. build the harness twice, once against each library, and diff
gcc -O1 -o h_c harness.c /tmp/cref/lib*.so -Wl,-rpath,/tmp/cref
gcc -O1 -o h_rs harness.c ../target/release/libtranslation.so -Wl,-rpath,$PWD/../target/release
./h_c  corpus/png corpus/inflate corpus/png/ok_ct6_f5_16x16_l6.png corpus/inflate/fixed_mix > out_c.txt
./h_rs corpus/png corpus/inflate corpus/png/ok_ct6_f5_16x16_l6.png corpus/inflate/fixed_mix > out_rs.txt
cmp out_c.txt out_rs.txt
```

`harness.c` prints, for every input: the returned `cp_image_t`, `cp_error_reason`,
a hash of the whole pixel buffer, the first 64 pixels, and for `cp_inflate` the
result over 4 input alignments x 7 output sizes. It also `free()`s `img.pix`
(proving the pointer came from libc `malloc`), exercises NULL/negative/truncated
arguments, and mutates each exported table at runtime to prove the library reads
them live. Each case runs in a forked child so a crash is reported as a signal
rather than ending the run.

`compare.py c1 c2 r1 r2` compares two runs of each library and reports
mismatches, ignoring cases that are nondeterministic in the C itself (the C
hashes uninitialised `malloc` memory when an image's declared size exceeds its
data, and reads far outside the input buffer for some malformed chunk lengths).

`dlharness.c` is the same idea via `dlopen`/`dlsym` (no copy relocations), which
is also how the exported-table `.data` layout is checked:
`./dlh <lib.so> corpus/png corpus/inflate`.
