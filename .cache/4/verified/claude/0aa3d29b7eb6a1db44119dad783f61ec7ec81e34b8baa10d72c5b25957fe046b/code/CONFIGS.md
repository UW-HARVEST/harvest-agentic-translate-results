# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Build-time configurations

`Cargo.toml` `[features]`:

```toml
[features]
default = []
```

There are **no** optional features (the section is empty apart from an empty
`default`), so the complete set of feature combinations is exactly one:

| # | feature combination | command |
|---|---------------------|---------|
| F0 | *(none)* — identical to default | `cargo test --offline --no-default-features` |

`c_src/CMakeLists.txt` defines no `option()`, no `add_definitions`, no
`target_compile_definitions`, and the C source contains **no `#ifdef`/`#if`**
(`grep -n '#if' c_src/src/lib.c` → no matches). So the C library also has a
single build configuration. Both the `dev` and `release` Rust profiles are
nevertheless exercised (`release` additionally enables `panic = "abort"` and
optimisation), giving two Rust artifacts under F0:
`target/debug/libdriver.so` and `target/release/libdriver.so`.

## Runtime option axes

`grep` of the public header and the source for flags / modes / `switch` /
`#ifdef`: none exist. The library has **no runtime options, no state, no
init/teardown, no global variables** — `tool_basename` is a pure function of its
single `char *` argument. Therefore the configuration surface is entirely the
set of **input shapes** the C code distinguishes.

## Axes the C code actually branches on

From `c_src/src/lib.c` lines 10–21 the branch axes are exactly:

* **A1** `s1 = strrchr(path,'/')` is NULL vs non-NULL (does a `/` occur?)
* **A2** `s2 = strrchr(path,'\\')` is NULL vs non-NULL (does a `\` occur?)
* **A3** when both occur: `s1 > s2` vs `s1 < s2` (which separator occurs *last*)
  → the four reachable outcomes: `return path`, `return s1+1`, `return s2+1`
  (via the both-branch), `return s2+1`/`s1+1` (via the single-separator branches)

Data-shape axes that interact with the above (these change which bytes
`strrchr` — a hand-optimised, block-at-a-time glibc routine — examines):

* **S1** length: 0, 1, small (1–64), block boundaries (15/16/17, 31/32/33,
  63/64/65, 127/128/129), large (1 MiB, 4 MiB)
* **S2** separator position: none / first byte / interior / last byte
* **S3** separator count: 0, 1, many, all bytes are separators
* **S4** adjacency: `"/\"`, `"\/"`, `"//"`, `"\\"` runs
* **S5** byte values: ASCII printable, NUL-adjacent data, high-bit bytes
  0x80–0xFF (`char` is signed on x86-64), the full 1..=255 alphabet
* **S6** buffer alignment: the string start offset 0..15 inside its allocation
* **S7** data after the NUL terminator (must be ignored)

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised through **both** `.so` files via `libloading` and is
checked with many randomized inputs (fixed seed, `tests/common/mod.rs`
`Rng`), comparing (a) the returned pointer's byte offset from the input base,
(b) the returned C string's bytes, and (c) that the input buffer was not
modified.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `tool_basename` | no options exist; empty string `""` (S1=0, A1=A2=NULL) → `return path` | [x] |
| C2 | `tool_basename` | no separator at all, random ASCII, random len 1..64 (A1=A2=NULL) | [x] |
| C3 | `tool_basename` | no separator, random len sweep over block boundaries 0..=129 | [x] |
| C4 | `tool_basename` | no separator, high-bit bytes 0x80..0xFF only (S5, signed-char trap) | [x] |
| C5 | `tool_basename` | `/` only, exactly one, interior position (A1 only branch) | [x] |
| C6 | `tool_basename` | `/` only, many occurrences, random positions (last one must win) | [x] |
| C7 | `tool_basename` | `/` only, leading `/` at index 0 (`"/abc"`) | [x] |
| C8 | `tool_basename` | `/` only, trailing `/` as last byte (`"abc/"` → empty basename) | [x] |
| C9 | `tool_basename` | `/` only, string is entirely `/` runs (`"/"`, `"////"`) | [x] |
| C10 | `tool_basename` | `\` only, exactly one, interior position (A2 only branch) | [x] |
| C11 | `tool_basename` | `\` only, many occurrences, random positions | [x] |
| C12 | `tool_basename` | `\` only, leading `\` at index 0 | [x] |
| C13 | `tool_basename` | `\` only, trailing `\` as last byte | [x] |
| C14 | `tool_basename` | `\` only, string is entirely `\` runs | [x] |
| C15 | `tool_basename` | both separators present, last `/` **after** last `\` (A3: `s1 > s2`) | [x] |
| C16 | `tool_basename` | both separators present, last `\` **after** last `/` (A3: `s1 < s2`) | [x] |
| C17 | `tool_basename` | both present, adjacent pair `"/\"` / `"\/"` at the very end (S4) | [x] |
| C18 | `tool_basename` | both present, separators at *both* ends (leading and trailing) | [x] |
| C19 | `tool_basename` | dense-separator alphabet `{'/','\\','a'}`, random len 0..=48 (many A3 flips) | [x] |
| C20 | `tool_basename` | full random alphabet 1..=255 **including** both separators, random len 0..=256 | [x] |
| C21 | `tool_basename` | large input: 1 MiB and 4 MiB, with a separator near the start, middle and end | [x] |
| C22 | `tool_basename` | large input: 1 MiB with **no** separator at all | [x] |
| C23 | `tool_basename` | unaligned string start: same content placed at offsets 0..15 in the allocation (S6) | [x] |
| C24 | `tool_basename` | garbage (including separators) **after** the NUL terminator (S7) | [x] |
| C25 | `tool_basename` | result fed back in: `tool_basename(tool_basename(p))` (interior pointer as input) | [x] |
| C26 | `tool_basename` | exhaustive: every single-separator placement `i` in `0..=64` for both separators, both lengths `i+1` and 65 | [x] |
| C27 | `tool_basename` | exhaustive pairwise: for len 8, every `(i,j)` placement of `/` at `i` and `\` at `j` (covers `s1>s2`, `s1<s2` at every distance) | [x] |
| C28 | `tool_basename` | pointer-identity property: result is always inside `[path, path+strlen]`, and offset agrees bit-exactly between C and Rust | [x] |

All 28 rows are implemented in `tests/configs.rs` and pass for the C `.so` vs
both the debug and release Rust `.so` under the single feature combination F0.
Row `C0` (`c0_harness_loads_two_distinct_shared_objects`) additionally proves the
harness really loaded two *different* `.so` files and that the calls reach them.

## Harness sensitivity (negative controls)

To prove the suite is not vacuously passing, two deliberately broken cdylibs were
built and loaded in place of the real Rust one (via `RUST_DRIVER_SO`):

| injected bug | result |
|--------------|--------|
| ignore the `'\'` separator entirely | 19 of 29 `configs` rows FAIL, 7 of 11 `errors` rows FAIL |
| null-check `path` and return it instead of faulting | `e1_null_pointer` FAILS (`C=Signaled(11)` vs `Rust=Exited(10)`) |

Rows that legitimately cannot observe a given bug (e.g. C1/C2/C3 contain no
backslash) keep passing, as expected.
