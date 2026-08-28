# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the C source, the mirror of `ERRORS.md` for **valid**
inputs.

## Axes the C code actually branches on

```sh
grep -rniE "#ifdef|#if |switch|if *\(|for *\(|while *\(" c_src/src c_src/include
#   -> no branch of any kind (only license comments / the include guard)
grep -vE "^//|^$" c_src/include/hello.h
#   -> #ifndef HELLO_H_ / #define HELLO_H_ / int helloworld(); / #endif
```

* **Runtime options / modes / flags:** none. The public header exposes no
  setters, no context struct, no flags, no `#ifdef`-selected behaviour, and
  `c_src/CMakeLists.txt` defines no compile-time options.
* **Input shapes:** none in the usual sense — `helloworld` takes no arguments,
  so there are no sizes, widths, element types, counts, formats, or byte-order
  variants to enumerate.
* **Public entry points (FULL set, lowest level included):** exactly one,
  `helloworld`. It *is* the lowest-level entry point; there is no convenience
  wrapper layered over anything else.

Because the function takes no arguments, the configuration surface is not the
argument space but the **observable environment the call acts on**: the state of
the C `stdout` stream that `printf`/`puts` writes through, the kind of file
descriptor underneath it, and the composition of many calls (including
interleaving with other writers and with the *other* implementation). Those are
the axes below; each row is a combination the code's single side effect actually
distinguishes.

Every row is driven with **many seeded-random inputs** (call counts, schedules,
buffer sizes, thread counts, payload tokens) — PRNG is a SplitMix64 with fixed
seed per row, so runs are reproducible.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | randomised over | [x] |
|---|----------------|--------------------------------------------|-----------------|-----|
| B1 | `helloworld` | stdout → **regular file**, default (fully buffered), one call, flush after | 64 repetitions | [x] |
| B2 | `helloworld` | stdout → **pipe**, one call | 64 repetitions | [x] |
| B3 | `helloworld` | stdout → **`/dev/null`** (write succeeds, bytes unobservable): return value only | 64 repetitions | [x] |
| B4 | `helloworld` | stdout → regular file, **N repeated calls**, expect exactly N `Hello World!\n` lines | N ∈ 1..=256 | [x] |
| B5 | `helloworld` | stdout **unbuffered** (`setvbuf _IONBF`), N calls — each `puts` becomes an immediate `write(2)` | N ∈ 1..=64 | [x] |
| B6 | `helloworld` | stdout **line buffered** (`setvbuf _IOLBF`), N calls | N ∈ 1..=64 | [x] |
| B7 | `helloworld` | stdout fully buffered with a **tiny caller-supplied buffer** (1..=8 bytes), forcing partial/split writes mid-line, N calls | buf size ∈ 1..=8, N ∈ 1..=64 | [x] |
| B8 | `helloworld` | **randomised interleaving schedule** of C and Rust calls; the mixed byte stream must equal the all-C stream and the all-Rust stream | 32 schedules × len 1..=64 | [x] |
| B9 | `helloworld` | calls **interleaved with the caller's own stdio writes** (`fputs` of random tokens through the same `FILE*`) — ordering through the shared stream | 32 schedules, random tokens | [x] |
| B10 | `helloworld` | calls **interleaved with the caller's raw `write(2)` to fd 1** while stdout is unbuffered — ordering is observable at the fd level | 32 schedules | [x] |
| B11 | `helloworld` | stdout → regular file **pre-seeded with content / positioned at a non-zero offset**; output must land at the current offset | random prefix len 0..=64 | [x] |
| B12 | `helloworld` | stdout → file opened **`O_APPEND`**, N calls | N ∈ 1..=32 | [x] |
| B13 | `helloworld` | **concurrent calls from T threads × K calls each** (stdio locking ⇒ whole lines, never torn); compare line count and line multiset | T ∈ 2..=8, K ∈ 1..=32 | [x] |
| B14 | `helloworld` | **return value / idempotence over many consecutive calls** — no hidden state, always `0` | 4096 calls | [x] |
| B15 | `helloworld` | called through the **unprototyped extra-argument signature** on the happy path (valid stdout) — bytes and return value unchanged | seeded extreme + random args | [x] |
| B16 | `helloworld` | **both `.so`s resident and used in the same process**, alternating, sharing one libc `stdout` — no cross-interference / no duplicate-symbol capture | 32 alternations | [x] |
| B17 | `helloworld` | symbol looked up **repeatedly via `dlsym`** and called through the freshly-resolved pointer each time (lazy-PLT / relocation path) | 64 lookups | [x] |

All 17 rows are asserted byte-for-byte between the two `.so`s.

## Verification evidence

`./verify.sh` (debug and release × default / `--no-default-features` /
`--all-features`):

```
tests/phase_b.rs — test result: ok. 17 passed; 0 failed
tests/smoke.rs   — test result: ok.  2 passed; 0 failed
```

Test ↔ row mapping: `b1_…` … `b17_…` correspond one-to-one to rows B1…B17.

Randomisation is seeded per row (`Rng::new(0xB0nn)`, SplitMix64), so a failure
reproduces exactly. Across the 17 rows the suite makes roughly 12 000
`helloworld` calls per profile, spread over 5 buffering modes, 5 sink kinds,
1–8 threads and 3 call signatures.

The interleaving rows (B8, B16) are the strongest: they assert that the mixed
C/Rust byte stream is identical to the pure-C stream *and* to the pure-Rust
stream, which catches any per-call byte difference regardless of ordering. B16
additionally asserts the two `helloworld` symbols resolve to *different*
addresses, so the comparison can never be vacuously true by one library having
shadowed the other.
