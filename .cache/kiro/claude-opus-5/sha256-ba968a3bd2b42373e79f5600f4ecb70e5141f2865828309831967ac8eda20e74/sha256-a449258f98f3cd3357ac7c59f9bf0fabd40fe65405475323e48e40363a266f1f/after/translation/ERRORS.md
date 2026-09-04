# Mismatches found while verifying the translation

Every entry below is a difference that was observed between
`c_src/build/driver` and `translation/target/release/driver` on the same stdin,
together with what caused it and what changed in the Rust code. The C program
was never modified; where it does something surprising, the Rust program was
changed to do the same thing.

How the comparison was done: build both, feed both the same bytes on stdin, and
compare stdout, stderr and the wait status. Roughly 2,250 generated inputs were
used during the hunt, plus 4,007 inputs aimed only at the `scanf` layer and
2,014,848 float values aimed only at the `printf` layer.

---

## 1. `stb_perlin_noise3_wrap_nonpow2` reads far outside the tables

**Symptom.** 115 of 694 inputs with `which == 5` disagreed. Example:

```
in : 5 900.5 900.25 900.125 1000 1000 1000 0 0 0 0 0
C  : 0.146484375
Rust: 0                       (before the fix)
```

**Cause.** Unlike `stb_perlin_noise3_internal`, which masks every coordinate
with `& 255` and therefore can only produce indices `0..=510`, the non-power-of-
two variant computes

```c
int x0 = px % x_wrap2, ...
r0 = stb__perlin_randtab[x0];
...
r00 = stb__perlin_randtab[r0+y0];
n000 = stb__perlin_grad(stb__perlin_randtab_grad_idx[r00+z0], ...);
```

`x0`, `y0` and `z0` are bounded only by the caller's `*_wrap` arguments, so the
indices routinely run off the ends of the 512-byte tables, and the resulting
`grad_idx` (an arbitrary `unsigned char` rather than one of the table's `0..=11`
values) then indexes `basis[12][4]` out of range too. The C program reads
whatever the linker placed next.

The translation had been written to return `0` for any out-of-range index. That
is a reasonable thing to do and it is *not* what the C does.

**Fix.** `src/memimage.rs` embeds the byte image of the mapped range
`[0x400000, 0x406000)` of the compiled C program, and `src/perlin.rs` reads the
tables out of that image using the same address arithmetic the compiler emits
(`base + (i64)(int)index`). The driver is a **non-PIE** executable, so these
addresses are fixed and the reads are reproducible. `noise3_internal` keeps
using the plain transcribed tables, with a `debug_assert!` recording that its
indices are provably in range.

## 2. Out-of-range indices that are far enough out kill the C process

**Symptom.**

```
in : 5 4032 0.25 0.125 4033 0 0 0 0 0 0 0
C  : no output, killed by SIGSEGV
Rust: 0                       (before the fix)
```

**Cause.** The writable segment mapping ends at `0x406000` and the read-only
segments start at `0x400000`, so `stb__perlin_randtab[i]` is readable exactly
for `i` in `-20544 ..= 4031`. Anything outside faults. Boundaries were
confirmed by probing the C program: index `4031` prints a value, `4032` faults;
`-20544` prints a value, `-20545` faults.

**Fix.** `src/trap.rs::sigsegv()` provokes a genuine `SIGSEGV` (a volatile read
through a `black_box`-hidden bad pointer) rather than calling `exit`. Death by
signal and `exit(139)` are different wait statuses, and the tests compare the
wait status.

## 3. `INT_MIN % -1` raises `SIGFPE`

**Symptom.**

```
in : 5 -2147483648 0.5 0.5 -1 1 1 0 0 0 0 0
C  : no output, killed by SIGFPE (status 136)
Rust: 0                       (before the fix)
```

**Cause.** `px % x_wrap2` becomes `idiv`, which raises `#DE` when the *quotient*
is unrepresentable — `-INT_MIN` overflows — even though the remainder would be
`0`. Rust's `%` panics on that input and `wrapping_rem` quietly returns `0`;
neither matches.

`px == INT_MIN` is reachable because `stb__perlin_fastfloor` is
`cvttss2si`, which yields `INT_MIN` for NaN and for anything out of range, and
because `x = -2147483648` floors to exactly `INT_MIN`.

**Fix.** `perlin::crem` checks for `INT_MIN % -1` (and for a zero divisor) and
calls `trap::sigfpe()`, which executes a real trapping `idiv`.

## 4. NaN sign: the final `addss` in `stb__perlin_grad` has its operands swapped

**Symptom.** Nine of the remaining mismatches were a NaN sign flip, e.g.

