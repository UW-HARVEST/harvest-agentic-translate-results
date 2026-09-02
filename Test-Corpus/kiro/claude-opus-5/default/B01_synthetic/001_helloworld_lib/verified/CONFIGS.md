# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

The mirror of `ERRORS.md`, for **valid** inputs. Axes are derived from what
the C code actually branches on / interacts with, not from a guess about what
matters.

## Axis derivation

Public entry points (`grep -rE '^[A-Za-z_].*\(' c_src/include/`):

| entry point | level | notes |
|---|---|---|
| `helloworld()` | **lowest level == only level** | there is no convenience wrapper and no lower-level function beneath it; this single symbol *is* the full public API |

Runtime options / modes / flags: **none.** There is no init function, no
context struct, no setter, no global variable, no environment lookup
(`getenv` appears 0 times in the C), and 0 `if`/`switch`/`#ifdef` branches.
Compile-time variants: **none** (`Cargo.toml` has no `[features]` section, and
`CMakeLists.txt` defines no options).

So the C code's behaviour cannot be varied by *arguments*. What it *does*
interact with is the one piece of external state it touches: the libc
`stdout` `FILE` stream, via `printf`. That makes the real configuration axes:

1. **Invocation count** — 0 / 1 / many (statelessness; the C keeps no state,
   so output must be exactly N repetitions).
2. **`stdout` buffering mode** — fully buffered, line buffered, unbuffered
   (`setvbuf`). `printf` behaves differently in each; a translation that wrote
   through a *different* buffer (e.g. `std::io::stdout()`) would diverge here.
3. **`stdout` destination shape** — regular file vs. pipe. libc picks the
   default buffering from `isatty`, so the destination changes the code path.
4. **Interleaving with caller-side stdio** — the caller emits its own
   `printf`/`fwrite` before/after/between calls. This is the composed-pipeline
   case: it detects buffer mismatch and out-of-order flushing, which
   per-call tests cannot see.
5. **Interleaving C and Rust implementations in one stream** — C, Rust, C,
   Rust… into a single fd, checking the merged byte stream.
6. **Concurrency** — N threads calling simultaneously.
7. **Call-shape / ABI variation on the valid path** — because the declaration
   `int helloworld();` is unprototyped, a valid C caller may invoke it through
   pointers of several arities. (Invalid *values* are `ERRORS.md`; here the
   concern is that the arity itself is a shape the ABI distinguishes.)

Randomization (fixed seed `0x5EED_C0FFEE`, SplitMix64) supplies the
per-row varying quantities: repetition counts, thread counts, buffer sizes,
and the caller-side interleaved payload bytes.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `helloworld` | single call; return value compared | [x] |
| C2 | `helloworld` | single call; stdout redirected to a regular file; captured bytes compared | [x] |
| C3 | `helloworld` | **zero** calls (empty case) — capture produces empty output for both | [x] |
| C4 | `helloworld` | many calls, randomized N in 1..=64, sequential; full byte stream compared | [x] |
| C5 | `helloworld` | fully buffered stdout (`setvbuf _IOFBF`, randomized buffer size) | [x] |
| C6 | `helloworld` | line buffered stdout (`setvbuf _IOLBF`, randomized buffer size) | [x] |
| C7 | `helloworld` | unbuffered stdout (`setvbuf _IONBF`) | [x] |
| C8 | `helloworld` | destination is a **pipe** rather than a regular file | [x] |
| C9 | `helloworld` | caller-side `printf` interleaved before/after each call (randomized payloads) — ordering within one libc buffer | [x] |
| C10 | `helloworld` | caller-side `write(2)` (raw fd, bypassing the FILE buffer) interleaved — detects a translation that buffers separately | [x] |
| C11 | `helloworld` | C and Rust alternating into the **same** captured fd, randomized interleave pattern; each half's slice compared | [x] |
| C12 | `helloworld` | randomized thread count 2..=8, each thread calling randomized 1..=16 times; line-multiset compared | [x] |
| C13 | `helloworld` | called through `extern "C" fn() -> c_int` (declared arity, the normal shape) | [x] |
| C14 | `helloworld` | called through 1-, 3-, and 6-integer-argument pointers (unprototyped decl, valid values) | [x] |
| C15 | `helloworld` | called through a pointer with float/SSE arguments (exercises a different ABI register class) | [x] |
| C16 | `helloworld` | repeated dlopen/dlclose of the `.so` between calls (no per-load state; ctor/dtor parity) | [x] |
| C17 | `helloworld` | both `.so`s loaded simultaneously in one process (symbol-collision safety, `RTLD_LOCAL`) | [x] |

## Gate

- [x] Every row above passes across its randomized inputs, C vs. Rust,
      both called through `dlsym` on their respective `.so`.
