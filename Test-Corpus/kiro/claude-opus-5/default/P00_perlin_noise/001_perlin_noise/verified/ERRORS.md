# Mismatches found between `c_src/` and this translation

The C program in `c_src/` is the ground truth. Everything below was found by
building both programs and running them side by side on the same stdin, then
comparing stdout, stderr and the exit status.

How to reproduce:

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # C
cd translation && cargo build --release                                # Rust
cd translation && cargo test                                           # compare
```

`tests/differential.rs` runs both binaries as subprocesses over 15 519 inputs in
25 input classes. `tests/golden.rs` pins the same behaviour against outcomes
recorded from the C program, so a regression is still caught when `c_src/`
cannot be built.

## 1. `INT_MIN % -1` exited with code 136 instead of dying of `SIGFPE`

**Inputs.** `which == 5` with a wrap of `-1` and a coordinate whose
`stb__perlin_fastfloor` result is `INT_MIN` — that is, any NaN, any infinity, or
any magnitude at or beyond 2^31, because `cvttss2si` yields `INT_MIN` for all of
them. For example:

```
5 nan 0.5 0.5 -1 0 0 0 0 0 0 0
5 0.5 1e20 0.5 0 -1 0 0 0 0 0 0
5 0.5 0.5 -2147483649 0 0 -1 0 0 0 0 0
```

**C behaviour.** `stb_perlin_noise3_wrap_nonpow2` computes `px % x_wrap2`. With
`px == INT_MIN` and `x_wrap2 == -1` the quotient is not representable, so the
`idiv` gcc emits at `-O0` raises `#DE`. The process is *killed by* `SIGFPE`
having printed nothing: empty stdout, empty stderr, `WIFSIGNALED` with signal 8.

**What the translation did.** It detected the case correctly but reported it with
`std::process::exit(136)`. A shell prints `136` for both outcomes, which is why
this hid for a while, but they are different wait statuses: `waitpid` reports
`WIFEXITED`/code 136 for the exit and `WIFSIGNALED`/signal 8 for the C program.
Any harness that inspects the exit *status* rather than a shell's rendering of it
sees the difference:

```
$ printf '5 nan 0.5 0.5 -1 0 0 0 0 0 0 0' | ./c_src/build/driver ; # returncode -8
$ printf '5 nan 0.5 0.5 -1 0 0 0 0 0 0 0' | ./translation/.../driver ; # returncode 136
```

**Fix.** `src/csig.rs`: reset the signal disposition to `SIG_DFL` and `raise` the
signal, so the Rust process dies the same way. Resetting first is necessary
because the Rust runtime installs its own `SIGSEGV`/`SIGBUS` handler for stack
overflow reporting, and a handled signal would not terminate the process.

## 2. Table reads past the last mapped page printed a number instead of dying of `SIGSEGV`

**Inputs.** `which == 5` with a wrap larger than the tables, which makes
`x0 = px % x_wrap2` exceed the 512-entry `stb__perlin_randtab`. A wrap of
2 000 000 000 makes the index equal to `floor(x)`, so these select one exact
offset each:

```
5 4030.5 0.5 0.5 2000000000 0 0 0 0 0 0 0     -> C prints -0.25
5 4031.5 0.5 0.5 2000000000 0 0 0 0 0 0 0     -> C dies of SIGSEGV
5 0.5 0.5 100000.5 0 0 65536 0 0 0 0 0        -> C dies of SIGSEGV
```

**C behaviour.** The reads are out of bounds — undefined behaviour — but in the
compiled program they are ordinary loads at fixed addresses, and the non-PIE
image places the objects deterministically:

```
0x400000  first mapped page of the program image
0x405020  .data begins: 32 bytes of zero padding
0x405040  stb__perlin_randtab           512 bytes
0x405240  stb__perlin_randtab_grad_idx  512 bytes
0x405440  basis.0                       192 bytes
0x405500  _edata; .bss and the rest of the page read as zero
0x406000  first unmapped page
```

So an index into `randtab` reads real table data, then the gradient-index table,
then `basis`, then zeros — and past `0x406000` the process dies of `SIGSEGV`.
Both edges were measured against the C program: index 4030 is the last one that
works and 4031 is the first that faults (4031 rather than 4032 because the
function also reads `randtab[x0 + 1]`), and on the low side −20544 works while
−20545 faults, matching `0x405040 ± ` the mapping bounds exactly.

**What the translation did.** It modelled the three tables and the padding
faithfully but treated *everything* outside that 1 248-byte window as zero, so it
printed a value where the C program crashed.

**Fix.** `src/stb_perlin.rs`: the image model now carries the bounds of the
mapped address range (`OFF_MAP_LO`/`OFF_MAP_HI`, i.e. `[0x400000, 0x406000)`
expressed relative to `&stb__perlin_randtab`). `data_byte` raises `SIGSEGV`
outside them, and still returns zero for the mapped-but-empty region above
`_edata`. This turned 497 of the 528 signal deaths in the out-of-bounds corpus
into exact matches; the remainder are covered by the known limitation below.

## 3. Index arithmetic panicked in a debug build where the C wraps

**Inputs.** The same `which == 5` out-of-range wraps as above. Only visible in a
debug build, which is what `cargo test` uses — the release profile wrapped
silently, so this class passed under `cargo build --release` and would have
failed under `cargo test`.