```
in : 0 -31 -nan NaN 1187515947 -5 -17 -200 1.23458773e+19 0.824976073 67.516731 2
C  : nan
Rust: -nan                    (before the fix)
```

**Cause.** `printf("%.9g")` prints `-nan` for a NaN with the sign bit set, so
NaN *sign* is observable, and on x86 `addss`/`subss`/`mulss` return the first
source operand when it is a NaN and only otherwise the second. The translation
already modelled that, but it had the operand order of one expression wrong.
For `grad[0]*x + grad[1]*y + grad[2]*z`, gcc keeps the running sum in `xmm1`,
computes `grad[2]*z` into `xmm0`, and emits `addss xmm0, xmm1` — the *product*
is the first source of the final add, not the partial sum:

```asm
mulss  xmm1, [rbp-0x18]   ; grad[0]*x
mulss  xmm0, [rbp-0x1c]   ; grad[1]*y
addss  xmm1, xmm0         ; fadd(g0*x, g1*y)
mulss  xmm0, [rbp-0x20]   ; grad[2]*z
addss  xmm0, xmm1         ; fadd(g2*z, partial)   <-- this way round
```

**Fix.** `perlin::grad_dot` now computes `fadd(fmul(g2, z), partial)`. The same
reading of the disassembly also corrected the accumulation order in all three
fractal functions, which are `fadd(<this octave>, sum)` and not
`fadd(sum, <this octave>)`:

```asm
mulss  xmm0, [rbp-0xc]    ; noise * amplitude
movss  xmm1, [rbp-0x10]   ; sum
addss  xmm0, xmm1         ; fadd(noise*amplitude, sum)
```

and the operand order inside the `stb__perlin_ease` macro, where gcc puts the
literal first (`mulss xmm0(=6), xmm1(=a)`, `addss xmm0(=10), xmm1`). The ease
change is cosmetic — only one operand there can ever be a NaN — but it is now
faithful.

`stb__perlin_lerp` was already correct: `subss xmm0(=b), a`,
`mulss xmm0, t`, `addss xmm0, a`, i.e. `fadd(fmul(fsub(b, a), t), a)`.

## 5. `(float) fabs(r)` is a sign-bit clear, not a `double` round trip

**Symptom.** Included in the NaN-sign group above, e.g.

```
in : 4 nan 0.25 0.125 0 0 0 0 2 0.5 0 1
```

**Cause.** The translation implemented `(float) fabs(r)` literally, as
`((r as f64).abs()) as f32`. gcc folds the whole thing into one instruction:

```asm
movss  xmm1, [rip+...]    ; 0x7fffffff
andps  xmm1, xmm0
```

The `f64` round trip differs from a bit-and for NaN payloads (`cvtsd2ss` quiets
a signalling NaN), which is not visible through `%g`, but writing it as a
bit-and is exactly right and cheaper.

**Fix.** `perlin::fabsf` is `f32::from_bits(r.to_bits() & 0x7fff_ffff)`.

## 6. Three slots in the process image that the file does not contain

**Symptom.** After fixes 1–5, three inputs still disagreed:

```
in : 5 0.030039715 -199.998690 -122.666708 256 -227 8 39 260.068491 0x1p1000 NaN 2
C  : 0.123439044      Rust: 0.123439014
in : 5 3.9209065e+19 215 -0.364172032 -61 -47 157 256 -8.06055017e+19 0x1p4 1.38143752 -1
C  : inf              Rust: -nan
in : 5 0.873111399 -0.237581491 -72 -155 -256 -279 72 -277.855425 -79.630795 -nan 0
C  : 0                Rust: killed by SIGSEGV
```

**Cause.** The image had been reconstructed from the ELF file, but the dynamic
linker overwrites several slots before `main` runs, and negative table indices
land on them. The affected bytes are the `.dynamic` `DT_DEBUG` pointer, the
`.got` entry for `__libc_start_main`, `.got.plt[1..2]` (the `link_map` pointer
and `_dl_runtime_resolve`), and the `.got.plt` slot for `__isoc99_scanf` — the
last of which is resolved because `scanf` is called *before* the noise code
runs, while the `printf` slot is still the unresolved PLT stub and so does match
the file.

**Fix.** The constant bytes of those pointers were recovered from the running C
program and baked into the image (see "how the image was checked" below). They
cross-check against the symbol tables of the system libraries: the recovered
low bytes of `&__libc_start_main`, `&__isoc99_scanf`, `&_r_debug` and
`&_dl_runtime_resolve_xsavec` are `0x40 0xa6`, `0x60 0x53`, `0x58` and `0x70`,
matching `readelf -s` offsets `0x2a640`, `0x55360`, `0x39358` and `0x14d70`.

