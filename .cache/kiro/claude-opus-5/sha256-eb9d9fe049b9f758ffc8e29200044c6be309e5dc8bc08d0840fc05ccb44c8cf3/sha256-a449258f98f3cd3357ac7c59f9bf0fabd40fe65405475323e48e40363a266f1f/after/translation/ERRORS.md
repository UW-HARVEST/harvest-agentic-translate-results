# ERRORS.md — differential verification of the Rust translation

Ground truth: `c_src/` (built with CMake, gcc). Subject: `translation/` (`cargo
build --release`). The two programs are compared by running them as
subprocesses on identical stdin bytes and diffing stdout, stderr and exit
status.

## Result

**No output mismatch was found.** Every enumerated input, plus 2,900 randomized
inputs, produced byte-identical stdout, byte-identical stderr (always empty),
and the same exit status.

Because a "no mismatches" claim is worth little on its own, the sections below
record what was exercised, how the harness was proven able to fail, and the
places where the Rust code deliberately diverges in *structure* without
diverging in *behavior*.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # 21 differential tests
```

## Harness self-check (negative control)

A passing suite proves nothing unless it can fail. Three faults were injected
into the Rust source one at a time; each was caught, and the source was
restored afterwards (verified with `diff` against a backup).

| Injected fault | Tests failed | Failure mode reported |
| --- | --- | --- |
| `std::process::exit(0)` → `exit(1)` on menu choice 7 | 13 | exit status differs: `Ok(0)` vs `Ok(1)` |
| `"Price: ${:.2}"` → `"${:.3}"` in `print_item` | 10 | stdout differs at the first byte of the price field |
| `INPUT_SIZE` 256 → 4096, defeating the `fgets` line split | 2 | stdout differs — the over-long line no longer splits into two loop iterations |

The first fault is the one that justifies asserting all three streams: all 13
failures reported *only* an exit-status difference, and **zero** reported a
stdout difference. A suite that diffed stdout alone would have passed while the
Rust program exited 1 where the C exited 0.

## Branch coverage of the C program

A coverage-instrumented copy of the C source (built in `/tmp`, so `c_src/` was
left untouched) was run under the enumerated inputs:

```
src/main.c       Lines executed: 100.00% of 250    Branches taken: 100.00% of 60
src/inventory.c  Lines executed:  81.91% of  94
```

`main.c` is fully covered — every `if`, every `switch` arm, every early
`return`. The uncovered `inventory.c` lines are **unreachable from `main()`**,
so no stdin input can distinguish them:

- `find_expensive_items` — declared in `inventory.h`, never called. Dead code in
  the executable.
- the `!items || items->size == 0` guard in `calculate_inventory_stats` —
  `demo_inventory_array` always pushes 10 items first.
- the `!orders || orders->size == 0` guard in `calculate_order_stats` —
  `demo_order_list` always appends 8 orders first.
- the `found == 0` arm of `find_items_by_category` — `main` only ever queries
  `"Electronics"` and `"Furniture"`, and both match items.
- the container operations `main` never uses: `array_*_get` (for most types),
  `array_*_clear`, `list_*_prepend`, `list_*_clear`, `list_*_size`, and the
  `create`/`push` variants for types only one container instantiates.

These were checked by reading the Rust translation instead of by execution; see
"Structural divergences" below.

## Input classes enumerated

Each row is asserted on all three of stdout, stderr and exit status.

| Class | Inputs | C behavior being pinned |
| --- | --- | --- |
| Empty input | 0 bytes, `/dev/null`, closed fd | `fgets` returns `NULL` on the first call → loop breaks → exit 0 after the banner and one menu |
| One item | `1`…`5` alone | one demo, then EOF → exit 0 |
| Maximum the code handles | `6` (runs all five demos), `6` ×25 (~134 KB of stdout) | crosses stdio's 4 KB block buffer many times |
| Early `return` | `7`, `7` with trailing input | prints `Goodbye!` and returns 0 *without* consuming the rest of stdin |
| `default:` arm | `0`, `8`, `9`, `100`, `-1`, `-2147483648` | `Invalid choice`, loop continues |
| `sscanf != 1` arm | `abc`, `!!!`, ` `, `\t`, `-`, `+`, `.`, blank lines | `Invalid input`, loop continues |
| Leading whitespace | `   3`, `\t4`, `\t\v\f 4` | `%d` skips all six C whitespace characters |
| Trailing garbage | `3abc`, `3 4 5`, `  3  `, `1;drop` | `%d` stops at the first non-digit and still returns 1 |
| Signs | `+5`, `+7`, `-0`, `007` | `%d` accepts an optional sign and leading zeros |
| Non-decimal forms | `0x3`, `3.9`, `.5`, `1e3`, `1_000` | `%d` is decimal-only: `0x3`→0, `3.9`→3, `.5`→no conversion |
| Overflow / truncation | `2147483648`, `-2147483649`, `4294967296`, `4294967303`, `2^63`, `-2^63`, 26 nines, 400 nines | glibc converts as `long` (saturating at `LONG_MAX`/`LONG_MIN`) then truncates to `int` — e.g. 400 nines → `LONG_MAX` → `(int)-1` |
| `fgets` buffer split | 254 spaces + `17`, 255 spaces + `7`, 254 `x` + `7`, exactly 255 bytes then `7`, 300-byte line, 600 digits | `fgets` stops after 255 bytes; the remainder of the line becomes the *next* loop iteration's input |
| No trailing newline | `3`, `7`, `8`, `abc`, `6\n1` | `fgets` returns the partial line, the next call returns `NULL` |
| Embedded NUL | `\0` + `5`, `5` + `\0` + `6`, `\0` alone | `fgets` copies the bytes, but the C string `sscanf` sees ends at the NUL |
| Non-UTF-8 bytes | `\x80`, `\xff\xfe`, `\xc3\x28\xa0\xa1`, U+FF13 (fullwidth 3) | input is never echoed, so raw bytes must be tolerated, and a fullwidth digit is *not* a digit |
| CR / CRLF | `3\r\n7\r\n`, `3\r7\r`, `\r3` | `\r` is whitespace to `%d`, but not a line terminator to `fgets` |
| Long loops | 500 invalid lines, 500 invalid choices, full menu walks, alternating valid/invalid | prompt/demo/error text interleaving across many iterations |

## Randomized differential fuzzing

2,900 total trials (400 + 2,500, fixed seeds) over random token sequences drawn
from the classes above, with ~6% of trials replaced by uniformly random byte
strings up to 800 bytes. Zero mismatches on stdout, stderr or exit status.

## Structural divergences that are not behavioral divergences

These are places where the Rust code does not look like the C but cannot be
distinguished by running the program. They are recorded so a future reader does
not have to re-derive why they are safe.

1. **Dropped NULL guards.** `calculate_inventory_stats`, `calculate_order_stats`,
   `find_items_by_category` and `find_expensive_items` all begin with `!items`
   (and `!category`) checks in C. The Rust versions take `&Array<T>` / `&List<T>`
   / `&str`, which cannot be null, so the checks are absent. `main` never passes
   null, so the guarded `return` is unreachable in the C executable too.
   `array_*_create` returning `NULL` on allocation failure is likewise not
   modeled; the Rust code aborts on OOM instead.

2. **`printf("%s", buf)` on fixed-size `char[]`.** `item.name`,
   `item.category` and `order.customer_name` are `char` arrays written by
   `strncpy` and are printed up to the first NUL. The Rust code keeps them as
   `[u8; N]` and writes the bytes before the first NUL with `stdio::out_raw`
   rather than going through `Display`, so no UTF-8 transcoding or escaping can
   alter them. All the literals in this program are ASCII, but the raw path
   makes that irrelevant.

3. **`strncpy` truncation is preserved.** `strncpy_terminated::<N>` copies at
   most `N - 1` bytes, zero-fills the rest, and always leaves a NUL — matching
   `strncpy(dst, src, N-1); dst[N-1] = '\0';`. No name in this program is long
   enough to truncate, so this is untested by execution.

4. **The linked list is an index arena.** `List<T>` stores nodes in a `Vec` with
   `head`/`tail` as `Option<usize>` instead of raw pointers, and `size()`
   returns `nodes.len()`. `LIST_FOREACH` walks the `next` chain from `head`, and
   `ListIter` does the same, so iteration order matches. This only coincides
   with `nodes.len()` because `main` never calls `prepend`, `clear` or any
   removal — `prepend` would still be correct (it links `head`, and `size` still
   equals the node count), but a hypothetical remove operation would not be.

5. **`array_*_capacity` bookkeeping is mirrored but unobservable.** `Array<T>`
   keeps an explicit `capacity` field and doubles it when `size >= capacity`,
   like the C macro, even though nothing prints capacity. `create(0)` bumping to
   16 is preserved for the same reason.

6. **Deliberately preserved C oddities.**
   - `calculate_inventory_stats` seeds `max_price = 0.0` but `min_price =
     items->data[0].price`. The asymmetry is kept: an inventory of only
     negative-priced items would report `Most expensive item: $0.00`.
   - `calculate_order_stats` uses `min_order = -1.0` as a sentinel and tests
     `min_order < 0`, so a genuinely negative order total would be treated as
     "not yet set". Kept as-is.
   - `total_value / total_items` in `calculate_inventory_stats` divides by an
     `int` sum of quantities, which is `0` when every quantity is 0 — that is a
     float division producing `inf`/`nan`, not a trap. Kept as-is.
   - `int sum` in `demo_integer_containers` is accumulated with `wrapping_add`
     and `long long product` with `wrapping_mul`, so signed overflow wraps as
     gcc's two's-complement codegen does rather than panicking in debug builds.
     Neither actually overflows with the hard-coded values.

7. **Non-ASCII literals are spelled as escapes.** The banner box-drawing
   characters (U+2554, U+2550, U+2557, U+2551, U+255A, U+255D) and the degree
   sign in `Minimum: %.1f°C` are written as `\u{...}` in the Rust source. Rust
   encodes them as UTF-8 into the output buffer, which is byte-identical to the
   UTF-8 literals in `main.c`. Verified by the byte-for-byte match on menu
   choices 2 and 6.

8. **Output buffering.** C's `stdout` is block-buffered when redirected, so
   `Choice: ` is not flushed before `fgets` blocks. The Rust `stdio` module
   buffers into a `Vec<u8>` and flushes before each `fgets` and before exit.
   The flush *timing* differs; the byte *sequence* on stdout does not, and only
   the sequence is compared. The explicit flush before `std::process::exit(0)`
   on menu choice 7 is required — `exit` does not run destructors.

9. **Unused source files.** `translation/src/cio.rs` and
   `translation/src/cstr.rs` are not declared as modules in `main.rs` and are
   therefore not compiled. They duplicate functionality that now lives in
   `stdio.rs` and `inventory.rs`. They are dead weight, not a behavioral risk.

## Limits of this verification

- Allocation failure paths (`malloc`/`realloc` returning `NULL`, and the `-1`
  returns from `array_*_push` / `list_*_append` that `main` discards anyway) are
  not exercised in either program.
- `find_expensive_items` and the empty-container / no-match branches listed
  under "Branch coverage" cannot be reached through stdin. Their Rust
  counterparts were checked by reading, not by running.
- Only stdin content varies. The C program takes no command-line arguments and
  reads no environment variables or files, so there is no other input surface.
- Comparison was done on x86-64 Linux with gcc and glibc. The `%d` overflow
  behavior in item 6 of the input table is glibc's `strtol`-style saturation; a
  different libc could differ, and the Rust code matches glibc.
