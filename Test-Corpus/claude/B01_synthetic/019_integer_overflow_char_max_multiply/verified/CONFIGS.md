# CONFIGS.md — Phase A configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs: every axis the C code actually
branches on, crossed and pruned to the combinations `main.c` treats differently.

## §0 Build configurations (feature combinations)

| source | knobs found | combinations |
|--------|-------------|--------------|
| `Cargo.toml` | no `[features]` table at all | `--no-default-features` ≡ default ≡ **1** |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions()`, no `target_compile_definitions()` | **1** |
| `c_src/src/main.c` | zero `#ifdef` / `#if` / `#ifndef` (`grep -c '#if' == 0`) | **1** |

**Total valid feature combinations: 1** (the empty set). Phases B and C are run
under `cargo test --no-default-features` (identical to the default) — see
`run_verification.sh`, which loops over the enumerated set so adding a feature
later automatically widens the matrix.

Compile-time platform assumptions the translation bakes in and that the tests
pin down on the host: `char` is **signed** (`CHAR_MAX == 127`), `int` is 32-bit,
`long` is 64-bit, x86-64 SysV ABI, glibc.

## §1 Axes the C code branches on

Runtime "options" this API exposes (there is no option struct, no mode flag, no
byte-order or width selector — the surface is five functions):

* **A. entry point** — `printLine`, `printHexCharLine`, `bad`, `good`, `main`
  (the low-level printers *and* the composed wrappers; `goodG2B`/`goodB2G` are
  reached only through `good`).
* **B. `printLine` payload shape** — NULL · empty · 1 byte · short ASCII ·
  bytes ≥ 0x80 / non-UTF-8 · embedded `%` specifiers · embedded whitespace &
  control bytes · length crossing the 1 KiB / 4 KiB / 8 KiB / 64 KiB stdio
  buffer boundaries.
* **C. `printHexCharLine` value shape** — the full `char` domain `-128..=127`
  (sign-extension boundary at `0`/`-1`, `CHAR_MAX`, `CHAR_MIN`), plus values
  passed in the argument register that do not fit a `char` (E8).
* **D. `main` stdin shape** — cross product of
  * leading whitespace: none · each of `\t \n \v \f \r` and space · a long mixed run;
  * sign: absent · `+` · `-`;
  * digit count: 1 · 2..9 · 10 (crosses `int`) · 19 · 20 (crosses `long`) · 100+;
  * magnitude class: zero · small · `INT_MAX`-ish · `> INT_MAX` · `> LONG_MAX`;
  * terminator: EOF · `\n` · other non-digit junk;
  * the resulting `if (x)` branch: `x == 0` → `bad` vs `x != 0` → `good`.
* **E. stdout destination** — regular file vs pipe. glibc picks full buffering
  for both but a *tty* would be line buffered; Rust's `Stdout` is always line
  buffered. Both orderings must still be identical, so both destinations are
  exercised.
* **F. invocation style** — via the `.so` export loaded with `libloading`
  (dlopen + dlsym, the way `SYMBOLS.md` parity is consumed) vs via the standalone
  executable (`c_src/build/driver` vs `target/release/driver`). Every row is run
  through the `.so` pair; the `main` rows additionally run through the binary pair.

## §2 Table

Each row is checked off only after **both** implementations agree byte-for-byte
across the randomized inputs listed for it (fixed seed `0x5EED_1234_ABCD_0001`,
SplitMix64, in `tests/common/mod.rs`).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C1 | `printHexCharLine` | exhaustive: every one of the 256 `char` values `-128..=127`, one call each | [x] |
| C2 | `printHexCharLine` | 2 000 random `i8` values (seeded), one call each — value-dependent formatting | [x] |
| C3 | `printHexCharLine` | 500 random *batches* of 1..32 calls in one capture — checks accumulated stream/buffering order, not just single calls | [x] |
| C4 | `printHexCharLine` | boundary sweep `{CHAR_MIN, -2, -1, 0, 1, 63, 64, CHAR_MAX/2, CHAR_MAX}` (the `CHAR_MAX/2` constant from `main.c:70`) | [x] |
| C5 | `printLine` | `NULL` (the `main.c:30` guard) | [x] |
| C6 | `printLine` | empty string, then 1-byte strings for every byte value `0x01..=0xFF` | [x] |
| C7 | `printLine` | 1 000 random printable-ASCII strings, length 0..=64 (seeded) | [x] |
| C8 | `printLine` | 1 000 random **arbitrary non-NUL byte** strings, length 0..=64 (seeded) — includes invalid UTF-8, `%`, control bytes, `\n` | [x] |
| C9 | `printLine` | length sweep across stdio buffer boundaries: 1, 127, 128, 1023, 1024, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536 | [x] |
| C10 | `printLine` | 200 random *batches* of 1..16 calls in one capture, mixing NULL and non-NULL — stream order across the NULL no-op | [x] |
| C11 | `printLine` + `printHexCharLine` | 300 random interleavings of the two printers in one capture — the low-level pair composed, which per-function tests cannot see | [x] |
| C12 | `bad` | no parameters; single call (the only configuration it has) | [x] |
| C13 | `bad` | 64 repeated calls in one capture — idempotence + stream order | [x] |
| C14 | `good` | no parameters; single call → `goodG2B` then `goodB2G` | [x] |
| C15 | `good` | 64 repeated calls in one capture | [x] |
| C16 | `good` + `bad` | 300 random interleavings of the two composed wrappers in one capture | [x] |
| C17 | `main` (`.so`) | stdin `"0"` → `x == 0` → `bad` branch | [x] |
| C18 | `main` (`.so`) | stdin `"1"` → `x != 0` → `good` branch | [x] |
| C19 | `main` (`.so`) | no leading whitespace, no sign, 1..9 digits, EOF terminator — 200 random values | [x] |
| C20 | `main` (`.so`) | each single leading whitespace byte `{\t,\n,\v,\f,\r,' '}` × sign `{none,+,-}` × random digits | [x] |
| C21 | `main` (`.so`) | random *runs* (len 0..8) of mixed whitespace bytes before the number | [x] |
| C22 | `main` (`.so`) | sign `+` / `-` × digit-count class `{1, 5, 10, 19, 20, 40}` — spans `int`, `long`, and beyond | [x] |
| C23 | `main` (`.so`) | magnitude classes: `0`, small, `INT_MAX`, `INT_MAX+1`, `UINT_MAX`, `2^32`, `LONG_MAX`, `LONG_MAX+1`, and negatives of each | [x] |
| C24 | `main` (`.so`) | terminator `{EOF, "\n", "\n" + more digits, junk letters, "."}` × random values — only the first conversion is consumed | [x] |
| C25 | `main` (`.so`) | leading-zero runs of length 0/1/2/20/400 before a random digit tail | [x] |
| C26 | `main` (`.so`) | 400 fully random stdin blobs, length 0..24, bytes drawn from a "scanf-interesting" alphabet (digits, signs, all 6 whitespace bytes, letters, `.`, `\0`, high bytes) | [x] |
| C27 | `main` (`.so`) | 200 fully random stdin blobs, length 0..64, bytes drawn uniformly from `0x00..=0xFF` | [x] |
| C28 | `main` (executable) | every stdin case of C17–C27 re-run against the two standalone binaries; compares stdout **and** stderr **and** exit status | [x] |
| C29 | all entry points | stdout redirected to a **regular file** (`dup2` of a temp file over fd 1) | [x] |
| C30 | all `main` cases | stdout redirected to a **pipe** (`Stdio::piped()` in the executable comparison) | [x] |