---

## The one thing that cannot be matched

19 bytes of the image carry ASLR entropy — the middle bytes of the four
relocated pointers listed in item 6. In `randtab` index terms:

| index | address | contents |
|---|---|---|
| `-383 ..= -379` | `0x404ec1..=0x404ec5` | `.dynamic` `DT_DEBUG` → `&_r_debug` in `ld.so` |
| `-118 ..= -116` | `0x404fca..=0x404fcc` | `.got` → `&__libc_start_main` in libc |
| `-79 ..= -76` | `0x404ff1..=0x404ff4` | `.got.plt[1]` → `struct link_map *` |
| `-71 ..= -68` | `0x404ff9..=0x404ffc` | `.got.plt[2]` → `&_dl_runtime_resolve` |
| `-54 ..= -52` | `0x40500a..=0x40500c` | `.got.plt` → resolved `&__isoc99_scanf` |

For the handful of inputs whose table indices reach those bytes, **the C program
does not agree with itself**. For instance
`5 -100.5 -100.25 -100.125 -256 -256 -256 0 0 0 0 0` printed `0.330785155`,
`0.220523417` and `0` on three consecutive runs and died of `SIGSEGV` on a
fourth, because a garbage `grad_idx` sometimes points past the end of the
mapping. No translation can reproduce a program whose output is a function of
the address-space layout, so the test suite does not assert on those inputs;
`tests/differential.rs` keeps the sweep's `which == 5` wrap values
non-negative, which makes every table index non-negative and therefore keeps
the sweep away from the region entirely.

Out of 24,576 image bytes, these 19 are the only ones that differ from what the
C process actually reads.

---

## How the image was checked

The C program was turned into an oracle for its own memory. For

```
which=5, x=<n>, y=0.25, z=0.125, x_wrap=<n+1>, y_wrap=z_wrap=0, seed=<s>
```

the only unusual read the driver performs is `stb__perlin_randtab[n]`
(`n % (n+1) == n` and `(n+1) % (n+1) == 0`, and the same holds for `x = -1`
with `x_wrap = n+1` when `n <= -3`), and the printed value is a function of that
single byte. Tabulating the model over all 256 candidate bytes and matching the
driver's actual output recovers the byte the C process really saw; repeating with
several `seed` values resolves the few collisions and repeating each probe
identifies the bytes that change between runs.

Sweeping every index in `-20544 ..= -3` and `1 ..= 4031` — the whole reachable
range — reported **0 anomalies** outside the 19 ASLR bytes above.

The other two emulation layers were checked the same way, against small C
programs that call the same libc functions:

* `%.9g`: 2,014,848 `float` values (every biased exponent crossed with a spread
  of mantissas, all 4,096 smallest subnormals of both signs, every power of two,
  and 2,000,000 random bit patterns) produced byte-identical output.
* `scanf("%d%f%f%f%d%d%d%d%f%f%f%d")`: 4,007 inputs (each tricky token in each
  of the twelve positions, hex floats, partial `inf`/`nan` spellings, `%d`
  overflow, whitespace variations, random token soup) produced the same twelve
  values and the same return count.

## Things that were already right

Worth recording, because they are the parts a translation usually gets wrong and
these did not need changing:

* `scanf` skipping newlines, so one call reads fields spread over many lines,
  and stopping at the first matching failure with every later variable left at
  its zero initialiser.
* glibc's `%d` being `(int) strtol(...)`, so `12345678901234567890` saturates to
  `LONG_MAX` and truncates to `-1`.
* glibc's `%f` not being `strtof`: a partially matched `inf`/`nan`/`0x` is a
  matching failure rather than a shorter match, and a stray exponent marker is
  swallowed.
* `stb__perlin_fastfloor` using `cvttss2si` semantics (`INT_MIN` for NaN and for
  out-of-range values) and the `a < ai` comparison being false for NaN.
* `(unsigned char) seed` and `(unsigned char) i` truncation.
* `printf("%.9g")` round-half-to-even, the two-digit minimum exponent field, and
  the `nan`/`-nan`/`inf`/`-inf` spellings.

## Portability of the memory image

`src/memimage.rs` describes one particular build of the C program: gcc 11.5.0,
`cmake ..` with no build type, non-PIE, glibc 2.34+. The parts of it that the C
program reads *in bounds* — the two tables and `basis` — are fixed by the
source. The parts it reads *out of bounds* are a property of that binary's
layout, and a different compiler or link would move them. There is no
input-independent way around this: the behaviour being reproduced is undefined
in C, and its observable result is whatever the linker did.
