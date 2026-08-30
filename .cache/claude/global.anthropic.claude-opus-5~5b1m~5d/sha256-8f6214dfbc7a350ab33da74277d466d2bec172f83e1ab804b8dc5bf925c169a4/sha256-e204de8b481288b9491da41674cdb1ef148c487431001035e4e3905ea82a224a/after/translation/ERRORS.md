# Differential verification log

The Rust translation in this crate is compared against the original C program by
running both as subprocesses and diffing stdout, stderr and the exit status
(`tests/differential.rs`).

Reference build of the C program (this is the build the translation is
calibrated against, and the one the task instructions specify):

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
```

i.e. `gcc 11.5.0`, **no** `CMAKE_BUILD_TYPE`, so no `-O` flag, non-PIE
executable loaded at `0x400000`.

Run the two programs like this:

```
./c_src/build/driver            < input      # C
./translation/target/release/driver < input  # Rust  (cargo build --release)
```

## Mismatch found and fixed

### 1. Out-of-bounds table reads *below* `stb__perlin_randtab` returned zeroes

* **Symptom** — for example

  ```
  printf '5 -133797.29085470736 0.5 -542121.4288026612 -164 -955542 197 5 INF -1e+38 355948.8906873644 1' | driver
  ```

  | | stdout | exit |
  |---|---|---|
  | C | `-0.0777234733` | 0 |
  | Rust (before) | `-0.141512632` | 0 |

  and, for the deeper reads,

  ```
  printf '5 -20.5 0.5 0.5 -30 1 1 0 0 0 0 0' | driver
  printf '5 -5000.5 0.5 0.5 -9000 1 1 0 0 0 0 0' | driver
  ```

* **Cause** — `stb_perlin_noise3_wrap_nonpow2` computes
  `x0 = px % x_wrap2; if (x0 < 0) x0 += x_wrap2;`. With a *negative* wrap
  argument, `x0` stays negative (C's `%` truncates towards zero, so the sign
  fix-up adds a negative number), and `stb__perlin_randtab[x0]` then reads
  *before* the table. The same happens for `y0`/`z0` and, indirectly, for
  `stb__perlin_randtab_grad_idx[r00+z0]`.

  The Rust memory model returned `0` for every byte below the table. In the real
  C process those bytes are not zero: `.data`, `.got.plt`, `.got`, `.dynamic`,
  `.eh_frame`, `.rodata` and the machine code all lie below
  `stb__perlin_randtab` (which the reference build places at `0x405040`, i.e.
  20544 bytes above the start of the image at `0x400000`).

* **Fix** — `src/c_data_image.bin` is a byte-for-byte copy of `0x400000 ..
  0x406000` of the reference C process, captured from `/proc/<pid>/mem` while it
  was blocked inside `scanf` (exactly the state the noise routines observe).
  `mem()` in `src/stb_perlin.rs` now reads that image for every offset outside
  the three tables instead of inventing zeroes. Reads inside the tables still go
  through the explicit `tables.rs` constants, and the unit test
  `stb_perlin::tests::data_image_agrees_with_the_tables` asserts that the blob
  agrees with them where they overlap (so a blob taken from a different build is
  caught). `tools/capture_c_data_image.sh` regenerates it.

## Verified-correct behaviour that looks like a bug (kept deliberately)

These were checked against the C program and are *not* mismatches; they are
listed so the next reader can re-check them.

* **`scanf` never checks its return value.** Any conversion failure leaves the
  remaining variables at their initialisers (`0` / `0.0f`) and the program still
  prints a result and exits 0. Empty input therefore prints `0`
  (`which == 0`, all coordinates `0.0f`).
* **`scanf` reads across newlines.** `%d`/`%f` skip any amount of leading
  whitespace, so `"0\n1\n2\n..."` and `"0 1 2 ..."` are the same input.
* **glibc `%f` quirks that the translation reproduces.**
  * `inf`, `infinity`, `nan` (any case) are accepted; a partial `infinity`
    longer than `inf` (`infi`, `infin`, `infinit`) is a *matching failure*.
  * `nan(chars)` is not accepted by `scanf`; the payload is left unread.
  * A malformed exponent (`1e`, `1e+`, `0x1p`) still consumes the `e`/`p` and
    its sign — glibc can only push one character back — but converts only the
    mantissa.
  * `0x` with no hex digits is a matching failure, and hex floats
    (`0x1.8p1`) are accepted.
* **`%d` saturates then truncates.** glibc converts with `strtol`, so
  `99999999999999999999999` saturates to `LONG_MAX` and is then truncated to
  `int` → `-1`. `4294967296` truncates to `0`.
* **`(int) a` on out-of-range / NaN floats** yields `INT_MIN` (x86-64
  `cvttss2si`), which is what `stb__perlin_fastfloor` propagates. Modelled by
  `f32_to_int`.
* **`INT_MIN % -1` traps.** `x_wrap == -1` together with a coordinate whose
  `fastfloor` is `INT_MIN` (`-1e30`, `nan`, `inf`, …) kills the C program with
  **SIGFPE**; the Rust program raises the same signal (`irem` →
  `raise_sigfpe`).
* **Reads that leave the mapped pages kill the process.** Relative to
  `stb__perlin_randtab` the mapped image spans `[-20544, 4032)`; the first read
  outside it dies with **SIGSEGV** (e.g. `5 4031.5 0.5 0.5 2147483647 1 1 0 …`,
  or `5 -21000.5 0.5 0.5 -22000 1 1 0 …`). `mem()` raises SIGSEGV for exactly
  that range.
* **`printf("%.9g")` formatting**, including `-0`, `nan`, `-nan`, `inf`,
  `-inf`, the two-digit exponent (`1.7673824e-07`) and the stripping of
  trailing zeros. `res` is a `float` promoted to `double`, so 9 significant
  digits always round-trip.
* **NaN sign/payload propagation** matches because every float operation goes
  through `src/sse.rs`, which models `addss`/`subss`/`mulss` operand order
  (`dst`'s NaN wins) and the `0xFFC00000` "indefinite" result of invalid
  operations on non-NaN operands.

## Inputs that cannot be tested (the C program is not deterministic)

A handful of bytes inside the mapped image hold pointers into libc / ld.so
(`.got`, `.got.plt`, one `.dynamic` entry). ASLR randomises them on every run,
so an out-of-bounds read that lands on them makes the *C program itself*
nondeterministic. Example:

```
printf '5 -82114 inf -1.694670431868586 -45 -257 -275 286 0 0 0 0' | ./c_src/build/driver
```

over 60 runs: 47 times `-nan` with exit 0, 13 times death by SIGSEGV — the
garbage byte read at offset `-79` (inside `.got.plt`) selects a gradient index
that sometimes points past the last mapped page.

Such inputs are excluded from the test suite: no translation can match them.
`tests/differential.rs::generated_sweep` therefore keeps the `which == 5` wrap
arguments in `0..=256`, and the explicit out-of-bounds cases in
`nonpow2_wrap` were each verified to give the same result over 16 consecutive
runs of the C program.

## Coverage

`cargo test` runs 8 differential tests covering ~330 distinct inputs:

| test | input classes |
|---|---|
| `input_shapes_and_scanf_failures` | empty input, 1..12 supplied fields, trailing junk, separators (space/tab/newline/CR/VT/FF), `%d` matching failures, `int`/`long` overflow and truncation |
| `float_conversions` | every glibc `%f` acceptance and failure path: signs, bare/leading/trailing dot, exponents (well- and malformed), hex floats, `inf`/`infinity`/`nan` and their partial spellings, overflow/underflow, denormals |
| `which_dispatch` | `which` = 0..5, 6, 7, negative, `INT_MIN`, `INT_MAX` (the `default: return NAN` arm) |
| `noise3_and_seeded_noise3` | wrap masks (0/1/2/pow2/non-pow2/negative/`INT_MIN`/`INT_MAX`), seed truncation to `unsigned char`, `fastfloor` on NaN/inf/huge/`2^31`/`2^24`, results printing `0`, `-0`, `nan`, `inf` |
| `fractal_noise_loops` | `octaves` = 0, 1, negative, 255/256/257/260/300 (the `(unsigned char) i` wrap), zero/negative/NaN/inf `lacunarity`, `gain`, `offset`, overflowing sums, missing parameters |
| `nonpow2_wrap` | `wrap ? wrap : 256` fallbacks, `x0 < 0` fix-ups, in-bounds and out-of-bounds table indices above and below the tables, the SIGSEGV boundary, the SIGFPE (`INT_MIN % -1`) paths |
| `non_text_input` | NUL bytes, non-UTF-8 bytes, escape sequences |
| `generated_sweep` | 260 deterministically generated combinations of coordinates × wraps × seeds × fractal parameters × field counts × separators |

Beyond the checked-in suite, the two binaries were compared on ~20000 randomly
generated inputs (both the debug and the release build of the Rust program) (including a mode that deliberately drives the non-pow2 wrap
indices out of bounds); after the fix above the only remaining differences were
the ASLR-nondeterministic inputs described in the previous section.