**C behaviour.** `stb__perlin_randtab[r0 + y0]` and
`stb__perlin_randtab_grad_idx[r00 + z0]` add an `int` index that can be near
`INT_MAX`/`INT_MIN` to a table byte. gcc at `-O0` wraps.

**What the translation did.** Used plain `+` on `i32`, which panics with
"attempt to add with overflow" when debug assertions are on.

**Fix.** `src/stb_perlin.rs`: all index arithmetic and the fractal octave counter
now use `wrapping_add`, so the behaviour no longer depends on the build profile.
The whole corpus is checked against both `target/release/driver` and
`target/debug/driver`.

## Known limitation: reads *below* `.data` cannot be reproduced, and the C program does not reproduce them either

**Inputs.** `which == 5` with a *negative* wrap and a coordinate that makes
`px` negative. `x0 = px % x_wrap2` is then negative, and the `if (x0 < 0) x0 +=
x_wrap2` fixup makes it *more* negative rather than less, leaving `x0` in
`(2*wrap, wrap]`. Once the index is below −32 it reaches past the zero padding at
the start of `.data` into `.got.plt`, `.got`, `.dynamic` and the read-only
segment before them.

Those bytes are produced by the linker and the dynamic loader, not by the C
source, so no translation of `stb_perlin.h` can predict them. More to the point,
**the C program is not deterministic for these inputs**: `.got.plt` holds
addresses inside `ld.so`, which ASLR relocates on every run. Eight consecutive
runs of one input give eight different answers:

```
$ for i in $(seq 8); do printf '5 -1746.157 1415.769 1959.224 -838 209 -928 210 0 0 0 0' \
      | ./c_src/build/driver; done
-0.269727886
0.0150961876
-0.133434951
-0.0406534076
0.118725419
0.368001729
0.193987966
-0.0793716908
```

15 of 400 randomly generated negative-wrap inputs behave this way. There is no
byte-identical output to match, so these inputs are deliberately excluded from
the test corpus. Everything the C source *does* determine is modelled: indices
from −32 (the start of `.data`) up to 4031 (the end of the mapped page) are
reproduced exactly, which `tests/differential.rs` checks offset by offset.

The negative-wrap cases that stay inside the zero padding — wraps of −1 through
−16, where `x0` never drops below −32 — are included and do match.

## Verified with no mismatch

These were audited against the C and confirmed already correct; the tests now pin
them so they stay that way.

- **`scanf` reads across newlines.** `%d`/`%f` skip leading whitespace, so the
  12 conversions happily span lines, blank lines, `\v`, `\f` and `\r`.
- **`scanf`'s return value is ignored.** Any conversion that fails stops the
  scan and leaves that variable and all later ones at their `0`/`0.0f`
  initialisers, so `printf` still runs. Every prefix length of the 12 tokens is
  covered, as is a matching failure in each of the 12 slots.
- **glibc `%f` quirks.** An incomplete exponent is consumed but contributes
  nothing (`1e` and `1e-` convert to 1); `inf` and `infinity` are accepted but
  any partial prefix (`in`, `infi`, `infinit`) is a matching failure; `nan` is
  accepted while the `nan(n-char-sequence)` form is not, so `nan(x)` leaves
  `(x)` unread; hex floats (`0x1.8p+1`, `0x.8p1`) are accepted while a bare `0x`
  is a matching failure.
- **`%d` overflow.** glibc saturates to `LONG_MAX`/`LONG_MIN` and the result is
  then truncated to `int`, so `99999999999999999999` becomes −1 and
  `2147483648` becomes `INT_MIN`.
- **`(int)` conversion of out-of-range floats.** `cvttss2si` yields `INT_MIN`
  for NaN and for anything that does not fit, in both directions.
- **NaN sign.** `addss`/`subss`/`mulss` return the *destination* operand when it
  is a NaN, which decides whether `printf` writes `nan` or `-nan`. The
  translation mirrors the operand order gcc emits at `-O0` (`src/sse.rs`), and
  all 10 368 combinations of specials across the six `which` arms agree.
- **`printf("%.9g")`.** `%e` style when the decimal exponent is `< -4` or
  `>= 9`, `%f` style otherwise, trailing zeros stripped, no trailing point, and
  `-0`, `nan`, `-nan`, `inf`, `-inf` spelled the way glibc spells them.
- **Truncation of `seed` and of the octave counter** to `unsigned char`, so both
  fold every 256 and negative values fold the same way.
- **`octaves <= 0`** never enters the fractal loop, so the result is `0.0f`.
- **The two constant tables and `basis`** were compared entry by entry against
  `stb_perlin.h`: both 512-entry tables consist of two identical 256-entry
  halves, matching the base tables in `src/stb_perlin.rs`, and all 12 `basis`
  rows match with the unwritten fourth column zero.

## Coverage of the C source

The enumerated inputs were replayed through a gcov-instrumented build of the C
(configured out-of-tree so `c_src/` is untouched):

```
c_src/src/main.c        lines 100.00% of 33   branches 100.00% of 7   (all taken)
c_src/src/stb_perlin.h  lines 100.00% of 121  branches 100.00% of 20  (all taken)
```

Every line and both directions of every branch in the C program are exercised by
the corpus.

Beyond the checked-in corpus, 43 065 hand-enumerated inputs and 40 000
randomised ones (including random 32-bit float bit patterns fed in as decimal
text) were compared against both the release and the debug Rust binary with no
remaining differences outside the documented limitation.
