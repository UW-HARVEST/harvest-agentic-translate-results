# CONFIGS.md — the configuration surface (Phase B)

Every axis below is derived from what the C code actually branches on: the
runtime options the public API can set (`png_set_*`), the `#ifdef`-selected
features listed in `c_src/include/pnglibconf.h`, and the input shapes the
`if` / `switch` statements in `c_src/src/*.c` distinguish (colour type, bit
depth, interlace method, filter method, chunk presence and ordering, IDAT
segmentation, buffer sizes, row-stride sign, ...).  Each row is one meaningful
*combination* of those axes; the cross-product is pruned to the combinations
the C treats differently.

The lowest-level public entry points are driven directly, not only through the
convenience wrappers: `png_write_row` / `png_write_rows` / `png_write_image` /
`png_write_png` / `png_write_sig` + `png_write_chunk*`; `png_read_row` /
`png_read_rows` / `png_read_image` / `png_read_png` / `png_process_data`; plus
`png_init_io`, the simplified `png_image_*` API and the raw `png_get_uint_32`
family.

Each row runs **both** libraries through their `.so` exports in that
configuration and compares a full record of the run byte for byte: every byte
written or decoded, every getter, the ordered list of warnings, the error
message if any, and the process exit status.  Rows marked `n=2` or `n=3` repeat
the configuration with that many independently seeded random images; rows in
groups B16/B17 are themselves drawn from a fixed-seed generator, so the whole
matrix is reproducible.

One field is deliberately excluded from the post-transform comparison:
after a `PNG_QUANTIZE` transform on a non-palette image the reference C leaves
`info_ptr->num_trans` non-zero while `info_ptr->trans_alpha` still points at
memory that was never written for that colour type.  The byte the C reads there
moves when unrelated allocations move, so it is not a function of the input;
`num_trans` and the pointer's nullness are still compared, and the array
contents are still compared in every pre-transform and end-of-stream dump.

**Result: 2433 of 2433 rows pass.**

## Feature combinations (Phase D)

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one build configuration; `cargo metadata` confirms an empty feature map.
Both of the reachable configurations were checked and the whole suite was run in
both:

```
cargo check --release --all-targets                     # OK
cargo check --release --all-targets --no-default-features  # OK (identical)
cargo test  --release       # 36 passed
cargo test                  # 36 passed (dev profile)
cargo test  --no-default-features  # 36 passed
```

The *libpng* feature set, by contrast, is fixed by `c_src/include/pnglibconf.h`
and is what the axes above enumerate: all 21 supported ancillary chunk types,
read and write transforms, the progressive reader, the simplified API, user
limits, user transforms, unknown-chunk handling, MNG extensions, benign errors,
and both the floating-point and fixed-point halves of every dual API.

| group | rows | passing | title |
|---|---|---|---|
| B1 | 120 | 120 | Write pipeline — colour type x bit depth x interlace x entry point |
| B2 | 28 | 28 | Write pipeline — zlib option matrix |
| B3 | 44 | 44 | Write pipeline — row filter matrix |
| B4 | 44 | 44 | Write pipeline — transforms (png_set_* and the png_write_png mask) |
| B5 | 70 | 70 | Write pipeline — ancillary chunk sets |
| B6 | 13 | 13 | Write pipeline — output plumbing (flush, status callback, raw chunk API, extreme shapes) |
| B7 | 210 | 210 | Read pipeline — colour type x bit depth x interlace x entry point |
| B8 | 178 | 178 | Read pipeline — transform matrix |
| B9 | 34 | 34 | Read pipeline — png_read_png transform mask |
| B10 | 164 | 164 | Read pipeline — ancillary chunk sets, stream layout, options, shapes |
| B11 | 12 | 12 | Unknown-chunk handling matrix |
| B12 | 48 | 48 | Progressive (push) reader |
| B13 | 189 | 189 | Simplified read API |
| B14 | 52 | 52 | Simplified write API |
| B15 | 42 | 42 | png_set_* / png_get_* round trips and library-wide state |
| B16 | 420 | 420 | Randomized read cross-product sweep |
| B17 | 320 | 320 | Randomized write cross-product sweep |
| B18 | 20 | 20 | Large images (multi-buffer zlib, long-row filter selection) |
| B19 | 70 | 70 | User transform callbacks |
| B20 | 99 | 99 | MNG extensions and png_set_sig_bytes hand-over |
| B21 | 180 | 180 | CRC errors x png_set_crc_action |
| B22 | 3 | 3 | Floating-point getters |
| B23 | 19 | 19 | stdio-based entry points (png_init_io, *_from_file, *_to_stdio) |
| B24 | 14 | 14 | png_free_data / png_data_freer / png_destroy_info_struct |
| B25 | 16 | 16 | Deprecated filter heuristics |
| B26 | 24 | 24 | B26 |

## B1 — Write pipeline — colour type x bit depth x interlace x entry point

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1 | `wr\|ct=0\|bd=1\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 2 | `wr\|ct=0\|bd=1\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=0 via png_write_image | exit 0 | [x] |
| 3 | `wr\|ct=0\|bd=1\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 4 | `wr\|ct=0\|bd=1\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=0 via png_write_png | exit 0 | [x] |
| 5 | `wr\|ct=0\|bd=1\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 6 | `wr\|ct=0\|bd=1\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=1 via png_write_image | exit 0 | [x] |
| 7 | `wr\|ct=0\|bd=1\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 8 | `wr\|ct=0\|bd=1\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1001` | write GRAY/1-bit interlace=1 via png_write_png | exit 0 | [x] |
| 9 | `wr\|ct=0\|bd=2\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 10 | `wr\|ct=0\|bd=2\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=0 via png_write_image | exit 0 | [x] |
| 11 | `wr\|ct=0\|bd=2\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 12 | `wr\|ct=0\|bd=2\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=0 via png_write_png | exit 0 | [x] |
| 13 | `wr\|ct=0\|bd=2\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 14 | `wr\|ct=0\|bd=2\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=1 via png_write_image | exit 0 | [x] |
| 15 | `wr\|ct=0\|bd=2\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 16 | `wr\|ct=0\|bd=2\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1002` | write GRAY/2-bit interlace=1 via png_write_png | exit 0 | [x] |
| 17 | `wr\|ct=0\|bd=4\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 18 | `wr\|ct=0\|bd=4\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=0 via png_write_image | exit 0 | [x] |
| 19 | `wr\|ct=0\|bd=4\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 20 | `wr\|ct=0\|bd=4\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=0 via png_write_png | exit 0 | [x] |
| 21 | `wr\|ct=0\|bd=4\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 22 | `wr\|ct=0\|bd=4\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=1 via png_write_image | exit 0 | [x] |
| 23 | `wr\|ct=0\|bd=4\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 24 | `wr\|ct=0\|bd=4\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1004` | write GRAY/4-bit interlace=1 via png_write_png | exit 0 | [x] |
| 25 | `wr\|ct=0\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 26 | `wr\|ct=0\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=0 via png_write_image | exit 0 | [x] |
| 27 | `wr\|ct=0\|bd=8\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 28 | `wr\|ct=0\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=0 via png_write_png | exit 0 | [x] |
| 29 | `wr\|ct=0\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 30 | `wr\|ct=0\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=1 via png_write_image | exit 0 | [x] |
| 31 | `wr\|ct=0\|bd=8\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 32 | `wr\|ct=0\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1008` | write GRAY/8-bit interlace=1 via png_write_png | exit 0 | [x] |
| 33 | `wr\|ct=0\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 34 | `wr\|ct=0\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=0 via png_write_image | exit 0 | [x] |
| 35 | `wr\|ct=0\|bd=16\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 36 | `wr\|ct=0\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=0 via png_write_png | exit 0 | [x] |
| 37 | `wr\|ct=0\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 38 | `wr\|ct=0\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=1 via png_write_image | exit 0 | [x] |
| 39 | `wr\|ct=0\|bd=16\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 40 | `wr\|ct=0\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1016` | write GRAY/16-bit interlace=1 via png_write_png | exit 0 | [x] |
| 41 | `wr\|ct=2\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 42 | `wr\|ct=2\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=0 via png_write_image | exit 0 | [x] |
| 43 | `wr\|ct=2\|bd=8\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 44 | `wr\|ct=2\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=0 via png_write_png | exit 0 | [x] |
| 45 | `wr\|ct=2\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 46 | `wr\|ct=2\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=1 via png_write_image | exit 0 | [x] |
| 47 | `wr\|ct=2\|bd=8\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 48 | `wr\|ct=2\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1042` | write RGB/8-bit interlace=1 via png_write_png | exit 0 | [x] |
| 49 | `wr\|ct=2\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 50 | `wr\|ct=2\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=0 via png_write_image | exit 0 | [x] |
| 51 | `wr\|ct=2\|bd=16\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 52 | `wr\|ct=2\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=0 via png_write_png | exit 0 | [x] |
| 53 | `wr\|ct=2\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 54 | `wr\|ct=2\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=1 via png_write_image | exit 0 | [x] |
| 55 | `wr\|ct=2\|bd=16\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 56 | `wr\|ct=2\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1050` | write RGB/16-bit interlace=1 via png_write_png | exit 0 | [x] |
| 57 | `wr\|ct=3\|bd=1\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 58 | `wr\|ct=3\|bd=1\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=0 via png_write_image | exit 0 | [x] |
| 59 | `wr\|ct=3\|bd=1\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 60 | `wr\|ct=3\|bd=1\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=0 via png_write_png | exit 0 | [x] |
| 61 | `wr\|ct=3\|bd=1\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 62 | `wr\|ct=3\|bd=1\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=1 via png_write_image | exit 0 | [x] |
| 63 | `wr\|ct=3\|bd=1\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 64 | `wr\|ct=3\|bd=1\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1052` | write PALETTE/1-bit interlace=1 via png_write_png | exit 0 | [x] |
| 65 | `wr\|ct=3\|bd=2\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 66 | `wr\|ct=3\|bd=2\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=0 via png_write_image | exit 0 | [x] |
| 67 | `wr\|ct=3\|bd=2\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 68 | `wr\|ct=3\|bd=2\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=0 via png_write_png | exit 0 | [x] |
| 69 | `wr\|ct=3\|bd=2\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 70 | `wr\|ct=3\|bd=2\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=1 via png_write_image | exit 0 | [x] |
| 71 | `wr\|ct=3\|bd=2\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 72 | `wr\|ct=3\|bd=2\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1053` | write PALETTE/2-bit interlace=1 via png_write_png | exit 0 | [x] |
| 73 | `wr\|ct=3\|bd=4\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 74 | `wr\|ct=3\|bd=4\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=0 via png_write_image | exit 0 | [x] |
| 75 | `wr\|ct=3\|bd=4\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 76 | `wr\|ct=3\|bd=4\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=0 via png_write_png | exit 0 | [x] |
| 77 | `wr\|ct=3\|bd=4\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 78 | `wr\|ct=3\|bd=4\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=1 via png_write_image | exit 0 | [x] |
| 79 | `wr\|ct=3\|bd=4\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 80 | `wr\|ct=3\|bd=4\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1055` | write PALETTE/4-bit interlace=1 via png_write_png | exit 0 | [x] |
| 81 | `wr\|ct=3\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 82 | `wr\|ct=3\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=0 via png_write_image | exit 0 | [x] |
| 83 | `wr\|ct=3\|bd=8\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 84 | `wr\|ct=3\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=0 via png_write_png | exit 0 | [x] |
| 85 | `wr\|ct=3\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 86 | `wr\|ct=3\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=1 via png_write_image | exit 0 | [x] |
| 87 | `wr\|ct=3\|bd=8\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 88 | `wr\|ct=3\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1059` | write PALETTE/8-bit interlace=1 via png_write_png | exit 0 | [x] |
| 89 | `wr\|ct=4\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 90 | `wr\|ct=4\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=0 via png_write_image | exit 0 | [x] |
| 91 | `wr\|ct=4\|bd=8\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 92 | `wr\|ct=4\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=0 via png_write_png | exit 0 | [x] |
| 93 | `wr\|ct=4\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 94 | `wr\|ct=4\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=1 via png_write_image | exit 0 | [x] |
| 95 | `wr\|ct=4\|bd=8\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 96 | `wr\|ct=4\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1076` | write GRAY_ALPHA/8-bit interlace=1 via png_write_png | exit 0 | [x] |
| 97 | `wr\|ct=4\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 98 | `wr\|ct=4\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=0 via png_write_image | exit 0 | [x] |
| 99 | `wr\|ct=4\|bd=16\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 100 | `wr\|ct=4\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=0 via png_write_png | exit 0 | [x] |
| 101 | `wr\|ct=4\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 102 | `wr\|ct=4\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=1 via png_write_image | exit 0 | [x] |
| 103 | `wr\|ct=4\|bd=16\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 104 | `wr\|ct=4\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1084` | write GRAY_ALPHA/16-bit interlace=1 via png_write_png | exit 0 | [x] |
| 105 | `wr\|ct=6\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 106 | `wr\|ct=6\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=0 via png_write_image | exit 0 | [x] |
| 107 | `wr\|ct=6\|bd=8\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 108 | `wr\|ct=6\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=0 via png_write_png | exit 0 | [x] |
| 109 | `wr\|ct=6\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 110 | `wr\|ct=6\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=1 via png_write_image | exit 0 | [x] |
| 111 | `wr\|ct=6\|bd=8\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 112 | `wr\|ct=6\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1110` | write RGBA/8-bit interlace=1 via png_write_png | exit 0 | [x] |
| 113 | `wr\|ct=6\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=0 via png_write_rows | exit 0 | [x] |
| 114 | `wr\|ct=6\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=0 via png_write_image | exit 0 | [x] |
| 115 | `wr\|ct=6\|bd=16\|il=0\|mode=split\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=0 via png_write_row (one row at a time) | exit 0 | [x] |
| 116 | `wr\|ct=6\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=0 via png_write_png | exit 0 | [x] |
| 117 | `wr\|ct=6\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=1 via png_write_rows | exit 0 | [x] |
| 118 | `wr\|ct=6\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=1 via png_write_image | exit 0 | [x] |
| 119 | `wr\|ct=6\|bd=16\|il=1\|mode=split\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=1 via png_write_row (one row at a time) | exit 0 | [x] |
| 120 | `wr\|ct=6\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=1118` | write RGBA/16-bit interlace=1 via png_write_png | exit 0 | [x] |

## B2 — Write pipeline — zlib option matrix

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 121 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=0\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(0) | exit 0 | [x] |
| 122 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=1\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(1) | exit 0 | [x] |
| 123 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=3\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(3) | exit 0 | [x] |
| 124 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=6\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(6) | exit 0 | [x] |
| 125 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=9\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(9) | exit 0 | [x] |
| 126 | `wr\|ct=2\|bd=8\|w=23\|h=17\|lvl=-1\|n=2\|seed=2001` | write RGB/8 with png_set_compression_level(-1) | exit 0 | [x] |
| 127 | `wr\|ct=2\|bd=8\|w=23\|h=17\|strat=0\|n=2\|seed=2002` | write RGB/8 with png_set_compression_strategy(0) | exit 0 | [x] |
| 128 | `wr\|ct=2\|bd=8\|w=23\|h=17\|strat=1\|n=2\|seed=2002` | write RGB/8 with png_set_compression_strategy(1) | exit 0 | [x] |
| 129 | `wr\|ct=2\|bd=8\|w=23\|h=17\|strat=2\|n=2\|seed=2002` | write RGB/8 with png_set_compression_strategy(2) | exit 0 | [x] |
| 130 | `wr\|ct=2\|bd=8\|w=23\|h=17\|strat=3\|n=2\|seed=2002` | write RGB/8 with png_set_compression_strategy(3) | exit 0 | [x] |
| 131 | `wr\|ct=2\|bd=8\|w=23\|h=17\|strat=4\|n=2\|seed=2002` | write RGB/8 with png_set_compression_strategy(4) | exit 0 | [x] |
| 132 | `wr\|ct=2\|bd=8\|w=23\|h=17\|wb=8\|n=2\|seed=2003` | write RGB/8 with png_set_compression_window_bits(8) | exit 0 | [x] |
| 133 | `wr\|ct=2\|bd=8\|w=23\|h=17\|wb=9\|n=2\|seed=2003` | write RGB/8 with png_set_compression_window_bits(9) | exit 0 | [x] |
| 134 | `wr\|ct=2\|bd=8\|w=23\|h=17\|wb=11\|n=2\|seed=2003` | write RGB/8 with png_set_compression_window_bits(11) | exit 0 | [x] |
| 135 | `wr\|ct=2\|bd=8\|w=23\|h=17\|wb=13\|n=2\|seed=2003` | write RGB/8 with png_set_compression_window_bits(13) | exit 0 | [x] |
| 136 | `wr\|ct=2\|bd=8\|w=23\|h=17\|wb=15\|n=2\|seed=2003` | write RGB/8 with png_set_compression_window_bits(15) | exit 0 | [x] |
| 137 | `wr\|ct=2\|bd=8\|w=23\|h=17\|ml=1\|n=2\|seed=2004` | write RGB/8 with png_set_compression_mem_level(1) | exit 0 | [x] |
| 138 | `wr\|ct=2\|bd=8\|w=23\|h=17\|ml=4\|n=2\|seed=2004` | write RGB/8 with png_set_compression_mem_level(4) | exit 0 | [x] |
| 139 | `wr\|ct=2\|bd=8\|w=23\|h=17\|ml=8\|n=2\|seed=2004` | write RGB/8 with png_set_compression_mem_level(8) | exit 0 | [x] |
| 140 | `wr\|ct=2\|bd=8\|w=23\|h=17\|ml=9\|n=2\|seed=2004` | write RGB/8 with png_set_compression_mem_level(9) | exit 0 | [x] |
| 141 | `wr\|ct=2\|bd=8\|w=61\|h=41\|cbuf=8\|n=1\|seed=2005` | write RGB/8 with png_set_compression_buffer_size(8) | exit 0 | [x] |
| 142 | `wr\|ct=2\|bd=8\|w=61\|h=41\|cbuf=64\|n=1\|seed=2005` | write RGB/8 with png_set_compression_buffer_size(64) | exit 0 | [x] |
| 143 | `wr\|ct=2\|bd=8\|w=61\|h=41\|cbuf=1024\|n=1\|seed=2005` | write RGB/8 with png_set_compression_buffer_size(1024) | exit 0 | [x] |
| 144 | `wr\|ct=2\|bd=8\|w=61\|h=41\|cbuf=8192\|n=1\|seed=2005` | write RGB/8 with png_set_compression_buffer_size(8192) | exit 0 | [x] |
| 145 | `wr\|ct=2\|bd=8\|w=61\|h=41\|cbuf=65536\|n=1\|seed=2005` | write RGB/8 with png_set_compression_buffer_size(65536) | exit 0 | [x] |
| 146 | `wr\|ct=2\|bd=8\|w=17\|h=9\|x=text\|tlvl=0\|n=1\|seed=2006` | write RGB/8 + zTXt with png_set_text_compression_level(0) | exit 0 | [x] |
| 147 | `wr\|ct=2\|bd=8\|w=17\|h=9\|x=text\|tlvl=6\|n=1\|seed=2006` | write RGB/8 + zTXt with png_set_text_compression_level(6) | exit 0 | [x] |
| 148 | `wr\|ct=2\|bd=8\|w=17\|h=9\|x=text\|tlvl=9\|n=1\|seed=2006` | write RGB/8 + zTXt with png_set_text_compression_level(9) | exit 0 | [x] |

## B3 — Write pipeline — row filter matrix

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 149 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_NO_FILTERS) | exit 0 | [x] |
| 150 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_NO_FILTERS) | exit 0 | [x] |
| 151 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_NO_FILTERS) | exit 0 | [x] |
| 152 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_NO_FILTERS) | exit 0 | [x] |
| 153 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=8\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FILTER_NONE) | exit 0 | [x] |
| 154 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=8\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FILTER_NONE) | exit 0 | [x] |
| 155 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=8\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FILTER_NONE) | exit 0 | [x] |
| 156 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=8\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FILTER_NONE) | exit 0 | [x] |
| 157 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=16\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FILTER_SUB) | exit 0 | [x] |
| 158 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=16\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FILTER_SUB) | exit 0 | [x] |
| 159 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=16\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FILTER_SUB) | exit 0 | [x] |
| 160 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=16\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FILTER_SUB) | exit 0 | [x] |
| 161 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=32\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FILTER_UP) | exit 0 | [x] |
| 162 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=32\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FILTER_UP) | exit 0 | [x] |
| 163 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=32\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FILTER_UP) | exit 0 | [x] |
| 164 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=32\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FILTER_UP) | exit 0 | [x] |
| 165 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=64\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FILTER_AVG) | exit 0 | [x] |
| 166 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=64\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FILTER_AVG) | exit 0 | [x] |
| 167 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=64\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FILTER_AVG) | exit 0 | [x] |
| 168 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=64\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FILTER_AVG) | exit 0 | [x] |
| 169 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=128\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FILTER_PAETH) | exit 0 | [x] |
| 170 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=128\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FILTER_PAETH) | exit 0 | [x] |
| 171 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=128\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FILTER_PAETH) | exit 0 | [x] |
| 172 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=128\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FILTER_PAETH) | exit 0 | [x] |
| 173 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=56\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_FAST_FILTERS) | exit 0 | [x] |
| 174 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=56\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_FAST_FILTERS) | exit 0 | [x] |
| 175 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=56\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_FAST_FILTERS) | exit 0 | [x] |
| 176 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=56\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_FAST_FILTERS) | exit 0 | [x] |
| 177 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=248\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, PNG_ALL_FILTERS) | exit 0 | [x] |
| 178 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=248\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, PNG_ALL_FILTERS) | exit 0 | [x] |
| 179 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=248\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, PNG_ALL_FILTERS) | exit 0 | [x] |
| 180 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=248\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, PNG_ALL_FILTERS) | exit 0 | [x] |
| 181 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=144\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, SUB\|PAETH) | exit 0 | [x] |
| 182 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=144\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, SUB\|PAETH) | exit 0 | [x] |
| 183 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=144\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, SUB\|PAETH) | exit 0 | [x] |
| 184 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=144\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, SUB\|PAETH) | exit 0 | [x] |
| 185 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, value 0 (fixed filter NONE)) | exit 0 | [x] |
| 186 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, value 0 (fixed filter NONE)) | exit 0 | [x] |
| 187 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, value 0 (fixed filter NONE)) | exit 0 | [x] |
| 188 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=0\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, value 0 (fixed filter NONE)) | exit 0 | [x] |
| 189 | `wr\|ct=2\|bd=8\|w=29\|h=13\|filt=4\|n=2\|seed=3001` | write RGB/8-bit with png_set_filter(0, value 4 (fixed filter PAETH)) | exit 0 | [x] |
| 190 | `wr\|ct=0\|bd=1\|w=29\|h=13\|filt=4\|n=2\|seed=3001` | write GRAY/1-bit with png_set_filter(0, value 4 (fixed filter PAETH)) | exit 0 | [x] |
| 191 | `wr\|ct=6\|bd=16\|w=29\|h=13\|filt=4\|n=2\|seed=3001` | write RGBA/16-bit with png_set_filter(0, value 4 (fixed filter PAETH)) | exit 0 | [x] |
| 192 | `wr\|ct=3\|bd=4\|w=29\|h=13\|filt=4\|n=2\|seed=3001` | write PALETTE/4-bit with png_set_filter(0, value 4 (fixed filter PAETH)) | exit 0 | [x] |

## B4 — Write pipeline — transforms (png_set_* and the png_write_png mask)

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 193 | `wr\|ct=2\|bd=8\|w=21\|h=9\|tr=bgr\|mode=split\|n=2\|seed=4001` | write RGB/8-bit with transform(s) bgr | exit 0 | [x] |
| 194 | `wr\|ct=2\|bd=16\|w=21\|h=9\|tr=bgr\|mode=split\|n=2\|seed=4001` | write RGB/16-bit with transform(s) bgr | exit 0 | [x] |
| 195 | `wr\|ct=6\|bd=8\|w=21\|h=9\|tr=bgr\|mode=split\|n=2\|seed=4001` | write RGBA/8-bit with transform(s) bgr | exit 0 | [x] |
| 196 | `wr\|ct=6\|bd=8\|w=21\|h=9\|tr=invalpha\|mode=split\|n=2\|seed=4001` | write RGBA/8-bit with transform(s) invalpha | exit 0 | [x] |
| 197 | `wr\|ct=6\|bd=8\|w=21\|h=9\|tr=swapalpha\|mode=split\|n=2\|seed=4001` | write RGBA/8-bit with transform(s) swapalpha | exit 0 | [x] |
| 198 | `wr\|ct=6\|bd=8\|w=21\|h=9\|tr=bgr+invalpha+swapalpha\|mode=split\|n=2\|seed=4001` | write RGBA/8-bit with transform(s) bgr+invalpha+swapalpha | exit 0 | [x] |
| 199 | `wr\|ct=6\|bd=16\|w=21\|h=9\|tr=invalpha\|mode=split\|n=2\|seed=4001` | write RGBA/16-bit with transform(s) invalpha | exit 0 | [x] |
| 200 | `wr\|ct=4\|bd=8\|w=21\|h=9\|tr=invalpha\|mode=split\|n=2\|seed=4001` | write GRAY_ALPHA/8-bit with transform(s) invalpha | exit 0 | [x] |
| 201 | `wr\|ct=4\|bd=8\|w=21\|h=9\|tr=swapalpha\|mode=split\|n=2\|seed=4001` | write GRAY_ALPHA/8-bit with transform(s) swapalpha | exit 0 | [x] |
| 202 | `wr\|ct=2\|bd=16\|w=21\|h=9\|tr=swap16\|mode=split\|n=2\|seed=4001` | write RGB/16-bit with transform(s) swap16 | exit 0 | [x] |
| 203 | `wr\|ct=6\|bd=16\|w=21\|h=9\|tr=swap16\|mode=split\|n=2\|seed=4001` | write RGBA/16-bit with transform(s) swap16 | exit 0 | [x] |
| 204 | `wr\|ct=0\|bd=16\|w=21\|h=9\|tr=swap16\|mode=split\|n=2\|seed=4001` | write GRAY/16-bit with transform(s) swap16 | exit 0 | [x] |
| 205 | `wr\|ct=0\|bd=1\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write GRAY/1-bit with transform(s) packing | exit 0 | [x] |
| 206 | `wr\|ct=0\|bd=2\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write GRAY/2-bit with transform(s) packing | exit 0 | [x] |
| 207 | `wr\|ct=0\|bd=4\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write GRAY/4-bit with transform(s) packing | exit 0 | [x] |
| 208 | `wr\|ct=3\|bd=1\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write PALETTE/1-bit with transform(s) packing | exit 0 | [x] |
| 209 | `wr\|ct=3\|bd=2\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write PALETTE/2-bit with transform(s) packing | exit 0 | [x] |
| 210 | `wr\|ct=3\|bd=4\|w=21\|h=9\|tr=packing\|mode=split\|n=2\|seed=4001` | write PALETTE/4-bit with transform(s) packing | exit 0 | [x] |
| 211 | `wr\|ct=0\|bd=1\|w=21\|h=9\|tr=packswap\|mode=split\|n=2\|seed=4001` | write GRAY/1-bit with transform(s) packswap | exit 0 | [x] |
| 212 | `wr\|ct=0\|bd=2\|w=21\|h=9\|tr=packswap\|mode=split\|n=2\|seed=4001` | write GRAY/2-bit with transform(s) packswap | exit 0 | [x] |
| 213 | `wr\|ct=0\|bd=4\|w=21\|h=9\|tr=packswap\|mode=split\|n=2\|seed=4001` | write GRAY/4-bit with transform(s) packswap | exit 0 | [x] |
| 214 | `wr\|ct=3\|bd=4\|w=21\|h=9\|tr=packswap\|mode=split\|n=2\|seed=4001` | write PALETTE/4-bit with transform(s) packswap | exit 0 | [x] |
| 215 | `wr\|ct=0\|bd=1\|w=21\|h=9\|tr=packing+packswap\|mode=split\|n=2\|seed=4001` | write GRAY/1-bit with transform(s) packing+packswap | exit 0 | [x] |
| 216 | `wr\|ct=0\|bd=1\|w=21\|h=9\|tr=invmono\|mode=split\|n=2\|seed=4001` | write GRAY/1-bit with transform(s) invmono | exit 0 | [x] |
| 217 | `wr\|ct=0\|bd=8\|w=21\|h=9\|tr=invmono\|mode=split\|n=2\|seed=4001` | write GRAY/8-bit with transform(s) invmono | exit 0 | [x] |
| 218 | `wr\|ct=4\|bd=8\|w=21\|h=9\|tr=invmono\|mode=split\|n=2\|seed=4001` | write GRAY_ALPHA/8-bit with transform(s) invmono | exit 0 | [x] |
| 219 | `wr\|ct=0\|bd=8\|w=21\|h=9\|tr=shift\|mode=split\|n=2\|seed=4001` | write GRAY/8-bit with transform(s) shift | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 220 | `wr\|ct=0\|bd=16\|w=21\|h=9\|tr=shift\|mode=split\|n=2\|seed=4001` | write GRAY/16-bit with transform(s) shift | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 221 | `wr\|ct=2\|bd=16\|w=21\|h=9\|tr=shift\|mode=split\|n=2\|seed=4001` | write RGB/16-bit with transform(s) shift | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 222 | `wr\|ct=6\|bd=16\|w=21\|h=9\|tr=shift\|mode=split\|n=2\|seed=4001` | write RGBA/16-bit with transform(s) shift | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 223 | `wr\|ct=2\|bd=8\|w=21\|h=9\|tr=filler_after\|mode=split\|n=2\|seed=4001` | write RGB/8-bit with transform(s) filler_after | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 224 | `wr\|ct=2\|bd=8\|w=21\|h=9\|tr=filler_before\|mode=split\|n=2\|seed=4001` | write RGB/8-bit with transform(s) filler_before | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 225 | `wr\|ct=0\|bd=8\|w=21\|h=9\|tr=filler_after\|mode=split\|n=2\|seed=4001` | write GRAY/8-bit with transform(s) filler_after | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 226 | `wr\|ct=2\|bd=16\|w=21\|h=9\|tr=filler_after\|mode=split\|n=2\|seed=4001` | write RGB/16-bit with transform(s) filler_after | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 227 | `wr\|ct=6\|bd=8\|w=15\|h=7\|mode=png\|wt=0\|n=2\|seed=4002` | png_write_png(IDENTITY) on RGBA/8-bit | exit 0 | [x] |
| 228 | `wr\|ct=0\|bd=4\|w=15\|h=7\|mode=png\|wt=4\|n=2\|seed=4002` | png_write_png(PACKING) on GRAY/4-bit | exit 0 | [x] |
| 229 | `wr\|ct=0\|bd=4\|w=15\|h=7\|mode=png\|wt=8\|n=2\|seed=4002` | png_write_png(PACKSWAP) on GRAY/4-bit | exit 0 | [x] |
| 230 | `wr\|ct=0\|bd=4\|w=15\|h=7\|mode=png\|wt=32\|n=2\|seed=4002` | png_write_png(INVERT_MONO) on GRAY/4-bit | exit 0 | [x] |
| 231 | `wr\|ct=2\|bd=8\|w=15\|h=7\|mode=png\|wt=64\|n=2\|seed=4002` | png_write_png(SHIFT) on RGB/8-bit | exit 0 | [x] |
| 232 | `wr\|ct=6\|bd=8\|w=15\|h=7\|mode=png\|wt=128\|n=2\|seed=4002` | png_write_png(BGR) on RGBA/8-bit | exit 0 | [x] |
| 233 | `wr\|ct=6\|bd=8\|w=15\|h=7\|mode=png\|wt=256\|n=2\|seed=4002` | png_write_png(SWAP_ALPHA) on RGBA/8-bit | exit 0 | [x] |
| 234 | `wr\|ct=2\|bd=16\|w=15\|h=7\|mode=png\|wt=512\|n=2\|seed=4002` | png_write_png(SWAP_ENDIAN) on RGB/16-bit | exit 0 | [x] |
| 235 | `wr\|ct=6\|bd=8\|w=15\|h=7\|mode=png\|wt=1024\|n=2\|seed=4002` | png_write_png(INVERT_ALPHA) on RGBA/8-bit | exit 0 | [x] |
| 236 | `wr\|ct=6\|bd=8\|w=15\|h=7\|mode=png\|wt=1152\|n=2\|seed=4002` | png_write_png(BGR\|INVERT_ALPHA) on RGBA/8-bit | exit 0 | [x] |

## B5 — Write pipeline — ancillary chunk sets

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 237 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=none\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [none] | exit 0 | [x] |
| 238 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=none\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [none] | exit 0 | [x] |
| 239 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=none\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [none] | exit 0 | [x] |
| 240 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=none\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [none] | exit 0 | [x] |
| 241 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=none\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [none] | exit 0 | [x] |
| 242 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=gama\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [gama] | exit 0 | [x] |
| 243 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=gama\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [gama] | exit 0 | [x] |
| 244 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=gama\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [gama] | exit 0 | [x] |
| 245 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=gama\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [gama] | exit 0 | [x] |
| 246 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=gama\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [gama] | exit 0 | [x] |
| 247 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=chrm\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 248 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=chrm\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 249 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=chrm\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [chrm] | exit 0 | [x] |
| 250 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=chrm\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 251 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=chrm\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 252 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=gamachrm\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 253 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=gamachrm\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 254 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=gamachrm\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 255 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=gamachrm\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 256 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=gamachrm\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 257 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=srgb\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 258 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=srgb\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 259 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=srgb\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [srgb] | exit 0 | [x] |
| 260 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=srgb\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 261 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=srgb\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 262 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=text\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [text] | exit 0 | [x] |
| 263 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=text\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [text] | exit 0 | [x] |
| 264 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=text\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [text] | exit 0 | [x] |
| 265 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=text\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [text] | exit 0 | [x] |
| 266 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=text\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [text] | exit 0 | [x] |
| 267 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=time\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [time] | exit 0 | [x] |
| 268 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=time\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [time] | exit 0 | [x] |
| 269 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=time\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [time] | exit 0 | [x] |
| 270 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=time\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [time] | exit 0 | [x] |
| 271 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=time\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [time] | exit 0 | [x] |
| 272 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=physoffs\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [physoffs] | exit 0 | [x] |
| 273 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=physoffs\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [physoffs] | exit 0 | [x] |
| 274 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=physoffs\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [physoffs] | exit 0 | [x] |
| 275 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=physoffs\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [physoffs] | exit 0 | [x] |
| 276 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=physoffs\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [physoffs] | exit 0 | [x] |
| 277 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=sbit\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 278 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=sbit\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 279 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=sbit\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [sbit] | exit 0 | [x] |
| 280 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=sbit\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 281 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=sbit\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 282 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=trns\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [trns] | exit 0 | [x] |
| 283 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=trns\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [trns] | exit 0 | [x] |
| 284 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=trns\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [trns] | exit 0 | [x] |
| 285 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=trns\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [trns] | exit 0 | [x] |
| 286 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=trns\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [trns] | exit 0 | [x] |
| 287 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=bkgd\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 288 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=bkgd\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 289 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=bkgd\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [bkgd] | exit 0 | [x] |
| 290 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=bkgd\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 291 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=bkgd\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 292 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 293 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=iccp\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 294 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=iccp\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [iccp] | exit 0 | [x] |
| 295 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=iccp\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 296 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=iccp\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 297 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=unk\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [unk] | exit 0 | [x] |
| 298 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=unk\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [unk] | exit 0 | [x] |
| 299 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=unk\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [unk] | exit 0 | [x] |
| 300 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=unk\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [unk] | exit 0 | [x] |
| 301 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=unk\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [unk] | exit 0 | [x] |
| 302 | `wr\|ct=2\|bd=8\|w=13\|h=7\|x=gamachrmtextphysoffstimesbit\|n=1\|seed=5001` | write RGB/8-bit with ancillary set [gamachrmtextphysoffstimesbit] | exit 0 | [x] |
| 303 | `wr\|ct=3\|bd=8\|w=13\|h=7\|x=gamachrmtextphysoffstimesbit\|n=1\|seed=5001` | write PALETTE/8-bit with ancillary set [gamachrmtextphysoffstimesbit] | exit 0 | [x] |
| 304 | `wr\|ct=0\|bd=16\|w=13\|h=7\|x=gamachrmtextphysoffstimesbit\|n=1\|seed=5001` | write GRAY/16-bit with ancillary set [gamachrmtextphysoffstimesbit] | exit 0 | [x] |
| 305 | `wr\|ct=6\|bd=8\|w=13\|h=7\|x=gamachrmtextphysoffstimesbit\|n=1\|seed=5001` | write RGBA/8-bit with ancillary set [gamachrmtextphysoffstimesbit] | exit 0 | [x] |
| 306 | `wr\|ct=4\|bd=8\|w=13\|h=7\|x=gamachrmtextphysoffstimesbit\|n=1\|seed=5001` | write GRAY_ALPHA/8-bit with ancillary set [gamachrmtextphysoffstimesbit] | exit 0 | [x] |

## B6 — Write pipeline — output plumbing (flush, status callback, raw chunk API, extreme shapes)

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 307 | `wr\|ct=2\|bd=8\|w=17\|h=11\|flush=0\|mode=split\|n=1\|seed=6001` | write with png_set_flush(0) + png_write_flush | exit 0 | [x] |
| 308 | `wr\|ct=2\|bd=8\|w=17\|h=11\|flush=1\|mode=split\|n=1\|seed=6001` | write with png_set_flush(1) + png_write_flush | exit 0 | [x] |
| 309 | `wr\|ct=2\|bd=8\|w=17\|h=11\|flush=3\|mode=split\|n=1\|seed=6001` | write with png_set_flush(3) + png_write_flush | exit 0 | [x] |
| 310 | `wr\|ct=2\|bd=8\|w=17\|h=11\|flush=100\|mode=split\|n=1\|seed=6001` | write with png_set_flush(100) + png_write_flush | exit 0 | [x] |
| 311 | `wr\|ct=2\|bd=8\|w=17\|h=11\|wstat=1\|mode=split\|n=1\|seed=6002` | write with png_set_write_status_fn callback | exit 0 | [x] |
| 312 | `wr\|ct=2\|bd=8\|w=8\|h=4\|mode=chunks\|n=1\|seed=6003` | png_write_sig + png_write_chunk / _start / _data / _end directly | exit 0 | [x] |
| 313 | `wr\|ct=6\|bd=8\|w=1\|h=1\|n=1\|seed=6004` | write extreme shape 1x1 | exit 0 | [x] |
| 314 | `wr\|ct=6\|bd=8\|w=1\|h=33\|n=1\|seed=6004` | write extreme shape 1x33 | exit 0 | [x] |
| 315 | `wr\|ct=6\|bd=8\|w=33\|h=1\|n=1\|seed=6004` | write extreme shape 33x1 | exit 0 | [x] |
| 316 | `wr\|ct=6\|bd=8\|w=2\|h=2\|n=1\|seed=6004` | write extreme shape 2x2 | exit 0 | [x] |
| 317 | `wr\|ct=6\|bd=8\|w=7\|h=1\|n=1\|seed=6004` | write extreme shape 7x1 | exit 0 | [x] |
| 318 | `wr\|ct=6\|bd=8\|w=1\|h=7\|n=1\|seed=6004` | write extreme shape 1x7 | exit 0 | [x] |
| 319 | `wr\|ct=6\|bd=8\|w=64\|h=64\|n=1\|seed=6004` | write extreme shape 64x64 | exit 0 | [x] |

## B7 — Read pipeline — colour type x bit depth x interlace x entry point

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 320 | `rd\|ct=0\|bd=1\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_image | exit 0 | [x] |
| 321 | `rd\|ct=0\|bd=1\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 322 | `rd\|ct=0\|bd=1\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 323 | `rd\|ct=0\|bd=1\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 324 | `rd\|ct=0\|bd=1\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 325 | `rd\|ct=0\|bd=1\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_read_png | exit 0 | [x] |
| 326 | `rd\|ct=0\|bd=1\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 327 | `rd\|ct=0\|bd=1\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_image | exit 0 | [x] |
| 328 | `rd\|ct=0\|bd=1\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 329 | `rd\|ct=0\|bd=1\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 330 | `rd\|ct=0\|bd=1\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 331 | `rd\|ct=0\|bd=1\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 332 | `rd\|ct=0\|bd=1\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_read_png | exit 0 | [x] |
| 333 | `rd\|ct=0\|bd=1\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7001` | read GRAY/1-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 334 | `rd\|ct=0\|bd=2\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_image | exit 0 | [x] |
| 335 | `rd\|ct=0\|bd=2\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 336 | `rd\|ct=0\|bd=2\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 337 | `rd\|ct=0\|bd=2\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 338 | `rd\|ct=0\|bd=2\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 339 | `rd\|ct=0\|bd=2\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_read_png | exit 0 | [x] |
| 340 | `rd\|ct=0\|bd=2\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 341 | `rd\|ct=0\|bd=2\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_image | exit 0 | [x] |
| 342 | `rd\|ct=0\|bd=2\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 343 | `rd\|ct=0\|bd=2\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 344 | `rd\|ct=0\|bd=2\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 345 | `rd\|ct=0\|bd=2\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 346 | `rd\|ct=0\|bd=2\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_read_png | exit 0 | [x] |
| 347 | `rd\|ct=0\|bd=2\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7002` | read GRAY/2-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 348 | `rd\|ct=0\|bd=4\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_image | exit 0 | [x] |
| 349 | `rd\|ct=0\|bd=4\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 350 | `rd\|ct=0\|bd=4\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 351 | `rd\|ct=0\|bd=4\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 352 | `rd\|ct=0\|bd=4\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 353 | `rd\|ct=0\|bd=4\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_read_png | exit 0 | [x] |
| 354 | `rd\|ct=0\|bd=4\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 355 | `rd\|ct=0\|bd=4\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_image | exit 0 | [x] |
| 356 | `rd\|ct=0\|bd=4\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 357 | `rd\|ct=0\|bd=4\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 358 | `rd\|ct=0\|bd=4\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 359 | `rd\|ct=0\|bd=4\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 360 | `rd\|ct=0\|bd=4\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_read_png | exit 0 | [x] |
| 361 | `rd\|ct=0\|bd=4\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7004` | read GRAY/4-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 362 | `rd\|ct=0\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_image | exit 0 | [x] |
| 363 | `rd\|ct=0\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 364 | `rd\|ct=0\|bd=8\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 365 | `rd\|ct=0\|bd=8\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 366 | `rd\|ct=0\|bd=8\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 367 | `rd\|ct=0\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_read_png | exit 0 | [x] |
| 368 | `rd\|ct=0\|bd=8\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 369 | `rd\|ct=0\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_image | exit 0 | [x] |
| 370 | `rd\|ct=0\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 371 | `rd\|ct=0\|bd=8\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 372 | `rd\|ct=0\|bd=8\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 373 | `rd\|ct=0\|bd=8\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 374 | `rd\|ct=0\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_read_png | exit 0 | [x] |
| 375 | `rd\|ct=0\|bd=8\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7008` | read GRAY/8-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 376 | `rd\|ct=0\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_image | exit 0 | [x] |
| 377 | `rd\|ct=0\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 378 | `rd\|ct=0\|bd=16\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 379 | `rd\|ct=0\|bd=16\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 380 | `rd\|ct=0\|bd=16\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 381 | `rd\|ct=0\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_read_png | exit 0 | [x] |
| 382 | `rd\|ct=0\|bd=16\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 383 | `rd\|ct=0\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_image | exit 0 | [x] |
| 384 | `rd\|ct=0\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 385 | `rd\|ct=0\|bd=16\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 386 | `rd\|ct=0\|bd=16\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 387 | `rd\|ct=0\|bd=16\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 388 | `rd\|ct=0\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_read_png | exit 0 | [x] |
| 389 | `rd\|ct=0\|bd=16\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7016` | read GRAY/16-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 390 | `rd\|ct=2\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_image | exit 0 | [x] |
| 391 | `rd\|ct=2\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 392 | `rd\|ct=2\|bd=8\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 393 | `rd\|ct=2\|bd=8\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 394 | `rd\|ct=2\|bd=8\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 395 | `rd\|ct=2\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_read_png | exit 0 | [x] |
| 396 | `rd\|ct=2\|bd=8\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 397 | `rd\|ct=2\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_image | exit 0 | [x] |
| 398 | `rd\|ct=2\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 399 | `rd\|ct=2\|bd=8\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 400 | `rd\|ct=2\|bd=8\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 401 | `rd\|ct=2\|bd=8\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 402 | `rd\|ct=2\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_read_png | exit 0 | [x] |
| 403 | `rd\|ct=2\|bd=8\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7070` | read RGB/8-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 404 | `rd\|ct=2\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_image | exit 0 | [x] |
| 405 | `rd\|ct=2\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 406 | `rd\|ct=2\|bd=16\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 407 | `rd\|ct=2\|bd=16\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 408 | `rd\|ct=2\|bd=16\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 409 | `rd\|ct=2\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_read_png | exit 0 | [x] |
| 410 | `rd\|ct=2\|bd=16\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 411 | `rd\|ct=2\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_image | exit 0 | [x] |
| 412 | `rd\|ct=2\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 413 | `rd\|ct=2\|bd=16\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 414 | `rd\|ct=2\|bd=16\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 415 | `rd\|ct=2\|bd=16\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 416 | `rd\|ct=2\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_read_png | exit 0 | [x] |
| 417 | `rd\|ct=2\|bd=16\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7078` | read RGB/16-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 418 | `rd\|ct=3\|bd=1\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_image | exit 0 | [x] |
| 419 | `rd\|ct=3\|bd=1\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 420 | `rd\|ct=3\|bd=1\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 421 | `rd\|ct=3\|bd=1\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 422 | `rd\|ct=3\|bd=1\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 423 | `rd\|ct=3\|bd=1\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_read_png | exit 0 | [x] |
| 424 | `rd\|ct=3\|bd=1\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 425 | `rd\|ct=3\|bd=1\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_image | exit 0 | [x] |
| 426 | `rd\|ct=3\|bd=1\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 427 | `rd\|ct=3\|bd=1\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 428 | `rd\|ct=3\|bd=1\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 429 | `rd\|ct=3\|bd=1\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 430 | `rd\|ct=3\|bd=1\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_read_png | exit 0 | [x] |
| 431 | `rd\|ct=3\|bd=1\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7094` | read PALETTE/1-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 432 | `rd\|ct=3\|bd=2\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_image | exit 0 | [x] |
| 433 | `rd\|ct=3\|bd=2\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 434 | `rd\|ct=3\|bd=2\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 435 | `rd\|ct=3\|bd=2\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 436 | `rd\|ct=3\|bd=2\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 437 | `rd\|ct=3\|bd=2\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_read_png | exit 0 | [x] |
| 438 | `rd\|ct=3\|bd=2\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 439 | `rd\|ct=3\|bd=2\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_image | exit 0 | [x] |
| 440 | `rd\|ct=3\|bd=2\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 441 | `rd\|ct=3\|bd=2\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 442 | `rd\|ct=3\|bd=2\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 443 | `rd\|ct=3\|bd=2\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 444 | `rd\|ct=3\|bd=2\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_read_png | exit 0 | [x] |
| 445 | `rd\|ct=3\|bd=2\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7095` | read PALETTE/2-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 446 | `rd\|ct=3\|bd=4\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_image | exit 0 | [x] |
| 447 | `rd\|ct=3\|bd=4\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 448 | `rd\|ct=3\|bd=4\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 449 | `rd\|ct=3\|bd=4\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 450 | `rd\|ct=3\|bd=4\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 451 | `rd\|ct=3\|bd=4\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_read_png | exit 0 | [x] |
| 452 | `rd\|ct=3\|bd=4\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 453 | `rd\|ct=3\|bd=4\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_image | exit 0 | [x] |
| 454 | `rd\|ct=3\|bd=4\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 455 | `rd\|ct=3\|bd=4\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 456 | `rd\|ct=3\|bd=4\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 457 | `rd\|ct=3\|bd=4\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 458 | `rd\|ct=3\|bd=4\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_read_png | exit 0 | [x] |
| 459 | `rd\|ct=3\|bd=4\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7097` | read PALETTE/4-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 460 | `rd\|ct=3\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_image | exit 0 | [x] |
| 461 | `rd\|ct=3\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 462 | `rd\|ct=3\|bd=8\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 463 | `rd\|ct=3\|bd=8\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 464 | `rd\|ct=3\|bd=8\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 465 | `rd\|ct=3\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_read_png | exit 0 | [x] |
| 466 | `rd\|ct=3\|bd=8\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 467 | `rd\|ct=3\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_image | exit 0 | [x] |
| 468 | `rd\|ct=3\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 469 | `rd\|ct=3\|bd=8\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 470 | `rd\|ct=3\|bd=8\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 471 | `rd\|ct=3\|bd=8\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 472 | `rd\|ct=3\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_read_png | exit 0 | [x] |
| 473 | `rd\|ct=3\|bd=8\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7101` | read PALETTE/8-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 474 | `rd\|ct=4\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_image | exit 0 | [x] |
| 475 | `rd\|ct=4\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 476 | `rd\|ct=4\|bd=8\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 477 | `rd\|ct=4\|bd=8\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 478 | `rd\|ct=4\|bd=8\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 479 | `rd\|ct=4\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_read_png | exit 0 | [x] |
| 480 | `rd\|ct=4\|bd=8\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 481 | `rd\|ct=4\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_image | exit 0 | [x] |
| 482 | `rd\|ct=4\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 483 | `rd\|ct=4\|bd=8\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 484 | `rd\|ct=4\|bd=8\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 485 | `rd\|ct=4\|bd=8\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 486 | `rd\|ct=4\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_read_png | exit 0 | [x] |
| 487 | `rd\|ct=4\|bd=8\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7132` | read GRAY_ALPHA/8-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 488 | `rd\|ct=4\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_image | exit 0 | [x] |
| 489 | `rd\|ct=4\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 490 | `rd\|ct=4\|bd=16\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 491 | `rd\|ct=4\|bd=16\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 492 | `rd\|ct=4\|bd=16\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 493 | `rd\|ct=4\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_read_png | exit 0 | [x] |
| 494 | `rd\|ct=4\|bd=16\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 495 | `rd\|ct=4\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_image | exit 0 | [x] |
| 496 | `rd\|ct=4\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 497 | `rd\|ct=4\|bd=16\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 498 | `rd\|ct=4\|bd=16\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 499 | `rd\|ct=4\|bd=16\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 500 | `rd\|ct=4\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_read_png | exit 0 | [x] |
| 501 | `rd\|ct=4\|bd=16\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7140` | read GRAY_ALPHA/16-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 502 | `rd\|ct=6\|bd=8\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_image | exit 0 | [x] |
| 503 | `rd\|ct=6\|bd=8\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 504 | `rd\|ct=6\|bd=8\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 505 | `rd\|ct=6\|bd=8\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 506 | `rd\|ct=6\|bd=8\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 507 | `rd\|ct=6\|bd=8\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_read_png | exit 0 | [x] |
| 508 | `rd\|ct=6\|bd=8\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 509 | `rd\|ct=6\|bd=8\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_image | exit 0 | [x] |
| 510 | `rd\|ct=6\|bd=8\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 511 | `rd\|ct=6\|bd=8\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 512 | `rd\|ct=6\|bd=8\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 513 | `rd\|ct=6\|bd=8\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 514 | `rd\|ct=6\|bd=8\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_read_png | exit 0 | [x] |
| 515 | `rd\|ct=6\|bd=8\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7194` | read RGBA/8-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 516 | `rd\|ct=6\|bd=16\|il=0\|mode=image\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_image | exit 0 | [x] |
| 517 | `rd\|ct=6\|bd=16\|il=0\|mode=rows\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_rows | exit 0 | [x] |
| 518 | `rd\|ct=6\|bd=16\|il=0\|mode=row\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_row (row+display) | exit 0 | [x] |
| 519 | `rd\|ct=6\|bd=16\|il=0\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_row (row only) | exit 0 | [x] |
| 520 | `rd\|ct=6\|bd=16\|il=0\|mode=disponly\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_row (display only) | exit 0 | [x] |
| 521 | `rd\|ct=6\|bd=16\|il=0\|mode=png\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_read_png | exit 0 | [x] |
| 522 | `rd\|ct=6\|bd=16\|il=0\|mode=startimage\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=0 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |
| 523 | `rd\|ct=6\|bd=16\|il=1\|mode=image\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_image | exit 0 | [x] |
| 524 | `rd\|ct=6\|bd=16\|il=1\|mode=rows\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_rows | exit 0 | [x] |
| 525 | `rd\|ct=6\|bd=16\|il=1\|mode=row\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_row (row+display) | exit 0 | [x] |
| 526 | `rd\|ct=6\|bd=16\|il=1\|mode=rowonly\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_row (row only) | exit 0 | [x] |
| 527 | `rd\|ct=6\|bd=16\|il=1\|mode=disponly\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_row (display only) | exit 0 | [x] |
| 528 | `rd\|ct=6\|bd=16\|il=1\|mode=png\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_read_png | exit 0 | [x] |
| 529 | `rd\|ct=6\|bd=16\|il=1\|mode=startimage\|w=19\|h=11\|n=3\|seed=7202` | read RGBA/16-bit interlace=1 via png_start_read_image + png_read_image (no png_read_update_info) | exit 0 | [x] |

## B8 — Read pipeline — transform matrix

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 530 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) expand | exit 0 | [x] |
| 531 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) expand | exit 0 | [x] |
| 532 | `rd\|ct=0\|bd=2\|il=0\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=0 with transform(s) expand | exit 0 | [x] |
| 533 | `rd\|ct=0\|bd=2\|il=1\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=1 with transform(s) expand | exit 0 | [x] |
| 534 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=0 with transform(s) expand | exit 0 | [x] |
| 535 | `rd\|ct=0\|bd=4\|il=1\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=1 with transform(s) expand | exit 0 | [x] |
| 536 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=expandgray\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) expandgray | exit 0 | [x] |
| 537 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=expandgray\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) expandgray | exit 0 | [x] |
| 538 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=11\|tr=expandgray\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=0 with transform(s) expandgray | exit 0 | [x] |
| 539 | `rd\|ct=0\|bd=4\|il=1\|w=19\|h=11\|tr=expandgray\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=1 with transform(s) expandgray | exit 0 | [x] |
| 540 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand | exit 0 | [x] |
| 541 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand | exit 0 | [x] |
| 542 | `rd\|ct=3\|bd=4\|il=0\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read PALETTE/4-bit interlace=0 with transform(s) expand | exit 0 | [x] |
| 543 | `rd\|ct=3\|bd=4\|il=1\|w=19\|h=11\|tr=expand\|mode=image\|n=2\|seed=8001` | read PALETTE/4-bit interlace=1 with transform(s) expand | exit 0 | [x] |
| 544 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=pal2rgb\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) pal2rgb | exit 0 | [x] |
| 545 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=pal2rgb\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) pal2rgb | exit 0 | [x] |
| 546 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) expand16 | exit 0 | [x] |
| 547 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) expand16 | exit 0 | [x] |
| 548 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) expand16 | exit 0 | [x] |
| 549 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) expand16 | exit 0 | [x] |
| 550 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand16 | exit 0 | [x] |
| 551 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand16 | exit 0 | [x] |
| 552 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) expand16 | exit 0 | [x] |
| 553 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=expand16\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) expand16 | exit 0 | [x] |
| 554 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=expand+expand16\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) expand+expand16 | exit 0 | [x] |
| 555 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=expand+expand16\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) expand+expand16 | exit 0 | [x] |
| 556 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) bgr | exit 0 | [x] |
| 557 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) bgr | exit 0 | [x] |
| 558 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) bgr | exit 0 | [x] |
| 559 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) bgr | exit 0 | [x] |
| 560 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) bgr | exit 0 | [x] |
| 561 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=bgr\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) bgr | exit 0 | [x] |
| 562 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) gray2rgb | exit 0 | [x] |
| 563 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) gray2rgb | exit 0 | [x] |
| 564 | `rd\|ct=4\|bd=8\|il=0\|w=19\|h=11\|tr=gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=0 with transform(s) gray2rgb | exit 0 | [x] |
| 565 | `rd\|ct=4\|bd=8\|il=1\|w=19\|h=11\|tr=gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=1 with transform(s) gray2rgb | exit 0 | [x] |
| 566 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=expand+gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) expand+gray2rgb | exit 0 | [x] |
| 567 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=expand+gray2rgb\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) expand+gray2rgb | exit 0 | [x] |
| 568 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) rgb2gray | exit 0 | [x] |
| 569 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) rgb2gray | exit 0 | [x] |
| 570 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) rgb2gray | exit 0 | [x] |
| 571 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) rgb2gray | exit 0 | [x] |
| 572 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) rgb2gray | exit 0 | [x] |
| 573 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=rgb2gray\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) rgb2gray | exit 0 | [x] |
| 574 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=rgb2graywarn\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) rgb2graywarn | exit 0; 22 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 575 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=rgb2graywarn\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) rgb2graywarn | exit 0; 44 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 576 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) stripalpha | exit 0 | [x] |
| 577 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) stripalpha | exit 0 | [x] |
| 578 | `rd\|ct=4\|bd=8\|il=0\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=0 with transform(s) stripalpha | exit 0 | [x] |
| 579 | `rd\|ct=4\|bd=8\|il=1\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=1 with transform(s) stripalpha | exit 0 | [x] |
| 580 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) stripalpha | exit 0 | [x] |
| 581 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) stripalpha | exit 0 | [x] |
| 582 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=swapalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) swapalpha | exit 0 | [x] |
| 583 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=swapalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) swapalpha | exit 0 | [x] |
| 584 | `rd\|ct=4\|bd=8\|il=0\|w=19\|h=11\|tr=swapalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=0 with transform(s) swapalpha | exit 0 | [x] |
| 585 | `rd\|ct=4\|bd=8\|il=1\|w=19\|h=11\|tr=swapalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=1 with transform(s) swapalpha | exit 0 | [x] |
| 586 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=invalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) invalpha | exit 0 | [x] |
| 587 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=invalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) invalpha | exit 0 | [x] |
| 588 | `rd\|ct=4\|bd=16\|il=0\|w=19\|h=11\|tr=invalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/16-bit interlace=0 with transform(s) invalpha | exit 0 | [x] |
| 589 | `rd\|ct=4\|bd=16\|il=1\|w=19\|h=11\|tr=invalpha\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/16-bit interlace=1 with transform(s) invalpha | exit 0 | [x] |
| 590 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=swapalpha+invalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) swapalpha+invalpha | exit 0 | [x] |
| 591 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=swapalpha+invalpha\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) swapalpha+invalpha | exit 0 | [x] |
| 592 | `rd\|ct=0\|bd=16\|il=0\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=0 with transform(s) swap16 | exit 0 | [x] |
| 593 | `rd\|ct=0\|bd=16\|il=1\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=1 with transform(s) swap16 | exit 0 | [x] |
| 594 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) swap16 | exit 0 | [x] |
| 595 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) swap16 | exit 0 | [x] |
| 596 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) swap16 | exit 0 | [x] |
| 597 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=swap16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) swap16 | exit 0 | [x] |
| 598 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 599 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 600 | `rd\|ct=0\|bd=2\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 601 | `rd\|ct=0\|bd=2\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 602 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 603 | `rd\|ct=0\|bd=4\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 604 | `rd\|ct=3\|bd=1\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/1-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 605 | `rd\|ct=3\|bd=1\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/1-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 606 | `rd\|ct=3\|bd=2\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/2-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 607 | `rd\|ct=3\|bd=2\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/2-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 608 | `rd\|ct=3\|bd=4\|il=0\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/4-bit interlace=0 with transform(s) packing | exit 0 | [x] |
| 609 | `rd\|ct=3\|bd=4\|il=1\|w=19\|h=11\|tr=packing\|mode=image\|n=2\|seed=8001` | read PALETTE/4-bit interlace=1 with transform(s) packing | exit 0 | [x] |
| 610 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) packswap | exit 0 | [x] |
| 611 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) packswap | exit 0 | [x] |
| 612 | `rd\|ct=0\|bd=2\|il=0\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=0 with transform(s) packswap | exit 0 | [x] |
| 613 | `rd\|ct=0\|bd=2\|il=1\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/2-bit interlace=1 with transform(s) packswap | exit 0 | [x] |
| 614 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=0 with transform(s) packswap | exit 0 | [x] |
| 615 | `rd\|ct=0\|bd=4\|il=1\|w=19\|h=11\|tr=packswap\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=1 with transform(s) packswap | exit 0 | [x] |
| 616 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=packing+packswap\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) packing+packswap | exit 0 | [x] |
| 617 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=packing+packswap\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) packing+packswap | exit 0 | [x] |
| 618 | `rd\|ct=0\|bd=1\|il=0\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=0 with transform(s) invmono | exit 0 | [x] |
| 619 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY/1-bit interlace=1 with transform(s) invmono | exit 0 | [x] |
| 620 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) invmono | exit 0 | [x] |
| 621 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) invmono | exit 0 | [x] |
| 622 | `rd\|ct=4\|bd=8\|il=0\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=0 with transform(s) invmono | exit 0 | [x] |
| 623 | `rd\|ct=4\|bd=8\|il=1\|w=19\|h=11\|tr=invmono\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=1 with transform(s) invmono | exit 0 | [x] |
| 624 | `rd\|ct=0\|bd=16\|il=0\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=0 with transform(s) strip16 | exit 0 | [x] |
| 625 | `rd\|ct=0\|bd=16\|il=1\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=1 with transform(s) strip16 | exit 0 | [x] |
| 626 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) strip16 | exit 0 | [x] |
| 627 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) strip16 | exit 0 | [x] |
| 628 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) strip16 | exit 0 | [x] |
| 629 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=strip16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) strip16 | exit 0 | [x] |
| 630 | `rd\|ct=0\|bd=16\|il=0\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=0 with transform(s) scale16 | exit 0 | [x] |
| 631 | `rd\|ct=0\|bd=16\|il=1\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read GRAY/16-bit interlace=1 with transform(s) scale16 | exit 0 | [x] |
| 632 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) scale16 | exit 0 | [x] |
| 633 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) scale16 | exit 0 | [x] |
| 634 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) scale16 | exit 0 | [x] |
| 635 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=scale16\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) scale16 | exit 0 | [x] |
| 636 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) filler_after | exit 0 | [x] |
| 637 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) filler_after | exit 0 | [x] |
| 638 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=filler_before\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) filler_before | exit 0 | [x] |
| 639 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=filler_before\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) filler_before | exit 0 | [x] |
| 640 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) filler_after | exit 0 | [x] |
| 641 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) filler_after | exit 0 | [x] |
| 642 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) filler_after | exit 0 | [x] |
| 643 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=filler_after\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) filler_after | exit 0 | [x] |
| 644 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=addalpha_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) addalpha_after | exit 0 | [x] |
| 645 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=addalpha_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) addalpha_after | exit 0 | [x] |
| 646 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=addalpha_before\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) addalpha_before | exit 0 | [x] |
| 647 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=addalpha_before\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) addalpha_before | exit 0 | [x] |
| 648 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=addalpha_after\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) addalpha_after | exit 0 | [x] |
| 649 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=addalpha_after\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) addalpha_after | exit 0 | [x] |
| 650 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) shift | exit 0 | [x] |
| 651 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) shift | exit 0 | [x] |
| 652 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) shift | exit 0 | [x] |
| 653 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) shift | exit 0 | [x] |
| 654 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) shift | exit 0 | [x] |
| 655 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) shift | exit 0 | [x] |
| 656 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) shift | exit 0 | [x] |
| 657 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=shift\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) shift | exit 0 | [x] |
| 658 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) gamma | exit 0 | [x] |
| 659 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) gamma | exit 0 | [x] |
| 660 | `rd\|ct=0\|bd=8\|il=0\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=0 with transform(s) gamma | exit 0 | [x] |
| 661 | `rd\|ct=0\|bd=8\|il=1\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read GRAY/8-bit interlace=1 with transform(s) gamma | exit 0 | [x] |
| 662 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) gamma | exit 0 | [x] |
| 663 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) gamma | exit 0 | [x] |
| 664 | `rd\|ct=2\|bd=16\|il=0\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=0 with transform(s) gamma | exit 0 | [x] |
| 665 | `rd\|ct=2\|bd=16\|il=1\|w=19\|h=11\|tr=gamma\|mode=image\|n=2\|seed=8001` | read RGB/16-bit interlace=1 with transform(s) gamma | exit 0 | [x] |
| 666 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand+gamma\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand+gamma | exit 0 | [x] |
| 667 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand+gamma\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand+gamma | exit 0 | [x] |
| 668 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=gammahigh\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) gammahigh | exit 0 | [x] |
| 669 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=gammahigh\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) gammahigh | exit 0 | [x] |
| 670 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=alphapng\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) alphapng | exit 0 | [x] |
| 671 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=alphapng\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) alphapng | exit 0 | [x] |
| 672 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=alphastd\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) alphastd | exit 0 | [x] |
| 673 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=alphastd\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) alphastd | exit 0 | [x] |
| 674 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=alphaopt\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) alphaopt | exit 0 | [x] |
| 675 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=alphaopt\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) alphaopt | exit 0 | [x] |
| 676 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=alphabroken\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) alphabroken | exit 0 | [x] |
| 677 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=alphabroken\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) alphabroken | exit 0 | [x] |
| 678 | `rd\|ct=4\|bd=16\|il=0\|w=19\|h=11\|tr=alphastd\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/16-bit interlace=0 with transform(s) alphastd | exit 0 | [x] |
| 679 | `rd\|ct=4\|bd=16\|il=1\|w=19\|h=11\|tr=alphastd\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/16-bit interlace=1 with transform(s) alphastd | exit 0 | [x] |
| 680 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) background | exit 0 | [x] |
| 681 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) background | exit 0 | [x] |
| 682 | `rd\|ct=4\|bd=8\|il=0\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=0 with transform(s) background | exit 0 | [x] |
| 683 | `rd\|ct=4\|bd=8\|il=1\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read GRAY_ALPHA/8-bit interlace=1 with transform(s) background | exit 0 | [x] |
| 684 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) background | exit 0 | [x] |
| 685 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=background\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) background | exit 0 | [x] |
| 686 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand+backgroundexp\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand+backgroundexp | exit 0 | [x] |
| 687 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand+backgroundexp\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand+backgroundexp | exit 0 | [x] |
| 688 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=backgroundunique\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) backgroundunique | exit 0 | [x] |
| 689 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=backgroundunique\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) backgroundunique | exit 0 | [x] |
| 690 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=background+gamma\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) background+gamma | exit 0 | [x] |
| 691 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=background+gamma\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) background+gamma | exit 0 | [x] |
| 692 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=quantize\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) quantize | exit 0 | [x] |
| 693 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=quantize\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) quantize | exit 0 | [x] |
| 694 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=11\|tr=quantize\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=0 with transform(s) quantize | exit 0 | [x] |
| 695 | `rd\|ct=6\|bd=8\|il=1\|w=19\|h=11\|tr=quantize\|mode=image\|n=2\|seed=8001` | read RGBA/8-bit interlace=1 with transform(s) quantize | exit 0 | [x] |
| 696 | `rd\|ct=2\|bd=8\|il=0\|w=19\|h=11\|tr=expand+bgr+invalpha+addalpha_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=0 with transform(s) expand+bgr+invalpha+addalpha_after | exit 0 | [x] |
| 697 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=11\|tr=expand+bgr+invalpha+addalpha_after\|mode=image\|n=2\|seed=8001` | read RGB/8-bit interlace=1 with transform(s) expand+bgr+invalpha+addalpha_after | exit 0 | [x] |
| 698 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=11\|tr=expand+gray2rgb+addalpha_after+swap16\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=0 with transform(s) expand+gray2rgb+addalpha_after+swap16 | exit 0 | [x] |
| 699 | `rd\|ct=0\|bd=4\|il=1\|w=19\|h=11\|tr=expand+gray2rgb+addalpha_after+swap16\|mode=image\|n=2\|seed=8001` | read GRAY/4-bit interlace=1 with transform(s) expand+gray2rgb+addalpha_after+swap16 | exit 0 | [x] |
| 700 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=strip16+bgr+swapalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) strip16+bgr+swapalpha | exit 0 | [x] |
| 701 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=strip16+bgr+swapalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) strip16+bgr+swapalpha | exit 0 | [x] |
| 702 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=11\|tr=scale16+stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=0 with transform(s) scale16+stripalpha | exit 0 | [x] |
| 703 | `rd\|ct=6\|bd=16\|il=1\|w=19\|h=11\|tr=scale16+stripalpha\|mode=image\|n=2\|seed=8001` | read RGBA/16-bit interlace=1 with transform(s) scale16+stripalpha | exit 0 | [x] |
| 704 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand+rgb2gray\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand+rgb2gray | exit 0 | [x] |
| 705 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand+rgb2gray\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand+rgb2gray | exit 0 | [x] |
| 706 | `rd\|ct=3\|bd=8\|il=0\|w=19\|h=11\|tr=expand+gamma+background\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=0 with transform(s) expand+gamma+background | exit 0 | [x] |
| 707 | `rd\|ct=3\|bd=8\|il=1\|w=19\|h=11\|tr=expand+gamma+background\|mode=image\|n=2\|seed=8001` | read PALETTE/8-bit interlace=1 with transform(s) expand+gamma+background | exit 0 | [x] |

## B9 — Read pipeline — png_read_png transform mask

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 708 | `rd\|ct=2\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=0\|n=2\|seed=9001` | png_read_png(IDENTITY) on RGB/8-bit interlace=0 | exit 0 | [x] |
| 709 | `rd\|ct=2\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=0\|n=2\|seed=9001` | png_read_png(IDENTITY) on RGB/8-bit interlace=1 | exit 0 | [x] |
| 710 | `rd\|ct=2\|bd=16\|il=0\|w=17\|h=9\|mode=png\|rt=1\|n=2\|seed=9001` | png_read_png(STRIP_16) on RGB/16-bit interlace=0 | exit 0 | [x] |
| 711 | `rd\|ct=2\|bd=16\|il=1\|w=17\|h=9\|mode=png\|rt=1\|n=2\|seed=9001` | png_read_png(STRIP_16) on RGB/16-bit interlace=1 | exit 0 | [x] |
| 712 | `rd\|ct=6\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=2\|n=2\|seed=9001` | png_read_png(STRIP_ALPHA) on RGBA/8-bit interlace=0 | exit 0 | [x] |
| 713 | `rd\|ct=6\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=2\|n=2\|seed=9001` | png_read_png(STRIP_ALPHA) on RGBA/8-bit interlace=1 | exit 0 | [x] |
| 714 | `rd\|ct=0\|bd=4\|il=0\|w=17\|h=9\|mode=png\|rt=4\|n=2\|seed=9001` | png_read_png(PACKING) on GRAY/4-bit interlace=0 | exit 0 | [x] |
| 715 | `rd\|ct=0\|bd=4\|il=1\|w=17\|h=9\|mode=png\|rt=4\|n=2\|seed=9001` | png_read_png(PACKING) on GRAY/4-bit interlace=1 | exit 0 | [x] |
| 716 | `rd\|ct=0\|bd=2\|il=0\|w=17\|h=9\|mode=png\|rt=8\|n=2\|seed=9001` | png_read_png(PACKSWAP) on GRAY/2-bit interlace=0 | exit 0 | [x] |
| 717 | `rd\|ct=0\|bd=2\|il=1\|w=17\|h=9\|mode=png\|rt=8\|n=2\|seed=9001` | png_read_png(PACKSWAP) on GRAY/2-bit interlace=1 | exit 0 | [x] |
| 718 | `rd\|ct=3\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=16\|n=2\|seed=9001` | png_read_png(EXPAND) on PALETTE/8-bit interlace=0 | exit 0 | [x] |
| 719 | `rd\|ct=3\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=16\|n=2\|seed=9001` | png_read_png(EXPAND) on PALETTE/8-bit interlace=1 | exit 0 | [x] |
| 720 | `rd\|ct=0\|bd=1\|il=0\|w=17\|h=9\|mode=png\|rt=32\|n=2\|seed=9001` | png_read_png(INVERT_MONO) on GRAY/1-bit interlace=0 | exit 0 | [x] |
| 721 | `rd\|ct=0\|bd=1\|il=1\|w=17\|h=9\|mode=png\|rt=32\|n=2\|seed=9001` | png_read_png(INVERT_MONO) on GRAY/1-bit interlace=1 | exit 0 | [x] |
| 722 | `rd\|ct=2\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=64\|n=2\|seed=9001` | png_read_png(SHIFT) on RGB/8-bit interlace=0 | exit 0 | [x] |
| 723 | `rd\|ct=2\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=64\|n=2\|seed=9001` | png_read_png(SHIFT) on RGB/8-bit interlace=1 | exit 0 | [x] |
| 724 | `rd\|ct=2\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=128\|n=2\|seed=9001` | png_read_png(BGR) on RGB/8-bit interlace=0 | exit 0 | [x] |
| 725 | `rd\|ct=2\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=128\|n=2\|seed=9001` | png_read_png(BGR) on RGB/8-bit interlace=1 | exit 0 | [x] |
| 726 | `rd\|ct=6\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=256\|n=2\|seed=9001` | png_read_png(SWAP_ALPHA) on RGBA/8-bit interlace=0 | exit 0 | [x] |
| 727 | `rd\|ct=6\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=256\|n=2\|seed=9001` | png_read_png(SWAP_ALPHA) on RGBA/8-bit interlace=1 | exit 0 | [x] |
| 728 | `rd\|ct=2\|bd=16\|il=0\|w=17\|h=9\|mode=png\|rt=512\|n=2\|seed=9001` | png_read_png(SWAP_ENDIAN) on RGB/16-bit interlace=0 | exit 0 | [x] |
| 729 | `rd\|ct=2\|bd=16\|il=1\|w=17\|h=9\|mode=png\|rt=512\|n=2\|seed=9001` | png_read_png(SWAP_ENDIAN) on RGB/16-bit interlace=1 | exit 0 | [x] |
| 730 | `rd\|ct=6\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=1024\|n=2\|seed=9001` | png_read_png(INVERT_ALPHA) on RGBA/8-bit interlace=0 | exit 0 | [x] |
| 731 | `rd\|ct=6\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=1024\|n=2\|seed=9001` | png_read_png(INVERT_ALPHA) on RGBA/8-bit interlace=1 | exit 0 | [x] |
| 732 | `rd\|ct=0\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=8192\|n=2\|seed=9001` | png_read_png(GRAY_TO_RGB) on GRAY/8-bit interlace=0 | exit 0 | [x] |
| 733 | `rd\|ct=0\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=8192\|n=2\|seed=9001` | png_read_png(GRAY_TO_RGB) on GRAY/8-bit interlace=1 | exit 0 | [x] |
| 734 | `rd\|ct=2\|bd=8\|il=0\|w=17\|h=9\|mode=png\|rt=16384\|n=2\|seed=9001` | png_read_png(EXPAND_16) on RGB/8-bit interlace=0 | exit 0 | [x] |
| 735 | `rd\|ct=2\|bd=8\|il=1\|w=17\|h=9\|mode=png\|rt=16384\|n=2\|seed=9001` | png_read_png(EXPAND_16) on RGB/8-bit interlace=1 | exit 0 | [x] |
| 736 | `rd\|ct=2\|bd=16\|il=0\|w=17\|h=9\|mode=png\|rt=32768\|n=2\|seed=9001` | png_read_png(SCALE_16) on RGB/16-bit interlace=0 | exit 0 | [x] |
| 737 | `rd\|ct=2\|bd=16\|il=1\|w=17\|h=9\|mode=png\|rt=32768\|n=2\|seed=9001` | png_read_png(SCALE_16) on RGB/16-bit interlace=1 | exit 0 | [x] |
| 738 | `rd\|ct=3\|bd=4\|il=0\|w=17\|h=9\|mode=png\|rt=8208\|n=2\|seed=9001` | png_read_png(EXPAND\|GRAY_TO_RGB) on PALETTE/4-bit interlace=0 | exit 0 | [x] |
| 739 | `rd\|ct=3\|bd=4\|il=1\|w=17\|h=9\|mode=png\|rt=8208\|n=2\|seed=9001` | png_read_png(EXPAND\|GRAY_TO_RGB) on PALETTE/4-bit interlace=1 | exit 0 | [x] |
| 740 | `rd\|ct=6\|bd=16\|il=0\|w=17\|h=9\|mode=png\|rt=1153\|n=2\|seed=9001` | png_read_png(STRIP_16\|BGR\|INVERT_ALPHA) on RGBA/16-bit interlace=0 | exit 0 | [x] |
| 741 | `rd\|ct=6\|bd=16\|il=1\|w=17\|h=9\|mode=png\|rt=1153\|n=2\|seed=9001` | png_read_png(STRIP_16\|BGR\|INVERT_ALPHA) on RGBA/16-bit interlace=1 | exit 0 | [x] |

## B10 — Read pipeline — ancillary chunk sets, stream layout, options, shapes

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 742 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=none\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [none] | exit 0 | [x] |
| 743 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=none\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [none] | exit 0 | [x] |
| 744 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=none\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [none] | exit 0 | [x] |
| 745 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=none\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [none] | exit 0 | [x] |
| 746 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=none\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [none] | exit 0 | [x] |
| 747 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gama\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [gama] | exit 0 | [x] |
| 748 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=gama\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [gama] | exit 0 | [x] |
| 749 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=gama\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [gama] | exit 0 | [x] |
| 750 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=gama\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [gama] | exit 0 | [x] |
| 751 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=gama\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [gama] | exit 0 | [x] |
| 752 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=chrm\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 753 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=chrm\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 754 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=chrm\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 755 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=chrm\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [chrm] | exit 0 | [x] |
| 756 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=chrm\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [chrm] | exit 0 | [x] |
| 757 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamachrm\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 758 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=gamachrm\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 759 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=gamachrm\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 760 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=gamachrm\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 761 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=gamachrm\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [gamachrm] | exit 0 | [x] |
| 762 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=srgb\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 763 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=srgb\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 764 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=srgb\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 765 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=srgb\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [srgb] | exit 0 | [x] |
| 766 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=srgb\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [srgb] | exit 0 | [x] |
| 767 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=sbit\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 768 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=sbit\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 769 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=sbit\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 770 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=sbit\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [sbit] | exit 0 | [x] |
| 771 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=sbit\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [sbit] | exit 0 | [x] |
| 772 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=trns\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [trns] | exit 0 | [x] |
| 773 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=trns\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [trns] | exit 0 | [x] |
| 774 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=trns\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [trns] | exit 0 | [x] |
| 775 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=trns\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [trns] | exit 0 | [x] |
| 776 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=trns\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [trns] | exit 0 | [x] |
| 777 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=bkgd\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 778 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=bkgd\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 779 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=bkgd\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 780 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=bkgd\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [bkgd] | exit 0 | [x] |
| 781 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=bkgd\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [bkgd] | exit 0 | [x] |
| 782 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=hist\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [hist] | exit 0 | [x] |
| 783 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=hist\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [hist] | exit 0; 1 warning(s): hIST: out of place | [x] |
| 784 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=hist\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [hist] | exit 0 | [x] |
| 785 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=hist\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [hist] | exit 0 | [x] |
| 786 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=hist\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [hist] | exit 0 | [x] |
| 787 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=phys\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [phys] | exit 0 | [x] |
| 788 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=phys\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [phys] | exit 0 | [x] |
| 789 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=phys\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [phys] | exit 0 | [x] |
| 790 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=phys\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [phys] | exit 0 | [x] |
| 791 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=phys\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [phys] | exit 0 | [x] |
| 792 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=offs\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [offs] | exit 0 | [x] |
| 793 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=offs\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [offs] | exit 0 | [x] |
| 794 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=offs\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [offs] | exit 0 | [x] |
| 795 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=offs\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [offs] | exit 0 | [x] |
| 796 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=offs\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [offs] | exit 0 | [x] |
| 797 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=scal\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [scal] | exit 0 | [x] |
| 798 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=scal\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [scal] | exit 0 | [x] |
| 799 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=scal\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [scal] | exit 0 | [x] |
| 800 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=scal\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [scal] | exit 0 | [x] |
| 801 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=scal\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [scal] | exit 0 | [x] |
| 802 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=pcal\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [pcal] | exit 0 | [x] |
| 803 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=pcal\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [pcal] | exit 0 | [x] |
| 804 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=pcal\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [pcal] | exit 0 | [x] |
| 805 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=pcal\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [pcal] | exit 0 | [x] |
| 806 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=pcal\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [pcal] | exit 0 | [x] |
| 807 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=splt\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [splt] | exit 0 | [x] |
| 808 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=splt\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [splt] | exit 0 | [x] |
| 809 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=splt\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [splt] | exit 0 | [x] |
| 810 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=splt\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [splt] | exit 0 | [x] |
| 811 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=splt\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [splt] | exit 0 | [x] |
| 812 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=text\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [text] | exit 0 | [x] |
| 813 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=text\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [text] | exit 0 | [x] |
| 814 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=text\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [text] | exit 0 | [x] |
| 815 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=text\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [text] | exit 0 | [x] |
| 816 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=text\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [text] | exit 0 | [x] |
| 817 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=time\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [time] | exit 0 | [x] |
| 818 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=time\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [time] | exit 0 | [x] |
| 819 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=time\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [time] | exit 0 | [x] |
| 820 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=time\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [time] | exit 0 | [x] |
| 821 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=time\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [time] | exit 0 | [x] |
| 822 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=exif\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [exif] | exit 0 | [x] |
| 823 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=exif\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [exif] | exit 0 | [x] |
| 824 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=exif\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [exif] | exit 0 | [x] |
| 825 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=exif\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [exif] | exit 0 | [x] |
| 826 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=exif\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [exif] | exit 0 | [x] |
| 827 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=cicp\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [cicp] | exit 0 | [x] |
| 828 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=cicp\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [cicp] | exit 0 | [x] |
| 829 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=cicp\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [cicp] | exit 0 | [x] |
| 830 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=cicp\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [cicp] | exit 0 | [x] |
| 831 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=cicp\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [cicp] | exit 0 | [x] |
| 832 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=clli\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [clli] | exit 0 | [x] |
| 833 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=clli\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [clli] | exit 0 | [x] |
| 834 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=clli\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [clli] | exit 0 | [x] |
| 835 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=clli\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [clli] | exit 0 | [x] |
| 836 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=clli\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [clli] | exit 0 | [x] |
| 837 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=mdcv\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [mdcv] | exit 0 | [x] |
| 838 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=mdcv\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [mdcv] | exit 0 | [x] |
| 839 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=mdcv\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [mdcv] | exit 0 | [x] |
| 840 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=mdcv\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [mdcv] | exit 0 | [x] |
| 841 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=mdcv\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [mdcv] | exit 0 | [x] |
| 842 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 843 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=iccp\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 844 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=iccp\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 845 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=iccp\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [iccp] | exit 0 | [x] |
| 846 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=iccp\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [iccp] | exit 0 | [x] |
| 847 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=unk\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [unk] | exit 0 | [x] |
| 848 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=unk\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [unk] | exit 0 | [x] |
| 849 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=unk\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [unk] | exit 0 | [x] |
| 850 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=unk\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [unk] | exit 0 | [x] |
| 851 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=unk\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [unk] | exit 0 | [x] |
| 852 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=tail\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [tail] | exit 0 | [x] |
| 853 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=tail\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [tail] | exit 0 | [x] |
| 854 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=tail\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [tail] | exit 0 | [x] |
| 855 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=tail\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [tail] | exit 0 | [x] |
| 856 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=tail\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [tail] | exit 0 | [x] |
| 857 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=plte\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [plte] | exit 0 | [x] |
| 858 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=plte\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [plte] | exit 0 | [x] |
| 859 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=plte\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [plte] | exit 0; 1 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 860 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=plte\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [plte] | exit 0 | [x] |
| 861 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=plte\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [plte] | exit 0; 1 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 862 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamachrmsbittrnsbkgdphysoffsscaltextexiftail\|mode=image\|n=1\|seed=10001` | read RGB/8-bit with ancillary set [gamachrmsbittrnsbkgdphysoffsscaltextexiftail] | exit 0 | [x] |
| 863 | `rd\|ct=3\|bd=8\|w=13\|h=7\|x=gamachrmsbittrnsbkgdphysoffsscaltextexiftail\|mode=image\|n=1\|seed=10001` | read PALETTE/8-bit with ancillary set [gamachrmsbittrnsbkgdphysoffsscaltextexiftail] | exit 0 | [x] |
| 864 | `rd\|ct=0\|bd=8\|w=13\|h=7\|x=gamachrmsbittrnsbkgdphysoffsscaltextexiftail\|mode=image\|n=1\|seed=10001` | read GRAY/8-bit with ancillary set [gamachrmsbittrnsbkgdphysoffsscaltextexiftail] | exit 0 | [x] |
| 865 | `rd\|ct=6\|bd=8\|w=13\|h=7\|x=gamachrmsbittrnsbkgdphysoffsscaltextexiftail\|mode=image\|n=1\|seed=10001` | read RGBA/8-bit with ancillary set [gamachrmsbittrnsbkgdphysoffsscaltextexiftail] | exit 0 | [x] |
| 866 | `rd\|ct=4\|bd=16\|w=13\|h=7\|x=gamachrmsbittrnsbkgdphysoffsscaltextexiftail\|mode=image\|n=1\|seed=10001` | read GRAY_ALPHA/16-bit with ancillary set [gamachrmsbittrnsbkgdphysoffsscaltextexiftail] | exit 0 | [x] |
| 867 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=1\|mode=image\|n=1\|seed=10002` | read with IDAT split into 1 byte pieces | exit 0 | [x] |
| 868 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=2\|mode=image\|n=1\|seed=10002` | read with IDAT split into 2 byte pieces | exit 0 | [x] |
| 869 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=3\|mode=image\|n=1\|seed=10002` | read with IDAT split into 3 byte pieces | exit 0 | [x] |
| 870 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=7\|mode=image\|n=1\|seed=10002` | read with IDAT split into 7 byte pieces | exit 0 | [x] |
| 871 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=64\|mode=image\|n=1\|seed=10002` | read with IDAT split into 64 byte pieces | exit 0 | [x] |
| 872 | `rd\|ct=2\|bd=8\|w=31\|h=17\|split=0\|mode=image\|n=1\|seed=10002` | read with IDAT split into single byte pieces | exit 0 | [x] |
| 873 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=0\|crca=0\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(0, 0) | exit 0 | [x] |
| 874 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=1\|crca=1\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(1, 1) | exit 0 | [x] |
| 875 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=2\|crca=2\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(2, 2) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 876 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=3\|crca=3\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(3, 3) | exit 0 | [x] |
| 877 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=4\|crca=4\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(4, 4) | exit 0 | [x] |
| 878 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=gamatext\|crc=5\|crca=5\|mode=image\|n=1\|seed=10003` | read with png_set_crc_action(5, 5) | exit 0 | [x] |
| 879 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=2\|optv=2\|mode=image\|n=1\|seed=10004` | read with png_set_option(2, 2) | exit 0 | [x] |
| 880 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=2\|optv=3\|mode=image\|n=1\|seed=10004` | read with png_set_option(2, 3) | exit 0 | [x] |
| 881 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=4\|optv=2\|mode=image\|n=1\|seed=10004` | read with png_set_option(4, 2) | exit 0 | [x] |
| 882 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=4\|optv=3\|mode=image\|n=1\|seed=10004` | read with png_set_option(4, 3) | exit 0 | [x] |
| 883 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=8\|optv=2\|mode=image\|n=1\|seed=10004` | read with png_set_option(8, 2) | exit 0 | [x] |
| 884 | `rd\|ct=2\|bd=8\|w=13\|h=7\|x=iccp\|opt=8\|optv=3\|mode=image\|n=1\|seed=10004` | read with png_set_option(8, 3) | exit 0 | [x] |
| 885 | `rd\|ct=3\|bd=8\|w=13\|h=7\|idx=0\|mode=image\|n=1\|seed=10005` | read palette image with png_set_check_for_invalid_index(0) | exit 0 | [x] |
| 886 | `rd\|ct=3\|bd=8\|w=13\|h=7\|idx=1\|mode=image\|n=1\|seed=10005` | read palette image with png_set_check_for_invalid_index(1) | exit 0 | [x] |
| 887 | `rd\|ct=2\|bd=8\|w=13\|h=7\|rstat=1\|mode=image\|n=1\|seed=10006` | read with png_set_read_status_fn callback | exit 0 | [x] |
| 888 | `rd\|ct=2\|bd=8\|w=13\|h=7\|mng=5\|mode=image\|n=1\|seed=10007` | read with png_permit_mng_features(PNG_ALL_MNG_FEATURES) | exit 0; 3 warning(s): MNG features are not allowed in a PNG datastream | [x] |
| 889 | `rd\|ct=2\|bd=8\|w=13\|h=7\|benign=1\|mode=image\|n=1\|seed=10008` | read with png_set_benign_errors(1) | exit 0 | [x] |
| 890 | `rd\|ct=6\|bd=8\|il=0\|w=1\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 1x1 interlace=0 | exit 0 | [x] |
| 891 | `rd\|ct=6\|bd=8\|il=1\|w=1\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 1x1 interlace=1 | exit 0 | [x] |
| 892 | `rd\|ct=6\|bd=8\|il=0\|w=1\|h=33\|mode=image\|n=1\|seed=10009` | read extreme shape 1x33 interlace=0 | exit 0 | [x] |
| 893 | `rd\|ct=6\|bd=8\|il=1\|w=1\|h=33\|mode=image\|n=1\|seed=10009` | read extreme shape 1x33 interlace=1 | exit 0 | [x] |
| 894 | `rd\|ct=6\|bd=8\|il=0\|w=33\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 33x1 interlace=0 | exit 0 | [x] |
| 895 | `rd\|ct=6\|bd=8\|il=1\|w=33\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 33x1 interlace=1 | exit 0 | [x] |
| 896 | `rd\|ct=6\|bd=8\|il=0\|w=2\|h=2\|mode=image\|n=1\|seed=10009` | read extreme shape 2x2 interlace=0 | exit 0 | [x] |
| 897 | `rd\|ct=6\|bd=8\|il=1\|w=2\|h=2\|mode=image\|n=1\|seed=10009` | read extreme shape 2x2 interlace=1 | exit 0 | [x] |
| 898 | `rd\|ct=6\|bd=8\|il=0\|w=7\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 7x1 interlace=0 | exit 0 | [x] |
| 899 | `rd\|ct=6\|bd=8\|il=1\|w=7\|h=1\|mode=image\|n=1\|seed=10009` | read extreme shape 7x1 interlace=1 | exit 0 | [x] |
| 900 | `rd\|ct=6\|bd=8\|il=0\|w=1\|h=7\|mode=image\|n=1\|seed=10009` | read extreme shape 1x7 interlace=0 | exit 0 | [x] |
| 901 | `rd\|ct=6\|bd=8\|il=1\|w=1\|h=7\|mode=image\|n=1\|seed=10009` | read extreme shape 1x7 interlace=1 | exit 0 | [x] |
| 902 | `rd\|ct=6\|bd=8\|il=0\|w=64\|h=64\|mode=image\|n=1\|seed=10009` | read extreme shape 64x64 interlace=0 | exit 0 | [x] |
| 903 | `rd\|ct=6\|bd=8\|il=1\|w=64\|h=64\|mode=image\|n=1\|seed=10009` | read extreme shape 64x64 interlace=1 | exit 0 | [x] |
| 904 | `rd\|ct=6\|bd=8\|il=0\|w=8\|h=8\|mode=image\|n=1\|seed=10009` | read extreme shape 8x8 interlace=0 | exit 0 | [x] |
| 905 | `rd\|ct=6\|bd=8\|il=1\|w=8\|h=8\|mode=image\|n=1\|seed=10009` | read extreme shape 8x8 interlace=1 | exit 0 | [x] |

## B11 — Unknown-chunk handling matrix

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 906 | `unk\|keep=0\|cb=0\|seed=11001` | png_set_keep_unknown_chunks(default=0), user callback=0 | exit 0 | [x] |
| 907 | `unk\|keep=0\|cb=1\|seed=11001` | png_set_keep_unknown_chunks(default=0), user callback=1 | exit 70; png_error: forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks; 1 warning(s): prIv: Saving unknown chunk: | [x] |
| 908 | `unk\|keep=1\|cb=0\|seed=11001` | png_set_keep_unknown_chunks(default=1), user callback=0 | exit 0 | [x] |
| 909 | `unk\|keep=1\|cb=1\|seed=11001` | png_set_keep_unknown_chunks(default=1), user callback=1 | exit 70; png_error: forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks; 1 warning(s): prIv: Saving unknown chunk: | [x] |
| 910 | `unk\|keep=2\|cb=0\|seed=11001` | png_set_keep_unknown_chunks(default=2), user callback=0 | exit 0 | [x] |
| 911 | `unk\|keep=2\|cb=1\|seed=11001` | png_set_keep_unknown_chunks(default=2), user callback=1 | exit 0 | [x] |
| 912 | `unk\|keep=3\|cb=0\|seed=11001` | png_set_keep_unknown_chunks(default=3), user callback=0 | exit 0 | [x] |
| 913 | `unk\|keep=3\|cb=1\|seed=11001` | png_set_keep_unknown_chunks(default=3), user callback=1 | exit 0 | [x] |
| 914 | `unk\|keep=1\|keep2=0\|list=prVt,prIv,tEXt,gAMA\|seed=11002` | per-chunk keep=0 for prVt,prIv,tEXt,gAMA | exit 0 | [x] |
| 915 | `unk\|keep=1\|keep2=1\|list=prVt,prIv,tEXt,gAMA\|seed=11002` | per-chunk keep=1 for prVt,prIv,tEXt,gAMA | exit 0 | [x] |
| 916 | `unk\|keep=1\|keep2=2\|list=prVt,prIv,tEXt,gAMA\|seed=11002` | per-chunk keep=2 for prVt,prIv,tEXt,gAMA | exit 0 | [x] |
| 917 | `unk\|keep=1\|keep2=3\|list=prVt,prIv,tEXt,gAMA\|seed=11002` | per-chunk keep=3 for prVt,prIv,tEXt,gAMA | exit 0 | [x] |

## B12 — Progressive (push) reader

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 918 | `prog\|ct=0\|bd=1\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/1-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 919 | `prog\|ct=0\|bd=1\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/1-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 920 | `prog\|ct=0\|bd=2\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/2-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 921 | `prog\|ct=0\|bd=2\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/2-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 922 | `prog\|ct=0\|bd=4\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/4-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 923 | `prog\|ct=0\|bd=4\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/4-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 924 | `prog\|ct=0\|bd=8\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/8-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 925 | `prog\|ct=0\|bd=8\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/8-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 926 | `prog\|ct=0\|bd=16\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/16-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 927 | `prog\|ct=0\|bd=16\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY/16-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 928 | `prog\|ct=2\|bd=8\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGB/8-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 929 | `prog\|ct=2\|bd=8\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGB/8-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 930 | `prog\|ct=2\|bd=16\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGB/16-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 931 | `prog\|ct=2\|bd=16\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGB/16-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 932 | `prog\|ct=3\|bd=1\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/1-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 933 | `prog\|ct=3\|bd=1\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/1-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 934 | `prog\|ct=3\|bd=2\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/2-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 935 | `prog\|ct=3\|bd=2\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/2-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 936 | `prog\|ct=3\|bd=4\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/4-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 937 | `prog\|ct=3\|bd=4\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/4-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 938 | `prog\|ct=3\|bd=8\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/8-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 939 | `prog\|ct=3\|bd=8\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read PALETTE/8-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 940 | `prog\|ct=4\|bd=8\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY_ALPHA/8-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 941 | `prog\|ct=4\|bd=8\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY_ALPHA/8-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 942 | `prog\|ct=4\|bd=16\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY_ALPHA/16-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 943 | `prog\|ct=4\|bd=16\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read GRAY_ALPHA/16-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 944 | `prog\|ct=6\|bd=8\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGBA/8-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 945 | `prog\|ct=6\|bd=8\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGBA/8-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 946 | `prog\|ct=6\|bd=16\|il=0\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGBA/16-bit interlace=0 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 947 | `prog\|ct=6\|bd=16\|il=1\|w=19\|h=11\|feed=7\|seed=12001` | progressive read RGBA/16-bit interlace=1 | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 948 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=1\|seed=12002` | progressive read fed 1 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 949 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=2\|seed=12002` | progressive read fed 2 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 950 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=3\|seed=12002` | progressive read fed 3 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 951 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=5\|seed=12002` | progressive read fed 5 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 952 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=13\|seed=12002` | progressive read fed 13 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 953 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=100\|seed=12002` | progressive read fed 100 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 954 | `prog\|ct=6\|bd=8\|w=23\|h=13\|feed=100000\|seed=12002` | progressive read fed 100000 bytes at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 955 | `prog\|ct=2\|bd=8\|w=23\|h=13\|feed=11\|pause=1\|seed=12003` | progressive read with png_process_data_pause every 1 feeds | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 956 | `prog\|ct=2\|bd=8\|w=23\|h=13\|feed=11\|pause=2\|seed=12003` | progressive read with png_process_data_pause every 2 feeds | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 957 | `prog\|ct=2\|bd=8\|w=23\|h=13\|feed=11\|pause=5\|seed=12003` | progressive read with png_process_data_pause every 5 feeds | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 958 | `prog\|ct=3\|bd=8\|w=17\|h=9\|x=gamachrmtext\|feed=9\|seed=12004` | progressive read with ancillary set [gamachrmtext] | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 959 | `prog\|ct=3\|bd=8\|w=17\|h=9\|x=trnsbkgd\|feed=9\|seed=12004` | progressive read with ancillary set [trnsbkgd] | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 960 | `prog\|ct=3\|bd=8\|w=17\|h=9\|x=unktail\|feed=9\|seed=12004` | progressive read with ancillary set [unktail] | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 961 | `prog\|ct=3\|bd=8\|w=17\|h=9\|x=iccp\|feed=9\|seed=12004` | progressive read with ancillary set [iccp] | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 962 | `prog\|ct=3\|bd=8\|w=17\|h=9\|x=splthist\|feed=9\|seed=12004` | progressive read with ancillary set [splthist] | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 963 | `prog\|ct=2\|bd=8\|w=29\|h=13\|split=1\|feed=7\|seed=12005` | progressive read of stream with IDAT split into 1 byte chunks | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 964 | `prog\|ct=2\|bd=8\|w=29\|h=13\|split=3\|feed=7\|seed=12005` | progressive read of stream with IDAT split into 3 byte chunks | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 965 | `prog\|ct=2\|bd=8\|w=29\|h=13\|split=20\|feed=7\|seed=12005` | progressive read of stream with IDAT split into 20 byte chunks | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |

## B13 — Simplified read API

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 966 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 967 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 968 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 969 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 970 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 971 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 972 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 973 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 974 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=0\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_GRAY | exit 0 | [x] |
| 975 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 976 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 977 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 978 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 979 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 980 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 981 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 982 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 983 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=1\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_GA | exit 0 | [x] |
| 984 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 985 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 986 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 987 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 988 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 989 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 990 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 991 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 992 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=33\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_AG | exit 0 | [x] |
| 993 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 994 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 995 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 996 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 997 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 998 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 999 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 1000 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 1001 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=2\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_RGB | exit 0 | [x] |
| 1002 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1003 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1004 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1005 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1006 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1007 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1008 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1009 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1010 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=18\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_BGR | exit 0 | [x] |
| 1011 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1012 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1013 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1014 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1015 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1016 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1017 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1018 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1019 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=3\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1020 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1021 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1022 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1023 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1024 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1025 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1026 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1027 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1028 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=35\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_ARGB | exit 0 | [x] |
| 1029 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1030 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1031 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1032 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1033 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1034 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1035 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1036 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1037 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=19\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_BGRA | exit 0 | [x] |
| 1038 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1039 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1040 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1041 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1042 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1043 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1044 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1045 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1046 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=51\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_ABGR | exit 0 | [x] |
| 1047 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1048 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1049 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1050 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1051 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1052 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1053 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1054 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1055 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=4\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_LINEAR_Y | exit 0 | [x] |
| 1056 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1057 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1058 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1059 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1060 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1061 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1062 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1063 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1064 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=5\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_LINEAR_Y_ALPHA | exit 0 | [x] |
| 1065 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1066 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1067 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1068 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1069 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1070 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1071 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1072 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1073 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=6\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_LINEAR_RGB | exit 0 | [x] |
| 1074 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1075 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1076 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1077 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1078 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1079 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1080 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1081 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1082 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=7\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1083 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1084 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1085 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1086 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1087 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1088 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1089 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1090 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1091 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=10\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_RGB_COLORMAP | exit 0 | [x] |
| 1092 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1093 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1094 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1095 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1096 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1097 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1098 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1099 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1100 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=26\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_BGR_COLORMAP | exit 0 | [x] |
| 1101 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1102 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1103 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1104 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1105 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1106 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1107 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1108 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1109 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=11\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_RGBA_COLORMAP | exit 0 | [x] |
| 1110 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1111 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1112 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1113 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1114 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1115 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1116 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1117 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1118 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=43\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_ARGB_COLORMAP | exit 0 | [x] |
| 1119 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1120 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1121 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1122 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1123 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1124 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1125 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1126 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1127 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=8\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_GRAY_COLORMAP | exit 0 | [x] |
| 1128 | `sr\|ct=0\|bd=8\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of GRAY/8-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1129 | `sr\|ct=2\|bd=8\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of RGB/8-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1130 | `sr\|ct=3\|bd=8\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of PALETTE/8-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1131 | `sr\|ct=4\|bd=8\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of GRAY_ALPHA/8-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1132 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of RGBA/8-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1133 | `sr\|ct=0\|bd=16\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of GRAY/16-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1134 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of RGBA/16-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1135 | `sr\|ct=0\|bd=1\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of GRAY/1-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1136 | `sr\|ct=3\|bd=4\|w=17\|h=11\|fmt=9\|n=1\|seed=13001` | simplified read of PALETTE/4-bit into PNG_FORMAT_GA_COLORMAP | exit 0 | [x] |
| 1137 | `sr\|ct=6\|bd=8\|il=0\|w=17\|h=11\|fmt=3\|n=1\|seed=13002` | simplified read interlace=0 into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1138 | `sr\|ct=6\|bd=8\|il=0\|w=17\|h=11\|fmt=7\|n=1\|seed=13002` | simplified read interlace=0 into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1139 | `sr\|ct=6\|bd=8\|il=1\|w=17\|h=11\|fmt=3\|n=1\|seed=13002` | simplified read interlace=1 into PNG_FORMAT_RGBA | exit 0 | [x] |
| 1140 | `sr\|ct=6\|bd=8\|il=1\|w=17\|h=11\|fmt=7\|n=1\|seed=13002` | simplified read interlace=1 into PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |
| 1141 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=3\|neg=1\|n=1\|seed=13003` | simplified read with negative row_stride (bottom-up buffer) | exit 0 | [x] |
| 1142 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=0\|bg=1\|n=1\|seed=13004` | simplified read of RGBA source into format 0x0 with a background colour | exit 0 | [x] |
| 1143 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=2\|bg=1\|n=1\|seed=13004` | simplified read of RGBA source into format 0x2 with a background colour | exit 0 | [x] |
| 1144 | `sr\|ct=6\|bd=8\|w=17\|h=11\|fmt=10\|bg=1\|n=1\|seed=13004` | simplified read of RGBA source into format 0xa with a background colour | exit 0 | [x] |
| 1145 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=7\|flags=1\|n=1\|seed=13005` | simplified read with png_image flags 0x1 | exit 0 | [x] |
| 1146 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=7\|flags=2\|n=1\|seed=13005` | simplified read with png_image flags 0x2 | exit 0 | [x] |
| 1147 | `sr\|ct=6\|bd=16\|w=17\|h=11\|fmt=7\|flags=4\|n=1\|seed=13005` | simplified read with png_image flags 0x4 | exit 0 | [x] |
| 1148 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=gama\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [gama] | exit 0 | [x] |
| 1149 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=srgb\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [srgb] | exit 0 | [x] |
| 1150 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=chrm\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [chrm] | exit 0 | [x] |
| 1151 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=trns\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [trns] | exit 0 | [x] |
| 1152 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=bkgd\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [bkgd] | exit 0 | [x] |
| 1153 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=iccp\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [iccp] | exit 0 | [x] |
| 1154 | `sr\|ct=2\|bd=8\|w=17\|h=11\|x=gamachrm\|fmt=3\|n=1\|seed=13006` | simplified read of source with ancillary set [gamachrm] | exit 0 | [x] |

## B14 — Simplified write API

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1155 | `sw\|fmt=0\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GRAY convert_to_8bit=0 | exit 0 | [x] |
| 1156 | `sw\|fmt=0\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GRAY convert_to_8bit=1 | exit 0 | [x] |
| 1157 | `sw\|fmt=1\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GA convert_to_8bit=0 | exit 0 | [x] |
| 1158 | `sw\|fmt=1\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GA convert_to_8bit=1 | exit 0 | [x] |
| 1159 | `sw\|fmt=33\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_AG convert_to_8bit=0 | exit 0 | [x] |
| 1160 | `sw\|fmt=33\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_AG convert_to_8bit=1 | exit 0 | [x] |
| 1161 | `sw\|fmt=2\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGB convert_to_8bit=0 | exit 0 | [x] |
| 1162 | `sw\|fmt=2\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGB convert_to_8bit=1 | exit 0 | [x] |
| 1163 | `sw\|fmt=18\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGR convert_to_8bit=0 | exit 0 | [x] |
| 1164 | `sw\|fmt=18\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGR convert_to_8bit=1 | exit 0 | [x] |
| 1165 | `sw\|fmt=3\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGBA convert_to_8bit=0 | exit 0 | [x] |
| 1166 | `sw\|fmt=3\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGBA convert_to_8bit=1 | exit 0 | [x] |
| 1167 | `sw\|fmt=35\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ARGB convert_to_8bit=0 | exit 0 | [x] |
| 1168 | `sw\|fmt=35\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ARGB convert_to_8bit=1 | exit 0 | [x] |
| 1169 | `sw\|fmt=19\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGRA convert_to_8bit=0 | exit 0 | [x] |
| 1170 | `sw\|fmt=19\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGRA convert_to_8bit=1 | exit 0 | [x] |
| 1171 | `sw\|fmt=51\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ABGR convert_to_8bit=0 | exit 0 | [x] |
| 1172 | `sw\|fmt=51\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ABGR convert_to_8bit=1 | exit 0 | [x] |
| 1173 | `sw\|fmt=4\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_Y convert_to_8bit=0 | exit 0 | [x] |
| 1174 | `sw\|fmt=4\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_Y convert_to_8bit=1 | exit 0 | [x] |
| 1175 | `sw\|fmt=5\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_Y_ALPHA convert_to_8bit=0 | exit 0 | [x] |
| 1176 | `sw\|fmt=5\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_Y_ALPHA convert_to_8bit=1 | exit 0 | [x] |
| 1177 | `sw\|fmt=6\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_RGB convert_to_8bit=0 | exit 0 | [x] |
| 1178 | `sw\|fmt=6\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_RGB convert_to_8bit=1 | exit 0 | [x] |
| 1179 | `sw\|fmt=7\|cme=0\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_RGB_ALPHA convert_to_8bit=0 | exit 0 | [x] |
| 1180 | `sw\|fmt=7\|cme=0\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_LINEAR_RGB_ALPHA convert_to_8bit=1 | exit 0 | [x] |
| 1181 | `sw\|fmt=10\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGB_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1182 | `sw\|fmt=10\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGB_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1183 | `sw\|fmt=26\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGR_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1184 | `sw\|fmt=26\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_BGR_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1185 | `sw\|fmt=11\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGBA_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1186 | `sw\|fmt=11\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_RGBA_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1187 | `sw\|fmt=43\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ARGB_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1188 | `sw\|fmt=43\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_ARGB_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1189 | `sw\|fmt=8\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GRAY_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1190 | `sw\|fmt=8\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GRAY_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1191 | `sw\|fmt=9\|cme=64\|c8=0\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GA_COLORMAP convert_to_8bit=0 | exit 0 | [x] |
| 1192 | `sw\|fmt=9\|cme=64\|c8=1\|w=17\|h=11\|n=1\|seed=14001` | simplified write PNG_FORMAT_GA_COLORMAP convert_to_8bit=1 | exit 0 | [x] |
| 1193 | `sw\|fmt=3\|flags=0\|w=23\|h=13\|n=1\|seed=14002` | simplified write RGBA with flags 0x0 | exit 0 | [x] |
| 1194 | `sw\|fmt=3\|flags=1\|w=23\|h=13\|n=1\|seed=14002` | simplified write RGBA with flags 0x1 | exit 0 | [x] |
| 1195 | `sw\|fmt=3\|flags=2\|w=23\|h=13\|n=1\|seed=14002` | simplified write RGBA with flags 0x2 | exit 0 | [x] |
| 1196 | `sw\|fmt=3\|neg=1\|w=17\|h=11\|n=1\|seed=14003` | simplified write with negative row_stride | exit 0 | [x] |
| 1197 | `sw\|fmt=3\|w=1\|h=1\|n=1\|seed=14004` | simplified write shape 1x1 | exit 0 | [x] |
| 1198 | `sw\|fmt=3\|w=1\|h=40\|n=1\|seed=14004` | simplified write shape 1x40 | exit 0 | [x] |
| 1199 | `sw\|fmt=3\|w=40\|h=1\|n=1\|seed=14004` | simplified write shape 40x1 | exit 0 | [x] |
| 1200 | `sw\|fmt=3\|w=2\|h=3\|n=1\|seed=14004` | simplified write shape 2x3 | exit 0 | [x] |
| 1201 | `sw\|fmt=3\|w=64\|h=64\|n=1\|seed=14004` | simplified write shape 64x64 | exit 0 | [x] |
| 1202 | `sw\|fmt=10\|cme=1\|w=17\|h=11\|n=1\|seed=14005` | simplified write colour-mapped with 1 entries | exit 0 | [x] |
| 1203 | `sw\|fmt=10\|cme=2\|w=17\|h=11\|n=1\|seed=14005` | simplified write colour-mapped with 2 entries | exit 0 | [x] |
| 1204 | `sw\|fmt=10\|cme=16\|w=17\|h=11\|n=1\|seed=14005` | simplified write colour-mapped with 16 entries | exit 0 | [x] |
| 1205 | `sw\|fmt=10\|cme=255\|w=17\|h=11\|n=1\|seed=14005` | simplified write colour-mapped with 255 entries | exit 0 | [x] |
| 1206 | `sw\|fmt=10\|cme=256\|w=17\|h=11\|n=1\|seed=14005` | simplified write colour-mapped with 256 entries | exit 0 | [x] |

## B15 — png_set_* / png_get_* round trips and library-wide state

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1207 | `sg\|g=ihdr\|seed=1` | randomized png_set_* / png_get_* round trip: ihdr (seed 1) | exit 0 | [x] |
| 1208 | `sg\|g=ihdr\|seed=2` | randomized png_set_* / png_get_* round trip: ihdr (seed 2) | exit 0 | [x] |
| 1209 | `sg\|g=gama\|seed=1` | randomized png_set_* / png_get_* round trip: gama (seed 1) | exit 0 | [x] |
| 1210 | `sg\|g=gama\|seed=2` | randomized png_set_* / png_get_* round trip: gama (seed 2) | exit 0 | [x] |
| 1211 | `sg\|g=chrm\|seed=1` | randomized png_set_* / png_get_* round trip: chrm (seed 1) | exit 0 | [x] |
| 1212 | `sg\|g=chrm\|seed=2` | randomized png_set_* / png_get_* round trip: chrm (seed 2) | exit 0 | [x] |
| 1213 | `sg\|g=plte\|seed=1` | randomized png_set_* / png_get_* round trip: plte (seed 1) | exit 0 | [x] |
| 1214 | `sg\|g=plte\|seed=2` | randomized png_set_* / png_get_* round trip: plte (seed 2) | exit 0 | [x] |
| 1215 | `sg\|g=trns\|seed=1` | randomized png_set_* / png_get_* round trip: trns (seed 1) | exit 0; 39 warning(s): tRNS chunk has out-of-range samples for bit_depth | [x] |
| 1216 | `sg\|g=trns\|seed=2` | randomized png_set_* / png_get_* round trip: trns (seed 2) | exit 0; 49 warning(s): tRNS chunk has out-of-range samples for bit_depth | [x] |
| 1217 | `sg\|g=misc\|seed=1` | randomized png_set_* / png_get_* round trip: misc (seed 1) | exit 0; 59 warning(s): fixed point overflow ignored / Ignoring invalid time value | [x] |
| 1218 | `sg\|g=misc\|seed=2` | randomized png_set_* / png_get_* round trip: misc (seed 2) | exit 0; 52 warning(s): Ignoring invalid time value / fixed point overflow ignored | [x] |
| 1219 | `sg\|g=scal\|seed=1` | randomized png_set_* / png_get_* round trip: scal (seed 1) | exit 70; png_error: Invalid sCAL unit | [x] |
| 1220 | `sg\|g=scal\|seed=2` | randomized png_set_* / png_get_* round trip: scal (seed 2) | exit 70; png_error: Invalid sCAL unit | [x] |
| 1221 | `sg\|g=newchunks\|seed=1` | randomized png_set_* / png_get_* round trip: newchunks (seed 1) | exit 0; 80 warning(s): Invalid cICP matrix coefficients | [x] |
| 1222 | `sg\|g=newchunks\|seed=2` | randomized png_set_* / png_get_* round trip: newchunks (seed 2) | exit 0; 80 warning(s): Invalid cICP matrix coefficients | [x] |
| 1223 | `sg\|g=text\|seed=1` | randomized png_set_* / png_get_* round trip: text (seed 1) | exit 0 | [x] |
| 1224 | `sg\|g=text\|seed=2` | randomized png_set_* / png_get_* round trip: text (seed 2) | exit 0 | [x] |
| 1225 | `sg\|g=iccp\|seed=1` | randomized png_set_* / png_get_* round trip: iccp (seed 1) | exit 0 | [x] |
| 1226 | `sg\|g=iccp\|seed=2` | randomized png_set_* / png_get_* round trip: iccp (seed 2) | exit 0 | [x] |
| 1227 | `sg\|g=splt\|seed=1` | randomized png_set_* / png_get_* round trip: splt (seed 1) | exit 0 | [x] |
| 1228 | `sg\|g=splt\|seed=2` | randomized png_set_* / png_get_* round trip: splt (seed 2) | exit 0 | [x] |
| 1229 | `sg\|g=pcal\|seed=1` | randomized png_set_* / png_get_* round trip: pcal (seed 1) | exit 0 | [x] |
| 1230 | `sg\|g=pcal\|seed=2` | randomized png_set_* / png_get_* round trip: pcal (seed 2) | exit 0 | [x] |
| 1231 | `sg\|g=exif\|seed=1` | randomized png_set_* / png_get_* round trip: exif (seed 1) | exit 0 | [x] |
| 1232 | `sg\|g=exif\|seed=2` | randomized png_set_* / png_get_* round trip: exif (seed 2) | exit 0 | [x] |
| 1233 | `sg\|g=hist\|seed=1` | randomized png_set_* / png_get_* round trip: hist (seed 1) | exit 0 | [x] |
| 1234 | `sg\|g=hist\|seed=2` | randomized png_set_* / png_get_* round trip: hist (seed 2) | exit 0 | [x] |
| 1235 | `sg\|g=bkgd\|seed=1` | randomized png_set_* / png_get_* round trip: bkgd (seed 1) | exit 0 | [x] |
| 1236 | `sg\|g=bkgd\|seed=2` | randomized png_set_* / png_get_* round trip: bkgd (seed 2) | exit 0 | [x] |
| 1237 | `lim\|seed=1` | user limits, png_set_option matrix, MNG features, allocator (seed 1) | exit 0 | [x] |
| 1238 | `lim\|seed=2` | user limits, png_set_option matrix, MNG features, allocator (seed 2) | exit 0 | [x] |
| 1239 | `util\|f=version` | library-wide accessor: version | exit 0 | [x] |
| 1240 | `util\|f=graypal` | library-wide accessor: graypal | exit 0 | [x] |
| 1241 | `util\|f=sigcmp\|seed=1` | pure utility function sigcmp over randomized inputs (seed 1) | exit 0 | [x] |
| 1242 | `util\|f=sigcmp\|seed=2` | pure utility function sigcmp over randomized inputs (seed 2) | exit 0 | [x] |
| 1243 | `util\|f=intfns\|seed=1` | pure utility function intfns over randomized inputs (seed 1) | exit 0 | [x] |
| 1244 | `util\|f=intfns\|seed=77` | pure utility function intfns over randomized inputs (seed 77) | exit 0 | [x] |
| 1245 | `util\|f=uint31\|seed=3` | pure utility function uint31 over randomized inputs (seed 3) | exit 0 | [x] |
| 1246 | `util\|f=rfc1123\|seed=5` | pure utility function rfc1123 over randomized inputs (seed 5) | signal 11; no record written | [x] |
| 1247 | `util\|f=rfc1123\|seed=6` | pure utility function rfc1123 over randomized inputs (seed 6) | signal 11; no record written | [x] |
| 1248 | `util\|f=timet\|seed=7` | pure utility function timet over randomized inputs (seed 7) | exit 0 | [x] |

## B16 — Randomized read cross-product sweep

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1249 | `rd\|ct=0\|bd=1\|il=0\|w=16\|h=21\|tr=expand16\|mode=startimage\|x=gamachrm\|split=3\|n=2\|seed=16000` | fuzz read GRAY/1-bit il=0 16x21 tr=[expand16] via startimage chunks=[gamachrm] idat_split=3 | exit 0 | [x] |
| 1250 | `rd\|ct=0\|bd=2\|il=0\|w=3\|h=23\|tr=stripalpha+bgr\|mode=rowonly\|x=unk\|split=1\|n=2\|seed=16001` | fuzz read GRAY/2-bit il=0 3x23 tr=[stripalpha+bgr] via rowonly chunks=[unk] idat_split=1 | exit 0 | [x] |
| 1251 | `rd\|ct=0\|bd=4\|il=1\|w=39\|h=22\|tr=invalpha+expand\|mode=rowonly\|x=srgb\|split=0\|n=2\|seed=16002` | fuzz read GRAY/4-bit il=1 39x22 tr=[invalpha+expand] via rowonly chunks=[srgb] idat_split=0 | exit 0 | [x] |
| 1252 | `rd\|ct=0\|bd=8\|il=0\|w=17\|h=15\|tr=gammahigh\|mode=image\|x=scal\|split=3\|n=2\|seed=16003` | fuzz read GRAY/8-bit il=0 17x15 tr=[gammahigh] via image chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1253 | `rd\|ct=0\|bd=16\|il=1\|w=15\|h=5\|tr=none\|mode=disponly\|x=tail\|split=1\|n=2\|seed=16004` | fuzz read GRAY/16-bit il=1 15x5 tr=[none] via disponly chunks=[tail] idat_split=1 | exit 0 | [x] |
| 1254 | `rd\|ct=2\|bd=8\|il=0\|w=15\|h=3\|tr=packing\|mode=disponly\|x=trns\|split=3\|n=2\|seed=16005` | fuzz read RGB/8-bit il=0 15x3 tr=[packing] via disponly chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1255 | `rd\|ct=2\|bd=16\|il=0\|w=26\|h=1\|tr=none\|mode=rows\|x=sbit\|split=3\|n=2\|seed=16006` | fuzz read RGB/16-bit il=0 26x1 tr=[none] via rows chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1256 | `rd\|ct=3\|bd=1\|il=0\|w=21\|h=18\|tr=expand+scale16+gray2rgb\|mode=row\|x=sbit\|split=1\|n=2\|seed=16007` | fuzz read PALETTE/1-bit il=0 21x18 tr=[expand+scale16+gray2rgb] via row chunks=[sbit] idat_split=1 | exit 0 | [x] |
| 1257 | `rd\|ct=3\|bd=2\|il=0\|w=15\|h=2\|tr=gamma+expand+background\|mode=image\|x=scal\|split=0\|n=2\|seed=16008` | fuzz read PALETTE/2-bit il=0 15x2 tr=[gamma+expand+background] via image chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1258 | `rd\|ct=3\|bd=4\|il=0\|w=11\|h=6\|tr=backgroundunique+gray2rgb+background\|mode=disponly\|x=hist\|split=17\|n=2\|seed=16009` | fuzz read PALETTE/4-bit il=0 11x6 tr=[backgroundunique+gray2rgb+background] via disponly chunks=[hist] idat_split=17 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1259 | `rd\|ct=3\|bd=8\|il=0\|w=11\|h=20\|tr=quantize+gammahigh\|mode=rows\|x=tail\|split=0\|n=2\|seed=16010` | fuzz read PALETTE/8-bit il=0 11x20 tr=[quantize+gammahigh] via rows chunks=[tail] idat_split=0 | exit 0; 2 warning(s): IDAT: Read palette index exceeding num_palette | [x] |
| 1260 | `rd\|ct=4\|bd=8\|il=0\|w=32\|h=11\|tr=backgroundexp\|mode=disponly\|x=gamachrm\|split=17\|n=2\|seed=16011` | fuzz read GRAY_ALPHA/8-bit il=0 32x11 tr=[backgroundexp] via disponly chunks=[gamachrm] idat_split=17 | exit 0 | [x] |
| 1261 | `rd\|ct=4\|bd=16\|il=1\|w=36\|h=3\|tr=swapalpha+rgb2graywarn+swap16\|mode=row\|x=sbit\|split=1\|n=2\|seed=16012` | fuzz read GRAY_ALPHA/16-bit il=1 36x3 tr=[swapalpha+rgb2graywarn+swap16] via row chunks=[sbit] idat_split=1 | exit 0 | [x] |
| 1262 | `rd\|ct=6\|bd=8\|il=1\|w=35\|h=16\|tr=swap16+backgroundexp+expand16\|mode=row\|x=sbit\|split=3\|n=2\|seed=16013` | fuzz read RGBA/8-bit il=1 35x16 tr=[swap16+backgroundexp+expand16] via row chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1263 | `rd\|ct=6\|bd=16\|il=0\|w=38\|h=24\|tr=alphastd+pal2rgb+swap16\|mode=startimage\|x=trnsbkgd\|split=17\|n=2\|seed=16014` | fuzz read RGBA/16-bit il=0 38x24 tr=[alphastd+pal2rgb+swap16] via startimage chunks=[trnsbkgd] idat_split=17 | exit 0 | [x] |
| 1264 | `rd\|ct=0\|bd=1\|il=1\|w=25\|h=24\|tr=none\|mode=image\|x=trnsbkgd\|split=17\|n=2\|seed=16015` | fuzz read GRAY/1-bit il=1 25x24 tr=[none] via image chunks=[trnsbkgd] idat_split=17 | exit 0 | [x] |
| 1265 | `rd\|ct=0\|bd=2\|il=0\|w=25\|h=9\|tr=pal2rgb+gamma+interlace\|mode=row\|x=hist\|split=3\|n=2\|seed=16016` | fuzz read GRAY/2-bit il=0 25x9 tr=[pal2rgb+gamma+interlace] via row chunks=[hist] idat_split=3 | exit 0 | [x] |
| 1266 | `rd\|ct=0\|bd=4\|il=1\|w=27\|h=16\|tr=none\|mode=disponly\|x=trns\|split=3\|n=2\|seed=16017` | fuzz read GRAY/4-bit il=1 27x16 tr=[none] via disponly chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1267 | `rd\|ct=0\|bd=8\|il=1\|w=30\|h=1\|tr=shift+rgb2graywarn+background\|mode=startimage\|x=gama\|split=0\|n=2\|seed=16018` | fuzz read GRAY/8-bit il=1 30x1 tr=[shift+rgb2graywarn+background] via startimage chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1268 | `rd\|ct=0\|bd=16\|il=0\|w=39\|h=1\|tr=none\|mode=rowonly\|x=none\|split=1\|n=2\|seed=16019` | fuzz read GRAY/16-bit il=0 39x1 tr=[none] via rowonly chunks=[none] idat_split=1 | exit 0 | [x] |
| 1269 | `rd\|ct=2\|bd=8\|il=0\|w=8\|h=12\|tr=trns2alpha+interlace+packswap\|mode=disponly\|x=hist\|split=17\|n=2\|seed=16020` | fuzz read RGB/8-bit il=0 8x12 tr=[trns2alpha+interlace+packswap] via disponly chunks=[hist] idat_split=17 | exit 0 | [x] |
| 1270 | `rd\|ct=2\|bd=16\|il=0\|w=28\|h=22\|tr=expand16\|mode=rows\|x=trnsbkgd\|split=1\|n=2\|seed=16021` | fuzz read RGB/16-bit il=0 28x22 tr=[expand16] via rows chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1271 | `rd\|ct=3\|bd=1\|il=0\|w=12\|h=15\|tr=none\|mode=rowonly\|x=none\|split=17\|n=2\|seed=16022` | fuzz read PALETTE/1-bit il=0 12x15 tr=[none] via rowonly chunks=[none] idat_split=17 | exit 0 | [x] |
| 1272 | `rd\|ct=3\|bd=2\|il=1\|w=16\|h=2\|tr=none\|mode=rowonly\|x=unk\|split=3\|n=2\|seed=16023` | fuzz read PALETTE/2-bit il=1 16x2 tr=[none] via rowonly chunks=[unk] idat_split=3 | exit 0 | [x] |
| 1273 | `rd\|ct=3\|bd=4\|il=0\|w=11\|h=16\|tr=packing\|mode=rows\|x=physoffs\|split=3\|n=2\|seed=16024` | fuzz read PALETTE/4-bit il=0 11x16 tr=[packing] via rows chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1274 | `rd\|ct=3\|bd=8\|il=0\|w=36\|h=3\|tr=expandgray\|mode=rows\|x=trns\|split=17\|n=2\|seed=16025` | fuzz read PALETTE/8-bit il=0 36x3 tr=[expandgray] via rows chunks=[trns] idat_split=17 | exit 0 | [x] |
| 1275 | `rd\|ct=4\|bd=8\|il=1\|w=9\|h=23\|tr=gray2rgb\|mode=rowonly\|x=gama\|split=0\|n=2\|seed=16026` | fuzz read GRAY_ALPHA/8-bit il=1 9x23 tr=[gray2rgb] via rowonly chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1276 | `rd\|ct=4\|bd=16\|il=0\|w=38\|h=2\|tr=alphaopt+expand16+filler_after\|mode=disponly\|x=clli\|split=0\|n=2\|seed=16027` | fuzz read GRAY_ALPHA/16-bit il=0 38x2 tr=[alphaopt+expand16+filler_after] via disponly chunks=[clli] idat_split=0 | exit 0 | [x] |
| 1277 | `rd\|ct=6\|bd=8\|il=1\|w=16\|h=3\|tr=backgroundexp\|mode=startimage\|x=exif\|split=1\|n=2\|seed=16028` | fuzz read RGBA/8-bit il=1 16x3 tr=[backgroundexp] via startimage chunks=[exif] idat_split=1 | exit 0 | [x] |
| 1278 | `rd\|ct=6\|bd=16\|il=1\|w=32\|h=24\|tr=none\|mode=image\|x=none\|split=17\|n=2\|seed=16029` | fuzz read RGBA/16-bit il=1 32x24 tr=[none] via image chunks=[none] idat_split=17 | exit 0 | [x] |
| 1279 | `rd\|ct=0\|bd=1\|il=1\|w=21\|h=10\|tr=interlace\|mode=rows\|x=bkgd\|split=3\|n=2\|seed=16030` | fuzz read GRAY/1-bit il=1 21x10 tr=[interlace] via rows chunks=[bkgd] idat_split=3 | exit 0 | [x] |
| 1280 | `rd\|ct=0\|bd=2\|il=1\|w=30\|h=19\|tr=interlace+filler_after\|mode=rowonly\|x=trns\|split=17\|n=2\|seed=16031` | fuzz read GRAY/2-bit il=1 30x19 tr=[interlace+filler_after] via rowonly chunks=[trns] idat_split=17 | exit 70; png_error: internal row size calculation error | [x] |
| 1281 | `rd\|ct=0\|bd=4\|il=0\|w=2\|h=24\|tr=packswap+rgb2gray+alphapng\|mode=disponly\|x=pcal\|split=17\|n=2\|seed=16032` | fuzz read GRAY/4-bit il=0 2x24 tr=[packswap+rgb2gray+alphapng] via disponly chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1282 | `rd\|ct=0\|bd=8\|il=0\|w=17\|h=20\|tr=bgr+gamma+interlace\|mode=rows\|x=mdcv\|split=3\|n=2\|seed=16033` | fuzz read GRAY/8-bit il=0 17x20 tr=[bgr+gamma+interlace] via rows chunks=[mdcv] idat_split=3 | exit 0 | [x] |
| 1283 | `rd\|ct=0\|bd=16\|il=0\|w=27\|h=23\|tr=filler_after+stripalpha\|mode=rowonly\|x=trns\|split=3\|n=2\|seed=16034` | fuzz read GRAY/16-bit il=0 27x23 tr=[filler_after+stripalpha] via rowonly chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1284 | `rd\|ct=2\|bd=8\|il=0\|w=15\|h=14\|tr=none\|mode=disponly\|x=none\|split=3\|n=2\|seed=16035` | fuzz read RGB/8-bit il=0 15x14 tr=[none] via disponly chunks=[none] idat_split=3 | exit 0 | [x] |
| 1285 | `rd\|ct=2\|bd=16\|il=0\|w=6\|h=19\|tr=none\|mode=rowonly\|x=time\|split=0\|n=2\|seed=16036` | fuzz read RGB/16-bit il=0 6x19 tr=[none] via rowonly chunks=[time] idat_split=0 | exit 0 | [x] |
| 1286 | `rd\|ct=3\|bd=1\|il=1\|w=39\|h=10\|tr=none\|mode=row\|x=gama\|split=0\|n=2\|seed=16037` | fuzz read PALETTE/1-bit il=1 39x10 tr=[none] via row chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1287 | `rd\|ct=3\|bd=2\|il=1\|w=22\|h=18\|tr=packswap\|mode=rowonly\|x=clli\|split=3\|n=2\|seed=16038` | fuzz read PALETTE/2-bit il=1 22x18 tr=[packswap] via rowonly chunks=[clli] idat_split=3 | exit 0 | [x] |
| 1288 | `rd\|ct=3\|bd=4\|il=1\|w=14\|h=7\|tr=none\|mode=row\|x=trns\|split=1\|n=2\|seed=16039` | fuzz read PALETTE/4-bit il=1 14x7 tr=[none] via row chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1289 | `rd\|ct=3\|bd=8\|il=1\|w=39\|h=10\|tr=gamma+addalpha_before\|mode=rowonly\|x=exif\|split=17\|n=2\|seed=16040` | fuzz read PALETTE/8-bit il=1 39x10 tr=[gamma+addalpha_before] via rowonly chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1290 | `rd\|ct=4\|bd=8\|il=0\|w=13\|h=14\|tr=alphapng+shift\|mode=image\|x=physoffs\|split=0\|n=2\|seed=16041` | fuzz read GRAY_ALPHA/8-bit il=0 13x14 tr=[alphapng+shift] via image chunks=[physoffs] idat_split=0 | exit 0 | [x] |
| 1291 | `rd\|ct=4\|bd=16\|il=0\|w=11\|h=15\|tr=none\|mode=row\|x=plte\|split=0\|n=2\|seed=16042` | fuzz read GRAY_ALPHA/16-bit il=0 11x15 tr=[none] via row chunks=[plte] idat_split=0 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1292 | `rd\|ct=6\|bd=8\|il=1\|w=35\|h=18\|tr=none\|mode=rowonly\|x=iccp\|split=17\|n=2\|seed=16043` | fuzz read RGBA/8-bit il=1 35x18 tr=[none] via rowonly chunks=[iccp] idat_split=17 | exit 0 | [x] |
| 1293 | `rd\|ct=6\|bd=16\|il=1\|w=3\|h=22\|tr=pal2rgb+backgroundexp+trns2alpha\|mode=startimage\|x=trnsbkgd\|split=17\|n=2\|seed=16044` | fuzz read RGBA/16-bit il=1 3x22 tr=[pal2rgb+backgroundexp+trns2alpha] via startimage chunks=[trnsbkgd] idat_split=17 | exit 0 | [x] |
| 1294 | `rd\|ct=0\|bd=1\|il=1\|w=10\|h=15\|tr=none\|mode=rows\|x=splt\|split=1\|n=2\|seed=16045` | fuzz read GRAY/1-bit il=1 10x15 tr=[none] via rows chunks=[splt] idat_split=1 | exit 0 | [x] |
| 1295 | `rd\|ct=0\|bd=2\|il=1\|w=37\|h=6\|tr=none\|mode=image\|x=pcal\|split=17\|n=2\|seed=16046` | fuzz read GRAY/2-bit il=1 37x6 tr=[none] via image chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1296 | `rd\|ct=0\|bd=4\|il=1\|w=29\|h=9\|tr=filler_before+shift+bgr\|mode=rows\|x=splt\|split=1\|n=2\|seed=16047` | fuzz read GRAY/4-bit il=1 29x9 tr=[filler_before+shift+bgr] via rows chunks=[splt] idat_split=1 | exit 70; png_error: internal row size calculation error | [x] |
| 1297 | `rd\|ct=0\|bd=8\|il=1\|w=11\|h=3\|tr=swapalpha+quantize+pal2rgb\|mode=startimage\|x=exif\|split=17\|n=2\|seed=16048` | fuzz read GRAY/8-bit il=1 11x3 tr=[swapalpha+quantize+pal2rgb] via startimage chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1298 | `rd\|ct=0\|bd=16\|il=0\|w=19\|h=16\|tr=rgb2gray\|mode=startimage\|x=chrm\|split=1\|n=2\|seed=16049` | fuzz read GRAY/16-bit il=0 19x16 tr=[rgb2gray] via startimage chunks=[chrm] idat_split=1 | exit 0 | [x] |
| 1299 | `rd\|ct=2\|bd=8\|il=1\|w=32\|h=3\|tr=interlace\|mode=disponly\|x=bkgd\|split=3\|n=2\|seed=16050` | fuzz read RGB/8-bit il=1 32x3 tr=[interlace] via disponly chunks=[bkgd] idat_split=3 | exit 0 | [x] |
| 1300 | `rd\|ct=2\|bd=16\|il=1\|w=11\|h=15\|tr=alphabroken\|mode=row\|x=unk\|split=17\|n=2\|seed=16051` | fuzz read RGB/16-bit il=1 11x15 tr=[alphabroken] via row chunks=[unk] idat_split=17 | exit 0 | [x] |
| 1301 | `rd\|ct=3\|bd=1\|il=0\|w=19\|h=12\|tr=none\|mode=image\|x=clli\|split=3\|n=2\|seed=16052` | fuzz read PALETTE/1-bit il=0 19x12 tr=[none] via image chunks=[clli] idat_split=3 | exit 0 | [x] |
| 1302 | `rd\|ct=3\|bd=2\|il=1\|w=39\|h=12\|tr=alphaopt+swap16\|mode=rows\|x=hist\|split=17\|n=2\|seed=16053` | fuzz read PALETTE/2-bit il=1 39x12 tr=[alphaopt+swap16] via rows chunks=[hist] idat_split=17 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1303 | `rd\|ct=3\|bd=4\|il=0\|w=17\|h=23\|tr=none\|mode=rows\|x=physoffs\|split=3\|n=2\|seed=16054` | fuzz read PALETTE/4-bit il=0 17x23 tr=[none] via rows chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1304 | `rd\|ct=3\|bd=8\|il=0\|w=14\|h=6\|tr=background+filler_after\|mode=disponly\|x=trns\|split=17\|n=2\|seed=16055` | fuzz read PALETTE/8-bit il=0 14x6 tr=[background+filler_after] via disponly chunks=[trns] idat_split=17 | exit 0 | [x] |
| 1305 | `rd\|ct=4\|bd=8\|il=0\|w=32\|h=2\|tr=swap16\|mode=disponly\|x=scal\|split=3\|n=2\|seed=16056` | fuzz read GRAY_ALPHA/8-bit il=0 32x2 tr=[swap16] via disponly chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1306 | `rd\|ct=4\|bd=16\|il=1\|w=1\|h=15\|tr=alphabroken+expand\|mode=disponly\|x=trns\|split=3\|n=2\|seed=16057` | fuzz read GRAY_ALPHA/16-bit il=1 1x15 tr=[alphabroken+expand] via disponly chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1307 | `rd\|ct=6\|bd=8\|il=0\|w=9\|h=13\|tr=alphapng\|mode=rows\|x=mdcv\|split=3\|n=2\|seed=16058` | fuzz read RGBA/8-bit il=0 9x13 tr=[alphapng] via rows chunks=[mdcv] idat_split=3 | exit 0 | [x] |
| 1308 | `rd\|ct=6\|bd=16\|il=1\|w=12\|h=19\|tr=shift\|mode=rowonly\|x=exif\|split=3\|n=2\|seed=16059` | fuzz read RGBA/16-bit il=1 12x19 tr=[shift] via rowonly chunks=[exif] idat_split=3 | exit 0 | [x] |
| 1309 | `rd\|ct=0\|bd=1\|il=0\|w=15\|h=17\|tr=expand+addalpha_after+invmono\|mode=startimage\|x=gamachrm\|split=3\|n=2\|seed=16060` | fuzz read GRAY/1-bit il=0 15x17 tr=[expand+addalpha_after+invmono] via startimage chunks=[gamachrm] idat_split=3 | exit 0 | [x] |
| 1310 | `rd\|ct=0\|bd=2\|il=0\|w=25\|h=9\|tr=none\|mode=image\|x=physoffs\|split=3\|n=2\|seed=16061` | fuzz read GRAY/2-bit il=0 25x9 tr=[none] via image chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1311 | `rd\|ct=0\|bd=4\|il=1\|w=15\|h=17\|tr=invmono+expand16\|mode=startimage\|x=sbit\|split=0\|n=2\|seed=16062` | fuzz read GRAY/4-bit il=1 15x17 tr=[invmono+expand16] via startimage chunks=[sbit] idat_split=0 | exit 0 | [x] |
| 1312 | `rd\|ct=0\|bd=8\|il=1\|w=15\|h=21\|tr=backgroundunique+gammahigh\|mode=startimage\|x=gamachrm\|split=1\|n=2\|seed=16063` | fuzz read GRAY/8-bit il=1 15x21 tr=[backgroundunique+gammahigh] via startimage chunks=[gamachrm] idat_split=1 | exit 0 | [x] |
| 1313 | `rd\|ct=0\|bd=16\|il=0\|w=17\|h=3\|tr=none\|mode=disponly\|x=trnsbkgd\|split=1\|n=2\|seed=16064` | fuzz read GRAY/16-bit il=0 17x3 tr=[none] via disponly chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1314 | `rd\|ct=2\|bd=8\|il=1\|w=21\|h=12\|tr=expand16\|mode=image\|x=iccp\|split=0\|n=2\|seed=16065` | fuzz read RGB/8-bit il=1 21x12 tr=[expand16] via image chunks=[iccp] idat_split=0 | exit 0 | [x] |
| 1315 | `rd\|ct=2\|bd=16\|il=0\|w=34\|h=21\|tr=gray2rgb+expand\|mode=row\|x=clli\|split=17\|n=2\|seed=16066` | fuzz read RGB/16-bit il=0 34x21 tr=[gray2rgb+expand] via row chunks=[clli] idat_split=17 | exit 0 | [x] |
| 1316 | `rd\|ct=3\|bd=1\|il=1\|w=31\|h=23\|tr=pal2rgb+interlace\|mode=disponly\|x=scal\|split=3\|n=2\|seed=16067` | fuzz read PALETTE/1-bit il=1 31x23 tr=[pal2rgb+interlace] via disponly chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1317 | `rd\|ct=3\|bd=2\|il=0\|w=39\|h=2\|tr=none\|mode=rows\|x=bkgd\|split=0\|n=2\|seed=16068` | fuzz read PALETTE/2-bit il=0 39x2 tr=[none] via rows chunks=[bkgd] idat_split=0 | exit 0 | [x] |
| 1318 | `rd\|ct=3\|bd=4\|il=0\|w=6\|h=19\|tr=scale16+quantize\|mode=disponly\|x=text\|split=3\|n=2\|seed=16069` | fuzz read PALETTE/4-bit il=0 6x19 tr=[scale16+quantize] via disponly chunks=[text] idat_split=3 | exit 0 | [x] |
| 1319 | `rd\|ct=3\|bd=8\|il=1\|w=27\|h=4\|tr=stripalpha\|mode=image\|x=none\|split=1\|n=2\|seed=16070` | fuzz read PALETTE/8-bit il=1 27x4 tr=[stripalpha] via image chunks=[none] idat_split=1 | exit 0 | [x] |
| 1320 | `rd\|ct=4\|bd=8\|il=0\|w=15\|h=24\|tr=none\|mode=rows\|x=gama\|split=0\|n=2\|seed=16071` | fuzz read GRAY_ALPHA/8-bit il=0 15x24 tr=[none] via rows chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1321 | `rd\|ct=4\|bd=16\|il=1\|w=13\|h=22\|tr=expand\|mode=image\|x=plte\|split=0\|n=2\|seed=16072` | fuzz read GRAY_ALPHA/16-bit il=1 13x22 tr=[expand] via image chunks=[plte] idat_split=0 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1322 | `rd\|ct=6\|bd=8\|il=1\|w=24\|h=7\|tr=gammahigh+backgroundunique+shift\|mode=rowonly\|x=hist\|split=3\|n=2\|seed=16073` | fuzz read RGBA/8-bit il=1 24x7 tr=[gammahigh+backgroundunique+shift] via rowonly chunks=[hist] idat_split=3 | exit 0 | [x] |
| 1323 | `rd\|ct=6\|bd=16\|il=0\|w=33\|h=13\|tr=alphaopt+backgroundunique\|mode=rowonly\|x=gama\|split=1\|n=2\|seed=16074` | fuzz read RGBA/16-bit il=0 33x13 tr=[alphaopt+backgroundunique] via rowonly chunks=[gama] idat_split=1 | exit 0 | [x] |
| 1324 | `rd\|ct=0\|bd=1\|il=1\|w=8\|h=24\|tr=gamma\|mode=rows\|x=plte\|split=3\|n=2\|seed=16075` | fuzz read GRAY/1-bit il=1 8x24 tr=[gamma] via rows chunks=[plte] idat_split=3 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1325 | `rd\|ct=0\|bd=2\|il=0\|w=18\|h=21\|tr=backgroundunique+trns2alpha\|mode=disponly\|x=iccp\|split=17\|n=2\|seed=16076` | fuzz read GRAY/2-bit il=0 18x21 tr=[backgroundunique+trns2alpha] via disponly chunks=[iccp] idat_split=17 | exit 0 | [x] |
| 1326 | `rd\|ct=0\|bd=4\|il=0\|w=17\|h=3\|tr=backgroundexp+trns2alpha\|mode=rowonly\|x=srgb\|split=17\|n=2\|seed=16077` | fuzz read GRAY/4-bit il=0 17x3 tr=[backgroundexp+trns2alpha] via rowonly chunks=[srgb] idat_split=17 | exit 0 | [x] |
| 1327 | `rd\|ct=0\|bd=8\|il=1\|w=16\|h=16\|tr=scale16+interlace+packswap\|mode=disponly\|x=iccp\|split=1\|n=2\|seed=16078` | fuzz read GRAY/8-bit il=1 16x16 tr=[scale16+interlace+packswap] via disponly chunks=[iccp] idat_split=1 | exit 0 | [x] |
| 1328 | `rd\|ct=0\|bd=16\|il=1\|w=6\|h=6\|tr=swap16\|mode=row\|x=gamachrmsbittrnsbkgdtexttail\|split=0\|n=2\|seed=16079` | fuzz read GRAY/16-bit il=1 6x6 tr=[swap16] via row chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=0 | exit 0 | [x] |
| 1329 | `rd\|ct=2\|bd=8\|il=0\|w=33\|h=19\|tr=packswap\|mode=image\|x=mdcv\|split=17\|n=2\|seed=16080` | fuzz read RGB/8-bit il=0 33x19 tr=[packswap] via image chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1330 | `rd\|ct=2\|bd=16\|il=1\|w=25\|h=8\|tr=pal2rgb+swap16\|mode=disponly\|x=chrm\|split=1\|n=2\|seed=16081` | fuzz read RGB/16-bit il=1 25x8 tr=[pal2rgb+swap16] via disponly chunks=[chrm] idat_split=1 | exit 0 | [x] |
| 1331 | `rd\|ct=3\|bd=1\|il=1\|w=5\|h=24\|tr=none\|mode=startimage\|x=pcal\|split=3\|n=2\|seed=16082` | fuzz read PALETTE/1-bit il=1 5x24 tr=[none] via startimage chunks=[pcal] idat_split=3 | exit 0 | [x] |
| 1332 | `rd\|ct=3\|bd=2\|il=0\|w=29\|h=16\|tr=alphapng\|mode=startimage\|x=none\|split=17\|n=2\|seed=16083` | fuzz read PALETTE/2-bit il=0 29x16 tr=[alphapng] via startimage chunks=[none] idat_split=17 | exit 0 | [x] |
| 1333 | `rd\|ct=3\|bd=4\|il=0\|w=23\|h=21\|tr=alphabroken\|mode=rows\|x=hist\|split=17\|n=2\|seed=16084` | fuzz read PALETTE/4-bit il=0 23x21 tr=[alphabroken] via rows chunks=[hist] idat_split=17 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1334 | `rd\|ct=3\|bd=8\|il=1\|w=6\|h=13\|tr=swap16+filler_before+expandgray\|mode=disponly\|x=splt\|split=17\|n=2\|seed=16085` | fuzz read PALETTE/8-bit il=1 6x13 tr=[swap16+filler_before+expandgray] via disponly chunks=[splt] idat_split=17 | exit 0 | [x] |
| 1335 | `rd\|ct=4\|bd=8\|il=1\|w=21\|h=17\|tr=shift+trns2alpha\|mode=rows\|x=unk\|split=3\|n=2\|seed=16086` | fuzz read GRAY_ALPHA/8-bit il=1 21x17 tr=[shift+trns2alpha] via rows chunks=[unk] idat_split=3 | exit 0 | [x] |
| 1336 | `rd\|ct=4\|bd=16\|il=1\|w=25\|h=10\|tr=quantize+packswap+expand\|mode=rowonly\|x=gamachrm\|split=0\|n=2\|seed=16087` | fuzz read GRAY_ALPHA/16-bit il=1 25x10 tr=[quantize+packswap+expand] via rowonly chunks=[gamachrm] idat_split=0 | exit 0 | [x] |
| 1337 | `rd\|ct=6\|bd=8\|il=1\|w=39\|h=9\|tr=pal2rgb+packswap+expand\|mode=image\|x=text\|split=0\|n=2\|seed=16088` | fuzz read RGBA/8-bit il=1 39x9 tr=[pal2rgb+packswap+expand] via image chunks=[text] idat_split=0 | exit 0 | [x] |
| 1338 | `rd\|ct=6\|bd=16\|il=1\|w=25\|h=14\|tr=shift+alphaopt\|mode=rowonly\|x=hist\|split=3\|n=2\|seed=16089` | fuzz read RGBA/16-bit il=1 25x14 tr=[shift+alphaopt] via rowonly chunks=[hist] idat_split=3 | exit 0 | [x] |
| 1339 | `rd\|ct=0\|bd=1\|il=0\|w=4\|h=15\|tr=none\|mode=row\|x=tail\|split=1\|n=2\|seed=16090` | fuzz read GRAY/1-bit il=0 4x15 tr=[none] via row chunks=[tail] idat_split=1 | exit 0 | [x] |
| 1340 | `rd\|ct=0\|bd=2\|il=0\|w=22\|h=7\|tr=trns2alpha+gamma\|mode=image\|x=srgb\|split=0\|n=2\|seed=16091` | fuzz read GRAY/2-bit il=0 22x7 tr=[trns2alpha+gamma] via image chunks=[srgb] idat_split=0 | exit 0 | [x] |
| 1341 | `rd\|ct=0\|bd=4\|il=1\|w=33\|h=8\|tr=none\|mode=startimage\|x=splt\|split=17\|n=2\|seed=16092` | fuzz read GRAY/4-bit il=1 33x8 tr=[none] via startimage chunks=[splt] idat_split=17 | exit 0 | [x] |
| 1342 | `rd\|ct=0\|bd=8\|il=1\|w=5\|h=7\|tr=filler_before+expand16\|mode=disponly\|x=iccp\|split=17\|n=2\|seed=16093` | fuzz read GRAY/8-bit il=1 5x7 tr=[filler_before+expand16] via disponly chunks=[iccp] idat_split=17 | exit 0 | [x] |
| 1343 | `rd\|ct=0\|bd=16\|il=1\|w=31\|h=12\|tr=rgb2gray+alphastd+gammahigh\|mode=rows\|x=cicp\|split=1\|n=2\|seed=16094` | fuzz read GRAY/16-bit il=1 31x12 tr=[rgb2gray+alphastd+gammahigh] via rows chunks=[cicp] idat_split=1 | exit 0 | [x] |
| 1344 | `rd\|ct=2\|bd=8\|il=1\|w=35\|h=21\|tr=none\|mode=rowonly\|x=scal\|split=0\|n=2\|seed=16095` | fuzz read RGB/8-bit il=1 35x21 tr=[none] via rowonly chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1345 | `rd\|ct=2\|bd=16\|il=1\|w=7\|h=7\|tr=rgb2gray+alphaopt\|mode=image\|x=text\|split=0\|n=2\|seed=16096` | fuzz read RGB/16-bit il=1 7x7 tr=[rgb2gray+alphaopt] via image chunks=[text] idat_split=0 | exit 0 | [x] |
| 1346 | `rd\|ct=3\|bd=1\|il=0\|w=17\|h=4\|tr=addalpha_before+gammahigh\|mode=disponly\|x=mdcv\|split=3\|n=2\|seed=16097` | fuzz read PALETTE/1-bit il=0 17x4 tr=[addalpha_before+gammahigh] via disponly chunks=[mdcv] idat_split=3 | exit 0 | [x] |
| 1347 | `rd\|ct=3\|bd=2\|il=1\|w=30\|h=7\|tr=alphaopt+trns2alpha\|mode=disponly\|x=trns\|split=0\|n=2\|seed=16098` | fuzz read PALETTE/2-bit il=1 30x7 tr=[alphaopt+trns2alpha] via disponly chunks=[trns] idat_split=0 | exit 0 | [x] |
| 1348 | `rd\|ct=3\|bd=4\|il=1\|w=5\|h=16\|tr=expand16+gray2rgb+interlace\|mode=rowonly\|x=pcal\|split=0\|n=2\|seed=16099` | fuzz read PALETTE/4-bit il=1 5x16 tr=[expand16+gray2rgb+interlace] via rowonly chunks=[pcal] idat_split=0 | exit 0 | [x] |
| 1349 | `rd\|ct=3\|bd=8\|il=1\|w=2\|h=10\|tr=expand\|mode=rows\|x=physoffs\|split=0\|n=2\|seed=16100` | fuzz read PALETTE/8-bit il=1 2x10 tr=[expand] via rows chunks=[physoffs] idat_split=0 | exit 0 | [x] |
| 1350 | `rd\|ct=4\|bd=8\|il=0\|w=31\|h=9\|tr=backgroundexp+pal2rgb\|mode=image\|x=iccp\|split=3\|n=2\|seed=16101` | fuzz read GRAY_ALPHA/8-bit il=0 31x9 tr=[backgroundexp+pal2rgb] via image chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1351 | `rd\|ct=4\|bd=16\|il=1\|w=17\|h=21\|tr=none\|mode=rows\|x=text\|split=3\|n=2\|seed=16102` | fuzz read GRAY_ALPHA/16-bit il=1 17x21 tr=[none] via rows chunks=[text] idat_split=3 | exit 0 | [x] |
| 1352 | `rd\|ct=6\|bd=8\|il=0\|w=19\|h=24\|tr=packing+gray2rgb\|mode=rows\|x=cicp\|split=3\|n=2\|seed=16103` | fuzz read RGBA/8-bit il=0 19x24 tr=[packing+gray2rgb] via rows chunks=[cicp] idat_split=3 | exit 0 | [x] |
| 1353 | `rd\|ct=6\|bd=16\|il=0\|w=13\|h=24\|tr=none\|mode=rowonly\|x=sbit\|split=17\|n=2\|seed=16104` | fuzz read RGBA/16-bit il=0 13x24 tr=[none] via rowonly chunks=[sbit] idat_split=17 | exit 0 | [x] |
| 1354 | `rd\|ct=0\|bd=1\|il=0\|w=36\|h=17\|tr=alphaopt\|mode=startimage\|x=unk\|split=1\|n=2\|seed=16105` | fuzz read GRAY/1-bit il=0 36x17 tr=[alphaopt] via startimage chunks=[unk] idat_split=1 | exit 0 | [x] |
| 1355 | `rd\|ct=0\|bd=2\|il=1\|w=13\|h=1\|tr=background\|mode=startimage\|x=splt\|split=17\|n=2\|seed=16106` | fuzz read GRAY/2-bit il=1 13x1 tr=[background] via startimage chunks=[splt] idat_split=17 | exit 0 | [x] |
| 1356 | `rd\|ct=0\|bd=4\|il=1\|w=2\|h=22\|tr=none\|mode=rowonly\|x=gama\|split=1\|n=2\|seed=16107` | fuzz read GRAY/4-bit il=1 2x22 tr=[none] via rowonly chunks=[gama] idat_split=1 | exit 0 | [x] |
| 1357 | `rd\|ct=0\|bd=8\|il=0\|w=31\|h=3\|tr=none\|mode=row\|x=text\|split=3\|n=2\|seed=16108` | fuzz read GRAY/8-bit il=0 31x3 tr=[none] via row chunks=[text] idat_split=3 | exit 0 | [x] |
| 1358 | `rd\|ct=0\|bd=16\|il=0\|w=24\|h=12\|tr=packswap\|mode=rowonly\|x=plte\|split=3\|n=2\|seed=16109` | fuzz read GRAY/16-bit il=0 24x12 tr=[packswap] via rowonly chunks=[plte] idat_split=3 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1359 | `rd\|ct=2\|bd=8\|il=1\|w=38\|h=21\|tr=none\|mode=disponly\|x=physoffs\|split=17\|n=2\|seed=16110` | fuzz read RGB/8-bit il=1 38x21 tr=[none] via disponly chunks=[physoffs] idat_split=17 | exit 0 | [x] |
| 1360 | `rd\|ct=2\|bd=16\|il=1\|w=31\|h=18\|tr=scale16+rgb2graywarn\|mode=startimage\|x=unk\|split=0\|n=2\|seed=16111` | fuzz read RGB/16-bit il=1 31x18 tr=[scale16+rgb2graywarn] via startimage chunks=[unk] idat_split=0 | exit 0; 70 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 1361 | `rd\|ct=3\|bd=1\|il=1\|w=40\|h=17\|tr=alphastd\|mode=image\|x=sbit\|split=0\|n=2\|seed=16112` | fuzz read PALETTE/1-bit il=1 40x17 tr=[alphastd] via image chunks=[sbit] idat_split=0 | exit 0 | [x] |
| 1362 | `rd\|ct=3\|bd=2\|il=0\|w=30\|h=3\|tr=interlace\|mode=rowonly\|x=gamachrmsbittrnsbkgdtexttail\|split=1\|n=2\|seed=16113` | fuzz read PALETTE/2-bit il=0 30x3 tr=[interlace] via rowonly chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=1 | exit 0 | [x] |
| 1363 | `rd\|ct=3\|bd=4\|il=1\|w=9\|h=8\|tr=addalpha_after\|mode=image\|x=plte\|split=17\|n=2\|seed=16114` | fuzz read PALETTE/4-bit il=1 9x8 tr=[addalpha_after] via image chunks=[plte] idat_split=17 | exit 0 | [x] |
| 1364 | `rd\|ct=3\|bd=8\|il=0\|w=20\|h=3\|tr=expandgray+invmono+bgr\|mode=row\|x=unk\|split=1\|n=2\|seed=16115` | fuzz read PALETTE/8-bit il=0 20x3 tr=[expandgray+invmono+bgr] via row chunks=[unk] idat_split=1 | exit 0 | [x] |
| 1365 | `rd\|ct=4\|bd=8\|il=0\|w=37\|h=3\|tr=none\|mode=rows\|x=text\|split=17\|n=2\|seed=16116` | fuzz read GRAY_ALPHA/8-bit il=0 37x3 tr=[none] via rows chunks=[text] idat_split=17 | exit 0 | [x] |
| 1366 | `rd\|ct=4\|bd=16\|il=1\|w=8\|h=24\|tr=backgroundexp+gray2rgb\|mode=image\|x=hist\|split=17\|n=2\|seed=16117` | fuzz read GRAY_ALPHA/16-bit il=1 8x24 tr=[backgroundexp+gray2rgb] via image chunks=[hist] idat_split=17 | exit 0 | [x] |
| 1367 | `rd\|ct=6\|bd=8\|il=1\|w=18\|h=19\|tr=none\|mode=rows\|x=time\|split=1\|n=2\|seed=16118` | fuzz read RGBA/8-bit il=1 18x19 tr=[none] via rows chunks=[time] idat_split=1 | exit 0 | [x] |
| 1368 | `rd\|ct=6\|bd=16\|il=0\|w=3\|h=6\|tr=none\|mode=row\|x=sbit\|split=3\|n=2\|seed=16119` | fuzz read RGBA/16-bit il=0 3x6 tr=[none] via row chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1369 | `rd\|ct=0\|bd=1\|il=1\|w=7\|h=3\|tr=expand16+bgr\|mode=disponly\|x=time\|split=1\|n=2\|seed=16120` | fuzz read GRAY/1-bit il=1 7x3 tr=[expand16+bgr] via disponly chunks=[time] idat_split=1 | exit 0 | [x] |
| 1370 | `rd\|ct=0\|bd=2\|il=1\|w=40\|h=1\|tr=expand16\|mode=startimage\|x=scal\|split=0\|n=2\|seed=16121` | fuzz read GRAY/2-bit il=1 40x1 tr=[expand16] via startimage chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1371 | `rd\|ct=0\|bd=4\|il=0\|w=35\|h=5\|tr=swap16+alphastd\|mode=rows\|x=none\|split=17\|n=2\|seed=16122` | fuzz read GRAY/4-bit il=0 35x5 tr=[swap16+alphastd] via rows chunks=[none] idat_split=17 | exit 0 | [x] |
| 1372 | `rd\|ct=0\|bd=8\|il=0\|w=6\|h=19\|tr=pal2rgb+backgroundexp\|mode=startimage\|x=unk\|split=3\|n=2\|seed=16123` | fuzz read GRAY/8-bit il=0 6x19 tr=[pal2rgb+backgroundexp] via startimage chunks=[unk] idat_split=3 | exit 0 | [x] |
| 1373 | `rd\|ct=0\|bd=16\|il=1\|w=31\|h=10\|tr=invmono+packing\|mode=disponly\|x=pcal\|split=0\|n=2\|seed=16124` | fuzz read GRAY/16-bit il=1 31x10 tr=[invmono+packing] via disponly chunks=[pcal] idat_split=0 | exit 0 | [x] |
| 1374 | `rd\|ct=2\|bd=8\|il=0\|w=16\|h=3\|tr=gamma+alphabroken+bgr\|mode=rows\|x=tail\|split=17\|n=2\|seed=16125` | fuzz read RGB/8-bit il=0 16x3 tr=[gamma+alphabroken+bgr] via rows chunks=[tail] idat_split=17 | exit 0 | [x] |
| 1375 | `rd\|ct=2\|bd=16\|il=0\|w=1\|h=16\|tr=alphapng+expandgray+scale16\|mode=rows\|x=gama\|split=3\|n=2\|seed=16126` | fuzz read RGB/16-bit il=0 1x16 tr=[alphapng+expandgray+scale16] via rows chunks=[gama] idat_split=3 | exit 0 | [x] |
| 1376 | `rd\|ct=3\|bd=1\|il=0\|w=18\|h=2\|tr=none\|mode=disponly\|x=plte\|split=1\|n=2\|seed=16127` | fuzz read PALETTE/1-bit il=0 18x2 tr=[none] via disponly chunks=[plte] idat_split=1 | exit 0 | [x] |
| 1377 | `rd\|ct=3\|bd=2\|il=0\|w=8\|h=5\|tr=none\|mode=disponly\|x=clli\|split=17\|n=2\|seed=16128` | fuzz read PALETTE/2-bit il=0 8x5 tr=[none] via disponly chunks=[clli] idat_split=17 | exit 0 | [x] |
| 1378 | `rd\|ct=3\|bd=4\|il=0\|w=12\|h=16\|tr=quantize+trns2alpha+rgb2gray\|mode=disponly\|x=sbit\|split=17\|n=2\|seed=16129` | fuzz read PALETTE/4-bit il=0 12x16 tr=[quantize+trns2alpha+rgb2gray] via disponly chunks=[sbit] idat_split=17 | exit 0 | [x] |
| 1379 | `rd\|ct=3\|bd=8\|il=1\|w=27\|h=2\|tr=filler_before+packing+addalpha_after\|mode=rows\|x=hist\|split=1\|n=2\|seed=16130` | fuzz read PALETTE/8-bit il=1 27x2 tr=[filler_before+packing+addalpha_after] via rows chunks=[hist] idat_split=1 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1380 | `rd\|ct=4\|bd=8\|il=0\|w=5\|h=17\|tr=none\|mode=startimage\|x=cicp\|split=1\|n=2\|seed=16131` | fuzz read GRAY_ALPHA/8-bit il=0 5x17 tr=[none] via startimage chunks=[cicp] idat_split=1 | exit 0 | [x] |
| 1381 | `rd\|ct=4\|bd=16\|il=1\|w=12\|h=13\|tr=filler_before+interlace\|mode=startimage\|x=splt\|split=3\|n=2\|seed=16132` | fuzz read GRAY_ALPHA/16-bit il=1 12x13 tr=[filler_before+interlace] via startimage chunks=[splt] idat_split=3 | exit 0 | [x] |
| 1382 | `rd\|ct=6\|bd=8\|il=1\|w=3\|h=7\|tr=invalpha\|mode=row\|x=clli\|split=1\|n=2\|seed=16133` | fuzz read RGBA/8-bit il=1 3x7 tr=[invalpha] via row chunks=[clli] idat_split=1 | exit 0 | [x] |
| 1383 | `rd\|ct=6\|bd=16\|il=0\|w=39\|h=4\|tr=filler_before+alphastd+invmono\|mode=row\|x=iccp\|split=17\|n=2\|seed=16134` | fuzz read RGBA/16-bit il=0 39x4 tr=[filler_before+alphastd+invmono] via row chunks=[iccp] idat_split=17 | exit 0 | [x] |
| 1384 | `rd\|ct=0\|bd=1\|il=1\|w=19\|h=10\|tr=swapalpha+backgroundexp+addalpha_after\|mode=disponly\|x=exif\|split=1\|n=2\|seed=16135` | fuzz read GRAY/1-bit il=1 19x10 tr=[swapalpha+backgroundexp+addalpha_after] via disponly chunks=[exif] idat_split=1 | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 1385 | `rd\|ct=0\|bd=2\|il=0\|w=22\|h=3\|tr=alphaopt+scale16+alphastd\|mode=disponly\|x=plte\|split=17\|n=2\|seed=16136` | fuzz read GRAY/2-bit il=0 22x3 tr=[alphaopt+scale16+alphastd] via disponly chunks=[plte] idat_split=17 | exit 70; png_error: conflicting calls to set alpha mode and background; 1 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1386 | `rd\|ct=0\|bd=4\|il=0\|w=20\|h=6\|tr=expand16+gammahigh+expandgray\|mode=rowonly\|x=physoffs\|split=3\|n=2\|seed=16137` | fuzz read GRAY/4-bit il=0 20x6 tr=[expand16+gammahigh+expandgray] via rowonly chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1387 | `rd\|ct=0\|bd=8\|il=0\|w=40\|h=15\|tr=rgb2graywarn+invmono\|mode=image\|x=iccp\|split=3\|n=2\|seed=16138` | fuzz read GRAY/8-bit il=0 40x15 tr=[rgb2graywarn+invmono] via image chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1388 | `rd\|ct=0\|bd=16\|il=0\|w=15\|h=13\|tr=none\|mode=startimage\|x=hist\|split=0\|n=2\|seed=16139` | fuzz read GRAY/16-bit il=0 15x13 tr=[none] via startimage chunks=[hist] idat_split=0 | exit 0 | [x] |
| 1389 | `rd\|ct=2\|bd=8\|il=1\|w=13\|h=4\|tr=gray2rgb+addalpha_before\|mode=rowonly\|x=plte\|split=3\|n=2\|seed=16140` | fuzz read RGB/8-bit il=1 13x4 tr=[gray2rgb+addalpha_before] via rowonly chunks=[plte] idat_split=3 | exit 0 | [x] |
| 1390 | `rd\|ct=2\|bd=16\|il=1\|w=13\|h=9\|tr=backgroundexp\|mode=disponly\|x=plte\|split=17\|n=2\|seed=16141` | fuzz read RGB/16-bit il=1 13x9 tr=[backgroundexp] via disponly chunks=[plte] idat_split=17 | exit 0 | [x] |
| 1391 | `rd\|ct=3\|bd=1\|il=1\|w=40\|h=15\|tr=alphastd+invalpha+quantize\|mode=image\|x=trnsbkgd\|split=0\|n=2\|seed=16142` | fuzz read PALETTE/1-bit il=1 40x15 tr=[alphastd+invalpha+quantize] via image chunks=[trnsbkgd] idat_split=0 | exit 0 | [x] |
| 1392 | `rd\|ct=3\|bd=2\|il=1\|w=23\|h=4\|tr=gray2rgb+quantize+expand16\|mode=image\|x=chrm\|split=3\|n=2\|seed=16143` | fuzz read PALETTE/2-bit il=1 23x4 tr=[gray2rgb+quantize+expand16] via image chunks=[chrm] idat_split=3 | exit 0 | [x] |
| 1393 | `rd\|ct=3\|bd=4\|il=0\|w=18\|h=19\|tr=alphaopt\|mode=rows\|x=iccp\|split=1\|n=2\|seed=16144` | fuzz read PALETTE/4-bit il=0 18x19 tr=[alphaopt] via rows chunks=[iccp] idat_split=1 | exit 0 | [x] |
| 1394 | `rd\|ct=3\|bd=8\|il=0\|w=7\|h=16\|tr=strip16+alphaopt\|mode=image\|x=bkgd\|split=0\|n=2\|seed=16145` | fuzz read PALETTE/8-bit il=0 7x16 tr=[strip16+alphaopt] via image chunks=[bkgd] idat_split=0 | exit 0 | [x] |
| 1395 | `rd\|ct=4\|bd=8\|il=1\|w=31\|h=1\|tr=expand+gray2rgb+backgroundexp\|mode=startimage\|x=splt\|split=1\|n=2\|seed=16146` | fuzz read GRAY_ALPHA/8-bit il=1 31x1 tr=[expand+gray2rgb+backgroundexp] via startimage chunks=[splt] idat_split=1 | exit 0 | [x] |
| 1396 | `rd\|ct=4\|bd=16\|il=1\|w=27\|h=16\|tr=alphapng+alphastd+expand\|mode=rowonly\|x=gamachrm\|split=17\|n=2\|seed=16147` | fuzz read GRAY_ALPHA/16-bit il=1 27x16 tr=[alphapng+alphastd+expand] via rowonly chunks=[gamachrm] idat_split=17 | exit 0 | [x] |
| 1397 | `rd\|ct=6\|bd=8\|il=0\|w=17\|h=22\|tr=gammahigh\|mode=rows\|x=gamachrm\|split=0\|n=2\|seed=16148` | fuzz read RGBA/8-bit il=0 17x22 tr=[gammahigh] via rows chunks=[gamachrm] idat_split=0 | exit 0 | [x] |
| 1398 | `rd\|ct=6\|bd=16\|il=0\|w=14\|h=6\|tr=swap16\|mode=disponly\|x=gama\|split=1\|n=2\|seed=16149` | fuzz read RGBA/16-bit il=0 14x6 tr=[swap16] via disponly chunks=[gama] idat_split=1 | exit 0 | [x] |
| 1399 | `rd\|ct=0\|bd=1\|il=0\|w=10\|h=12\|tr=expand\|mode=rowonly\|x=sbit\|split=1\|n=2\|seed=16150` | fuzz read GRAY/1-bit il=0 10x12 tr=[expand] via rowonly chunks=[sbit] idat_split=1 | exit 0 | [x] |
| 1400 | `rd\|ct=0\|bd=2\|il=0\|w=32\|h=3\|tr=rgb2gray+alphabroken\|mode=disponly\|x=gamachrmsbittrnsbkgdtexttail\|split=0\|n=2\|seed=16151` | fuzz read GRAY/2-bit il=0 32x3 tr=[rgb2gray+alphabroken] via disponly chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=0 | exit 0; 2 warning(s): libpng does not support gamma+background+rgb_to_gray | [x] |
| 1401 | `rd\|ct=0\|bd=4\|il=1\|w=10\|h=24\|tr=filler_after+background\|mode=row\|x=physoffs\|split=0\|n=2\|seed=16152` | fuzz read GRAY/4-bit il=1 10x24 tr=[filler_after+background] via row chunks=[physoffs] idat_split=0 | exit 70; png_error: internal row size calculation error | [x] |
| 1402 | `rd\|ct=0\|bd=8\|il=0\|w=39\|h=8\|tr=bgr+stripalpha\|mode=image\|x=clli\|split=1\|n=2\|seed=16153` | fuzz read GRAY/8-bit il=0 39x8 tr=[bgr+stripalpha] via image chunks=[clli] idat_split=1 | exit 0 | [x] |
| 1403 | `rd\|ct=0\|bd=16\|il=0\|w=40\|h=20\|tr=none\|mode=image\|x=none\|split=0\|n=2\|seed=16154` | fuzz read GRAY/16-bit il=0 40x20 tr=[none] via image chunks=[none] idat_split=0 | exit 0 | [x] |
| 1404 | `rd\|ct=2\|bd=8\|il=0\|w=9\|h=6\|tr=quantize+alphaopt\|mode=rowonly\|x=clli\|split=0\|n=2\|seed=16155` | fuzz read RGB/8-bit il=0 9x6 tr=[quantize+alphaopt] via rowonly chunks=[clli] idat_split=0 | exit 0 | [x] |
| 1405 | `rd\|ct=2\|bd=16\|il=0\|w=3\|h=15\|tr=rgb2gray+shift\|mode=startimage\|x=text\|split=1\|n=2\|seed=16156` | fuzz read RGB/16-bit il=0 3x15 tr=[rgb2gray+shift] via startimage chunks=[text] idat_split=1 | exit 0 | [x] |
| 1406 | `rd\|ct=3\|bd=1\|il=1\|w=13\|h=5\|tr=pal2rgb+expandgray+addalpha_before\|mode=image\|x=physoffs\|split=1\|n=2\|seed=16157` | fuzz read PALETTE/1-bit il=1 13x5 tr=[pal2rgb+expandgray+addalpha_before] via image chunks=[physoffs] idat_split=1 | exit 0 | [x] |
| 1407 | `rd\|ct=3\|bd=2\|il=1\|w=35\|h=13\|tr=packswap+filler_before\|mode=image\|x=scal\|split=17\|n=2\|seed=16158` | fuzz read PALETTE/2-bit il=1 35x13 tr=[packswap+filler_before] via image chunks=[scal] idat_split=17 | exit 0 | [x] |
| 1408 | `rd\|ct=3\|bd=4\|il=0\|w=31\|h=16\|tr=none\|mode=rows\|x=cicp\|split=1\|n=2\|seed=16159` | fuzz read PALETTE/4-bit il=0 31x16 tr=[none] via rows chunks=[cicp] idat_split=1 | exit 0 | [x] |
| 1409 | `rd\|ct=3\|bd=8\|il=1\|w=16\|h=16\|tr=gammahigh+pal2rgb\|mode=rows\|x=pcal\|split=17\|n=2\|seed=16160` | fuzz read PALETTE/8-bit il=1 16x16 tr=[gammahigh+pal2rgb] via rows chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1410 | `rd\|ct=4\|bd=8\|il=1\|w=9\|h=2\|tr=invmono+addalpha_after\|mode=disponly\|x=scal\|split=0\|n=2\|seed=16161` | fuzz read GRAY_ALPHA/8-bit il=1 9x2 tr=[invmono+addalpha_after] via disponly chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1411 | `rd\|ct=4\|bd=16\|il=0\|w=38\|h=16\|tr=swap16+rgb2graywarn+quantize\|mode=image\|x=iccp\|split=1\|n=2\|seed=16162` | fuzz read GRAY_ALPHA/16-bit il=0 38x16 tr=[swap16+rgb2graywarn+quantize] via image chunks=[iccp] idat_split=1 | exit 0 | [x] |
| 1412 | `rd\|ct=6\|bd=8\|il=1\|w=7\|h=5\|tr=none\|mode=row\|x=sbit\|split=1\|n=2\|seed=16163` | fuzz read RGBA/8-bit il=1 7x5 tr=[none] via row chunks=[sbit] idat_split=1 | exit 0 | [x] |
| 1413 | `rd\|ct=6\|bd=16\|il=1\|w=5\|h=5\|tr=packswap\|mode=rows\|x=text\|split=0\|n=2\|seed=16164` | fuzz read RGBA/16-bit il=1 5x5 tr=[packswap] via rows chunks=[text] idat_split=0 | exit 0 | [x] |
| 1414 | `rd\|ct=0\|bd=1\|il=0\|w=35\|h=7\|tr=rgb2gray+addalpha_after+expand16\|mode=rowonly\|x=exif\|split=17\|n=2\|seed=16165` | fuzz read GRAY/1-bit il=0 35x7 tr=[rgb2gray+addalpha_after+expand16] via rowonly chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1415 | `rd\|ct=0\|bd=2\|il=0\|w=32\|h=24\|tr=swapalpha+quantize+rgb2gray\|mode=disponly\|x=none\|split=1\|n=2\|seed=16166` | fuzz read GRAY/2-bit il=0 32x24 tr=[swapalpha+quantize+rgb2gray] via disponly chunks=[none] idat_split=1 | exit 0 | [x] |
| 1416 | `rd\|ct=0\|bd=4\|il=1\|w=39\|h=21\|tr=none\|mode=startimage\|x=srgb\|split=17\|n=2\|seed=16167` | fuzz read GRAY/4-bit il=1 39x21 tr=[none] via startimage chunks=[srgb] idat_split=17 | exit 0 | [x] |
| 1417 | `rd\|ct=0\|bd=8\|il=1\|w=24\|h=9\|tr=interlace+expand\|mode=rowonly\|x=gama\|split=0\|n=2\|seed=16168` | fuzz read GRAY/8-bit il=1 24x9 tr=[interlace+expand] via rowonly chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1418 | `rd\|ct=0\|bd=16\|il=0\|w=37\|h=13\|tr=interlace\|mode=disponly\|x=gama\|split=3\|n=2\|seed=16169` | fuzz read GRAY/16-bit il=0 37x13 tr=[interlace] via disponly chunks=[gama] idat_split=3 | exit 0 | [x] |
| 1419 | `rd\|ct=2\|bd=8\|il=1\|w=4\|h=20\|tr=filler_after+packing+trns2alpha\|mode=rowonly\|x=mdcv\|split=1\|n=2\|seed=16170` | fuzz read RGB/8-bit il=1 4x20 tr=[filler_after+packing+trns2alpha] via rowonly chunks=[mdcv] idat_split=1 | exit 0 | [x] |
| 1420 | `rd\|ct=2\|bd=16\|il=1\|w=33\|h=19\|tr=packswap+backgroundexp\|mode=image\|x=text\|split=1\|n=2\|seed=16171` | fuzz read RGB/16-bit il=1 33x19 tr=[packswap+backgroundexp] via image chunks=[text] idat_split=1 | exit 0 | [x] |
| 1421 | `rd\|ct=3\|bd=1\|il=1\|w=19\|h=9\|tr=interlace\|mode=rows\|x=mdcv\|split=17\|n=2\|seed=16172` | fuzz read PALETTE/1-bit il=1 19x9 tr=[interlace] via rows chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1422 | `rd\|ct=3\|bd=2\|il=1\|w=8\|h=21\|tr=none\|mode=disponly\|x=trnsbkgd\|split=3\|n=2\|seed=16173` | fuzz read PALETTE/2-bit il=1 8x21 tr=[none] via disponly chunks=[trnsbkgd] idat_split=3 | exit 0 | [x] |
| 1423 | `rd\|ct=3\|bd=4\|il=0\|w=10\|h=22\|tr=swapalpha+addalpha_before+pal2rgb\|mode=rows\|x=time\|split=0\|n=2\|seed=16174` | fuzz read PALETTE/4-bit il=0 10x22 tr=[swapalpha+addalpha_before+pal2rgb] via rows chunks=[time] idat_split=0 | exit 0 | [x] |
| 1424 | `rd\|ct=3\|bd=8\|il=1\|w=6\|h=21\|tr=invalpha\|mode=disponly\|x=chrm\|split=3\|n=2\|seed=16175` | fuzz read PALETTE/8-bit il=1 6x21 tr=[invalpha] via disponly chunks=[chrm] idat_split=3 | exit 0 | [x] |
| 1425 | `rd\|ct=4\|bd=8\|il=0\|w=9\|h=5\|tr=gray2rgb\|mode=rows\|x=plte\|split=3\|n=2\|seed=16176` | fuzz read GRAY_ALPHA/8-bit il=0 9x5 tr=[gray2rgb] via rows chunks=[plte] idat_split=3 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1426 | `rd\|ct=4\|bd=16\|il=0\|w=11\|h=4\|tr=addalpha_before\|mode=image\|x=trnsbkgd\|split=1\|n=2\|seed=16177` | fuzz read GRAY_ALPHA/16-bit il=0 11x4 tr=[addalpha_before] via image chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1427 | `rd\|ct=6\|bd=8\|il=0\|w=28\|h=9\|tr=stripalpha\|mode=rows\|x=scal\|split=17\|n=2\|seed=16178` | fuzz read RGBA/8-bit il=0 28x9 tr=[stripalpha] via rows chunks=[scal] idat_split=17 | exit 0 | [x] |
| 1428 | `rd\|ct=6\|bd=16\|il=0\|w=32\|h=13\|tr=none\|mode=image\|x=scal\|split=1\|n=2\|seed=16179` | fuzz read RGBA/16-bit il=0 32x13 tr=[none] via image chunks=[scal] idat_split=1 | exit 0 | [x] |
| 1429 | `rd\|ct=0\|bd=1\|il=0\|w=6\|h=21\|tr=none\|mode=image\|x=tail\|split=17\|n=2\|seed=16180` | fuzz read GRAY/1-bit il=0 6x21 tr=[none] via image chunks=[tail] idat_split=17 | exit 0 | [x] |
| 1430 | `rd\|ct=0\|bd=2\|il=0\|w=1\|h=15\|tr=backgroundexp+quantize\|mode=rowonly\|x=trnsbkgd\|split=0\|n=2\|seed=16181` | fuzz read GRAY/2-bit il=0 1x15 tr=[backgroundexp+quantize] via rowonly chunks=[trnsbkgd] idat_split=0 | exit 0 | [x] |
| 1431 | `rd\|ct=0\|bd=4\|il=1\|w=38\|h=23\|tr=swap16+expandgray+stripalpha\|mode=image\|x=trns\|split=3\|n=2\|seed=16182` | fuzz read GRAY/4-bit il=1 38x23 tr=[swap16+expandgray+stripalpha] via image chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1432 | `rd\|ct=0\|bd=8\|il=0\|w=16\|h=2\|tr=alphapng+addalpha_before\|mode=rowonly\|x=gamachrmsbittrnsbkgdtexttail\|split=1\|n=2\|seed=16183` | fuzz read GRAY/8-bit il=0 16x2 tr=[alphapng+addalpha_before] via rowonly chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=1 | exit 0 | [x] |
| 1433 | `rd\|ct=0\|bd=16\|il=0\|w=18\|h=22\|tr=addalpha_after+filler_after\|mode=rowonly\|x=gamachrmsbittrnsbkgdtexttail\|split=0\|n=2\|seed=16184` | fuzz read GRAY/16-bit il=0 18x22 tr=[addalpha_after+filler_after] via rowonly chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=0 | exit 0 | [x] |
| 1434 | `rd\|ct=2\|bd=8\|il=0\|w=23\|h=7\|tr=rgb2graywarn+gammahigh+scale16\|mode=row\|x=scal\|split=17\|n=2\|seed=16185` | fuzz read RGB/8-bit il=0 23x7 tr=[rgb2graywarn+gammahigh+scale16] via row chunks=[scal] idat_split=17 | exit 0; 14 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 1435 | `rd\|ct=2\|bd=16\|il=1\|w=36\|h=18\|tr=alphastd+backgroundunique\|mode=rows\|x=gamachrm\|split=1\|n=2\|seed=16186` | fuzz read RGB/16-bit il=1 36x18 tr=[alphastd+backgroundunique] via rows chunks=[gamachrm] idat_split=1 | exit 0 | [x] |
| 1436 | `rd\|ct=3\|bd=1\|il=1\|w=37\|h=12\|tr=gamma+interlace\|mode=rowonly\|x=clli\|split=3\|n=2\|seed=16187` | fuzz read PALETTE/1-bit il=1 37x12 tr=[gamma+interlace] via rowonly chunks=[clli] idat_split=3 | exit 0 | [x] |
| 1437 | `rd\|ct=3\|bd=2\|il=1\|w=19\|h=11\|tr=none\|mode=rows\|x=iccp\|split=1\|n=2\|seed=16188` | fuzz read PALETTE/2-bit il=1 19x11 tr=[none] via rows chunks=[iccp] idat_split=1 | exit 0 | [x] |
| 1438 | `rd\|ct=3\|bd=4\|il=0\|w=29\|h=9\|tr=none\|mode=startimage\|x=pcal\|split=1\|n=2\|seed=16189` | fuzz read PALETTE/4-bit il=0 29x9 tr=[none] via startimage chunks=[pcal] idat_split=1 | exit 0 | [x] |
| 1439 | `rd\|ct=3\|bd=8\|il=0\|w=5\|h=12\|tr=addalpha_after+shift+alphapng\|mode=startimage\|x=unk\|split=17\|n=2\|seed=16190` | fuzz read PALETTE/8-bit il=0 5x12 tr=[addalpha_after+shift+alphapng] via startimage chunks=[unk] idat_split=17 | exit 0 | [x] |
| 1440 | `rd\|ct=4\|bd=8\|il=1\|w=8\|h=8\|tr=gray2rgb\|mode=rowonly\|x=gamachrmsbittrnsbkgdtexttail\|split=0\|n=2\|seed=16191` | fuzz read GRAY_ALPHA/8-bit il=1 8x8 tr=[gray2rgb] via rowonly chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=0 | exit 0 | [x] |
| 1441 | `rd\|ct=4\|bd=16\|il=1\|w=31\|h=4\|tr=none\|mode=image\|x=chrm\|split=0\|n=2\|seed=16192` | fuzz read GRAY_ALPHA/16-bit il=1 31x4 tr=[none] via image chunks=[chrm] idat_split=0 | exit 0 | [x] |
| 1442 | `rd\|ct=6\|bd=8\|il=0\|w=39\|h=17\|tr=expand16+expandgray\|mode=rows\|x=time\|split=1\|n=2\|seed=16193` | fuzz read RGBA/8-bit il=0 39x17 tr=[expand16+expandgray] via rows chunks=[time] idat_split=1 | exit 0 | [x] |
| 1443 | `rd\|ct=6\|bd=16\|il=0\|w=14\|h=20\|tr=none\|mode=disponly\|x=unk\|split=1\|n=2\|seed=16194` | fuzz read RGBA/16-bit il=0 14x20 tr=[none] via disponly chunks=[unk] idat_split=1 | exit 0 | [x] |
| 1444 | `rd\|ct=0\|bd=1\|il=0\|w=8\|h=5\|tr=stripalpha+packswap\|mode=row\|x=trns\|split=0\|n=2\|seed=16195` | fuzz read GRAY/1-bit il=0 8x5 tr=[stripalpha+packswap] via row chunks=[trns] idat_split=0 | exit 0 | [x] |
| 1445 | `rd\|ct=0\|bd=2\|il=1\|w=17\|h=11\|tr=none\|mode=rows\|x=trnsbkgd\|split=1\|n=2\|seed=16196` | fuzz read GRAY/2-bit il=1 17x11 tr=[none] via rows chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1446 | `rd\|ct=0\|bd=4\|il=1\|w=1\|h=4\|tr=none\|mode=rows\|x=text\|split=3\|n=2\|seed=16197` | fuzz read GRAY/4-bit il=1 1x4 tr=[none] via rows chunks=[text] idat_split=3 | exit 0 | [x] |
| 1447 | `rd\|ct=0\|bd=8\|il=0\|w=38\|h=1\|tr=none\|mode=rows\|x=bkgd\|split=1\|n=2\|seed=16198` | fuzz read GRAY/8-bit il=0 38x1 tr=[none] via rows chunks=[bkgd] idat_split=1 | exit 0 | [x] |
| 1448 | `rd\|ct=0\|bd=16\|il=1\|w=34\|h=10\|tr=trns2alpha\|mode=row\|x=gama\|split=1\|n=2\|seed=16199` | fuzz read GRAY/16-bit il=1 34x10 tr=[trns2alpha] via row chunks=[gama] idat_split=1 | exit 0 | [x] |
| 1449 | `rd\|ct=2\|bd=8\|il=0\|w=8\|h=23\|tr=background+alphastd\|mode=image\|x=scal\|split=0\|n=2\|seed=16200` | fuzz read RGB/8-bit il=0 8x23 tr=[background+alphastd] via image chunks=[scal] idat_split=0 | exit 70; png_error: conflicting calls to set alpha mode and background | [x] |
| 1450 | `rd\|ct=2\|bd=16\|il=1\|w=32\|h=20\|tr=packing\|mode=disponly\|x=text\|split=17\|n=2\|seed=16201` | fuzz read RGB/16-bit il=1 32x20 tr=[packing] via disponly chunks=[text] idat_split=17 | exit 0 | [x] |
| 1451 | `rd\|ct=3\|bd=1\|il=0\|w=35\|h=2\|tr=rgb2gray\|mode=rowonly\|x=tail\|split=17\|n=2\|seed=16202` | fuzz read PALETTE/1-bit il=0 35x2 tr=[rgb2gray] via rowonly chunks=[tail] idat_split=17 | exit 0 | [x] |
| 1452 | `rd\|ct=3\|bd=2\|il=0\|w=22\|h=20\|tr=strip16\|mode=disponly\|x=srgb\|split=0\|n=2\|seed=16203` | fuzz read PALETTE/2-bit il=0 22x20 tr=[strip16] via disponly chunks=[srgb] idat_split=0 | exit 0 | [x] |
| 1453 | `rd\|ct=3\|bd=4\|il=0\|w=23\|h=4\|tr=alphapng+addalpha_after\|mode=startimage\|x=splt\|split=0\|n=2\|seed=16204` | fuzz read PALETTE/4-bit il=0 23x4 tr=[alphapng+addalpha_after] via startimage chunks=[splt] idat_split=0 | exit 0 | [x] |
| 1454 | `rd\|ct=3\|bd=8\|il=1\|w=39\|h=17\|tr=none\|mode=rowonly\|x=unk\|split=0\|n=2\|seed=16205` | fuzz read PALETTE/8-bit il=1 39x17 tr=[none] via rowonly chunks=[unk] idat_split=0 | exit 0 | [x] |
| 1455 | `rd\|ct=4\|bd=8\|il=1\|w=33\|h=10\|tr=alphapng+background+addalpha_after\|mode=row\|x=unk\|split=0\|n=2\|seed=16206` | fuzz read GRAY_ALPHA/8-bit il=1 33x10 tr=[alphapng+background+addalpha_after] via row chunks=[unk] idat_split=0 | exit 0 | [x] |
| 1456 | `rd\|ct=4\|bd=16\|il=1\|w=18\|h=19\|tr=alphabroken+packing\|mode=rows\|x=iccp\|split=3\|n=2\|seed=16207` | fuzz read GRAY_ALPHA/16-bit il=1 18x19 tr=[alphabroken+packing] via rows chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1457 | `rd\|ct=6\|bd=8\|il=0\|w=30\|h=14\|tr=expand+rgb2graywarn\|mode=rowonly\|x=srgb\|split=0\|n=2\|seed=16208` | fuzz read RGBA/8-bit il=0 30x14 tr=[expand+rgb2graywarn] via rowonly chunks=[srgb] idat_split=0 | exit 0; 28 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 1458 | `rd\|ct=6\|bd=16\|il=1\|w=34\|h=20\|tr=strip16+alphapng+bgr\|mode=rowonly\|x=iccp\|split=0\|n=2\|seed=16209` | fuzz read RGBA/16-bit il=1 34x20 tr=[strip16+alphapng+bgr] via rowonly chunks=[iccp] idat_split=0 | exit 0 | [x] |
| 1459 | `rd\|ct=0\|bd=1\|il=1\|w=28\|h=15\|tr=packing+stripalpha+backgroundunique\|mode=image\|x=gamachrm\|split=0\|n=2\|seed=16210` | fuzz read GRAY/1-bit il=1 28x15 tr=[packing+stripalpha+backgroundunique] via image chunks=[gamachrm] idat_split=0 | exit 0 | [x] |
| 1460 | `rd\|ct=0\|bd=2\|il=0\|w=26\|h=15\|tr=none\|mode=rowonly\|x=exif\|split=3\|n=2\|seed=16211` | fuzz read GRAY/2-bit il=0 26x15 tr=[none] via rowonly chunks=[exif] idat_split=3 | exit 0 | [x] |
| 1461 | `rd\|ct=0\|bd=4\|il=1\|w=40\|h=10\|tr=gamma\|mode=rows\|x=splt\|split=17\|n=2\|seed=16212` | fuzz read GRAY/4-bit il=1 40x10 tr=[gamma] via rows chunks=[splt] idat_split=17 | exit 0 | [x] |
| 1462 | `rd\|ct=0\|bd=8\|il=0\|w=36\|h=2\|tr=none\|mode=image\|x=none\|split=3\|n=2\|seed=16213` | fuzz read GRAY/8-bit il=0 36x2 tr=[none] via image chunks=[none] idat_split=3 | exit 0 | [x] |
| 1463 | `rd\|ct=0\|bd=16\|il=1\|w=7\|h=24\|tr=none\|mode=image\|x=none\|split=1\|n=2\|seed=16214` | fuzz read GRAY/16-bit il=1 7x24 tr=[none] via image chunks=[none] idat_split=1 | exit 0 | [x] |
| 1464 | `rd\|ct=2\|bd=8\|il=0\|w=12\|h=2\|tr=alphabroken+addalpha_after\|mode=row\|x=iccp\|split=3\|n=2\|seed=16215` | fuzz read RGB/8-bit il=0 12x2 tr=[alphabroken+addalpha_after] via row chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1465 | `rd\|ct=2\|bd=16\|il=1\|w=17\|h=24\|tr=backgroundexp+backgroundunique+swapalpha\|mode=startimage\|x=hist\|split=17\|n=2\|seed=16216` | fuzz read RGB/16-bit il=1 17x24 tr=[backgroundexp+backgroundunique+swapalpha] via startimage chunks=[hist] idat_split=17 | exit 0 | [x] |
| 1466 | `rd\|ct=3\|bd=1\|il=1\|w=25\|h=2\|tr=scale16+gamma\|mode=startimage\|x=hist\|split=17\|n=2\|seed=16217` | fuzz read PALETTE/1-bit il=1 25x2 tr=[scale16+gamma] via startimage chunks=[hist] idat_split=17 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1467 | `rd\|ct=3\|bd=2\|il=0\|w=33\|h=2\|tr=expand+trns2alpha\|mode=disponly\|x=mdcv\|split=17\|n=2\|seed=16218` | fuzz read PALETTE/2-bit il=0 33x2 tr=[expand+trns2alpha] via disponly chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1468 | `rd\|ct=3\|bd=4\|il=1\|w=16\|h=2\|tr=addalpha_before+gray2rgb\|mode=startimage\|x=mdcv\|split=0\|n=2\|seed=16219` | fuzz read PALETTE/4-bit il=1 16x2 tr=[addalpha_before+gray2rgb] via startimage chunks=[mdcv] idat_split=0 | exit 0 | [x] |
| 1469 | `rd\|ct=3\|bd=8\|il=1\|w=37\|h=18\|tr=packing\|mode=rowonly\|x=pcal\|split=1\|n=2\|seed=16220` | fuzz read PALETTE/8-bit il=1 37x18 tr=[packing] via rowonly chunks=[pcal] idat_split=1 | exit 0 | [x] |
| 1470 | `rd\|ct=4\|bd=8\|il=0\|w=30\|h=10\|tr=shift+rgb2gray+trns2alpha\|mode=rows\|x=time\|split=0\|n=2\|seed=16221` | fuzz read GRAY_ALPHA/8-bit il=0 30x10 tr=[shift+rgb2gray+trns2alpha] via rows chunks=[time] idat_split=0 | exit 0 | [x] |
| 1471 | `rd\|ct=4\|bd=16\|il=1\|w=26\|h=5\|tr=expand16+expand\|mode=rows\|x=text\|split=1\|n=2\|seed=16222` | fuzz read GRAY_ALPHA/16-bit il=1 26x5 tr=[expand16+expand] via rows chunks=[text] idat_split=1 | exit 0 | [x] |
| 1472 | `rd\|ct=6\|bd=8\|il=0\|w=20\|h=14\|tr=none\|mode=row\|x=pcal\|split=0\|n=2\|seed=16223` | fuzz read RGBA/8-bit il=0 20x14 tr=[none] via row chunks=[pcal] idat_split=0 | exit 0 | [x] |
| 1473 | `rd\|ct=6\|bd=16\|il=1\|w=32\|h=18\|tr=expand+interlace+backgroundunique\|mode=image\|x=exif\|split=0\|n=2\|seed=16224` | fuzz read RGBA/16-bit il=1 32x18 tr=[expand+interlace+backgroundunique] via image chunks=[exif] idat_split=0 | exit 0 | [x] |
| 1474 | `rd\|ct=0\|bd=1\|il=0\|w=37\|h=15\|tr=quantize+stripalpha\|mode=image\|x=unk\|split=17\|n=2\|seed=16225` | fuzz read GRAY/1-bit il=0 37x15 tr=[quantize+stripalpha] via image chunks=[unk] idat_split=17 | exit 0 | [x] |
| 1475 | `rd\|ct=0\|bd=2\|il=1\|w=27\|h=9\|tr=interlace+alphaopt+filler_after\|mode=rows\|x=clli\|split=0\|n=2\|seed=16226` | fuzz read GRAY/2-bit il=1 27x9 tr=[interlace+alphaopt+filler_after] via rows chunks=[clli] idat_split=0 | exit 70; png_error: internal row size calculation error | [x] |
| 1476 | `rd\|ct=0\|bd=4\|il=1\|w=26\|h=9\|tr=backgroundexp+filler_after+rgb2gray\|mode=rowonly\|x=srgb\|split=17\|n=2\|seed=16227` | fuzz read GRAY/4-bit il=1 26x9 tr=[backgroundexp+filler_after+rgb2gray] via rowonly chunks=[srgb] idat_split=17 | exit 70; png_error: internal row size calculation error | [x] |
| 1477 | `rd\|ct=0\|bd=8\|il=0\|w=36\|h=23\|tr=trns2alpha+gamma\|mode=rows\|x=gamachrm\|split=0\|n=2\|seed=16228` | fuzz read GRAY/8-bit il=0 36x23 tr=[trns2alpha+gamma] via rows chunks=[gamachrm] idat_split=0 | exit 0 | [x] |
| 1478 | `rd\|ct=0\|bd=16\|il=0\|w=7\|h=20\|tr=trns2alpha\|mode=rowonly\|x=hist\|split=0\|n=2\|seed=16229` | fuzz read GRAY/16-bit il=0 7x20 tr=[trns2alpha] via rowonly chunks=[hist] idat_split=0 | exit 0 | [x] |
| 1479 | `rd\|ct=2\|bd=8\|il=1\|w=19\|h=2\|tr=gamma+alphabroken\|mode=rowonly\|x=plte\|split=0\|n=2\|seed=16230` | fuzz read RGB/8-bit il=1 19x2 tr=[gamma+alphabroken] via rowonly chunks=[plte] idat_split=0 | exit 0 | [x] |
| 1480 | `rd\|ct=2\|bd=16\|il=0\|w=2\|h=20\|tr=none\|mode=image\|x=splt\|split=1\|n=2\|seed=16231` | fuzz read RGB/16-bit il=0 2x20 tr=[none] via image chunks=[splt] idat_split=1 | exit 0 | [x] |
| 1481 | `rd\|ct=3\|bd=1\|il=0\|w=7\|h=8\|tr=invmono\|mode=row\|x=trns\|split=3\|n=2\|seed=16232` | fuzz read PALETTE/1-bit il=0 7x8 tr=[invmono] via row chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1482 | `rd\|ct=3\|bd=2\|il=0\|w=13\|h=19\|tr=background+invalpha\|mode=rows\|x=cicp\|split=1\|n=2\|seed=16233` | fuzz read PALETTE/2-bit il=0 13x19 tr=[background+invalpha] via rows chunks=[cicp] idat_split=1 | exit 0 | [x] |
| 1483 | `rd\|ct=3\|bd=4\|il=0\|w=14\|h=5\|tr=swap16+addalpha_after+bgr\|mode=rows\|x=clli\|split=3\|n=2\|seed=16234` | fuzz read PALETTE/4-bit il=0 14x5 tr=[swap16+addalpha_after+bgr] via rows chunks=[clli] idat_split=3 | exit 0 | [x] |
| 1484 | `rd\|ct=3\|bd=8\|il=1\|w=10\|h=14\|tr=filler_after+stripalpha\|mode=disponly\|x=plte\|split=3\|n=2\|seed=16235` | fuzz read PALETTE/8-bit il=1 10x14 tr=[filler_after+stripalpha] via disponly chunks=[plte] idat_split=3 | exit 0 | [x] |
| 1485 | `rd\|ct=4\|bd=8\|il=0\|w=27\|h=3\|tr=packing\|mode=startimage\|x=pcal\|split=17\|n=2\|seed=16236` | fuzz read GRAY_ALPHA/8-bit il=0 27x3 tr=[packing] via startimage chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1486 | `rd\|ct=4\|bd=16\|il=0\|w=27\|h=22\|tr=none\|mode=row\|x=scal\|split=3\|n=2\|seed=16237` | fuzz read GRAY_ALPHA/16-bit il=0 27x22 tr=[none] via row chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1487 | `rd\|ct=6\|bd=8\|il=0\|w=26\|h=11\|tr=rgb2gray\|mode=disponly\|x=srgb\|split=17\|n=2\|seed=16238` | fuzz read RGBA/8-bit il=0 26x11 tr=[rgb2gray] via disponly chunks=[srgb] idat_split=17 | exit 0 | [x] |
| 1488 | `rd\|ct=6\|bd=16\|il=0\|w=19\|h=19\|tr=none\|mode=rows\|x=text\|split=3\|n=2\|seed=16239` | fuzz read RGBA/16-bit il=0 19x19 tr=[none] via rows chunks=[text] idat_split=3 | exit 0 | [x] |
| 1489 | `rd\|ct=0\|bd=1\|il=0\|w=39\|h=3\|tr=invmono+swap16+swapalpha\|mode=disponly\|x=none\|split=3\|n=2\|seed=16240` | fuzz read GRAY/1-bit il=0 39x3 tr=[invmono+swap16+swapalpha] via disponly chunks=[none] idat_split=3 | exit 0 | [x] |
| 1490 | `rd\|ct=0\|bd=2\|il=1\|w=37\|h=4\|tr=expandgray\|mode=disponly\|x=none\|split=0\|n=2\|seed=16241` | fuzz read GRAY/2-bit il=1 37x4 tr=[expandgray] via disponly chunks=[none] idat_split=0 | exit 0 | [x] |
| 1491 | `rd\|ct=0\|bd=4\|il=0\|w=19\|h=10\|tr=stripalpha+alphastd+packing\|mode=row\|x=iccp\|split=3\|n=2\|seed=16242` | fuzz read GRAY/4-bit il=0 19x10 tr=[stripalpha+alphastd+packing] via row chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1492 | `rd\|ct=0\|bd=8\|il=0\|w=36\|h=4\|tr=scale16+gammahigh+packing\|mode=row\|x=iccp\|split=17\|n=2\|seed=16243` | fuzz read GRAY/8-bit il=0 36x4 tr=[scale16+gammahigh+packing] via row chunks=[iccp] idat_split=17 | exit 0 | [x] |
| 1493 | `rd\|ct=0\|bd=16\|il=1\|w=33\|h=4\|tr=invalpha+background+interlace\|mode=rowonly\|x=scal\|split=3\|n=2\|seed=16244` | fuzz read GRAY/16-bit il=1 33x4 tr=[invalpha+background+interlace] via rowonly chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1494 | `rd\|ct=2\|bd=8\|il=0\|w=34\|h=14\|tr=filler_before+shift\|mode=rowonly\|x=plte\|split=3\|n=2\|seed=16245` | fuzz read RGB/8-bit il=0 34x14 tr=[filler_before+shift] via rowonly chunks=[plte] idat_split=3 | exit 0 | [x] |
| 1495 | `rd\|ct=2\|bd=16\|il=0\|w=23\|h=24\|tr=strip16+expand16\|mode=startimage\|x=mdcv\|split=17\|n=2\|seed=16246` | fuzz read RGB/16-bit il=0 23x24 tr=[strip16+expand16] via startimage chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1496 | `rd\|ct=3\|bd=1\|il=1\|w=22\|h=7\|tr=invalpha\|mode=rowonly\|x=scal\|split=3\|n=2\|seed=16247` | fuzz read PALETTE/1-bit il=1 22x7 tr=[invalpha] via rowonly chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1497 | `rd\|ct=3\|bd=2\|il=0\|w=29\|h=11\|tr=none\|mode=rows\|x=iccp\|split=3\|n=2\|seed=16248` | fuzz read PALETTE/2-bit il=0 29x11 tr=[none] via rows chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1498 | `rd\|ct=3\|bd=4\|il=1\|w=26\|h=10\|tr=background\|mode=image\|x=pcal\|split=0\|n=2\|seed=16249` | fuzz read PALETTE/4-bit il=1 26x10 tr=[background] via image chunks=[pcal] idat_split=0 | exit 0 | [x] |
| 1499 | `rd\|ct=3\|bd=8\|il=1\|w=14\|h=14\|tr=alphastd+invalpha+filler_before\|mode=rowonly\|x=exif\|split=0\|n=2\|seed=16250` | fuzz read PALETTE/8-bit il=1 14x14 tr=[alphastd+invalpha+filler_before] via rowonly chunks=[exif] idat_split=0 | exit 0 | [x] |
| 1500 | `rd\|ct=4\|bd=8\|il=0\|w=5\|h=1\|tr=alphabroken+filler_after\|mode=row\|x=trnsbkgd\|split=17\|n=2\|seed=16251` | fuzz read GRAY_ALPHA/8-bit il=0 5x1 tr=[alphabroken+filler_after] via row chunks=[trnsbkgd] idat_split=17 | exit 0 | [x] |
| 1501 | `rd\|ct=4\|bd=16\|il=0\|w=15\|h=21\|tr=quantize\|mode=startimage\|x=bkgd\|split=3\|n=2\|seed=16252` | fuzz read GRAY_ALPHA/16-bit il=0 15x21 tr=[quantize] via startimage chunks=[bkgd] idat_split=3 | exit 0 | [x] |
| 1502 | `rd\|ct=6\|bd=8\|il=0\|w=27\|h=23\|tr=alphapng+packswap+interlace\|mode=disponly\|x=plte\|split=3\|n=2\|seed=16253` | fuzz read RGBA/8-bit il=0 27x23 tr=[alphapng+packswap+interlace] via disponly chunks=[plte] idat_split=3 | exit 0 | [x] |
| 1503 | `rd\|ct=6\|bd=16\|il=1\|w=6\|h=11\|tr=interlace+invalpha+quantize\|mode=image\|x=trns\|split=0\|n=2\|seed=16254` | fuzz read RGBA/16-bit il=1 6x11 tr=[interlace+invalpha+quantize] via image chunks=[trns] idat_split=0 | exit 0 | [x] |
| 1504 | `rd\|ct=0\|bd=1\|il=1\|w=35\|h=2\|tr=expand16\|mode=rowonly\|x=scal\|split=0\|n=2\|seed=16255` | fuzz read GRAY/1-bit il=1 35x2 tr=[expand16] via rowonly chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1505 | `rd\|ct=0\|bd=2\|il=0\|w=5\|h=16\|tr=interlace+expand16\|mode=image\|x=mdcv\|split=17\|n=2\|seed=16256` | fuzz read GRAY/2-bit il=0 5x16 tr=[interlace+expand16] via image chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1506 | `rd\|ct=0\|bd=4\|il=0\|w=20\|h=12\|tr=none\|mode=row\|x=trnsbkgd\|split=0\|n=2\|seed=16257` | fuzz read GRAY/4-bit il=0 20x12 tr=[none] via row chunks=[trnsbkgd] idat_split=0 | exit 0 | [x] |
| 1507 | `rd\|ct=0\|bd=8\|il=0\|w=10\|h=19\|tr=addalpha_before+interlace\|mode=rowonly\|x=gama\|split=0\|n=2\|seed=16258` | fuzz read GRAY/8-bit il=0 10x19 tr=[addalpha_before+interlace] via rowonly chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1508 | `rd\|ct=0\|bd=16\|il=1\|w=15\|h=7\|tr=alphapng+trns2alpha+interlace\|mode=startimage\|x=exif\|split=17\|n=2\|seed=16259` | fuzz read GRAY/16-bit il=1 15x7 tr=[alphapng+trns2alpha+interlace] via startimage chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1509 | `rd\|ct=2\|bd=8\|il=1\|w=20\|h=6\|tr=alphapng+filler_before\|mode=row\|x=tail\|split=1\|n=2\|seed=16260` | fuzz read RGB/8-bit il=1 20x6 tr=[alphapng+filler_before] via row chunks=[tail] idat_split=1 | exit 0 | [x] |
| 1510 | `rd\|ct=2\|bd=16\|il=0\|w=17\|h=19\|tr=none\|mode=image\|x=srgb\|split=3\|n=2\|seed=16261` | fuzz read RGB/16-bit il=0 17x19 tr=[none] via image chunks=[srgb] idat_split=3 | exit 0 | [x] |
| 1511 | `rd\|ct=3\|bd=1\|il=0\|w=14\|h=22\|tr=invmono+addalpha_before\|mode=image\|x=gamachrm\|split=17\|n=2\|seed=16262` | fuzz read PALETTE/1-bit il=0 14x22 tr=[invmono+addalpha_before] via image chunks=[gamachrm] idat_split=17 | exit 0 | [x] |
| 1512 | `rd\|ct=3\|bd=2\|il=0\|w=27\|h=17\|tr=packing\|mode=image\|x=trnsbkgd\|split=0\|n=2\|seed=16263` | fuzz read PALETTE/2-bit il=0 27x17 tr=[packing] via image chunks=[trnsbkgd] idat_split=0 | exit 0 | [x] |
| 1513 | `rd\|ct=3\|bd=4\|il=0\|w=12\|h=11\|tr=packing+packswap+addalpha_before\|mode=disponly\|x=trnsbkgd\|split=1\|n=2\|seed=16264` | fuzz read PALETTE/4-bit il=0 12x11 tr=[packing+packswap+addalpha_before] via disponly chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1514 | `rd\|ct=3\|bd=8\|il=1\|w=2\|h=5\|tr=alphabroken+packing\|mode=image\|x=physoffs\|split=1\|n=2\|seed=16265` | fuzz read PALETTE/8-bit il=1 2x5 tr=[alphabroken+packing] via image chunks=[physoffs] idat_split=1 | exit 0 | [x] |
| 1515 | `rd\|ct=4\|bd=8\|il=0\|w=20\|h=16\|tr=none\|mode=image\|x=chrm\|split=3\|n=2\|seed=16266` | fuzz read GRAY_ALPHA/8-bit il=0 20x16 tr=[none] via image chunks=[chrm] idat_split=3 | exit 0 | [x] |
| 1516 | `rd\|ct=4\|bd=16\|il=0\|w=29\|h=13\|tr=none\|mode=disponly\|x=trns\|split=0\|n=2\|seed=16267` | fuzz read GRAY_ALPHA/16-bit il=0 29x13 tr=[none] via disponly chunks=[trns] idat_split=0 | exit 0 | [x] |
| 1517 | `rd\|ct=6\|bd=8\|il=1\|w=31\|h=11\|tr=packing\|mode=image\|x=plte\|split=1\|n=2\|seed=16268` | fuzz read RGBA/8-bit il=1 31x11 tr=[packing] via image chunks=[plte] idat_split=1 | exit 0 | [x] |
| 1518 | `rd\|ct=6\|bd=16\|il=1\|w=16\|h=17\|tr=filler_before+trns2alpha\|mode=startimage\|x=pcal\|split=17\|n=2\|seed=16269` | fuzz read RGBA/16-bit il=1 16x17 tr=[filler_before+trns2alpha] via startimage chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1519 | `rd\|ct=0\|bd=1\|il=0\|w=28\|h=20\|tr=alphastd+trns2alpha\|mode=rowonly\|x=scal\|split=3\|n=2\|seed=16270` | fuzz read GRAY/1-bit il=0 28x20 tr=[alphastd+trns2alpha] via rowonly chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1520 | `rd\|ct=0\|bd=2\|il=0\|w=2\|h=6\|tr=invalpha+filler_before\|mode=image\|x=splt\|split=0\|n=2\|seed=16271` | fuzz read GRAY/2-bit il=0 2x6 tr=[invalpha+filler_before] via image chunks=[splt] idat_split=0 | exit 0 | [x] |
| 1521 | `rd\|ct=0\|bd=4\|il=1\|w=2\|h=10\|tr=backgroundexp+alphabroken\|mode=startimage\|x=iccp\|split=0\|n=2\|seed=16272` | fuzz read GRAY/4-bit il=1 2x10 tr=[backgroundexp+alphabroken] via startimage chunks=[iccp] idat_split=0 | exit 70; png_error: conflicting calls to set alpha mode and background | [x] |
| 1522 | `rd\|ct=0\|bd=8\|il=1\|w=9\|h=3\|tr=swapalpha+backgroundunique+swap16\|mode=disponly\|x=cicp\|split=0\|n=2\|seed=16273` | fuzz read GRAY/8-bit il=1 9x3 tr=[swapalpha+backgroundunique+swap16] via disponly chunks=[cicp] idat_split=0 | exit 0 | [x] |
| 1523 | `rd\|ct=0\|bd=16\|il=0\|w=39\|h=1\|tr=background\|mode=rowonly\|x=splt\|split=0\|n=2\|seed=16274` | fuzz read GRAY/16-bit il=0 39x1 tr=[background] via rowonly chunks=[splt] idat_split=0 | exit 0 | [x] |
| 1524 | `rd\|ct=2\|bd=8\|il=1\|w=39\|h=20\|tr=scale16+alphapng+shift\|mode=rowonly\|x=hist\|split=0\|n=2\|seed=16275` | fuzz read RGB/8-bit il=1 39x20 tr=[scale16+alphapng+shift] via rowonly chunks=[hist] idat_split=0 | exit 0 | [x] |
| 1525 | `rd\|ct=2\|bd=16\|il=1\|w=16\|h=12\|tr=filler_after+swap16+packing\|mode=rows\|x=text\|split=1\|n=2\|seed=16276` | fuzz read RGB/16-bit il=1 16x12 tr=[filler_after+swap16+packing] via rows chunks=[text] idat_split=1 | exit 0 | [x] |
| 1526 | `rd\|ct=3\|bd=1\|il=1\|w=7\|h=9\|tr=none\|mode=image\|x=chrm\|split=0\|n=2\|seed=16277` | fuzz read PALETTE/1-bit il=1 7x9 tr=[none] via image chunks=[chrm] idat_split=0 | exit 0 | [x] |
| 1527 | `rd\|ct=3\|bd=2\|il=1\|w=20\|h=20\|tr=none\|mode=disponly\|x=cicp\|split=17\|n=2\|seed=16278` | fuzz read PALETTE/2-bit il=1 20x20 tr=[none] via disponly chunks=[cicp] idat_split=17 | exit 0 | [x] |
| 1528 | `rd\|ct=3\|bd=4\|il=0\|w=11\|h=10\|tr=none\|mode=disponly\|x=cicp\|split=17\|n=2\|seed=16279` | fuzz read PALETTE/4-bit il=0 11x10 tr=[none] via disponly chunks=[cicp] idat_split=17 | exit 0 | [x] |
| 1529 | `rd\|ct=3\|bd=8\|il=0\|w=3\|h=24\|tr=none\|mode=row\|x=plte\|split=17\|n=2\|seed=16280` | fuzz read PALETTE/8-bit il=0 3x24 tr=[none] via row chunks=[plte] idat_split=17 | exit 0 | [x] |
| 1530 | `rd\|ct=4\|bd=8\|il=0\|w=4\|h=20\|tr=rgb2graywarn+invalpha+gamma\|mode=rowonly\|x=sbit\|split=17\|n=2\|seed=16281` | fuzz read GRAY_ALPHA/8-bit il=0 4x20 tr=[rgb2graywarn+invalpha+gamma] via rowonly chunks=[sbit] idat_split=17 | exit 0 | [x] |
| 1531 | `rd\|ct=4\|bd=16\|il=0\|w=15\|h=12\|tr=stripalpha+alphapng+interlace\|mode=row\|x=clli\|split=1\|n=2\|seed=16282` | fuzz read GRAY_ALPHA/16-bit il=0 15x12 tr=[stripalpha+alphapng+interlace] via row chunks=[clli] idat_split=1 | exit 0 | [x] |
| 1532 | `rd\|ct=6\|bd=8\|il=1\|w=26\|h=21\|tr=addalpha_after+gray2rgb+expandgray\|mode=rows\|x=gamachrm\|split=1\|n=2\|seed=16283` | fuzz read RGBA/8-bit il=1 26x21 tr=[addalpha_after+gray2rgb+expandgray] via rows chunks=[gamachrm] idat_split=1 | exit 0 | [x] |
| 1533 | `rd\|ct=6\|bd=16\|il=0\|w=31\|h=16\|tr=strip16+alphastd\|mode=image\|x=gama\|split=0\|n=2\|seed=16284` | fuzz read RGBA/16-bit il=0 31x16 tr=[strip16+alphastd] via image chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1534 | `rd\|ct=0\|bd=1\|il=0\|w=1\|h=4\|tr=trns2alpha\|mode=image\|x=gamachrm\|split=1\|n=2\|seed=16285` | fuzz read GRAY/1-bit il=0 1x4 tr=[trns2alpha] via image chunks=[gamachrm] idat_split=1 | exit 0 | [x] |
| 1535 | `rd\|ct=0\|bd=2\|il=0\|w=23\|h=22\|tr=alphapng+gammahigh+quantize\|mode=rowonly\|x=text\|split=3\|n=2\|seed=16286` | fuzz read GRAY/2-bit il=0 23x22 tr=[alphapng+gammahigh+quantize] via rowonly chunks=[text] idat_split=3 | exit 0 | [x] |
| 1536 | `rd\|ct=0\|bd=4\|il=0\|w=25\|h=17\|tr=none\|mode=row\|x=splt\|split=0\|n=2\|seed=16287` | fuzz read GRAY/4-bit il=0 25x17 tr=[none] via row chunks=[splt] idat_split=0 | exit 0 | [x] |
| 1537 | `rd\|ct=0\|bd=8\|il=0\|w=1\|h=10\|tr=gray2rgb+addalpha_before\|mode=rowonly\|x=text\|split=0\|n=2\|seed=16288` | fuzz read GRAY/8-bit il=0 1x10 tr=[gray2rgb+addalpha_before] via rowonly chunks=[text] idat_split=0 | exit 0 | [x] |
| 1538 | `rd\|ct=0\|bd=16\|il=0\|w=39\|h=13\|tr=alphaopt+expand+trns2alpha\|mode=rows\|x=splt\|split=3\|n=2\|seed=16289` | fuzz read GRAY/16-bit il=0 39x13 tr=[alphaopt+expand+trns2alpha] via rows chunks=[splt] idat_split=3 | exit 0 | [x] |
| 1539 | `rd\|ct=2\|bd=8\|il=1\|w=5\|h=19\|tr=invalpha\|mode=row\|x=cicp\|split=1\|n=2\|seed=16290` | fuzz read RGB/8-bit il=1 5x19 tr=[invalpha] via row chunks=[cicp] idat_split=1 | exit 0 | [x] |
| 1540 | `rd\|ct=2\|bd=16\|il=0\|w=10\|h=14\|tr=packing+alphastd+backgroundexp\|mode=startimage\|x=iccp\|split=0\|n=2\|seed=16291` | fuzz read RGB/16-bit il=0 10x14 tr=[packing+alphastd+backgroundexp] via startimage chunks=[iccp] idat_split=0 | exit 0 | [x] |
| 1541 | `rd\|ct=3\|bd=1\|il=1\|w=39\|h=6\|tr=alphabroken\|mode=disponly\|x=physoffs\|split=3\|n=2\|seed=16292` | fuzz read PALETTE/1-bit il=1 39x6 tr=[alphabroken] via disponly chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1542 | `rd\|ct=3\|bd=2\|il=0\|w=1\|h=19\|tr=expandgray+filler_before+packing\|mode=image\|x=trns\|split=3\|n=2\|seed=16293` | fuzz read PALETTE/2-bit il=0 1x19 tr=[expandgray+filler_before+packing] via image chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1543 | `rd\|ct=3\|bd=4\|il=0\|w=27\|h=8\|tr=filler_before\|mode=image\|x=trns\|split=1\|n=2\|seed=16294` | fuzz read PALETTE/4-bit il=0 27x8 tr=[filler_before] via image chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1544 | `rd\|ct=3\|bd=8\|il=1\|w=37\|h=6\|tr=expand+swap16\|mode=image\|x=trns\|split=1\|n=2\|seed=16295` | fuzz read PALETTE/8-bit il=1 37x6 tr=[expand+swap16] via image chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1545 | `rd\|ct=4\|bd=8\|il=1\|w=35\|h=4\|tr=none\|mode=startimage\|x=unk\|split=3\|n=2\|seed=16296` | fuzz read GRAY_ALPHA/8-bit il=1 35x4 tr=[none] via startimage chunks=[unk] idat_split=3 | exit 0 | [x] |
| 1546 | `rd\|ct=4\|bd=16\|il=1\|w=1\|h=22\|tr=none\|mode=rowonly\|x=time\|split=0\|n=2\|seed=16297` | fuzz read GRAY_ALPHA/16-bit il=1 1x22 tr=[none] via rowonly chunks=[time] idat_split=0 | exit 0 | [x] |
| 1547 | `rd\|ct=6\|bd=8\|il=0\|w=27\|h=19\|tr=none\|mode=image\|x=tail\|split=3\|n=2\|seed=16298` | fuzz read RGBA/8-bit il=0 27x19 tr=[none] via image chunks=[tail] idat_split=3 | exit 0 | [x] |
| 1548 | `rd\|ct=6\|bd=16\|il=0\|w=28\|h=7\|tr=swap16+background\|mode=rows\|x=sbit\|split=3\|n=2\|seed=16299` | fuzz read RGBA/16-bit il=0 28x7 tr=[swap16+background] via rows chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1549 | `rd\|ct=0\|bd=1\|il=1\|w=6\|h=22\|tr=gamma+addalpha_before\|mode=rows\|x=exif\|split=17\|n=2\|seed=16300` | fuzz read GRAY/1-bit il=1 6x22 tr=[gamma+addalpha_before] via rows chunks=[exif] idat_split=17 | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 1550 | `rd\|ct=0\|bd=2\|il=0\|w=18\|h=19\|tr=filler_after\|mode=row\|x=unk\|split=17\|n=2\|seed=16301` | fuzz read GRAY/2-bit il=0 18x19 tr=[filler_after] via row chunks=[unk] idat_split=17 | exit 70; png_error: internal row size calculation error | [x] |
| 1551 | `rd\|ct=0\|bd=4\|il=1\|w=21\|h=10\|tr=invmono+background+strip16\|mode=image\|x=plte\|split=1\|n=2\|seed=16302` | fuzz read GRAY/4-bit il=1 21x10 tr=[invmono+background+strip16] via image chunks=[plte] idat_split=1 | exit 0; 2 warning(s): PLTE: ignored in grayscale PNG | [x] |
| 1552 | `rd\|ct=0\|bd=8\|il=1\|w=36\|h=1\|tr=none\|mode=image\|x=mdcv\|split=17\|n=2\|seed=16303` | fuzz read GRAY/8-bit il=1 36x1 tr=[none] via image chunks=[mdcv] idat_split=17 | exit 0 | [x] |
| 1553 | `rd\|ct=0\|bd=16\|il=1\|w=34\|h=5\|tr=filler_before\|mode=image\|x=gama\|split=0\|n=2\|seed=16304` | fuzz read GRAY/16-bit il=1 34x5 tr=[filler_before] via image chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1554 | `rd\|ct=2\|bd=8\|il=0\|w=5\|h=7\|tr=none\|mode=disponly\|x=gama\|split=17\|n=2\|seed=16305` | fuzz read RGB/8-bit il=0 5x7 tr=[none] via disponly chunks=[gama] idat_split=17 | exit 0 | [x] |
| 1555 | `rd\|ct=2\|bd=16\|il=1\|w=15\|h=19\|tr=backgroundunique+filler_before\|mode=startimage\|x=gama\|split=3\|n=2\|seed=16306` | fuzz read RGB/16-bit il=1 15x19 tr=[backgroundunique+filler_before] via startimage chunks=[gama] idat_split=3 | exit 0 | [x] |
| 1556 | `rd\|ct=3\|bd=1\|il=0\|w=40\|h=20\|tr=shift\|mode=rows\|x=chrm\|split=1\|n=2\|seed=16307` | fuzz read PALETTE/1-bit il=0 40x20 tr=[shift] via rows chunks=[chrm] idat_split=1 | exit 0 | [x] |
| 1557 | `rd\|ct=3\|bd=2\|il=1\|w=37\|h=10\|tr=filler_after+strip16+gamma\|mode=disponly\|x=cicp\|split=0\|n=2\|seed=16308` | fuzz read PALETTE/2-bit il=1 37x10 tr=[filler_after+strip16+gamma] via disponly chunks=[cicp] idat_split=0 | exit 0 | [x] |
| 1558 | `rd\|ct=3\|bd=4\|il=0\|w=23\|h=16\|tr=none\|mode=row\|x=unk\|split=0\|n=2\|seed=16309` | fuzz read PALETTE/4-bit il=0 23x16 tr=[none] via row chunks=[unk] idat_split=0 | exit 0 | [x] |
| 1559 | `rd\|ct=3\|bd=8\|il=0\|w=15\|h=13\|tr=backgroundexp+packing\|mode=image\|x=trns\|split=1\|n=2\|seed=16310` | fuzz read PALETTE/8-bit il=0 15x13 tr=[backgroundexp+packing] via image chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1560 | `rd\|ct=4\|bd=8\|il=1\|w=31\|h=4\|tr=expand16\|mode=image\|x=physoffs\|split=0\|n=2\|seed=16311` | fuzz read GRAY_ALPHA/8-bit il=1 31x4 tr=[expand16] via image chunks=[physoffs] idat_split=0 | exit 0 | [x] |
| 1561 | `rd\|ct=4\|bd=16\|il=1\|w=17\|h=23\|tr=expand+alphaopt+swap16\|mode=rowonly\|x=hist\|split=3\|n=2\|seed=16312` | fuzz read GRAY_ALPHA/16-bit il=1 17x23 tr=[expand+alphaopt+swap16] via rowonly chunks=[hist] idat_split=3 | exit 0 | [x] |
| 1562 | `rd\|ct=6\|bd=8\|il=0\|w=22\|h=1\|tr=alphaopt\|mode=disponly\|x=exif\|split=17\|n=2\|seed=16313` | fuzz read RGBA/8-bit il=0 22x1 tr=[alphaopt] via disponly chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1563 | `rd\|ct=6\|bd=16\|il=0\|w=7\|h=18\|tr=backgroundunique+swapalpha+scale16\|mode=row\|x=bkgd\|split=17\|n=2\|seed=16314` | fuzz read RGBA/16-bit il=0 7x18 tr=[backgroundunique+swapalpha+scale16] via row chunks=[bkgd] idat_split=17 | exit 0 | [x] |
| 1564 | `rd\|ct=0\|bd=1\|il=0\|w=18\|h=18\|tr=expand\|mode=rows\|x=iccp\|split=3\|n=2\|seed=16315` | fuzz read GRAY/1-bit il=0 18x18 tr=[expand] via rows chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1565 | `rd\|ct=0\|bd=2\|il=0\|w=14\|h=16\|tr=rgb2gray+filler_after+pal2rgb\|mode=row\|x=splt\|split=1\|n=2\|seed=16316` | fuzz read GRAY/2-bit il=0 14x16 tr=[rgb2gray+filler_after+pal2rgb] via row chunks=[splt] idat_split=1 | exit 0 | [x] |
| 1566 | `rd\|ct=0\|bd=4\|il=0\|w=20\|h=9\|tr=packing+stripalpha\|mode=disponly\|x=text\|split=0\|n=2\|seed=16317` | fuzz read GRAY/4-bit il=0 20x9 tr=[packing+stripalpha] via disponly chunks=[text] idat_split=0 | exit 0 | [x] |
| 1567 | `rd\|ct=0\|bd=8\|il=1\|w=12\|h=3\|tr=shift+background\|mode=disponly\|x=hist\|split=17\|n=2\|seed=16318` | fuzz read GRAY/8-bit il=1 12x3 tr=[shift+background] via disponly chunks=[hist] idat_split=17 | exit 0 | [x] |
| 1568 | `rd\|ct=0\|bd=16\|il=1\|w=35\|h=17\|tr=background\|mode=row\|x=text\|split=17\|n=2\|seed=16319` | fuzz read GRAY/16-bit il=1 35x17 tr=[background] via row chunks=[text] idat_split=17 | exit 0 | [x] |
| 1569 | `rd\|ct=2\|bd=8\|il=1\|w=34\|h=2\|tr=alphaopt+alphabroken\|mode=rows\|x=plte\|split=3\|n=2\|seed=16320` | fuzz read RGB/8-bit il=1 34x2 tr=[alphaopt+alphabroken] via rows chunks=[plte] idat_split=3 | exit 70; png_error: conflicting calls to set alpha mode and background | [x] |
| 1570 | `rd\|ct=2\|bd=16\|il=1\|w=3\|h=5\|tr=packing\|mode=rows\|x=scal\|split=3\|n=2\|seed=16321` | fuzz read RGB/16-bit il=1 3x5 tr=[packing] via rows chunks=[scal] idat_split=3 | exit 0 | [x] |
| 1571 | `rd\|ct=3\|bd=1\|il=0\|w=11\|h=14\|tr=backgroundexp+background+gamma\|mode=row\|x=bkgd\|split=3\|n=2\|seed=16322` | fuzz read PALETTE/1-bit il=0 11x14 tr=[backgroundexp+background+gamma] via row chunks=[bkgd] idat_split=3 | exit 0 | [x] |
| 1572 | `rd\|ct=3\|bd=2\|il=1\|w=17\|h=2\|tr=gammahigh\|mode=row\|x=scal\|split=0\|n=2\|seed=16323` | fuzz read PALETTE/2-bit il=1 17x2 tr=[gammahigh] via row chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1573 | `rd\|ct=3\|bd=4\|il=1\|w=30\|h=23\|tr=invmono+alphapng+backgroundunique\|mode=row\|x=bkgd\|split=3\|n=2\|seed=16324` | fuzz read PALETTE/4-bit il=1 30x23 tr=[invmono+alphapng+backgroundunique] via row chunks=[bkgd] idat_split=3 | exit 0 | [x] |
| 1574 | `rd\|ct=3\|bd=8\|il=0\|w=32\|h=13\|tr=backgroundunique+gray2rgb+background\|mode=row\|x=time\|split=1\|n=2\|seed=16325` | fuzz read PALETTE/8-bit il=0 32x13 tr=[backgroundunique+gray2rgb+background] via row chunks=[time] idat_split=1 | exit 0 | [x] |
| 1575 | `rd\|ct=4\|bd=8\|il=1\|w=28\|h=12\|tr=none\|mode=rowonly\|x=splt\|split=0\|n=2\|seed=16326` | fuzz read GRAY_ALPHA/8-bit il=1 28x12 tr=[none] via rowonly chunks=[splt] idat_split=0 | exit 0 | [x] |
| 1576 | `rd\|ct=4\|bd=16\|il=0\|w=23\|h=22\|tr=gammahigh\|mode=image\|x=hist\|split=0\|n=2\|seed=16327` | fuzz read GRAY_ALPHA/16-bit il=0 23x22 tr=[gammahigh] via image chunks=[hist] idat_split=0 | exit 0 | [x] |
| 1577 | `rd\|ct=6\|bd=8\|il=1\|w=23\|h=5\|tr=gray2rgb\|mode=row\|x=trnsbkgd\|split=3\|n=2\|seed=16328` | fuzz read RGBA/8-bit il=1 23x5 tr=[gray2rgb] via row chunks=[trnsbkgd] idat_split=3 | exit 0 | [x] |
| 1578 | `rd\|ct=6\|bd=16\|il=0\|w=28\|h=19\|tr=strip16\|mode=startimage\|x=scal\|split=17\|n=2\|seed=16329` | fuzz read RGBA/16-bit il=0 28x19 tr=[strip16] via startimage chunks=[scal] idat_split=17 | exit 0 | [x] |
| 1579 | `rd\|ct=0\|bd=1\|il=1\|w=25\|h=20\|tr=filler_before+rgb2gray+invmono\|mode=image\|x=pcal\|split=3\|n=2\|seed=16330` | fuzz read GRAY/1-bit il=1 25x20 tr=[filler_before+rgb2gray+invmono] via image chunks=[pcal] idat_split=3 | exit 70; png_error: internal row size calculation error | [x] |
| 1580 | `rd\|ct=0\|bd=2\|il=1\|w=14\|h=4\|tr=pal2rgb+backgroundexp+shift\|mode=startimage\|x=pcal\|split=17\|n=2\|seed=16331` | fuzz read GRAY/2-bit il=1 14x4 tr=[pal2rgb+backgroundexp+shift] via startimage chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1581 | `rd\|ct=0\|bd=4\|il=1\|w=14\|h=23\|tr=gamma+packing+filler_before\|mode=rowonly\|x=chrm\|split=17\|n=2\|seed=16332` | fuzz read GRAY/4-bit il=1 14x23 tr=[gamma+packing+filler_before] via rowonly chunks=[chrm] idat_split=17 | exit 0 | [x] |
| 1582 | `rd\|ct=0\|bd=8\|il=0\|w=32\|h=8\|tr=backgroundexp\|mode=row\|x=exif\|split=3\|n=2\|seed=16333` | fuzz read GRAY/8-bit il=0 32x8 tr=[backgroundexp] via row chunks=[exif] idat_split=3 | exit 0 | [x] |
| 1583 | `rd\|ct=0\|bd=16\|il=0\|w=22\|h=10\|tr=background+addalpha_after+quantize\|mode=disponly\|x=iccp\|split=0\|n=2\|seed=16334` | fuzz read GRAY/16-bit il=0 22x10 tr=[background+addalpha_after+quantize] via disponly chunks=[iccp] idat_split=0 | exit 0 | [x] |
| 1584 | `rd\|ct=2\|bd=8\|il=0\|w=38\|h=20\|tr=expandgray\|mode=rows\|x=trns\|split=0\|n=2\|seed=16335` | fuzz read RGB/8-bit il=0 38x20 tr=[expandgray] via rows chunks=[trns] idat_split=0 | exit 0 | [x] |
| 1585 | `rd\|ct=2\|bd=16\|il=0\|w=20\|h=6\|tr=none\|mode=rows\|x=trns\|split=3\|n=2\|seed=16336` | fuzz read RGB/16-bit il=0 20x6 tr=[none] via rows chunks=[trns] idat_split=3 | exit 0 | [x] |
| 1586 | `rd\|ct=3\|bd=1\|il=0\|w=27\|h=19\|tr=stripalpha\|mode=rowonly\|x=tail\|split=0\|n=2\|seed=16337` | fuzz read PALETTE/1-bit il=0 27x19 tr=[stripalpha] via rowonly chunks=[tail] idat_split=0 | exit 0 | [x] |
| 1587 | `rd\|ct=3\|bd=2\|il=1\|w=30\|h=2\|tr=alphastd+addalpha_after\|mode=row\|x=hist\|split=3\|n=2\|seed=16338` | fuzz read PALETTE/2-bit il=1 30x2 tr=[alphastd+addalpha_after] via row chunks=[hist] idat_split=3 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1588 | `rd\|ct=3\|bd=4\|il=1\|w=28\|h=19\|tr=filler_before+packing+shift\|mode=rows\|x=clli\|split=0\|n=2\|seed=16339` | fuzz read PALETTE/4-bit il=1 28x19 tr=[filler_before+packing+shift] via rows chunks=[clli] idat_split=0 | exit 0 | [x] |
| 1589 | `rd\|ct=3\|bd=8\|il=0\|w=9\|h=24\|tr=none\|mode=startimage\|x=gama\|split=1\|n=2\|seed=16340` | fuzz read PALETTE/8-bit il=0 9x24 tr=[none] via startimage chunks=[gama] idat_split=1 | exit 0 | [x] |
| 1590 | `rd\|ct=4\|bd=8\|il=1\|w=10\|h=4\|tr=stripalpha\|mode=startimage\|x=pcal\|split=17\|n=2\|seed=16341` | fuzz read GRAY_ALPHA/8-bit il=1 10x4 tr=[stripalpha] via startimage chunks=[pcal] idat_split=17 | exit 0 | [x] |
| 1591 | `rd\|ct=4\|bd=16\|il=1\|w=15\|h=24\|tr=none\|mode=rows\|x=exif\|split=3\|n=2\|seed=16342` | fuzz read GRAY_ALPHA/16-bit il=1 15x24 tr=[none] via rows chunks=[exif] idat_split=3 | exit 0 | [x] |
| 1592 | `rd\|ct=6\|bd=8\|il=1\|w=33\|h=9\|tr=expandgray+alphapng\|mode=startimage\|x=clli\|split=0\|n=2\|seed=16343` | fuzz read RGBA/8-bit il=1 33x9 tr=[expandgray+alphapng] via startimage chunks=[clli] idat_split=0 | exit 0 | [x] |
| 1593 | `rd\|ct=6\|bd=16\|il=0\|w=18\|h=2\|tr=expand16+alphapng+trns2alpha\|mode=disponly\|x=tail\|split=0\|n=2\|seed=16344` | fuzz read RGBA/16-bit il=0 18x2 tr=[expand16+alphapng+trns2alpha] via disponly chunks=[tail] idat_split=0 | exit 0 | [x] |
| 1594 | `rd\|ct=0\|bd=1\|il=0\|w=36\|h=1\|tr=backgroundexp\|mode=rows\|x=cicp\|split=0\|n=2\|seed=16345` | fuzz read GRAY/1-bit il=0 36x1 tr=[backgroundexp] via rows chunks=[cicp] idat_split=0 | exit 0 | [x] |
| 1595 | `rd\|ct=0\|bd=2\|il=0\|w=40\|h=18\|tr=pal2rgb+trns2alpha\|mode=rows\|x=gama\|split=0\|n=2\|seed=16346` | fuzz read GRAY/2-bit il=0 40x18 tr=[pal2rgb+trns2alpha] via rows chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1596 | `rd\|ct=0\|bd=4\|il=1\|w=14\|h=16\|tr=none\|mode=startimage\|x=exif\|split=17\|n=2\|seed=16347` | fuzz read GRAY/4-bit il=1 14x16 tr=[none] via startimage chunks=[exif] idat_split=17 | exit 0 | [x] |
| 1597 | `rd\|ct=0\|bd=8\|il=0\|w=37\|h=24\|tr=invalpha+packing\|mode=startimage\|x=sbit\|split=3\|n=2\|seed=16348` | fuzz read GRAY/8-bit il=0 37x24 tr=[invalpha+packing] via startimage chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1598 | `rd\|ct=0\|bd=16\|il=1\|w=23\|h=9\|tr=trns2alpha+shift\|mode=rowonly\|x=pcal\|split=1\|n=2\|seed=16349` | fuzz read GRAY/16-bit il=1 23x9 tr=[trns2alpha+shift] via rowonly chunks=[pcal] idat_split=1 | exit 0 | [x] |
| 1599 | `rd\|ct=2\|bd=8\|il=1\|w=34\|h=3\|tr=none\|mode=row\|x=chrm\|split=0\|n=2\|seed=16350` | fuzz read RGB/8-bit il=1 34x3 tr=[none] via row chunks=[chrm] idat_split=0 | exit 0 | [x] |
| 1600 | `rd\|ct=2\|bd=16\|il=0\|w=20\|h=8\|tr=expand+alphabroken+alphastd\|mode=rows\|x=gamachrm\|split=0\|n=2\|seed=16351` | fuzz read RGB/16-bit il=0 20x8 tr=[expand+alphabroken+alphastd] via rows chunks=[gamachrm] idat_split=0 | exit 70; png_error: conflicting calls to set alpha mode and background | [x] |
| 1601 | `rd\|ct=3\|bd=1\|il=0\|w=37\|h=23\|tr=shift\|mode=startimage\|x=tail\|split=17\|n=2\|seed=16352` | fuzz read PALETTE/1-bit il=0 37x23 tr=[shift] via startimage chunks=[tail] idat_split=17 | exit 0 | [x] |
| 1602 | `rd\|ct=3\|bd=2\|il=0\|w=35\|h=11\|tr=swap16\|mode=image\|x=none\|split=1\|n=2\|seed=16353` | fuzz read PALETTE/2-bit il=0 35x11 tr=[swap16] via image chunks=[none] idat_split=1 | exit 0 | [x] |
| 1603 | `rd\|ct=3\|bd=4\|il=1\|w=30\|h=6\|tr=none\|mode=row\|x=cicp\|split=17\|n=2\|seed=16354` | fuzz read PALETTE/4-bit il=1 30x6 tr=[none] via row chunks=[cicp] idat_split=17 | exit 0 | [x] |
| 1604 | `rd\|ct=3\|bd=8\|il=0\|w=21\|h=23\|tr=bgr+filler_after+alphapng\|mode=image\|x=srgb\|split=1\|n=2\|seed=16355` | fuzz read PALETTE/8-bit il=0 21x23 tr=[bgr+filler_after+alphapng] via image chunks=[srgb] idat_split=1 | exit 0 | [x] |
| 1605 | `rd\|ct=4\|bd=8\|il=0\|w=6\|h=7\|tr=swapalpha+packing\|mode=rowonly\|x=unk\|split=1\|n=2\|seed=16356` | fuzz read GRAY_ALPHA/8-bit il=0 6x7 tr=[swapalpha+packing] via rowonly chunks=[unk] idat_split=1 | exit 0 | [x] |
| 1606 | `rd\|ct=4\|bd=16\|il=1\|w=34\|h=12\|tr=alphastd+alphabroken+gray2rgb\|mode=rows\|x=chrm\|split=3\|n=2\|seed=16357` | fuzz read GRAY_ALPHA/16-bit il=1 34x12 tr=[alphastd+alphabroken+gray2rgb] via rows chunks=[chrm] idat_split=3 | exit 70; png_error: conflicting calls to set alpha mode and background | [x] |
| 1607 | `rd\|ct=6\|bd=8\|il=0\|w=15\|h=24\|tr=shift+invalpha+rgb2gray\|mode=disponly\|x=pcal\|split=0\|n=2\|seed=16358` | fuzz read RGBA/8-bit il=0 15x24 tr=[shift+invalpha+rgb2gray] via disponly chunks=[pcal] idat_split=0 | exit 0 | [x] |
| 1608 | `rd\|ct=6\|bd=16\|il=0\|w=28\|h=5\|tr=shift+packswap+trns2alpha\|mode=rowonly\|x=tail\|split=0\|n=2\|seed=16359` | fuzz read RGBA/16-bit il=0 28x5 tr=[shift+packswap+trns2alpha] via rowonly chunks=[tail] idat_split=0 | exit 0 | [x] |
| 1609 | `rd\|ct=0\|bd=1\|il=0\|w=26\|h=14\|tr=none\|mode=rows\|x=exif\|split=1\|n=2\|seed=16360` | fuzz read GRAY/1-bit il=0 26x14 tr=[none] via rows chunks=[exif] idat_split=1 | exit 0 | [x] |
| 1610 | `rd\|ct=0\|bd=2\|il=0\|w=7\|h=8\|tr=addalpha_after+alphapng\|mode=image\|x=pcal\|split=0\|n=2\|seed=16361` | fuzz read GRAY/2-bit il=0 7x8 tr=[addalpha_after+alphapng] via image chunks=[pcal] idat_split=0 | exit 70; png_error: Invalid IHDR data; 1 warning(s): Invalid color type/bit depth combination in IHDR | [x] |
| 1611 | `rd\|ct=0\|bd=4\|il=1\|w=40\|h=3\|tr=pal2rgb\|mode=image\|x=time\|split=3\|n=2\|seed=16362` | fuzz read GRAY/4-bit il=1 40x3 tr=[pal2rgb] via image chunks=[time] idat_split=3 | exit 0 | [x] |
| 1612 | `rd\|ct=0\|bd=8\|il=0\|w=10\|h=10\|tr=alphapng+background\|mode=rowonly\|x=scal\|split=17\|n=2\|seed=16363` | fuzz read GRAY/8-bit il=0 10x10 tr=[alphapng+background] via rowonly chunks=[scal] idat_split=17 | exit 0 | [x] |
| 1613 | `rd\|ct=0\|bd=16\|il=0\|w=32\|h=3\|tr=none\|mode=rowonly\|x=text\|split=3\|n=2\|seed=16364` | fuzz read GRAY/16-bit il=0 32x3 tr=[none] via rowonly chunks=[text] idat_split=3 | exit 0 | [x] |
| 1614 | `rd\|ct=2\|bd=8\|il=0\|w=27\|h=10\|tr=pal2rgb+shift+scale16\|mode=rows\|x=text\|split=3\|n=2\|seed=16365` | fuzz read RGB/8-bit il=0 27x10 tr=[pal2rgb+shift+scale16] via rows chunks=[text] idat_split=3 | exit 0 | [x] |
| 1615 | `rd\|ct=2\|bd=16\|il=0\|w=1\|h=16\|tr=addalpha_before+trns2alpha\|mode=disponly\|x=physoffs\|split=3\|n=2\|seed=16366` | fuzz read RGB/16-bit il=0 1x16 tr=[addalpha_before+trns2alpha] via disponly chunks=[physoffs] idat_split=3 | exit 0 | [x] |
| 1616 | `rd\|ct=3\|bd=1\|il=1\|w=2\|h=21\|tr=gray2rgb\|mode=disponly\|x=trns\|split=1\|n=2\|seed=16367` | fuzz read PALETTE/1-bit il=1 2x21 tr=[gray2rgb] via disponly chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1617 | `rd\|ct=3\|bd=2\|il=0\|w=32\|h=9\|tr=invmono\|mode=startimage\|x=none\|split=1\|n=2\|seed=16368` | fuzz read PALETTE/2-bit il=0 32x9 tr=[invmono] via startimage chunks=[none] idat_split=1 | exit 0 | [x] |
| 1618 | `rd\|ct=3\|bd=4\|il=0\|w=9\|h=16\|tr=interlace+expandgray\|mode=row\|x=gama\|split=17\|n=2\|seed=16369` | fuzz read PALETTE/4-bit il=0 9x16 tr=[interlace+expandgray] via row chunks=[gama] idat_split=17 | exit 0 | [x] |
| 1619 | `rd\|ct=3\|bd=8\|il=1\|w=6\|h=23\|tr=addalpha_after\|mode=row\|x=cicp\|split=0\|n=2\|seed=16370` | fuzz read PALETTE/8-bit il=1 6x23 tr=[addalpha_after] via row chunks=[cicp] idat_split=0 | exit 0 | [x] |
| 1620 | `rd\|ct=4\|bd=8\|il=0\|w=10\|h=10\|tr=alphaopt+pal2rgb+interlace\|mode=image\|x=time\|split=17\|n=2\|seed=16371` | fuzz read GRAY_ALPHA/8-bit il=0 10x10 tr=[alphaopt+pal2rgb+interlace] via image chunks=[time] idat_split=17 | exit 0 | [x] |
| 1621 | `rd\|ct=4\|bd=16\|il=0\|w=3\|h=15\|tr=filler_before+backgroundexp+rgb2gray\|mode=image\|x=physoffs\|split=17\|n=2\|seed=16372` | fuzz read GRAY_ALPHA/16-bit il=0 3x15 tr=[filler_before+backgroundexp+rgb2gray] via image chunks=[physoffs] idat_split=17 | exit 0 | [x] |
| 1622 | `rd\|ct=6\|bd=8\|il=0\|w=15\|h=23\|tr=pal2rgb+expand16+background\|mode=rows\|x=scal\|split=0\|n=2\|seed=16373` | fuzz read RGBA/8-bit il=0 15x23 tr=[pal2rgb+expand16+background] via rows chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1623 | `rd\|ct=6\|bd=16\|il=0\|w=32\|h=11\|tr=rgb2gray+gamma\|mode=startimage\|x=none\|split=1\|n=2\|seed=16374` | fuzz read RGBA/16-bit il=0 32x11 tr=[rgb2gray+gamma] via startimage chunks=[none] idat_split=1 | exit 0 | [x] |
| 1624 | `rd\|ct=0\|bd=1\|il=1\|w=29\|h=2\|tr=packswap\|mode=rowonly\|x=iccp\|split=0\|n=2\|seed=16375` | fuzz read GRAY/1-bit il=1 29x2 tr=[packswap] via rowonly chunks=[iccp] idat_split=0 | exit 0 | [x] |
| 1625 | `rd\|ct=0\|bd=2\|il=1\|w=23\|h=21\|tr=swapalpha+filler_before\|mode=startimage\|x=text\|split=17\|n=2\|seed=16376` | fuzz read GRAY/2-bit il=1 23x21 tr=[swapalpha+filler_before] via startimage chunks=[text] idat_split=17 | exit 0 | [x] |
| 1626 | `rd\|ct=0\|bd=4\|il=0\|w=14\|h=12\|tr=packswap+pal2rgb\|mode=startimage\|x=sbit\|split=1\|n=2\|seed=16377` | fuzz read GRAY/4-bit il=0 14x12 tr=[packswap+pal2rgb] via startimage chunks=[sbit] idat_split=1 | exit 0 | [x] |
| 1627 | `rd\|ct=0\|bd=8\|il=0\|w=22\|h=14\|tr=none\|mode=row\|x=tail\|split=17\|n=2\|seed=16378` | fuzz read GRAY/8-bit il=0 22x14 tr=[none] via row chunks=[tail] idat_split=17 | exit 0 | [x] |
| 1628 | `rd\|ct=0\|bd=16\|il=0\|w=28\|h=20\|tr=packing\|mode=disponly\|x=gamachrm\|split=17\|n=2\|seed=16379` | fuzz read GRAY/16-bit il=0 28x20 tr=[packing] via disponly chunks=[gamachrm] idat_split=17 | exit 0 | [x] |
| 1629 | `rd\|ct=2\|bd=8\|il=0\|w=7\|h=21\|tr=none\|mode=image\|x=text\|split=17\|n=2\|seed=16380` | fuzz read RGB/8-bit il=0 7x21 tr=[none] via image chunks=[text] idat_split=17 | exit 0 | [x] |
| 1630 | `rd\|ct=2\|bd=16\|il=1\|w=27\|h=16\|tr=none\|mode=rows\|x=srgb\|split=3\|n=2\|seed=16381` | fuzz read RGB/16-bit il=1 27x16 tr=[none] via rows chunks=[srgb] idat_split=3 | exit 0 | [x] |
| 1631 | `rd\|ct=3\|bd=1\|il=0\|w=2\|h=10\|tr=alphaopt+addalpha_before+pal2rgb\|mode=image\|x=clli\|split=3\|n=2\|seed=16382` | fuzz read PALETTE/1-bit il=0 2x10 tr=[alphaopt+addalpha_before+pal2rgb] via image chunks=[clli] idat_split=3 | exit 0 | [x] |
| 1632 | `rd\|ct=3\|bd=2\|il=0\|w=21\|h=1\|tr=none\|mode=row\|x=cicp\|split=17\|n=2\|seed=16383` | fuzz read PALETTE/2-bit il=0 21x1 tr=[none] via row chunks=[cicp] idat_split=17 | exit 0 | [x] |
| 1633 | `rd\|ct=3\|bd=4\|il=1\|w=26\|h=7\|tr=gamma\|mode=rows\|x=hist\|split=0\|n=2\|seed=16384` | fuzz read PALETTE/4-bit il=1 26x7 tr=[gamma] via rows chunks=[hist] idat_split=0 | exit 0; 2 warning(s): hIST: out of place | [x] |
| 1634 | `rd\|ct=3\|bd=8\|il=0\|w=22\|h=5\|tr=none\|mode=rowonly\|x=gamachrm\|split=3\|n=2\|seed=16385` | fuzz read PALETTE/8-bit il=0 22x5 tr=[none] via rowonly chunks=[gamachrm] idat_split=3 | exit 0 | [x] |
| 1635 | `rd\|ct=4\|bd=8\|il=0\|w=32\|h=5\|tr=quantize+packswap+filler_before\|mode=disponly\|x=trnsbkgd\|split=1\|n=2\|seed=16386` | fuzz read GRAY_ALPHA/8-bit il=0 32x5 tr=[quantize+packswap+filler_before] via disponly chunks=[trnsbkgd] idat_split=1 | exit 0 | [x] |
| 1636 | `rd\|ct=4\|bd=16\|il=0\|w=36\|h=13\|tr=stripalpha+expand16\|mode=rows\|x=gamachrmsbittrnsbkgdtexttail\|split=3\|n=2\|seed=16387` | fuzz read GRAY_ALPHA/16-bit il=0 36x13 tr=[stripalpha+expand16] via rows chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=3 | exit 0 | [x] |
| 1637 | `rd\|ct=6\|bd=8\|il=1\|w=5\|h=5\|tr=none\|mode=rowonly\|x=sbit\|split=0\|n=2\|seed=16388` | fuzz read RGBA/8-bit il=1 5x5 tr=[none] via rowonly chunks=[sbit] idat_split=0 | exit 0 | [x] |
| 1638 | `rd\|ct=6\|bd=16\|il=1\|w=14\|h=16\|tr=packswap+rgb2graywarn\|mode=rowonly\|x=iccp\|split=0\|n=2\|seed=16389` | fuzz read RGBA/16-bit il=1 14x16 tr=[packswap+rgb2graywarn] via rowonly chunks=[iccp] idat_split=0 | exit 0; 60 warning(s): png_do_rgb_to_gray found nongray pixel | [x] |
| 1639 | `rd\|ct=0\|bd=1\|il=0\|w=16\|h=5\|tr=trns2alpha+expand\|mode=row\|x=tail\|split=3\|n=2\|seed=16390` | fuzz read GRAY/1-bit il=0 16x5 tr=[trns2alpha+expand] via row chunks=[tail] idat_split=3 | exit 0 | [x] |
| 1640 | `rd\|ct=0\|bd=2\|il=0\|w=8\|h=15\|tr=packing\|mode=rowonly\|x=tail\|split=0\|n=2\|seed=16391` | fuzz read GRAY/2-bit il=0 8x15 tr=[packing] via rowonly chunks=[tail] idat_split=0 | exit 0 | [x] |
| 1641 | `rd\|ct=0\|bd=4\|il=1\|w=36\|h=13\|tr=scale16+quantize\|mode=row\|x=unk\|split=17\|n=2\|seed=16392` | fuzz read GRAY/4-bit il=1 36x13 tr=[scale16+quantize] via row chunks=[unk] idat_split=17 | exit 0 | [x] |
| 1642 | `rd\|ct=0\|bd=8\|il=0\|w=28\|h=13\|tr=swap16\|mode=disponly\|x=none\|split=3\|n=2\|seed=16393` | fuzz read GRAY/8-bit il=0 28x13 tr=[swap16] via disponly chunks=[none] idat_split=3 | exit 0 | [x] |
| 1643 | `rd\|ct=0\|bd=16\|il=0\|w=9\|h=8\|tr=scale16+packing+backgroundexp\|mode=row\|x=time\|split=3\|n=2\|seed=16394` | fuzz read GRAY/16-bit il=0 9x8 tr=[scale16+packing+backgroundexp] via row chunks=[time] idat_split=3 | exit 0 | [x] |
| 1644 | `rd\|ct=2\|bd=8\|il=1\|w=20\|h=5\|tr=addalpha_after+pal2rgb+filler_after\|mode=image\|x=tail\|split=0\|n=2\|seed=16395` | fuzz read RGB/8-bit il=1 20x5 tr=[addalpha_after+pal2rgb+filler_after] via image chunks=[tail] idat_split=0 | exit 0 | [x] |
| 1645 | `rd\|ct=2\|bd=16\|il=1\|w=35\|h=13\|tr=filler_before+backgroundexp\|mode=rows\|x=iccp\|split=3\|n=2\|seed=16396` | fuzz read RGB/16-bit il=1 35x13 tr=[filler_before+backgroundexp] via rows chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1646 | `rd\|ct=3\|bd=1\|il=0\|w=20\|h=12\|tr=none\|mode=startimage\|x=tail\|split=1\|n=2\|seed=16397` | fuzz read PALETTE/1-bit il=0 20x12 tr=[none] via startimage chunks=[tail] idat_split=1 | exit 0 | [x] |
| 1647 | `rd\|ct=3\|bd=2\|il=0\|w=13\|h=21\|tr=swapalpha\|mode=rowonly\|x=physoffs\|split=17\|n=2\|seed=16398` | fuzz read PALETTE/2-bit il=0 13x21 tr=[swapalpha] via rowonly chunks=[physoffs] idat_split=17 | exit 0 | [x] |
| 1648 | `rd\|ct=3\|bd=4\|il=0\|w=24\|h=12\|tr=alphabroken\|mode=image\|x=gama\|split=3\|n=2\|seed=16399` | fuzz read PALETTE/4-bit il=0 24x12 tr=[alphabroken] via image chunks=[gama] idat_split=3 | exit 0 | [x] |
| 1649 | `rd\|ct=3\|bd=8\|il=0\|w=16\|h=3\|tr=expandgray+bgr+gray2rgb\|mode=row\|x=tail\|split=3\|n=2\|seed=16400` | fuzz read PALETTE/8-bit il=0 16x3 tr=[expandgray+bgr+gray2rgb] via row chunks=[tail] idat_split=3 | exit 0 | [x] |
| 1650 | `rd\|ct=4\|bd=8\|il=0\|w=26\|h=4\|tr=none\|mode=rowonly\|x=physoffs\|split=1\|n=2\|seed=16401` | fuzz read GRAY_ALPHA/8-bit il=0 26x4 tr=[none] via rowonly chunks=[physoffs] idat_split=1 | exit 0 | [x] |
| 1651 | `rd\|ct=4\|bd=16\|il=1\|w=23\|h=16\|tr=alphaopt+trns2alpha\|mode=disponly\|x=physoffs\|split=1\|n=2\|seed=16402` | fuzz read GRAY_ALPHA/16-bit il=1 23x16 tr=[alphaopt+trns2alpha] via disponly chunks=[physoffs] idat_split=1 | exit 0 | [x] |
| 1652 | `rd\|ct=6\|bd=8\|il=1\|w=24\|h=13\|tr=none\|mode=row\|x=chrm\|split=3\|n=2\|seed=16403` | fuzz read RGBA/8-bit il=1 24x13 tr=[none] via row chunks=[chrm] idat_split=3 | exit 0 | [x] |
| 1653 | `rd\|ct=6\|bd=16\|il=1\|w=28\|h=13\|tr=alphapng+addalpha_after\|mode=image\|x=plte\|split=1\|n=2\|seed=16404` | fuzz read RGBA/16-bit il=1 28x13 tr=[alphapng+addalpha_after] via image chunks=[plte] idat_split=1 | exit 0 | [x] |
| 1654 | `rd\|ct=0\|bd=1\|il=0\|w=7\|h=9\|tr=none\|mode=image\|x=text\|split=0\|n=2\|seed=16405` | fuzz read GRAY/1-bit il=0 7x9 tr=[none] via image chunks=[text] idat_split=0 | exit 0 | [x] |
| 1655 | `rd\|ct=0\|bd=2\|il=1\|w=31\|h=6\|tr=none\|mode=startimage\|x=trnsbkgd\|split=0\|n=2\|seed=16406` | fuzz read GRAY/2-bit il=1 31x6 tr=[none] via startimage chunks=[trnsbkgd] idat_split=0 | exit 0 | [x] |
| 1656 | `rd\|ct=0\|bd=4\|il=1\|w=37\|h=2\|tr=expand+background\|mode=rows\|x=sbit\|split=3\|n=2\|seed=16407` | fuzz read GRAY/4-bit il=1 37x2 tr=[expand+background] via rows chunks=[sbit] idat_split=3 | exit 0 | [x] |
| 1657 | `rd\|ct=0\|bd=8\|il=1\|w=2\|h=3\|tr=none\|mode=rowonly\|x=time\|split=3\|n=2\|seed=16408` | fuzz read GRAY/8-bit il=1 2x3 tr=[none] via rowonly chunks=[time] idat_split=3 | exit 0 | [x] |
| 1658 | `rd\|ct=0\|bd=16\|il=0\|w=29\|h=5\|tr=shift+addalpha_before\|mode=row\|x=scal\|split=0\|n=2\|seed=16409` | fuzz read GRAY/16-bit il=0 29x5 tr=[shift+addalpha_before] via row chunks=[scal] idat_split=0 | exit 0 | [x] |
| 1659 | `rd\|ct=2\|bd=8\|il=1\|w=9\|h=15\|tr=none\|mode=row\|x=gamachrmsbittrnsbkgdtexttail\|split=3\|n=2\|seed=16410` | fuzz read RGB/8-bit il=1 9x15 tr=[none] via row chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=3 | exit 0 | [x] |
| 1660 | `rd\|ct=2\|bd=16\|il=1\|w=15\|h=15\|tr=none\|mode=image\|x=tail\|split=3\|n=2\|seed=16411` | fuzz read RGB/16-bit il=1 15x15 tr=[none] via image chunks=[tail] idat_split=3 | exit 0 | [x] |
| 1661 | `rd\|ct=3\|bd=1\|il=0\|w=22\|h=24\|tr=none\|mode=row\|x=trns\|split=1\|n=2\|seed=16412` | fuzz read PALETTE/1-bit il=0 22x24 tr=[none] via row chunks=[trns] idat_split=1 | exit 0 | [x] |
| 1662 | `rd\|ct=3\|bd=2\|il=1\|w=14\|h=14\|tr=none\|mode=rowonly\|x=physoffs\|split=0\|n=2\|seed=16413` | fuzz read PALETTE/2-bit il=1 14x14 tr=[none] via rowonly chunks=[physoffs] idat_split=0 | exit 0 | [x] |
| 1663 | `rd\|ct=3\|bd=4\|il=0\|w=19\|h=6\|tr=gamma\|mode=image\|x=cicp\|split=3\|n=2\|seed=16414` | fuzz read PALETTE/4-bit il=0 19x6 tr=[gamma] via image chunks=[cicp] idat_split=3 | exit 0 | [x] |
| 1664 | `rd\|ct=3\|bd=8\|il=0\|w=10\|h=22\|tr=none\|mode=image\|x=gama\|split=0\|n=2\|seed=16415` | fuzz read PALETTE/8-bit il=0 10x22 tr=[none] via image chunks=[gama] idat_split=0 | exit 0 | [x] |
| 1665 | `rd\|ct=4\|bd=8\|il=1\|w=3\|h=14\|tr=pal2rgb\|mode=disponly\|x=iccp\|split=3\|n=2\|seed=16416` | fuzz read GRAY_ALPHA/8-bit il=1 3x14 tr=[pal2rgb] via disponly chunks=[iccp] idat_split=3 | exit 0 | [x] |
| 1666 | `rd\|ct=4\|bd=16\|il=0\|w=32\|h=16\|tr=addalpha_before+stripalpha+packswap\|mode=rows\|x=gama\|split=3\|n=2\|seed=16417` | fuzz read GRAY_ALPHA/16-bit il=0 32x16 tr=[addalpha_before+stripalpha+packswap] via rows chunks=[gama] idat_split=3 | exit 0 | [x] |
| 1667 | `rd\|ct=6\|bd=8\|il=1\|w=5\|h=7\|tr=expandgray+filler_after\|mode=disponly\|x=clli\|split=1\|n=2\|seed=16418` | fuzz read RGBA/8-bit il=1 5x7 tr=[expandgray+filler_after] via disponly chunks=[clli] idat_split=1 | exit 0 | [x] |
| 1668 | `rd\|ct=6\|bd=16\|il=0\|w=12\|h=20\|tr=none\|mode=rows\|x=gamachrmsbittrnsbkgdtexttail\|split=3\|n=2\|seed=16419` | fuzz read RGBA/16-bit il=0 12x20 tr=[none] via rows chunks=[gamachrmsbittrnsbkgdtexttail] idat_split=3 | exit 0 | [x] |

## B17 — Randomized write cross-product sweep

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1669 | `wr\|ct=0\|bd=1\|il=0\|w=32\|h=12\|tr=shift\|mode=split\|x=time\|lvl=-1\|strat=1\|filt=248\|n=2\|seed=17000` | fuzz write GRAY/1-bit il=0 32x12 tr=[shift] via split chunks=[time] level=-1 strategy=1 filters=0xf8 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1670 | `wr\|ct=0\|bd=2\|il=0\|w=13\|h=16\|tr=none\|mode=rows\|x=bkgd\|lvl=1\|strat=3\|filt=128\|n=2\|seed=17001` | fuzz write GRAY/2-bit il=0 13x16 tr=[none] via rows chunks=[bkgd] level=1 strategy=3 filters=0x80 | exit 0; 2 warning(s): Ignoring attempt to write bKGD chunk out-of-range for bit_depth | [x] |
| 1671 | `wr\|ct=0\|bd=4\|il=0\|w=32\|h=22\|tr=none\|mode=png\|x=iccp\|lvl=-1\|strat=1\|filt=16\|n=2\|seed=17002` | fuzz write GRAY/4-bit il=0 32x22 tr=[none] via png chunks=[iccp] level=-1 strategy=1 filters=0x10 | exit 0 | [x] |
| 1672 | `wr\|ct=0\|bd=8\|il=1\|w=25\|h=12\|tr=none\|mode=image\|x=iccp\|lvl=-1\|strat=2\|filt=32\|n=2\|seed=17003` | fuzz write GRAY/8-bit il=1 25x12 tr=[none] via image chunks=[iccp] level=-1 strategy=2 filters=0x20 | exit 0 | [x] |
| 1673 | `wr\|ct=0\|bd=16\|il=1\|w=2\|h=5\|tr=filler_after\|mode=png\|x=chrm\|lvl=0\|strat=3\|filt=0\|n=2\|seed=17004` | fuzz write GRAY/16-bit il=1 2x5 tr=[filler_after] via png chunks=[chrm] level=0 strategy=3 filters=0x00 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1674 | `wr\|ct=2\|bd=8\|il=1\|w=17\|h=4\|tr=none\|mode=image\|x=sbit\|lvl=9\|strat=3\|filt=32\|n=2\|seed=17005` | fuzz write RGB/8-bit il=1 17x4 tr=[none] via image chunks=[sbit] level=9 strategy=3 filters=0x20 | exit 0 | [x] |
| 1675 | `wr\|ct=2\|bd=16\|il=0\|w=40\|h=11\|tr=none\|mode=image\|x=srgb\|lvl=-1\|strat=3\|filt=56\|n=2\|seed=17006` | fuzz write RGB/16-bit il=0 40x11 tr=[none] via image chunks=[srgb] level=-1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1676 | `wr\|ct=3\|bd=1\|il=0\|w=36\|h=11\|tr=none\|mode=rows\|x=gamachrmtext\|lvl=-1\|strat=1\|filt=32\|n=2\|seed=17007` | fuzz write PALETTE/1-bit il=0 36x11 tr=[none] via rows chunks=[gamachrmtext] level=-1 strategy=1 filters=0x20 | exit 0 | [x] |
| 1677 | `wr\|ct=3\|bd=2\|il=1\|w=7\|h=18\|tr=none\|mode=rows\|x=text\|lvl=0\|strat=1\|filt=56\|n=2\|seed=17008` | fuzz write PALETTE/2-bit il=1 7x18 tr=[none] via rows chunks=[text] level=0 strategy=1 filters=0x38 | exit 0 | [x] |
| 1678 | `wr\|ct=3\|bd=4\|il=1\|w=2\|h=7\|tr=none\|mode=image\|x=time\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17009` | fuzz write PALETTE/4-bit il=1 2x7 tr=[none] via image chunks=[time] level=-1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1679 | `wr\|ct=3\|bd=8\|il=0\|w=29\|h=13\|tr=none\|mode=image\|x=trns\|lvl=-1\|strat=2\|filt=128\|n=2\|seed=17010` | fuzz write PALETTE/8-bit il=0 29x13 tr=[none] via image chunks=[trns] level=-1 strategy=2 filters=0x80 | exit 0 | [x] |
| 1680 | `wr\|ct=4\|bd=8\|il=0\|w=40\|h=11\|tr=none\|mode=split\|x=none\|lvl=5\|strat=0\|filt=56\|n=2\|seed=17011` | fuzz write GRAY_ALPHA/8-bit il=0 40x11 tr=[none] via split chunks=[none] level=5 strategy=0 filters=0x38 | exit 0 | [x] |
| 1681 | `wr\|ct=4\|bd=16\|il=0\|w=15\|h=15\|tr=none\|mode=image\|x=srgb\|lvl=9\|strat=1\|filt=56\|n=2\|seed=17012` | fuzz write GRAY_ALPHA/16-bit il=0 15x15 tr=[none] via image chunks=[srgb] level=9 strategy=1 filters=0x38 | exit 0 | [x] |
| 1682 | `wr\|ct=6\|bd=8\|il=1\|w=28\|h=4\|tr=none\|mode=rows\|x=sbit\|lvl=0\|strat=0\|filt=32\|n=2\|seed=17013` | fuzz write RGBA/8-bit il=1 28x4 tr=[none] via rows chunks=[sbit] level=0 strategy=0 filters=0x20 | exit 0 | [x] |
| 1683 | `wr\|ct=6\|bd=16\|il=0\|w=16\|h=3\|tr=bgr\|mode=png\|x=sbit\|lvl=9\|strat=1\|filt=56\|n=2\|seed=17014` | fuzz write RGBA/16-bit il=0 16x3 tr=[bgr] via png chunks=[sbit] level=9 strategy=1 filters=0x38 | exit 0 | [x] |
| 1684 | `wr\|ct=0\|bd=1\|il=1\|w=13\|h=12\|tr=none\|mode=rows\|x=unk\|lvl=1\|strat=0\|filt=0\|n=2\|seed=17015` | fuzz write GRAY/1-bit il=1 13x12 tr=[none] via rows chunks=[unk] level=1 strategy=0 filters=0x00 | exit 0 | [x] |
| 1685 | `wr\|ct=0\|bd=2\|il=1\|w=5\|h=21\|tr=none\|mode=split\|x=physoffs\|lvl=-1\|strat=3\|filt=32\|n=2\|seed=17016` | fuzz write GRAY/2-bit il=1 5x21 tr=[none] via split chunks=[physoffs] level=-1 strategy=3 filters=0x20 | exit 0 | [x] |
| 1686 | `wr\|ct=0\|bd=4\|il=1\|w=7\|h=20\|tr=shift\|mode=split\|x=trns\|lvl=-1\|strat=1\|filt=32\|n=2\|seed=17017` | fuzz write GRAY/4-bit il=1 7x20 tr=[shift] via split chunks=[trns] level=-1 strategy=1 filters=0x20 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1687 | `wr\|ct=0\|bd=8\|il=0\|w=35\|h=15\|tr=shift\|mode=split\|x=none\|lvl=0\|strat=3\|filt=0\|n=2\|seed=17018` | fuzz write GRAY/8-bit il=0 35x15 tr=[shift] via split chunks=[none] level=0 strategy=3 filters=0x00 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1688 | `wr\|ct=0\|bd=16\|il=0\|w=12\|h=23\|tr=none\|mode=rows\|x=gamachrmtext\|lvl=0\|strat=3\|filt=248\|n=2\|seed=17019` | fuzz write GRAY/16-bit il=0 12x23 tr=[none] via rows chunks=[gamachrmtext] level=0 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1689 | `wr\|ct=2\|bd=8\|il=0\|w=20\|h=24\|tr=none\|mode=png\|x=time\|lvl=0\|strat=0\|filt=248\|n=2\|seed=17020` | fuzz write RGB/8-bit il=0 20x24 tr=[none] via png chunks=[time] level=0 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1690 | `wr\|ct=2\|bd=16\|il=0\|w=12\|h=4\|tr=none\|mode=png\|x=bkgd\|lvl=1\|strat=0\|filt=16\|n=2\|seed=17021` | fuzz write RGB/16-bit il=0 12x4 tr=[none] via png chunks=[bkgd] level=1 strategy=0 filters=0x10 | exit 0 | [x] |
| 1691 | `wr\|ct=3\|bd=1\|il=1\|w=37\|h=18\|tr=none\|mode=image\|x=chrm\|lvl=0\|strat=4\|filt=0\|n=2\|seed=17022` | fuzz write PALETTE/1-bit il=1 37x18 tr=[none] via image chunks=[chrm] level=0 strategy=4 filters=0x00 | exit 0 | [x] |
| 1692 | `wr\|ct=3\|bd=2\|il=0\|w=24\|h=7\|tr=packing\|mode=rows\|x=srgb\|lvl=1\|strat=2\|filt=248\|n=2\|seed=17023` | fuzz write PALETTE/2-bit il=0 24x7 tr=[packing] via rows chunks=[srgb] level=1 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1693 | `wr\|ct=3\|bd=4\|il=0\|w=12\|h=8\|tr=none\|mode=split\|x=text\|lvl=5\|strat=3\|filt=64\|n=2\|seed=17024` | fuzz write PALETTE/4-bit il=0 12x8 tr=[none] via split chunks=[text] level=5 strategy=3 filters=0x40 | exit 0 | [x] |
| 1694 | `wr\|ct=3\|bd=8\|il=1\|w=2\|h=17\|tr=shift\|mode=rows\|x=srgb\|lvl=0\|strat=2\|filt=56\|n=2\|seed=17025` | fuzz write PALETTE/8-bit il=1 2x17 tr=[shift] via rows chunks=[srgb] level=0 strategy=2 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1695 | `wr\|ct=4\|bd=8\|il=1\|w=1\|h=24\|tr=invalpha\|mode=rows\|x=text\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17026` | fuzz write GRAY_ALPHA/8-bit il=1 1x24 tr=[invalpha] via rows chunks=[text] level=-1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1696 | `wr\|ct=4\|bd=16\|il=0\|w=4\|h=17\|tr=shift\|mode=png\|x=gamachrmtext\|lvl=0\|strat=2\|filt=32\|n=2\|seed=17027` | fuzz write GRAY_ALPHA/16-bit il=0 4x17 tr=[shift] via png chunks=[gamachrmtext] level=0 strategy=2 filters=0x20 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1697 | `wr\|ct=6\|bd=8\|il=0\|w=19\|h=2\|tr=bgr\|mode=image\|x=srgb\|lvl=1\|strat=0\|filt=248\|n=2\|seed=17028` | fuzz write RGBA/8-bit il=0 19x2 tr=[bgr] via image chunks=[srgb] level=1 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1698 | `wr\|ct=6\|bd=16\|il=1\|w=31\|h=9\|tr=none\|mode=png\|x=gamachrmtext\|lvl=-1\|strat=4\|filt=0\|n=2\|seed=17029` | fuzz write RGBA/16-bit il=1 31x9 tr=[none] via png chunks=[gamachrmtext] level=-1 strategy=4 filters=0x00 | exit 0 | [x] |
| 1699 | `wr\|ct=0\|bd=1\|il=0\|w=13\|h=11\|tr=filler_before\|mode=png\|x=unk\|lvl=5\|strat=1\|filt=8\|n=2\|seed=17030` | fuzz write GRAY/1-bit il=0 13x11 tr=[filler_before] via png chunks=[unk] level=5 strategy=1 filters=0x08 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1700 | `wr\|ct=0\|bd=2\|il=1\|w=1\|h=15\|tr=shift+packswap\|mode=png\|x=unk\|lvl=-1\|strat=1\|filt=56\|n=2\|seed=17031` | fuzz write GRAY/2-bit il=1 1x15 tr=[shift+packswap] via png chunks=[unk] level=-1 strategy=1 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1701 | `wr\|ct=0\|bd=4\|il=0\|w=32\|h=10\|tr=invmono\|mode=split\|x=bkgd\|lvl=5\|strat=3\|filt=56\|n=2\|seed=17032` | fuzz write GRAY/4-bit il=0 32x10 tr=[invmono] via split chunks=[bkgd] level=5 strategy=3 filters=0x38 | exit 0 | [x] |
| 1702 | `wr\|ct=0\|bd=8\|il=1\|w=17\|h=9\|tr=none\|mode=rows\|x=iccp\|lvl=9\|strat=3\|filt=64\|n=2\|seed=17033` | fuzz write GRAY/8-bit il=1 17x9 tr=[none] via rows chunks=[iccp] level=9 strategy=3 filters=0x40 | exit 0 | [x] |
| 1703 | `wr\|ct=0\|bd=16\|il=1\|w=5\|h=8\|tr=none\|mode=split\|x=physoffs\|lvl=-1\|strat=1\|filt=0\|n=2\|seed=17034` | fuzz write GRAY/16-bit il=1 5x8 tr=[none] via split chunks=[physoffs] level=-1 strategy=1 filters=0x00 | exit 0 | [x] |
| 1704 | `wr\|ct=2\|bd=8\|il=0\|w=1\|h=9\|tr=none\|mode=split\|x=chrm\|lvl=-1\|strat=2\|filt=32\|n=2\|seed=17035` | fuzz write RGB/8-bit il=0 1x9 tr=[none] via split chunks=[chrm] level=-1 strategy=2 filters=0x20 | exit 0 | [x] |
| 1705 | `wr\|ct=2\|bd=16\|il=0\|w=20\|h=11\|tr=bgr\|mode=image\|x=sbit\|lvl=5\|strat=1\|filt=248\|n=2\|seed=17036` | fuzz write RGB/16-bit il=0 20x11 tr=[bgr] via image chunks=[sbit] level=5 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1706 | `wr\|ct=3\|bd=1\|il=1\|w=36\|h=24\|tr=shift\|mode=image\|x=text\|lvl=0\|strat=1\|filt=248\|n=2\|seed=17037` | fuzz write PALETTE/1-bit il=1 36x24 tr=[shift] via image chunks=[text] level=0 strategy=1 filters=0xf8 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1707 | `wr\|ct=3\|bd=2\|il=0\|w=28\|h=11\|tr=none\|mode=image\|x=physoffs\|lvl=9\|strat=0\|filt=0\|n=2\|seed=17038` | fuzz write PALETTE/2-bit il=0 28x11 tr=[none] via image chunks=[physoffs] level=9 strategy=0 filters=0x00 | exit 0 | [x] |
| 1708 | `wr\|ct=3\|bd=4\|il=1\|w=24\|h=17\|tr=none\|mode=image\|x=chrm\|lvl=9\|strat=1\|filt=8\|n=2\|seed=17039` | fuzz write PALETTE/4-bit il=1 24x17 tr=[none] via image chunks=[chrm] level=9 strategy=1 filters=0x08 | exit 0 | [x] |
| 1709 | `wr\|ct=3\|bd=8\|il=1\|w=20\|h=24\|tr=none\|mode=rows\|x=chrm\|lvl=-1\|strat=4\|filt=64\|n=2\|seed=17040` | fuzz write PALETTE/8-bit il=1 20x24 tr=[none] via rows chunks=[chrm] level=-1 strategy=4 filters=0x40 | exit 0 | [x] |
| 1710 | `wr\|ct=4\|bd=8\|il=0\|w=36\|h=9\|tr=none\|mode=image\|x=chrm\|lvl=1\|strat=0\|filt=32\|n=2\|seed=17041` | fuzz write GRAY_ALPHA/8-bit il=0 36x9 tr=[none] via image chunks=[chrm] level=1 strategy=0 filters=0x20 | exit 0 | [x] |
| 1711 | `wr\|ct=4\|bd=16\|il=1\|w=2\|h=3\|tr=none\|mode=split\|x=gama\|lvl=0\|strat=1\|filt=0\|n=2\|seed=17042` | fuzz write GRAY_ALPHA/16-bit il=1 2x3 tr=[none] via split chunks=[gama] level=0 strategy=1 filters=0x00 | exit 0 | [x] |
| 1712 | `wr\|ct=6\|bd=8\|il=1\|w=28\|h=22\|tr=none\|mode=split\|x=iccp\|lvl=0\|strat=3\|filt=8\|n=2\|seed=17043` | fuzz write RGBA/8-bit il=1 28x22 tr=[none] via split chunks=[iccp] level=0 strategy=3 filters=0x08 | exit 0 | [x] |
| 1713 | `wr\|ct=6\|bd=16\|il=1\|w=9\|h=7\|tr=bgr\|mode=image\|x=srgb\|lvl=9\|strat=2\|filt=128\|n=2\|seed=17044` | fuzz write RGBA/16-bit il=1 9x7 tr=[bgr] via image chunks=[srgb] level=9 strategy=2 filters=0x80 | exit 0 | [x] |
| 1714 | `wr\|ct=0\|bd=1\|il=1\|w=12\|h=5\|tr=filler_after\|mode=split\|x=time\|lvl=9\|strat=4\|filt=8\|n=2\|seed=17045` | fuzz write GRAY/1-bit il=1 12x5 tr=[filler_after] via split chunks=[time] level=9 strategy=4 filters=0x08 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1715 | `wr\|ct=0\|bd=2\|il=1\|w=11\|h=16\|tr=filler_after\|mode=image\|x=chrm\|lvl=5\|strat=2\|filt=0\|n=2\|seed=17046` | fuzz write GRAY/2-bit il=1 11x16 tr=[filler_after] via image chunks=[chrm] level=5 strategy=2 filters=0x00 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1716 | `wr\|ct=0\|bd=4\|il=0\|w=33\|h=2\|tr=filler_before\|mode=rows\|x=gamachrmtext\|lvl=5\|strat=3\|filt=128\|n=2\|seed=17047` | fuzz write GRAY/4-bit il=0 33x2 tr=[filler_before] via rows chunks=[gamachrmtext] level=5 strategy=3 filters=0x80 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1717 | `wr\|ct=0\|bd=8\|il=1\|w=21\|h=5\|tr=none\|mode=png\|x=gamachrmtext\|lvl=5\|strat=2\|filt=0\|n=2\|seed=17048` | fuzz write GRAY/8-bit il=1 21x5 tr=[none] via png chunks=[gamachrmtext] level=5 strategy=2 filters=0x00 | exit 0 | [x] |
| 1718 | `wr\|ct=0\|bd=16\|il=1\|w=16\|h=20\|tr=shift\|mode=split\|x=none\|lvl=1\|strat=3\|filt=0\|n=2\|seed=17049` | fuzz write GRAY/16-bit il=1 16x20 tr=[shift] via split chunks=[none] level=1 strategy=3 filters=0x00 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1719 | `wr\|ct=2\|bd=8\|il=0\|w=24\|h=23\|tr=none\|mode=rows\|x=sbit\|lvl=5\|strat=4\|filt=248\|n=2\|seed=17050` | fuzz write RGB/8-bit il=0 24x23 tr=[none] via rows chunks=[sbit] level=5 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1720 | `wr\|ct=2\|bd=16\|il=0\|w=10\|h=22\|tr=none\|mode=png\|x=trns\|lvl=9\|strat=0\|filt=8\|n=2\|seed=17051` | fuzz write RGB/16-bit il=0 10x22 tr=[none] via png chunks=[trns] level=9 strategy=0 filters=0x08 | exit 0 | [x] |
| 1721 | `wr\|ct=3\|bd=1\|il=0\|w=18\|h=5\|tr=shift\|mode=rows\|x=gama\|lvl=0\|strat=0\|filt=56\|n=2\|seed=17052` | fuzz write PALETTE/1-bit il=0 18x5 tr=[shift] via rows chunks=[gama] level=0 strategy=0 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1722 | `wr\|ct=3\|bd=2\|il=1\|w=16\|h=5\|tr=shift\|mode=image\|x=gama\|lvl=0\|strat=2\|filt=8\|n=2\|seed=17053` | fuzz write PALETTE/2-bit il=1 16x5 tr=[shift] via image chunks=[gama] level=0 strategy=2 filters=0x08 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1723 | `wr\|ct=3\|bd=4\|il=1\|w=15\|h=14\|tr=none\|mode=png\|x=sbit\|lvl=0\|strat=0\|filt=64\|n=2\|seed=17054` | fuzz write PALETTE/4-bit il=1 15x14 tr=[none] via png chunks=[sbit] level=0 strategy=0 filters=0x40 | exit 0 | [x] |
| 1724 | `wr\|ct=3\|bd=8\|il=1\|w=1\|h=7\|tr=none\|mode=split\|x=gamachrmtext\|lvl=-1\|strat=0\|filt=56\|n=2\|seed=17055` | fuzz write PALETTE/8-bit il=1 1x7 tr=[none] via split chunks=[gamachrmtext] level=-1 strategy=0 filters=0x38 | exit 0 | [x] |
| 1725 | `wr\|ct=4\|bd=8\|il=1\|w=24\|h=16\|tr=none\|mode=image\|x=srgb\|lvl=9\|strat=3\|filt=8\|n=2\|seed=17056` | fuzz write GRAY_ALPHA/8-bit il=1 24x16 tr=[none] via image chunks=[srgb] level=9 strategy=3 filters=0x08 | exit 0 | [x] |
| 1726 | `wr\|ct=4\|bd=16\|il=1\|w=35\|h=23\|tr=invmono\|mode=image\|x=srgb\|lvl=0\|strat=4\|filt=16\|n=2\|seed=17057` | fuzz write GRAY_ALPHA/16-bit il=1 35x23 tr=[invmono] via image chunks=[srgb] level=0 strategy=4 filters=0x10 | exit 0 | [x] |
| 1727 | `wr\|ct=6\|bd=8\|il=0\|w=16\|h=5\|tr=swapalpha\|mode=rows\|x=bkgd\|lvl=-1\|strat=4\|filt=8\|n=2\|seed=17058` | fuzz write RGBA/8-bit il=0 16x5 tr=[swapalpha] via rows chunks=[bkgd] level=-1 strategy=4 filters=0x08 | exit 0 | [x] |
| 1728 | `wr\|ct=6\|bd=16\|il=0\|w=25\|h=23\|tr=none\|mode=image\|x=none\|lvl=1\|strat=0\|filt=0\|n=2\|seed=17059` | fuzz write RGBA/16-bit il=0 25x23 tr=[none] via image chunks=[none] level=1 strategy=0 filters=0x00 | exit 0 | [x] |
| 1729 | `wr\|ct=0\|bd=1\|il=1\|w=8\|h=20\|tr=filler_after\|mode=png\|x=bkgd\|lvl=0\|strat=1\|filt=64\|n=2\|seed=17060` | fuzz write GRAY/1-bit il=1 8x20 tr=[filler_after] via png chunks=[bkgd] level=0 strategy=1 filters=0x40 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1730 | `wr\|ct=0\|bd=2\|il=0\|w=34\|h=21\|tr=none\|mode=rows\|x=text\|lvl=-1\|strat=4\|filt=32\|n=2\|seed=17061` | fuzz write GRAY/2-bit il=0 34x21 tr=[none] via rows chunks=[text] level=-1 strategy=4 filters=0x20 | exit 0 | [x] |
| 1731 | `wr\|ct=0\|bd=4\|il=1\|w=1\|h=19\|tr=none\|mode=rows\|x=physoffs\|lvl=5\|strat=2\|filt=64\|n=2\|seed=17062` | fuzz write GRAY/4-bit il=1 1x19 tr=[none] via rows chunks=[physoffs] level=5 strategy=2 filters=0x40 | exit 0 | [x] |
| 1732 | `wr\|ct=0\|bd=8\|il=1\|w=9\|h=15\|tr=none\|mode=image\|x=time\|lvl=0\|strat=3\|filt=32\|n=2\|seed=17063` | fuzz write GRAY/8-bit il=1 9x15 tr=[none] via image chunks=[time] level=0 strategy=3 filters=0x20 | exit 0 | [x] |
| 1733 | `wr\|ct=0\|bd=16\|il=1\|w=18\|h=1\|tr=shift+swap16\|mode=rows\|x=none\|lvl=9\|strat=2\|filt=0\|n=2\|seed=17064` | fuzz write GRAY/16-bit il=1 18x1 tr=[shift+swap16] via rows chunks=[none] level=9 strategy=2 filters=0x00 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1734 | `wr\|ct=2\|bd=8\|il=0\|w=27\|h=8\|tr=bgr\|mode=image\|x=text\|lvl=-1\|strat=3\|filt=8\|n=2\|seed=17065` | fuzz write RGB/8-bit il=0 27x8 tr=[bgr] via image chunks=[text] level=-1 strategy=3 filters=0x08 | exit 0 | [x] |
| 1735 | `wr\|ct=2\|bd=16\|il=0\|w=4\|h=14\|tr=filler_after\|mode=image\|x=iccp\|lvl=0\|strat=3\|filt=128\|n=2\|seed=17066` | fuzz write RGB/16-bit il=0 4x14 tr=[filler_after] via image chunks=[iccp] level=0 strategy=3 filters=0x80 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1736 | `wr\|ct=3\|bd=1\|il=0\|w=8\|h=11\|tr=none\|mode=png\|x=chrm\|lvl=9\|strat=4\|filt=248\|n=2\|seed=17067` | fuzz write PALETTE/1-bit il=0 8x11 tr=[none] via png chunks=[chrm] level=9 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1737 | `wr\|ct=3\|bd=2\|il=0\|w=8\|h=5\|tr=none\|mode=rows\|x=bkgd\|lvl=0\|strat=0\|filt=8\|n=2\|seed=17068` | fuzz write PALETTE/2-bit il=0 8x5 tr=[none] via rows chunks=[bkgd] level=0 strategy=0 filters=0x08 | exit 0 | [x] |
| 1738 | `wr\|ct=3\|bd=4\|il=0\|w=1\|h=9\|tr=shift\|mode=image\|x=gama\|lvl=0\|strat=2\|filt=56\|n=2\|seed=17069` | fuzz write PALETTE/4-bit il=0 1x9 tr=[shift] via image chunks=[gama] level=0 strategy=2 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1739 | `wr\|ct=3\|bd=8\|il=1\|w=30\|h=9\|tr=none\|mode=rows\|x=text\|lvl=0\|strat=1\|filt=248\|n=2\|seed=17070` | fuzz write PALETTE/8-bit il=1 30x9 tr=[none] via rows chunks=[text] level=0 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1740 | `wr\|ct=4\|bd=8\|il=1\|w=12\|h=16\|tr=none\|mode=rows\|x=gama\|lvl=0\|strat=0\|filt=8\|n=2\|seed=17071` | fuzz write GRAY_ALPHA/8-bit il=1 12x16 tr=[none] via rows chunks=[gama] level=0 strategy=0 filters=0x08 | exit 0 | [x] |
| 1741 | `wr\|ct=4\|bd=16\|il=0\|w=2\|h=2\|tr=none\|mode=split\|x=srgb\|lvl=5\|strat=4\|filt=16\|n=2\|seed=17072` | fuzz write GRAY_ALPHA/16-bit il=0 2x2 tr=[none] via split chunks=[srgb] level=5 strategy=4 filters=0x10 | exit 0 | [x] |
| 1742 | `wr\|ct=6\|bd=8\|il=1\|w=2\|h=11\|tr=none\|mode=png\|x=iccp\|lvl=5\|strat=4\|filt=128\|n=2\|seed=17073` | fuzz write RGBA/8-bit il=1 2x11 tr=[none] via png chunks=[iccp] level=5 strategy=4 filters=0x80 | exit 0 | [x] |
| 1743 | `wr\|ct=6\|bd=16\|il=0\|w=18\|h=12\|tr=swap16\|mode=rows\|x=srgb\|lvl=5\|strat=1\|filt=64\|n=2\|seed=17074` | fuzz write RGBA/16-bit il=0 18x12 tr=[swap16] via rows chunks=[srgb] level=5 strategy=1 filters=0x40 | exit 0 | [x] |
| 1744 | `wr\|ct=0\|bd=1\|il=1\|w=39\|h=16\|tr=none\|mode=split\|x=chrm\|lvl=9\|strat=4\|filt=64\|n=2\|seed=17075` | fuzz write GRAY/1-bit il=1 39x16 tr=[none] via split chunks=[chrm] level=9 strategy=4 filters=0x40 | exit 0 | [x] |
| 1745 | `wr\|ct=0\|bd=2\|il=0\|w=1\|h=3\|tr=none\|mode=png\|x=gamachrmtext\|lvl=-1\|strat=3\|filt=248\|n=2\|seed=17076` | fuzz write GRAY/2-bit il=0 1x3 tr=[none] via png chunks=[gamachrmtext] level=-1 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1746 | `wr\|ct=0\|bd=4\|il=0\|w=6\|h=1\|tr=packing\|mode=image\|x=srgb\|lvl=-1\|strat=1\|filt=8\|n=2\|seed=17077` | fuzz write GRAY/4-bit il=0 6x1 tr=[packing] via image chunks=[srgb] level=-1 strategy=1 filters=0x08 | exit 0 | [x] |
| 1747 | `wr\|ct=0\|bd=8\|il=1\|w=1\|h=23\|tr=none\|mode=rows\|x=trns\|lvl=5\|strat=1\|filt=8\|n=2\|seed=17078` | fuzz write GRAY/8-bit il=1 1x23 tr=[none] via rows chunks=[trns] level=5 strategy=1 filters=0x08 | exit 0 | [x] |
| 1748 | `wr\|ct=0\|bd=16\|il=1\|w=10\|h=12\|tr=none\|mode=split\|x=sbit\|lvl=9\|strat=4\|filt=32\|n=2\|seed=17079` | fuzz write GRAY/16-bit il=1 10x12 tr=[none] via split chunks=[sbit] level=9 strategy=4 filters=0x20 | exit 0 | [x] |
| 1749 | `wr\|ct=2\|bd=8\|il=0\|w=3\|h=16\|tr=none\|mode=png\|x=none\|lvl=0\|strat=0\|filt=0\|n=2\|seed=17080` | fuzz write RGB/8-bit il=0 3x16 tr=[none] via png chunks=[none] level=0 strategy=0 filters=0x00 | exit 0 | [x] |
| 1750 | `wr\|ct=2\|bd=16\|il=0\|w=17\|h=15\|tr=none\|mode=image\|x=trns\|lvl=0\|strat=1\|filt=0\|n=2\|seed=17081` | fuzz write RGB/16-bit il=0 17x15 tr=[none] via image chunks=[trns] level=0 strategy=1 filters=0x00 | exit 0 | [x] |
| 1751 | `wr\|ct=3\|bd=1\|il=1\|w=10\|h=17\|tr=none\|mode=png\|x=bkgd\|lvl=9\|strat=1\|filt=128\|n=2\|seed=17082` | fuzz write PALETTE/1-bit il=1 10x17 tr=[none] via png chunks=[bkgd] level=9 strategy=1 filters=0x80 | exit 0 | [x] |
| 1752 | `wr\|ct=3\|bd=2\|il=1\|w=19\|h=1\|tr=none\|mode=rows\|x=physoffs\|lvl=9\|strat=3\|filt=56\|n=2\|seed=17083` | fuzz write PALETTE/2-bit il=1 19x1 tr=[none] via rows chunks=[physoffs] level=9 strategy=3 filters=0x38 | exit 0 | [x] |
| 1753 | `wr\|ct=3\|bd=4\|il=0\|w=30\|h=2\|tr=none\|mode=split\|x=none\|lvl=9\|strat=0\|filt=56\|n=2\|seed=17084` | fuzz write PALETTE/4-bit il=0 30x2 tr=[none] via split chunks=[none] level=9 strategy=0 filters=0x38 | exit 0 | [x] |
| 1754 | `wr\|ct=3\|bd=8\|il=1\|w=17\|h=10\|tr=shift\|mode=png\|x=bkgd\|lvl=0\|strat=1\|filt=16\|n=2\|seed=17085` | fuzz write PALETTE/8-bit il=1 17x10 tr=[shift] via png chunks=[bkgd] level=0 strategy=1 filters=0x10 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1755 | `wr\|ct=4\|bd=8\|il=0\|w=40\|h=18\|tr=none\|mode=image\|x=iccp\|lvl=5\|strat=4\|filt=16\|n=2\|seed=17086` | fuzz write GRAY_ALPHA/8-bit il=0 40x18 tr=[none] via image chunks=[iccp] level=5 strategy=4 filters=0x10 | exit 0 | [x] |
| 1756 | `wr\|ct=4\|bd=16\|il=0\|w=20\|h=6\|tr=swap16\|mode=rows\|x=physoffs\|lvl=9\|strat=4\|filt=128\|n=2\|seed=17087` | fuzz write GRAY_ALPHA/16-bit il=0 20x6 tr=[swap16] via rows chunks=[physoffs] level=9 strategy=4 filters=0x80 | exit 0 | [x] |
| 1757 | `wr\|ct=6\|bd=8\|il=1\|w=4\|h=1\|tr=bgr\|mode=rows\|x=text\|lvl=0\|strat=1\|filt=16\|n=2\|seed=17088` | fuzz write RGBA/8-bit il=1 4x1 tr=[bgr] via rows chunks=[text] level=0 strategy=1 filters=0x10 | exit 0 | [x] |
| 1758 | `wr\|ct=6\|bd=16\|il=0\|w=15\|h=14\|tr=none\|mode=png\|x=unk\|lvl=0\|strat=3\|filt=248\|n=2\|seed=17089` | fuzz write RGBA/16-bit il=0 15x14 tr=[none] via png chunks=[unk] level=0 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1759 | `wr\|ct=0\|bd=1\|il=1\|w=23\|h=6\|tr=none\|mode=png\|x=gama\|lvl=5\|strat=3\|filt=64\|n=2\|seed=17090` | fuzz write GRAY/1-bit il=1 23x6 tr=[none] via png chunks=[gama] level=5 strategy=3 filters=0x40 | exit 0 | [x] |
| 1760 | `wr\|ct=0\|bd=2\|il=0\|w=17\|h=15\|tr=invmono\|mode=rows\|x=bkgd\|lvl=9\|strat=3\|filt=8\|n=2\|seed=17091` | fuzz write GRAY/2-bit il=0 17x15 tr=[invmono] via rows chunks=[bkgd] level=9 strategy=3 filters=0x08 | exit 0; 2 warning(s): Ignoring attempt to write bKGD chunk out-of-range for bit_depth | [x] |
| 1761 | `wr\|ct=0\|bd=4\|il=1\|w=28\|h=4\|tr=none\|mode=rows\|x=chrm\|lvl=0\|strat=2\|filt=128\|n=2\|seed=17092` | fuzz write GRAY/4-bit il=1 28x4 tr=[none] via rows chunks=[chrm] level=0 strategy=2 filters=0x80 | exit 0 | [x] |
| 1762 | `wr\|ct=0\|bd=8\|il=0\|w=10\|h=6\|tr=filler_before\|mode=split\|x=trns\|lvl=1\|strat=4\|filt=64\|n=2\|seed=17093` | fuzz write GRAY/8-bit il=0 10x6 tr=[filler_before] via split chunks=[trns] level=1 strategy=4 filters=0x40 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1763 | `wr\|ct=0\|bd=16\|il=1\|w=33\|h=6\|tr=none\|mode=split\|x=chrm\|lvl=9\|strat=3\|filt=56\|n=2\|seed=17094` | fuzz write GRAY/16-bit il=1 33x6 tr=[none] via split chunks=[chrm] level=9 strategy=3 filters=0x38 | exit 0 | [x] |
| 1764 | `wr\|ct=2\|bd=8\|il=1\|w=12\|h=6\|tr=none\|mode=image\|x=bkgd\|lvl=1\|strat=1\|filt=128\|n=2\|seed=17095` | fuzz write RGB/8-bit il=1 12x6 tr=[none] via image chunks=[bkgd] level=1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1765 | `wr\|ct=2\|bd=16\|il=1\|w=9\|h=21\|tr=bgr+shift\|mode=image\|x=trns\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17096` | fuzz write RGB/16-bit il=1 9x21 tr=[bgr+shift] via image chunks=[trns] level=-1 strategy=4 filters=0x10 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1766 | `wr\|ct=3\|bd=1\|il=1\|w=16\|h=10\|tr=shift\|mode=rows\|x=iccp\|lvl=0\|strat=3\|filt=128\|n=2\|seed=17097` | fuzz write PALETTE/1-bit il=1 16x10 tr=[shift] via rows chunks=[iccp] level=0 strategy=3 filters=0x80 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1767 | `wr\|ct=3\|bd=2\|il=1\|w=21\|h=22\|tr=none\|mode=split\|x=bkgd\|lvl=9\|strat=1\|filt=32\|n=2\|seed=17098` | fuzz write PALETTE/2-bit il=1 21x22 tr=[none] via split chunks=[bkgd] level=9 strategy=1 filters=0x20 | exit 0 | [x] |
| 1768 | `wr\|ct=3\|bd=4\|il=0\|w=12\|h=18\|tr=shift\|mode=split\|x=bkgd\|lvl=9\|strat=3\|filt=248\|n=2\|seed=17099` | fuzz write PALETTE/4-bit il=0 12x18 tr=[shift] via split chunks=[bkgd] level=9 strategy=3 filters=0xf8 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1769 | `wr\|ct=3\|bd=8\|il=1\|w=39\|h=12\|tr=none\|mode=rows\|x=gama\|lvl=0\|strat=3\|filt=56\|n=2\|seed=17100` | fuzz write PALETTE/8-bit il=1 39x12 tr=[none] via rows chunks=[gama] level=0 strategy=3 filters=0x38 | exit 0 | [x] |
| 1770 | `wr\|ct=4\|bd=8\|il=0\|w=14\|h=20\|tr=none\|mode=image\|x=bkgd\|lvl=5\|strat=2\|filt=128\|n=2\|seed=17101` | fuzz write GRAY_ALPHA/8-bit il=0 14x20 tr=[none] via image chunks=[bkgd] level=5 strategy=2 filters=0x80 | exit 0 | [x] |
| 1771 | `wr\|ct=4\|bd=16\|il=0\|w=13\|h=20\|tr=none\|mode=png\|x=none\|lvl=9\|strat=1\|filt=248\|n=2\|seed=17102` | fuzz write GRAY_ALPHA/16-bit il=0 13x20 tr=[none] via png chunks=[none] level=9 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1772 | `wr\|ct=6\|bd=8\|il=1\|w=38\|h=9\|tr=none\|mode=image\|x=bkgd\|lvl=-1\|strat=1\|filt=0\|n=2\|seed=17103` | fuzz write RGBA/8-bit il=1 38x9 tr=[none] via image chunks=[bkgd] level=-1 strategy=1 filters=0x00 | exit 0 | [x] |
| 1773 | `wr\|ct=6\|bd=16\|il=1\|w=23\|h=5\|tr=none\|mode=image\|x=time\|lvl=0\|strat=3\|filt=8\|n=2\|seed=17104` | fuzz write RGBA/16-bit il=1 23x5 tr=[none] via image chunks=[time] level=0 strategy=3 filters=0x08 | exit 0 | [x] |
| 1774 | `wr\|ct=0\|bd=1\|il=1\|w=17\|h=22\|tr=filler_before+filler_after\|mode=rows\|x=iccp\|lvl=-1\|strat=3\|filt=8\|n=2\|seed=17105` | fuzz write GRAY/1-bit il=1 17x22 tr=[filler_before+filler_after] via rows chunks=[iccp] level=-1 strategy=3 filters=0x08 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1775 | `wr\|ct=0\|bd=2\|il=1\|w=9\|h=13\|tr=none\|mode=split\|x=unk\|lvl=0\|strat=4\|filt=248\|n=2\|seed=17106` | fuzz write GRAY/2-bit il=1 9x13 tr=[none] via split chunks=[unk] level=0 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1776 | `wr\|ct=0\|bd=4\|il=1\|w=34\|h=18\|tr=shift\|mode=png\|x=sbit\|lvl=-1\|strat=2\|filt=0\|n=2\|seed=17107` | fuzz write GRAY/4-bit il=1 34x18 tr=[shift] via png chunks=[sbit] level=-1 strategy=2 filters=0x00 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1777 | `wr\|ct=0\|bd=8\|il=0\|w=5\|h=6\|tr=none\|mode=image\|x=bkgd\|lvl=9\|strat=2\|filt=64\|n=2\|seed=17108` | fuzz write GRAY/8-bit il=0 5x6 tr=[none] via image chunks=[bkgd] level=9 strategy=2 filters=0x40 | exit 0 | [x] |
| 1778 | `wr\|ct=0\|bd=16\|il=1\|w=36\|h=2\|tr=none\|mode=image\|x=unk\|lvl=1\|strat=0\|filt=56\|n=2\|seed=17109` | fuzz write GRAY/16-bit il=1 36x2 tr=[none] via image chunks=[unk] level=1 strategy=0 filters=0x38 | exit 0 | [x] |
| 1779 | `wr\|ct=2\|bd=8\|il=1\|w=18\|h=6\|tr=none\|mode=png\|x=none\|lvl=-1\|strat=2\|filt=0\|n=2\|seed=17110` | fuzz write RGB/8-bit il=1 18x6 tr=[none] via png chunks=[none] level=-1 strategy=2 filters=0x00 | exit 0 | [x] |
| 1780 | `wr\|ct=2\|bd=16\|il=1\|w=20\|h=23\|tr=none\|mode=png\|x=trns\|lvl=9\|strat=3\|filt=56\|n=2\|seed=17111` | fuzz write RGB/16-bit il=1 20x23 tr=[none] via png chunks=[trns] level=9 strategy=3 filters=0x38 | exit 0 | [x] |
| 1781 | `wr\|ct=3\|bd=1\|il=1\|w=34\|h=2\|tr=none\|mode=image\|x=none\|lvl=-1\|strat=0\|filt=16\|n=2\|seed=17112` | fuzz write PALETTE/1-bit il=1 34x2 tr=[none] via image chunks=[none] level=-1 strategy=0 filters=0x10 | exit 0 | [x] |
| 1782 | `wr\|ct=3\|bd=2\|il=1\|w=7\|h=12\|tr=packswap\|mode=rows\|x=time\|lvl=9\|strat=3\|filt=32\|n=2\|seed=17113` | fuzz write PALETTE/2-bit il=1 7x12 tr=[packswap] via rows chunks=[time] level=9 strategy=3 filters=0x20 | exit 0 | [x] |
| 1783 | `wr\|ct=3\|bd=4\|il=0\|w=24\|h=4\|tr=none\|mode=split\|x=sbit\|lvl=0\|strat=0\|filt=128\|n=2\|seed=17114` | fuzz write PALETTE/4-bit il=0 24x4 tr=[none] via split chunks=[sbit] level=0 strategy=0 filters=0x80 | exit 0 | [x] |
| 1784 | `wr\|ct=3\|bd=8\|il=0\|w=11\|h=8\|tr=none\|mode=image\|x=gamachrmtext\|lvl=9\|strat=3\|filt=248\|n=2\|seed=17115` | fuzz write PALETTE/8-bit il=0 11x8 tr=[none] via image chunks=[gamachrmtext] level=9 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1785 | `wr\|ct=4\|bd=8\|il=0\|w=31\|h=20\|tr=swapalpha+invalpha\|mode=image\|x=unk\|lvl=-1\|strat=1\|filt=16\|n=2\|seed=17116` | fuzz write GRAY_ALPHA/8-bit il=0 31x20 tr=[swapalpha+invalpha] via image chunks=[unk] level=-1 strategy=1 filters=0x10 | exit 0 | [x] |
| 1786 | `wr\|ct=4\|bd=16\|il=0\|w=21\|h=15\|tr=none\|mode=png\|x=chrm\|lvl=1\|strat=1\|filt=128\|n=2\|seed=17117` | fuzz write GRAY_ALPHA/16-bit il=0 21x15 tr=[none] via png chunks=[chrm] level=1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1787 | `wr\|ct=6\|bd=8\|il=0\|w=39\|h=4\|tr=invalpha\|mode=image\|x=gamachrmtext\|lvl=9\|strat=3\|filt=128\|n=2\|seed=17118` | fuzz write RGBA/8-bit il=0 39x4 tr=[invalpha] via image chunks=[gamachrmtext] level=9 strategy=3 filters=0x80 | exit 0 | [x] |
| 1788 | `wr\|ct=6\|bd=16\|il=1\|w=11\|h=22\|tr=invalpha\|mode=png\|x=bkgd\|lvl=-1\|strat=0\|filt=32\|n=2\|seed=17119` | fuzz write RGBA/16-bit il=1 11x22 tr=[invalpha] via png chunks=[bkgd] level=-1 strategy=0 filters=0x20 | exit 0 | [x] |
| 1789 | `wr\|ct=0\|bd=1\|il=1\|w=7\|h=24\|tr=none\|mode=png\|x=chrm\|lvl=5\|strat=4\|filt=56\|n=2\|seed=17120` | fuzz write GRAY/1-bit il=1 7x24 tr=[none] via png chunks=[chrm] level=5 strategy=4 filters=0x38 | exit 0 | [x] |
| 1790 | `wr\|ct=0\|bd=2\|il=1\|w=30\|h=12\|tr=packswap\|mode=image\|x=gamachrmtext\|lvl=-1\|strat=3\|filt=64\|n=2\|seed=17121` | fuzz write GRAY/2-bit il=1 30x12 tr=[packswap] via image chunks=[gamachrmtext] level=-1 strategy=3 filters=0x40 | exit 0 | [x] |
| 1791 | `wr\|ct=0\|bd=4\|il=1\|w=28\|h=2\|tr=invmono\|mode=rows\|x=physoffs\|lvl=1\|strat=4\|filt=16\|n=2\|seed=17122` | fuzz write GRAY/4-bit il=1 28x2 tr=[invmono] via rows chunks=[physoffs] level=1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1792 | `wr\|ct=0\|bd=8\|il=1\|w=28\|h=24\|tr=none\|mode=split\|x=trns\|lvl=-1\|strat=3\|filt=56\|n=2\|seed=17123` | fuzz write GRAY/8-bit il=1 28x24 tr=[none] via split chunks=[trns] level=-1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1793 | `wr\|ct=0\|bd=16\|il=1\|w=28\|h=23\|tr=none\|mode=png\|x=bkgd\|lvl=9\|strat=2\|filt=16\|n=2\|seed=17124` | fuzz write GRAY/16-bit il=1 28x23 tr=[none] via png chunks=[bkgd] level=9 strategy=2 filters=0x10 | exit 0 | [x] |
| 1794 | `wr\|ct=2\|bd=8\|il=0\|w=7\|h=2\|tr=none\|mode=split\|x=trns\|lvl=-1\|strat=3\|filt=64\|n=2\|seed=17125` | fuzz write RGB/8-bit il=0 7x2 tr=[none] via split chunks=[trns] level=-1 strategy=3 filters=0x40 | exit 0 | [x] |
| 1795 | `wr\|ct=2\|bd=16\|il=0\|w=4\|h=19\|tr=none\|mode=rows\|x=physoffs\|lvl=-1\|strat=4\|filt=56\|n=2\|seed=17126` | fuzz write RGB/16-bit il=0 4x19 tr=[none] via rows chunks=[physoffs] level=-1 strategy=4 filters=0x38 | exit 0 | [x] |
| 1796 | `wr\|ct=3\|bd=1\|il=0\|w=22\|h=15\|tr=packswap\|mode=split\|x=sbit\|lvl=1\|strat=4\|filt=128\|n=2\|seed=17127` | fuzz write PALETTE/1-bit il=0 22x15 tr=[packswap] via split chunks=[sbit] level=1 strategy=4 filters=0x80 | exit 0 | [x] |
| 1797 | `wr\|ct=3\|bd=2\|il=0\|w=14\|h=15\|tr=none\|mode=png\|x=srgb\|lvl=1\|strat=3\|filt=32\|n=2\|seed=17128` | fuzz write PALETTE/2-bit il=0 14x15 tr=[none] via png chunks=[srgb] level=1 strategy=3 filters=0x20 | exit 0 | [x] |
| 1798 | `wr\|ct=3\|bd=4\|il=1\|w=40\|h=11\|tr=packswap\|mode=png\|x=gama\|lvl=0\|strat=3\|filt=16\|n=2\|seed=17129` | fuzz write PALETTE/4-bit il=1 40x11 tr=[packswap] via png chunks=[gama] level=0 strategy=3 filters=0x10 | exit 0 | [x] |
| 1799 | `wr\|ct=3\|bd=8\|il=1\|w=24\|h=8\|tr=none\|mode=split\|x=sbit\|lvl=9\|strat=0\|filt=16\|n=2\|seed=17130` | fuzz write PALETTE/8-bit il=1 24x8 tr=[none] via split chunks=[sbit] level=9 strategy=0 filters=0x10 | exit 0 | [x] |
| 1800 | `wr\|ct=4\|bd=8\|il=0\|w=6\|h=2\|tr=none\|mode=split\|x=text\|lvl=9\|strat=1\|filt=64\|n=2\|seed=17131` | fuzz write GRAY_ALPHA/8-bit il=0 6x2 tr=[none] via split chunks=[text] level=9 strategy=1 filters=0x40 | exit 0 | [x] |
| 1801 | `wr\|ct=4\|bd=16\|il=0\|w=25\|h=17\|tr=shift\|mode=png\|x=none\|lvl=1\|strat=1\|filt=64\|n=2\|seed=17132` | fuzz write GRAY_ALPHA/16-bit il=0 25x17 tr=[shift] via png chunks=[none] level=1 strategy=1 filters=0x40 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1802 | `wr\|ct=6\|bd=8\|il=0\|w=9\|h=14\|tr=none\|mode=rows\|x=physoffs\|lvl=9\|strat=0\|filt=16\|n=2\|seed=17133` | fuzz write RGBA/8-bit il=0 9x14 tr=[none] via rows chunks=[physoffs] level=9 strategy=0 filters=0x10 | exit 0 | [x] |
| 1803 | `wr\|ct=6\|bd=16\|il=1\|w=20\|h=8\|tr=bgr\|mode=rows\|x=gamachrmtext\|lvl=1\|strat=3\|filt=56\|n=2\|seed=17134` | fuzz write RGBA/16-bit il=1 20x8 tr=[bgr] via rows chunks=[gamachrmtext] level=1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1804 | `wr\|ct=0\|bd=1\|il=0\|w=18\|h=11\|tr=none\|mode=image\|x=time\|lvl=5\|strat=3\|filt=64\|n=2\|seed=17135` | fuzz write GRAY/1-bit il=0 18x11 tr=[none] via image chunks=[time] level=5 strategy=3 filters=0x40 | exit 0 | [x] |
| 1805 | `wr\|ct=0\|bd=2\|il=1\|w=31\|h=20\|tr=none\|mode=png\|x=text\|lvl=9\|strat=2\|filt=128\|n=2\|seed=17136` | fuzz write GRAY/2-bit il=1 31x20 tr=[none] via png chunks=[text] level=9 strategy=2 filters=0x80 | exit 0 | [x] |
| 1806 | `wr\|ct=0\|bd=4\|il=0\|w=17\|h=20\|tr=none\|mode=split\|x=time\|lvl=9\|strat=1\|filt=0\|n=2\|seed=17137` | fuzz write GRAY/4-bit il=0 17x20 tr=[none] via split chunks=[time] level=9 strategy=1 filters=0x00 | exit 0 | [x] |
| 1807 | `wr\|ct=0\|bd=8\|il=1\|w=24\|h=16\|tr=invmono\|mode=rows\|x=chrm\|lvl=5\|strat=0\|filt=248\|n=2\|seed=17138` | fuzz write GRAY/8-bit il=1 24x16 tr=[invmono] via rows chunks=[chrm] level=5 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1808 | `wr\|ct=0\|bd=16\|il=0\|w=9\|h=14\|tr=none\|mode=image\|x=gama\|lvl=-1\|strat=1\|filt=16\|n=2\|seed=17139` | fuzz write GRAY/16-bit il=0 9x14 tr=[none] via image chunks=[gama] level=-1 strategy=1 filters=0x10 | exit 0 | [x] |
| 1809 | `wr\|ct=2\|bd=8\|il=0\|w=24\|h=24\|tr=none\|mode=split\|x=text\|lvl=0\|strat=4\|filt=248\|n=2\|seed=17140` | fuzz write RGB/8-bit il=0 24x24 tr=[none] via split chunks=[text] level=0 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1810 | `wr\|ct=2\|bd=16\|il=1\|w=30\|h=21\|tr=bgr\|mode=png\|x=chrm\|lvl=-1\|strat=3\|filt=0\|n=2\|seed=17141` | fuzz write RGB/16-bit il=1 30x21 tr=[bgr] via png chunks=[chrm] level=-1 strategy=3 filters=0x00 | exit 0 | [x] |
| 1811 | `wr\|ct=3\|bd=1\|il=1\|w=18\|h=18\|tr=none\|mode=png\|x=unk\|lvl=-1\|strat=1\|filt=56\|n=2\|seed=17142` | fuzz write PALETTE/1-bit il=1 18x18 tr=[none] via png chunks=[unk] level=-1 strategy=1 filters=0x38 | exit 0 | [x] |
| 1812 | `wr\|ct=3\|bd=2\|il=0\|w=12\|h=12\|tr=none\|mode=image\|x=chrm\|lvl=0\|strat=4\|filt=248\|n=2\|seed=17143` | fuzz write PALETTE/2-bit il=0 12x12 tr=[none] via image chunks=[chrm] level=0 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1813 | `wr\|ct=3\|bd=4\|il=0\|w=6\|h=3\|tr=none\|mode=split\|x=physoffs\|lvl=9\|strat=0\|filt=56\|n=2\|seed=17144` | fuzz write PALETTE/4-bit il=0 6x3 tr=[none] via split chunks=[physoffs] level=9 strategy=0 filters=0x38 | exit 0 | [x] |
| 1814 | `wr\|ct=3\|bd=8\|il=0\|w=39\|h=12\|tr=none\|mode=rows\|x=bkgd\|lvl=1\|strat=1\|filt=0\|n=2\|seed=17145` | fuzz write PALETTE/8-bit il=0 39x12 tr=[none] via rows chunks=[bkgd] level=1 strategy=1 filters=0x00 | exit 0 | [x] |
| 1815 | `wr\|ct=4\|bd=8\|il=0\|w=7\|h=2\|tr=swapalpha\|mode=split\|x=srgb\|lvl=0\|strat=1\|filt=248\|n=2\|seed=17146` | fuzz write GRAY_ALPHA/8-bit il=0 7x2 tr=[swapalpha] via split chunks=[srgb] level=0 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1816 | `wr\|ct=4\|bd=16\|il=1\|w=2\|h=24\|tr=invmono\|mode=png\|x=none\|lvl=5\|strat=1\|filt=64\|n=2\|seed=17147` | fuzz write GRAY_ALPHA/16-bit il=1 2x24 tr=[invmono] via png chunks=[none] level=5 strategy=1 filters=0x40 | exit 0 | [x] |
| 1817 | `wr\|ct=6\|bd=8\|il=0\|w=27\|h=20\|tr=none\|mode=rows\|x=srgb\|lvl=1\|strat=1\|filt=248\|n=2\|seed=17148` | fuzz write RGBA/8-bit il=0 27x20 tr=[none] via rows chunks=[srgb] level=1 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1818 | `wr\|ct=6\|bd=16\|il=0\|w=9\|h=2\|tr=none\|mode=png\|x=srgb\|lvl=5\|strat=0\|filt=56\|n=2\|seed=17149` | fuzz write RGBA/16-bit il=0 9x2 tr=[none] via png chunks=[srgb] level=5 strategy=0 filters=0x38 | exit 0 | [x] |
| 1819 | `wr\|ct=0\|bd=1\|il=1\|w=38\|h=4\|tr=none\|mode=png\|x=physoffs\|lvl=9\|strat=3\|filt=32\|n=2\|seed=17150` | fuzz write GRAY/1-bit il=1 38x4 tr=[none] via png chunks=[physoffs] level=9 strategy=3 filters=0x20 | exit 0 | [x] |
| 1820 | `wr\|ct=0\|bd=2\|il=0\|w=18\|h=16\|tr=none\|mode=split\|x=text\|lvl=-1\|strat=3\|filt=32\|n=2\|seed=17151` | fuzz write GRAY/2-bit il=0 18x16 tr=[none] via split chunks=[text] level=-1 strategy=3 filters=0x20 | exit 0 | [x] |
| 1821 | `wr\|ct=0\|bd=4\|il=1\|w=4\|h=23\|tr=filler_before\|mode=rows\|x=bkgd\|lvl=9\|strat=3\|filt=64\|n=2\|seed=17152` | fuzz write GRAY/4-bit il=1 4x23 tr=[filler_before] via rows chunks=[bkgd] level=9 strategy=3 filters=0x40 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1822 | `wr\|ct=0\|bd=8\|il=1\|w=37\|h=5\|tr=none\|mode=rows\|x=gama\|lvl=0\|strat=0\|filt=248\|n=2\|seed=17153` | fuzz write GRAY/8-bit il=1 37x5 tr=[none] via rows chunks=[gama] level=0 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1823 | `wr\|ct=0\|bd=16\|il=0\|w=30\|h=19\|tr=none\|mode=png\|x=srgb\|lvl=-1\|strat=2\|filt=0\|n=2\|seed=17154` | fuzz write GRAY/16-bit il=0 30x19 tr=[none] via png chunks=[srgb] level=-1 strategy=2 filters=0x00 | exit 0 | [x] |
| 1824 | `wr\|ct=2\|bd=8\|il=0\|w=11\|h=14\|tr=none\|mode=png\|x=iccp\|lvl=-1\|strat=0\|filt=56\|n=2\|seed=17155` | fuzz write RGB/8-bit il=0 11x14 tr=[none] via png chunks=[iccp] level=-1 strategy=0 filters=0x38 | exit 0 | [x] |
| 1825 | `wr\|ct=2\|bd=16\|il=1\|w=17\|h=23\|tr=none\|mode=rows\|x=bkgd\|lvl=9\|strat=2\|filt=56\|n=2\|seed=17156` | fuzz write RGB/16-bit il=1 17x23 tr=[none] via rows chunks=[bkgd] level=9 strategy=2 filters=0x38 | exit 0 | [x] |
| 1826 | `wr\|ct=3\|bd=1\|il=0\|w=23\|h=17\|tr=none\|mode=split\|x=text\|lvl=0\|strat=2\|filt=128\|n=2\|seed=17157` | fuzz write PALETTE/1-bit il=0 23x17 tr=[none] via split chunks=[text] level=0 strategy=2 filters=0x80 | exit 0 | [x] |
| 1827 | `wr\|ct=3\|bd=2\|il=1\|w=5\|h=10\|tr=packing\|mode=image\|x=chrm\|lvl=1\|strat=2\|filt=32\|n=2\|seed=17158` | fuzz write PALETTE/2-bit il=1 5x10 tr=[packing] via image chunks=[chrm] level=1 strategy=2 filters=0x20 | exit 0 | [x] |
| 1828 | `wr\|ct=3\|bd=4\|il=1\|w=16\|h=20\|tr=none\|mode=rows\|x=srgb\|lvl=5\|strat=0\|filt=248\|n=2\|seed=17159` | fuzz write PALETTE/4-bit il=1 16x20 tr=[none] via rows chunks=[srgb] level=5 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1829 | `wr\|ct=3\|bd=8\|il=0\|w=16\|h=14\|tr=none\|mode=rows\|x=trns\|lvl=-1\|strat=0\|filt=128\|n=2\|seed=17160` | fuzz write PALETTE/8-bit il=0 16x14 tr=[none] via rows chunks=[trns] level=-1 strategy=0 filters=0x80 | exit 0 | [x] |
| 1830 | `wr\|ct=4\|bd=8\|il=1\|w=23\|h=12\|tr=none\|mode=png\|x=text\|lvl=5\|strat=2\|filt=248\|n=2\|seed=17161` | fuzz write GRAY_ALPHA/8-bit il=1 23x12 tr=[none] via png chunks=[text] level=5 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1831 | `wr\|ct=4\|bd=16\|il=1\|w=3\|h=21\|tr=invalpha+swap16\|mode=png\|x=text\|lvl=9\|strat=3\|filt=8\|n=2\|seed=17162` | fuzz write GRAY_ALPHA/16-bit il=1 3x21 tr=[invalpha+swap16] via png chunks=[text] level=9 strategy=3 filters=0x08 | exit 0 | [x] |
| 1832 | `wr\|ct=6\|bd=8\|il=1\|w=24\|h=4\|tr=invalpha\|mode=png\|x=text\|lvl=-1\|strat=4\|filt=64\|n=2\|seed=17163` | fuzz write RGBA/8-bit il=1 24x4 tr=[invalpha] via png chunks=[text] level=-1 strategy=4 filters=0x40 | exit 0 | [x] |
| 1833 | `wr\|ct=6\|bd=16\|il=0\|w=29\|h=12\|tr=swapalpha\|mode=image\|x=sbit\|lvl=5\|strat=2\|filt=16\|n=2\|seed=17164` | fuzz write RGBA/16-bit il=0 29x12 tr=[swapalpha] via image chunks=[sbit] level=5 strategy=2 filters=0x10 | exit 0 | [x] |
| 1834 | `wr\|ct=0\|bd=1\|il=0\|w=29\|h=1\|tr=filler_after\|mode=image\|x=srgb\|lvl=0\|strat=2\|filt=128\|n=2\|seed=17165` | fuzz write GRAY/1-bit il=0 29x1 tr=[filler_after] via image chunks=[srgb] level=0 strategy=2 filters=0x80 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1835 | `wr\|ct=0\|bd=2\|il=1\|w=36\|h=22\|tr=none\|mode=split\|x=trns\|lvl=5\|strat=2\|filt=16\|n=2\|seed=17166` | fuzz write GRAY/2-bit il=1 36x22 tr=[none] via split chunks=[trns] level=5 strategy=2 filters=0x10 | exit 0 | [x] |
| 1836 | `wr\|ct=0\|bd=4\|il=1\|w=5\|h=23\|tr=none\|mode=rows\|x=physoffs\|lvl=5\|strat=3\|filt=16\|n=2\|seed=17167` | fuzz write GRAY/4-bit il=1 5x23 tr=[none] via rows chunks=[physoffs] level=5 strategy=3 filters=0x10 | exit 0 | [x] |
| 1837 | `wr\|ct=0\|bd=8\|il=1\|w=31\|h=10\|tr=filler_after\|mode=png\|x=text\|lvl=9\|strat=3\|filt=56\|n=2\|seed=17168` | fuzz write GRAY/8-bit il=1 31x10 tr=[filler_after] via png chunks=[text] level=9 strategy=3 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1838 | `wr\|ct=0\|bd=16\|il=0\|w=3\|h=17\|tr=none\|mode=png\|x=bkgd\|lvl=9\|strat=1\|filt=32\|n=2\|seed=17169` | fuzz write GRAY/16-bit il=0 3x17 tr=[none] via png chunks=[bkgd] level=9 strategy=1 filters=0x20 | exit 0 | [x] |
| 1839 | `wr\|ct=2\|bd=8\|il=0\|w=38\|h=15\|tr=none\|mode=rows\|x=iccp\|lvl=-1\|strat=1\|filt=64\|n=2\|seed=17170` | fuzz write RGB/8-bit il=0 38x15 tr=[none] via rows chunks=[iccp] level=-1 strategy=1 filters=0x40 | exit 0 | [x] |
| 1840 | `wr\|ct=2\|bd=16\|il=1\|w=10\|h=17\|tr=filler_after\|mode=png\|x=sbit\|lvl=5\|strat=4\|filt=32\|n=2\|seed=17171` | fuzz write RGB/16-bit il=1 10x17 tr=[filler_after] via png chunks=[sbit] level=5 strategy=4 filters=0x20 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1841 | `wr\|ct=3\|bd=1\|il=0\|w=21\|h=2\|tr=packing\|mode=png\|x=bkgd\|lvl=1\|strat=1\|filt=248\|n=2\|seed=17172` | fuzz write PALETTE/1-bit il=0 21x2 tr=[packing] via png chunks=[bkgd] level=1 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1842 | `wr\|ct=3\|bd=2\|il=1\|w=27\|h=5\|tr=none\|mode=image\|x=time\|lvl=5\|strat=4\|filt=248\|n=2\|seed=17173` | fuzz write PALETTE/2-bit il=1 27x5 tr=[none] via image chunks=[time] level=5 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1843 | `wr\|ct=3\|bd=4\|il=1\|w=12\|h=18\|tr=none\|mode=png\|x=trns\|lvl=1\|strat=3\|filt=128\|n=2\|seed=17174` | fuzz write PALETTE/4-bit il=1 12x18 tr=[none] via png chunks=[trns] level=1 strategy=3 filters=0x80 | exit 0 | [x] |
| 1844 | `wr\|ct=3\|bd=8\|il=0\|w=22\|h=5\|tr=none\|mode=rows\|x=gama\|lvl=-1\|strat=1\|filt=0\|n=2\|seed=17175` | fuzz write PALETTE/8-bit il=0 22x5 tr=[none] via rows chunks=[gama] level=-1 strategy=1 filters=0x00 | exit 0 | [x] |
| 1845 | `wr\|ct=4\|bd=8\|il=1\|w=28\|h=11\|tr=none\|mode=image\|x=time\|lvl=0\|strat=3\|filt=32\|n=2\|seed=17176` | fuzz write GRAY_ALPHA/8-bit il=1 28x11 tr=[none] via image chunks=[time] level=0 strategy=3 filters=0x20 | exit 0 | [x] |
| 1846 | `wr\|ct=4\|bd=16\|il=1\|w=16\|h=7\|tr=none\|mode=split\|x=trns\|lvl=5\|strat=4\|filt=128\|n=2\|seed=17177` | fuzz write GRAY_ALPHA/16-bit il=1 16x7 tr=[none] via split chunks=[trns] level=5 strategy=4 filters=0x80 | exit 0 | [x] |
| 1847 | `wr\|ct=6\|bd=8\|il=1\|w=10\|h=14\|tr=none\|mode=image\|x=time\|lvl=1\|strat=1\|filt=128\|n=2\|seed=17178` | fuzz write RGBA/8-bit il=1 10x14 tr=[none] via image chunks=[time] level=1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1848 | `wr\|ct=6\|bd=16\|il=1\|w=5\|h=16\|tr=swapalpha\|mode=image\|x=trns\|lvl=0\|strat=3\|filt=32\|n=2\|seed=17179` | fuzz write RGBA/16-bit il=1 5x16 tr=[swapalpha] via image chunks=[trns] level=0 strategy=3 filters=0x20 | exit 0 | [x] |
| 1849 | `wr\|ct=0\|bd=1\|il=1\|w=32\|h=16\|tr=none\|mode=image\|x=text\|lvl=-1\|strat=1\|filt=128\|n=2\|seed=17180` | fuzz write GRAY/1-bit il=1 32x16 tr=[none] via image chunks=[text] level=-1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1850 | `wr\|ct=0\|bd=2\|il=0\|w=18\|h=17\|tr=none\|mode=split\|x=time\|lvl=-1\|strat=4\|filt=56\|n=2\|seed=17181` | fuzz write GRAY/2-bit il=0 18x17 tr=[none] via split chunks=[time] level=-1 strategy=4 filters=0x38 | exit 0 | [x] |
| 1851 | `wr\|ct=0\|bd=4\|il=1\|w=39\|h=6\|tr=none\|mode=png\|x=text\|lvl=9\|strat=1\|filt=16\|n=2\|seed=17182` | fuzz write GRAY/4-bit il=1 39x6 tr=[none] via png chunks=[text] level=9 strategy=1 filters=0x10 | exit 0 | [x] |
| 1852 | `wr\|ct=0\|bd=8\|il=1\|w=16\|h=17\|tr=none\|mode=image\|x=srgb\|lvl=1\|strat=3\|filt=248\|n=2\|seed=17183` | fuzz write GRAY/8-bit il=1 16x17 tr=[none] via image chunks=[srgb] level=1 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1853 | `wr\|ct=0\|bd=16\|il=0\|w=20\|h=6\|tr=filler_after\|mode=rows\|x=bkgd\|lvl=1\|strat=4\|filt=8\|n=2\|seed=17184` | fuzz write GRAY/16-bit il=0 20x6 tr=[filler_after] via rows chunks=[bkgd] level=1 strategy=4 filters=0x08 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1854 | `wr\|ct=2\|bd=8\|il=0\|w=15\|h=21\|tr=none\|mode=rows\|x=bkgd\|lvl=9\|strat=4\|filt=32\|n=2\|seed=17185` | fuzz write RGB/8-bit il=0 15x21 tr=[none] via rows chunks=[bkgd] level=9 strategy=4 filters=0x20 | exit 0 | [x] |
| 1855 | `wr\|ct=2\|bd=16\|il=1\|w=40\|h=10\|tr=none\|mode=rows\|x=physoffs\|lvl=5\|strat=4\|filt=248\|n=2\|seed=17186` | fuzz write RGB/16-bit il=1 40x10 tr=[none] via rows chunks=[physoffs] level=5 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1856 | `wr\|ct=3\|bd=1\|il=1\|w=27\|h=3\|tr=packing\|mode=split\|x=gamachrmtext\|lvl=1\|strat=2\|filt=8\|n=2\|seed=17187` | fuzz write PALETTE/1-bit il=1 27x3 tr=[packing] via split chunks=[gamachrmtext] level=1 strategy=2 filters=0x08 | exit 0 | [x] |
| 1857 | `wr\|ct=3\|bd=2\|il=0\|w=37\|h=13\|tr=none\|mode=png\|x=physoffs\|lvl=5\|strat=2\|filt=32\|n=2\|seed=17188` | fuzz write PALETTE/2-bit il=0 37x13 tr=[none] via png chunks=[physoffs] level=5 strategy=2 filters=0x20 | exit 0 | [x] |
| 1858 | `wr\|ct=3\|bd=4\|il=0\|w=31\|h=17\|tr=none\|mode=rows\|x=unk\|lvl=0\|strat=3\|filt=8\|n=2\|seed=17189` | fuzz write PALETTE/4-bit il=0 31x17 tr=[none] via rows chunks=[unk] level=0 strategy=3 filters=0x08 | exit 0 | [x] |
| 1859 | `wr\|ct=3\|bd=8\|il=1\|w=7\|h=3\|tr=shift\|mode=png\|x=bkgd\|lvl=0\|strat=1\|filt=32\|n=2\|seed=17190` | fuzz write PALETTE/8-bit il=1 7x3 tr=[shift] via png chunks=[bkgd] level=0 strategy=1 filters=0x20 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1860 | `wr\|ct=4\|bd=8\|il=1\|w=35\|h=2\|tr=none\|mode=image\|x=srgb\|lvl=0\|strat=0\|filt=32\|n=2\|seed=17191` | fuzz write GRAY_ALPHA/8-bit il=1 35x2 tr=[none] via image chunks=[srgb] level=0 strategy=0 filters=0x20 | exit 0 | [x] |
| 1861 | `wr\|ct=4\|bd=16\|il=0\|w=27\|h=1\|tr=swap16\|mode=png\|x=trns\|lvl=0\|strat=3\|filt=64\|n=2\|seed=17192` | fuzz write GRAY_ALPHA/16-bit il=0 27x1 tr=[swap16] via png chunks=[trns] level=0 strategy=3 filters=0x40 | exit 0 | [x] |
| 1862 | `wr\|ct=6\|bd=8\|il=1\|w=35\|h=8\|tr=none\|mode=png\|x=time\|lvl=1\|strat=1\|filt=128\|n=2\|seed=17193` | fuzz write RGBA/8-bit il=1 35x8 tr=[none] via png chunks=[time] level=1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1863 | `wr\|ct=6\|bd=16\|il=0\|w=23\|h=21\|tr=swapalpha\|mode=image\|x=iccp\|lvl=0\|strat=4\|filt=248\|n=2\|seed=17194` | fuzz write RGBA/16-bit il=0 23x21 tr=[swapalpha] via image chunks=[iccp] level=0 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1864 | `wr\|ct=0\|bd=1\|il=0\|w=6\|h=15\|tr=filler_before\|mode=png\|x=chrm\|lvl=9\|strat=0\|filt=56\|n=2\|seed=17195` | fuzz write GRAY/1-bit il=0 6x15 tr=[filler_before] via png chunks=[chrm] level=9 strategy=0 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1865 | `wr\|ct=0\|bd=2\|il=0\|w=2\|h=23\|tr=shift\|mode=image\|x=unk\|lvl=-1\|strat=0\|filt=16\|n=2\|seed=17196` | fuzz write GRAY/2-bit il=0 2x23 tr=[shift] via image chunks=[unk] level=-1 strategy=0 filters=0x10 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1866 | `wr\|ct=0\|bd=4\|il=1\|w=27\|h=8\|tr=none\|mode=rows\|x=none\|lvl=1\|strat=4\|filt=128\|n=2\|seed=17197` | fuzz write GRAY/4-bit il=1 27x8 tr=[none] via rows chunks=[none] level=1 strategy=4 filters=0x80 | exit 0 | [x] |
| 1867 | `wr\|ct=0\|bd=8\|il=1\|w=16\|h=20\|tr=none\|mode=split\|x=trns\|lvl=9\|strat=4\|filt=56\|n=2\|seed=17198` | fuzz write GRAY/8-bit il=1 16x20 tr=[none] via split chunks=[trns] level=9 strategy=4 filters=0x38 | exit 0 | [x] |
| 1868 | `wr\|ct=0\|bd=16\|il=0\|w=7\|h=18\|tr=invmono\|mode=image\|x=chrm\|lvl=-1\|strat=2\|filt=0\|n=2\|seed=17199` | fuzz write GRAY/16-bit il=0 7x18 tr=[invmono] via image chunks=[chrm] level=-1 strategy=2 filters=0x00 | exit 0 | [x] |
| 1869 | `wr\|ct=2\|bd=8\|il=0\|w=12\|h=7\|tr=none\|mode=split\|x=time\|lvl=5\|strat=0\|filt=128\|n=2\|seed=17200` | fuzz write RGB/8-bit il=0 12x7 tr=[none] via split chunks=[time] level=5 strategy=0 filters=0x80 | exit 0 | [x] |
| 1870 | `wr\|ct=2\|bd=16\|il=0\|w=35\|h=8\|tr=none\|mode=png\|x=physoffs\|lvl=-1\|strat=2\|filt=0\|n=2\|seed=17201` | fuzz write RGB/16-bit il=0 35x8 tr=[none] via png chunks=[physoffs] level=-1 strategy=2 filters=0x00 | exit 0 | [x] |
| 1871 | `wr\|ct=3\|bd=1\|il=1\|w=3\|h=23\|tr=none\|mode=split\|x=text\|lvl=9\|strat=3\|filt=128\|n=2\|seed=17202` | fuzz write PALETTE/1-bit il=1 3x23 tr=[none] via split chunks=[text] level=9 strategy=3 filters=0x80 | exit 0 | [x] |
| 1872 | `wr\|ct=3\|bd=2\|il=1\|w=34\|h=2\|tr=none\|mode=split\|x=gamachrmtext\|lvl=1\|strat=2\|filt=56\|n=2\|seed=17203` | fuzz write PALETTE/2-bit il=1 34x2 tr=[none] via split chunks=[gamachrmtext] level=1 strategy=2 filters=0x38 | exit 0 | [x] |
| 1873 | `wr\|ct=3\|bd=4\|il=1\|w=8\|h=23\|tr=none\|mode=rows\|x=iccp\|lvl=9\|strat=1\|filt=248\|n=2\|seed=17204` | fuzz write PALETTE/4-bit il=1 8x23 tr=[none] via rows chunks=[iccp] level=9 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1874 | `wr\|ct=3\|bd=8\|il=1\|w=30\|h=15\|tr=shift\|mode=rows\|x=none\|lvl=0\|strat=3\|filt=128\|n=2\|seed=17205` | fuzz write PALETTE/8-bit il=1 30x15 tr=[shift] via rows chunks=[none] level=0 strategy=3 filters=0x80 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1875 | `wr\|ct=4\|bd=8\|il=0\|w=10\|h=4\|tr=none\|mode=image\|x=bkgd\|lvl=5\|strat=0\|filt=32\|n=2\|seed=17206` | fuzz write GRAY_ALPHA/8-bit il=0 10x4 tr=[none] via image chunks=[bkgd] level=5 strategy=0 filters=0x20 | exit 0 | [x] |
| 1876 | `wr\|ct=4\|bd=16\|il=1\|w=35\|h=17\|tr=none\|mode=split\|x=iccp\|lvl=-1\|strat=3\|filt=16\|n=2\|seed=17207` | fuzz write GRAY_ALPHA/16-bit il=1 35x17 tr=[none] via split chunks=[iccp] level=-1 strategy=3 filters=0x10 | exit 0 | [x] |
| 1877 | `wr\|ct=6\|bd=8\|il=0\|w=4\|h=19\|tr=none\|mode=png\|x=unk\|lvl=0\|strat=1\|filt=0\|n=2\|seed=17208` | fuzz write RGBA/8-bit il=0 4x19 tr=[none] via png chunks=[unk] level=0 strategy=1 filters=0x00 | exit 0 | [x] |
| 1878 | `wr\|ct=6\|bd=16\|il=0\|w=34\|h=24\|tr=swap16+swapalpha\|mode=image\|x=gama\|lvl=9\|strat=3\|filt=56\|n=2\|seed=17209` | fuzz write RGBA/16-bit il=0 34x24 tr=[swap16+swapalpha] via image chunks=[gama] level=9 strategy=3 filters=0x38 | exit 0 | [x] |
| 1879 | `wr\|ct=0\|bd=1\|il=0\|w=17\|h=3\|tr=none\|mode=split\|x=text\|lvl=9\|strat=0\|filt=64\|n=2\|seed=17210` | fuzz write GRAY/1-bit il=0 17x3 tr=[none] via split chunks=[text] level=9 strategy=0 filters=0x40 | exit 0 | [x] |
| 1880 | `wr\|ct=0\|bd=2\|il=0\|w=28\|h=14\|tr=none\|mode=png\|x=none\|lvl=1\|strat=1\|filt=56\|n=2\|seed=17211` | fuzz write GRAY/2-bit il=0 28x14 tr=[none] via png chunks=[none] level=1 strategy=1 filters=0x38 | exit 0 | [x] |
| 1881 | `wr\|ct=0\|bd=4\|il=1\|w=31\|h=21\|tr=none\|mode=image\|x=physoffs\|lvl=-1\|strat=4\|filt=56\|n=2\|seed=17212` | fuzz write GRAY/4-bit il=1 31x21 tr=[none] via image chunks=[physoffs] level=-1 strategy=4 filters=0x38 | exit 0 | [x] |
| 1882 | `wr\|ct=0\|bd=8\|il=1\|w=15\|h=6\|tr=invmono\|mode=png\|x=chrm\|lvl=5\|strat=2\|filt=0\|n=2\|seed=17213` | fuzz write GRAY/8-bit il=1 15x6 tr=[invmono] via png chunks=[chrm] level=5 strategy=2 filters=0x00 | exit 0 | [x] |
| 1883 | `wr\|ct=0\|bd=16\|il=1\|w=30\|h=9\|tr=none\|mode=image\|x=trns\|lvl=-1\|strat=3\|filt=32\|n=2\|seed=17214` | fuzz write GRAY/16-bit il=1 30x9 tr=[none] via image chunks=[trns] level=-1 strategy=3 filters=0x20 | exit 0 | [x] |
| 1884 | `wr\|ct=2\|bd=8\|il=0\|w=15\|h=20\|tr=none\|mode=png\|x=gamachrmtext\|lvl=9\|strat=0\|filt=64\|n=2\|seed=17215` | fuzz write RGB/8-bit il=0 15x20 tr=[none] via png chunks=[gamachrmtext] level=9 strategy=0 filters=0x40 | exit 0 | [x] |
| 1885 | `wr\|ct=2\|bd=16\|il=1\|w=35\|h=8\|tr=filler_after\|mode=image\|x=iccp\|lvl=9\|strat=3\|filt=8\|n=2\|seed=17216` | fuzz write RGB/16-bit il=1 35x8 tr=[filler_after] via image chunks=[iccp] level=9 strategy=3 filters=0x08 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1886 | `wr\|ct=3\|bd=1\|il=1\|w=6\|h=5\|tr=packswap\|mode=image\|x=time\|lvl=9\|strat=2\|filt=0\|n=2\|seed=17217` | fuzz write PALETTE/1-bit il=1 6x5 tr=[packswap] via image chunks=[time] level=9 strategy=2 filters=0x00 | exit 0 | [x] |
| 1887 | `wr\|ct=3\|bd=2\|il=0\|w=26\|h=12\|tr=packing\|mode=rows\|x=none\|lvl=5\|strat=4\|filt=32\|n=2\|seed=17218` | fuzz write PALETTE/2-bit il=0 26x12 tr=[packing] via rows chunks=[none] level=5 strategy=4 filters=0x20 | exit 0 | [x] |
| 1888 | `wr\|ct=3\|bd=4\|il=0\|w=31\|h=20\|tr=none\|mode=png\|x=none\|lvl=-1\|strat=1\|filt=128\|n=2\|seed=17219` | fuzz write PALETTE/4-bit il=0 31x20 tr=[none] via png chunks=[none] level=-1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1889 | `wr\|ct=3\|bd=8\|il=0\|w=19\|h=16\|tr=none\|mode=split\|x=trns\|lvl=9\|strat=2\|filt=16\|n=2\|seed=17220` | fuzz write PALETTE/8-bit il=0 19x16 tr=[none] via split chunks=[trns] level=9 strategy=2 filters=0x10 | exit 0 | [x] |
| 1890 | `wr\|ct=4\|bd=8\|il=1\|w=9\|h=24\|tr=swapalpha\|mode=png\|x=unk\|lvl=-1\|strat=0\|filt=248\|n=2\|seed=17221` | fuzz write GRAY_ALPHA/8-bit il=1 9x24 tr=[swapalpha] via png chunks=[unk] level=-1 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1891 | `wr\|ct=4\|bd=16\|il=1\|w=39\|h=13\|tr=none\|mode=image\|x=trns\|lvl=1\|strat=2\|filt=64\|n=2\|seed=17222` | fuzz write GRAY_ALPHA/16-bit il=1 39x13 tr=[none] via image chunks=[trns] level=1 strategy=2 filters=0x40 | exit 0 | [x] |
| 1892 | `wr\|ct=6\|bd=8\|il=0\|w=24\|h=20\|tr=none\|mode=png\|x=trns\|lvl=-1\|strat=3\|filt=0\|n=2\|seed=17223` | fuzz write RGBA/8-bit il=0 24x20 tr=[none] via png chunks=[trns] level=-1 strategy=3 filters=0x00 | exit 0 | [x] |
| 1893 | `wr\|ct=6\|bd=16\|il=0\|w=24\|h=1\|tr=none\|mode=split\|x=bkgd\|lvl=1\|strat=0\|filt=0\|n=2\|seed=17224` | fuzz write RGBA/16-bit il=0 24x1 tr=[none] via split chunks=[bkgd] level=1 strategy=0 filters=0x00 | exit 0 | [x] |
| 1894 | `wr\|ct=0\|bd=1\|il=0\|w=37\|h=24\|tr=none\|mode=image\|x=none\|lvl=-1\|strat=4\|filt=128\|n=2\|seed=17225` | fuzz write GRAY/1-bit il=0 37x24 tr=[none] via image chunks=[none] level=-1 strategy=4 filters=0x80 | exit 0 | [x] |
| 1895 | `wr\|ct=0\|bd=2\|il=1\|w=8\|h=8\|tr=none\|mode=image\|x=physoffs\|lvl=1\|strat=3\|filt=56\|n=2\|seed=17226` | fuzz write GRAY/2-bit il=1 8x8 tr=[none] via image chunks=[physoffs] level=1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1896 | `wr\|ct=0\|bd=4\|il=1\|w=32\|h=19\|tr=none\|mode=rows\|x=chrm\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17227` | fuzz write GRAY/4-bit il=1 32x19 tr=[none] via rows chunks=[chrm] level=-1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1897 | `wr\|ct=0\|bd=8\|il=1\|w=17\|h=21\|tr=shift\|mode=split\|x=unk\|lvl=5\|strat=1\|filt=64\|n=2\|seed=17228` | fuzz write GRAY/8-bit il=1 17x21 tr=[shift] via split chunks=[unk] level=5 strategy=1 filters=0x40 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1898 | `wr\|ct=0\|bd=16\|il=0\|w=12\|h=19\|tr=none\|mode=split\|x=sbit\|lvl=9\|strat=2\|filt=248\|n=2\|seed=17229` | fuzz write GRAY/16-bit il=0 12x19 tr=[none] via split chunks=[sbit] level=9 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1899 | `wr\|ct=2\|bd=8\|il=1\|w=14\|h=12\|tr=bgr\|mode=rows\|x=physoffs\|lvl=5\|strat=3\|filt=128\|n=2\|seed=17230` | fuzz write RGB/8-bit il=1 14x12 tr=[bgr] via rows chunks=[physoffs] level=5 strategy=3 filters=0x80 | exit 0 | [x] |
| 1900 | `wr\|ct=2\|bd=16\|il=0\|w=17\|h=12\|tr=filler_after\|mode=image\|x=unk\|lvl=5\|strat=4\|filt=56\|n=2\|seed=17231` | fuzz write RGB/16-bit il=0 17x12 tr=[filler_after] via image chunks=[unk] level=5 strategy=4 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1901 | `wr\|ct=3\|bd=1\|il=0\|w=25\|h=22\|tr=none\|mode=rows\|x=bkgd\|lvl=5\|strat=3\|filt=248\|n=2\|seed=17232` | fuzz write PALETTE/1-bit il=0 25x22 tr=[none] via rows chunks=[bkgd] level=5 strategy=3 filters=0xf8 | exit 0 | [x] |
| 1902 | `wr\|ct=3\|bd=2\|il=0\|w=13\|h=22\|tr=packing\|mode=split\|x=physoffs\|lvl=0\|strat=2\|filt=32\|n=2\|seed=17233` | fuzz write PALETTE/2-bit il=0 13x22 tr=[packing] via split chunks=[physoffs] level=0 strategy=2 filters=0x20 | exit 0 | [x] |
| 1903 | `wr\|ct=3\|bd=4\|il=1\|w=32\|h=22\|tr=none\|mode=rows\|x=iccp\|lvl=0\|strat=2\|filt=16\|n=2\|seed=17234` | fuzz write PALETTE/4-bit il=1 32x22 tr=[none] via rows chunks=[iccp] level=0 strategy=2 filters=0x10 | exit 0 | [x] |
| 1904 | `wr\|ct=3\|bd=8\|il=0\|w=36\|h=16\|tr=none\|mode=split\|x=unk\|lvl=1\|strat=1\|filt=56\|n=2\|seed=17235` | fuzz write PALETTE/8-bit il=0 36x16 tr=[none] via split chunks=[unk] level=1 strategy=1 filters=0x38 | exit 0 | [x] |
| 1905 | `wr\|ct=4\|bd=8\|il=1\|w=4\|h=5\|tr=invalpha\|mode=rows\|x=gama\|lvl=1\|strat=3\|filt=56\|n=2\|seed=17236` | fuzz write GRAY_ALPHA/8-bit il=1 4x5 tr=[invalpha] via rows chunks=[gama] level=1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1906 | `wr\|ct=4\|bd=16\|il=0\|w=13\|h=1\|tr=none\|mode=image\|x=bkgd\|lvl=5\|strat=1\|filt=56\|n=2\|seed=17237` | fuzz write GRAY_ALPHA/16-bit il=0 13x1 tr=[none] via image chunks=[bkgd] level=5 strategy=1 filters=0x38 | exit 0 | [x] |
| 1907 | `wr\|ct=6\|bd=8\|il=0\|w=29\|h=20\|tr=none\|mode=split\|x=unk\|lvl=1\|strat=3\|filt=56\|n=2\|seed=17238` | fuzz write RGBA/8-bit il=0 29x20 tr=[none] via split chunks=[unk] level=1 strategy=3 filters=0x38 | exit 0 | [x] |
| 1908 | `wr\|ct=6\|bd=16\|il=1\|w=18\|h=21\|tr=none\|mode=png\|x=chrm\|lvl=5\|strat=0\|filt=16\|n=2\|seed=17239` | fuzz write RGBA/16-bit il=1 18x21 tr=[none] via png chunks=[chrm] level=5 strategy=0 filters=0x10 | exit 0 | [x] |
| 1909 | `wr\|ct=0\|bd=1\|il=1\|w=13\|h=6\|tr=filler_after\|mode=rows\|x=physoffs\|lvl=9\|strat=1\|filt=0\|n=2\|seed=17240` | fuzz write GRAY/1-bit il=1 13x6 tr=[filler_after] via rows chunks=[physoffs] level=9 strategy=1 filters=0x00 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1910 | `wr\|ct=0\|bd=2\|il=1\|w=29\|h=16\|tr=filler_before\|mode=image\|x=chrm\|lvl=0\|strat=3\|filt=0\|n=2\|seed=17241` | fuzz write GRAY/2-bit il=1 29x16 tr=[filler_before] via image chunks=[chrm] level=0 strategy=3 filters=0x00 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1911 | `wr\|ct=0\|bd=4\|il=0\|w=13\|h=6\|tr=shift\|mode=image\|x=bkgd\|lvl=1\|strat=4\|filt=128\|n=2\|seed=17242` | fuzz write GRAY/4-bit il=0 13x6 tr=[shift] via image chunks=[bkgd] level=1 strategy=4 filters=0x80 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1912 | `wr\|ct=0\|bd=8\|il=1\|w=6\|h=18\|tr=none\|mode=png\|x=srgb\|lvl=-1\|strat=3\|filt=16\|n=2\|seed=17243` | fuzz write GRAY/8-bit il=1 6x18 tr=[none] via png chunks=[srgb] level=-1 strategy=3 filters=0x10 | exit 0 | [x] |
| 1913 | `wr\|ct=0\|bd=16\|il=0\|w=10\|h=5\|tr=invmono\|mode=split\|x=gamachrmtext\|lvl=0\|strat=4\|filt=64\|n=2\|seed=17244` | fuzz write GRAY/16-bit il=0 10x5 tr=[invmono] via split chunks=[gamachrmtext] level=0 strategy=4 filters=0x40 | exit 0 | [x] |
| 1914 | `wr\|ct=2\|bd=8\|il=1\|w=22\|h=13\|tr=shift\|mode=png\|x=time\|lvl=1\|strat=3\|filt=56\|n=2\|seed=17245` | fuzz write RGB/8-bit il=1 22x13 tr=[shift] via png chunks=[time] level=1 strategy=3 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1915 | `wr\|ct=2\|bd=16\|il=0\|w=16\|h=23\|tr=filler_before\|mode=image\|x=time\|lvl=0\|strat=1\|filt=248\|n=2\|seed=17246` | fuzz write RGB/16-bit il=0 16x23 tr=[filler_before] via image chunks=[time] level=0 strategy=1 filters=0xf8 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1916 | `wr\|ct=3\|bd=1\|il=0\|w=7\|h=12\|tr=none\|mode=image\|x=chrm\|lvl=1\|strat=0\|filt=0\|n=2\|seed=17247` | fuzz write PALETTE/1-bit il=0 7x12 tr=[none] via image chunks=[chrm] level=1 strategy=0 filters=0x00 | exit 0 | [x] |
| 1917 | `wr\|ct=3\|bd=2\|il=0\|w=12\|h=24\|tr=none\|mode=split\|x=unk\|lvl=-1\|strat=4\|filt=0\|n=2\|seed=17248` | fuzz write PALETTE/2-bit il=0 12x24 tr=[none] via split chunks=[unk] level=-1 strategy=4 filters=0x00 | exit 0 | [x] |
| 1918 | `wr\|ct=3\|bd=4\|il=1\|w=35\|h=5\|tr=none\|mode=png\|x=iccp\|lvl=1\|strat=2\|filt=248\|n=2\|seed=17249` | fuzz write PALETTE/4-bit il=1 35x5 tr=[none] via png chunks=[iccp] level=1 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1919 | `wr\|ct=3\|bd=8\|il=1\|w=36\|h=4\|tr=none\|mode=image\|x=time\|lvl=-1\|strat=4\|filt=56\|n=2\|seed=17250` | fuzz write PALETTE/8-bit il=1 36x4 tr=[none] via image chunks=[time] level=-1 strategy=4 filters=0x38 | exit 0 | [x] |
| 1920 | `wr\|ct=4\|bd=8\|il=0\|w=29\|h=9\|tr=none\|mode=rows\|x=gama\|lvl=1\|strat=2\|filt=128\|n=2\|seed=17251` | fuzz write GRAY_ALPHA/8-bit il=0 29x9 tr=[none] via rows chunks=[gama] level=1 strategy=2 filters=0x80 | exit 0 | [x] |
| 1921 | `wr\|ct=4\|bd=16\|il=1\|w=37\|h=5\|tr=shift\|mode=image\|x=time\|lvl=1\|strat=4\|filt=32\|n=2\|seed=17252` | fuzz write GRAY_ALPHA/16-bit il=1 37x5 tr=[shift] via image chunks=[time] level=1 strategy=4 filters=0x20 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1922 | `wr\|ct=6\|bd=8\|il=1\|w=28\|h=1\|tr=shift+swapalpha\|mode=png\|x=physoffs\|lvl=0\|strat=4\|filt=56\|n=2\|seed=17253` | fuzz write RGBA/8-bit il=1 28x1 tr=[shift+swapalpha] via png chunks=[physoffs] level=0 strategy=4 filters=0x38 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1923 | `wr\|ct=6\|bd=16\|il=0\|w=36\|h=1\|tr=none\|mode=png\|x=time\|lvl=0\|strat=4\|filt=0\|n=2\|seed=17254` | fuzz write RGBA/16-bit il=0 36x1 tr=[none] via png chunks=[time] level=0 strategy=4 filters=0x00 | exit 0 | [x] |
| 1924 | `wr\|ct=0\|bd=1\|il=1\|w=2\|h=4\|tr=none\|mode=rows\|x=srgb\|lvl=0\|strat=2\|filt=8\|n=2\|seed=17255` | fuzz write GRAY/1-bit il=1 2x4 tr=[none] via rows chunks=[srgb] level=0 strategy=2 filters=0x08 | exit 0 | [x] |
| 1925 | `wr\|ct=0\|bd=2\|il=0\|w=39\|h=5\|tr=none\|mode=image\|x=iccp\|lvl=9\|strat=3\|filt=64\|n=2\|seed=17256` | fuzz write GRAY/2-bit il=0 39x5 tr=[none] via image chunks=[iccp] level=9 strategy=3 filters=0x40 | exit 0 | [x] |
| 1926 | `wr\|ct=0\|bd=4\|il=0\|w=1\|h=11\|tr=filler_before\|mode=rows\|x=physoffs\|lvl=5\|strat=4\|filt=16\|n=2\|seed=17257` | fuzz write GRAY/4-bit il=0 1x11 tr=[filler_before] via rows chunks=[physoffs] level=5 strategy=4 filters=0x10 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1927 | `wr\|ct=0\|bd=8\|il=0\|w=31\|h=3\|tr=shift\|mode=split\|x=none\|lvl=1\|strat=3\|filt=128\|n=2\|seed=17258` | fuzz write GRAY/8-bit il=0 31x3 tr=[shift] via split chunks=[none] level=1 strategy=3 filters=0x80 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1928 | `wr\|ct=0\|bd=16\|il=0\|w=15\|h=8\|tr=none\|mode=rows\|x=text\|lvl=9\|strat=1\|filt=16\|n=2\|seed=17259` | fuzz write GRAY/16-bit il=0 15x8 tr=[none] via rows chunks=[text] level=9 strategy=1 filters=0x10 | exit 0 | [x] |
| 1929 | `wr\|ct=2\|bd=8\|il=0\|w=39\|h=4\|tr=filler_after\|mode=rows\|x=trns\|lvl=9\|strat=2\|filt=56\|n=2\|seed=17260` | fuzz write RGB/8-bit il=0 39x4 tr=[filler_after] via rows chunks=[trns] level=9 strategy=2 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1930 | `wr\|ct=2\|bd=16\|il=0\|w=16\|h=22\|tr=none\|mode=image\|x=gamachrmtext\|lvl=5\|strat=0\|filt=32\|n=2\|seed=17261` | fuzz write RGB/16-bit il=0 16x22 tr=[none] via image chunks=[gamachrmtext] level=5 strategy=0 filters=0x20 | exit 0 | [x] |
| 1931 | `wr\|ct=3\|bd=1\|il=0\|w=32\|h=10\|tr=none\|mode=png\|x=iccp\|lvl=9\|strat=0\|filt=0\|n=2\|seed=17262` | fuzz write PALETTE/1-bit il=0 32x10 tr=[none] via png chunks=[iccp] level=9 strategy=0 filters=0x00 | exit 0 | [x] |
| 1932 | `wr\|ct=3\|bd=2\|il=0\|w=4\|h=7\|tr=none\|mode=rows\|x=unk\|lvl=1\|strat=3\|filt=64\|n=2\|seed=17263` | fuzz write PALETTE/2-bit il=0 4x7 tr=[none] via rows chunks=[unk] level=1 strategy=3 filters=0x40 | exit 0 | [x] |
| 1933 | `wr\|ct=3\|bd=4\|il=0\|w=31\|h=1\|tr=shift+packing\|mode=png\|x=bkgd\|lvl=-1\|strat=3\|filt=32\|n=2\|seed=17264` | fuzz write PALETTE/4-bit il=0 31x1 tr=[shift+packing] via png chunks=[bkgd] level=-1 strategy=3 filters=0x20 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1934 | `wr\|ct=3\|bd=8\|il=0\|w=23\|h=12\|tr=none\|mode=image\|x=unk\|lvl=5\|strat=3\|filt=8\|n=2\|seed=17265` | fuzz write PALETTE/8-bit il=0 23x12 tr=[none] via image chunks=[unk] level=5 strategy=3 filters=0x08 | exit 0 | [x] |
| 1935 | `wr\|ct=4\|bd=8\|il=1\|w=3\|h=9\|tr=none\|mode=rows\|x=time\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17266` | fuzz write GRAY_ALPHA/8-bit il=1 3x9 tr=[none] via rows chunks=[time] level=-1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1936 | `wr\|ct=4\|bd=16\|il=1\|w=33\|h=12\|tr=swap16\|mode=png\|x=physoffs\|lvl=0\|strat=3\|filt=32\|n=2\|seed=17267` | fuzz write GRAY_ALPHA/16-bit il=1 33x12 tr=[swap16] via png chunks=[physoffs] level=0 strategy=3 filters=0x20 | exit 0 | [x] |
| 1937 | `wr\|ct=6\|bd=8\|il=1\|w=10\|h=2\|tr=none\|mode=split\|x=chrm\|lvl=9\|strat=4\|filt=16\|n=2\|seed=17268` | fuzz write RGBA/8-bit il=1 10x2 tr=[none] via split chunks=[chrm] level=9 strategy=4 filters=0x10 | exit 0 | [x] |
| 1938 | `wr\|ct=6\|bd=16\|il=1\|w=20\|h=3\|tr=none\|mode=png\|x=gamachrmtext\|lvl=-1\|strat=4\|filt=248\|n=2\|seed=17269` | fuzz write RGBA/16-bit il=1 20x3 tr=[none] via png chunks=[gamachrmtext] level=-1 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1939 | `wr\|ct=0\|bd=1\|il=1\|w=32\|h=22\|tr=none\|mode=rows\|x=chrm\|lvl=0\|strat=3\|filt=128\|n=2\|seed=17270` | fuzz write GRAY/1-bit il=1 32x22 tr=[none] via rows chunks=[chrm] level=0 strategy=3 filters=0x80 | exit 0 | [x] |
| 1940 | `wr\|ct=0\|bd=2\|il=0\|w=29\|h=8\|tr=none\|mode=image\|x=physoffs\|lvl=5\|strat=2\|filt=16\|n=2\|seed=17271` | fuzz write GRAY/2-bit il=0 29x8 tr=[none] via image chunks=[physoffs] level=5 strategy=2 filters=0x10 | exit 0 | [x] |
| 1941 | `wr\|ct=0\|bd=4\|il=0\|w=16\|h=2\|tr=packswap+packing\|mode=png\|x=none\|lvl=1\|strat=2\|filt=248\|n=2\|seed=17272` | fuzz write GRAY/4-bit il=0 16x2 tr=[packswap+packing] via png chunks=[none] level=1 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1942 | `wr\|ct=0\|bd=8\|il=1\|w=23\|h=8\|tr=none\|mode=image\|x=gamachrmtext\|lvl=5\|strat=3\|filt=64\|n=2\|seed=17273` | fuzz write GRAY/8-bit il=1 23x8 tr=[none] via image chunks=[gamachrmtext] level=5 strategy=3 filters=0x40 | exit 0 | [x] |
| 1943 | `wr\|ct=0\|bd=16\|il=1\|w=10\|h=7\|tr=filler_after\|mode=image\|x=physoffs\|lvl=-1\|strat=3\|filt=0\|n=2\|seed=17274` | fuzz write GRAY/16-bit il=1 10x7 tr=[filler_after] via image chunks=[physoffs] level=-1 strategy=3 filters=0x00 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1944 | `wr\|ct=2\|bd=8\|il=1\|w=19\|h=16\|tr=filler_before\|mode=image\|x=iccp\|lvl=9\|strat=4\|filt=56\|n=2\|seed=17275` | fuzz write RGB/8-bit il=1 19x16 tr=[filler_before] via image chunks=[iccp] level=9 strategy=4 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1945 | `wr\|ct=2\|bd=16\|il=0\|w=3\|h=7\|tr=none\|mode=rows\|x=physoffs\|lvl=1\|strat=3\|filt=16\|n=2\|seed=17276` | fuzz write RGB/16-bit il=0 3x7 tr=[none] via rows chunks=[physoffs] level=1 strategy=3 filters=0x10 | exit 0 | [x] |
| 1946 | `wr\|ct=3\|bd=1\|il=1\|w=1\|h=15\|tr=none\|mode=split\|x=physoffs\|lvl=0\|strat=2\|filt=56\|n=2\|seed=17277` | fuzz write PALETTE/1-bit il=1 1x15 tr=[none] via split chunks=[physoffs] level=0 strategy=2 filters=0x38 | exit 0 | [x] |
| 1947 | `wr\|ct=3\|bd=2\|il=0\|w=35\|h=22\|tr=none\|mode=image\|x=unk\|lvl=-1\|strat=1\|filt=248\|n=2\|seed=17278` | fuzz write PALETTE/2-bit il=0 35x22 tr=[none] via image chunks=[unk] level=-1 strategy=1 filters=0xf8 | exit 0 | [x] |
| 1948 | `wr\|ct=3\|bd=4\|il=1\|w=29\|h=23\|tr=packing\|mode=png\|x=trns\|lvl=0\|strat=1\|filt=8\|n=2\|seed=17279` | fuzz write PALETTE/4-bit il=1 29x23 tr=[packing] via png chunks=[trns] level=0 strategy=1 filters=0x08 | exit 0 | [x] |
| 1949 | `wr\|ct=3\|bd=8\|il=0\|w=40\|h=6\|tr=none\|mode=png\|x=gama\|lvl=9\|strat=4\|filt=16\|n=2\|seed=17280` | fuzz write PALETTE/8-bit il=0 40x6 tr=[none] via png chunks=[gama] level=9 strategy=4 filters=0x10 | exit 0 | [x] |
| 1950 | `wr\|ct=4\|bd=8\|il=1\|w=5\|h=6\|tr=none\|mode=png\|x=text\|lvl=9\|strat=2\|filt=128\|n=2\|seed=17281` | fuzz write GRAY_ALPHA/8-bit il=1 5x6 tr=[none] via png chunks=[text] level=9 strategy=2 filters=0x80 | exit 0 | [x] |
| 1951 | `wr\|ct=4\|bd=16\|il=1\|w=3\|h=22\|tr=none\|mode=png\|x=iccp\|lvl=-1\|strat=2\|filt=64\|n=2\|seed=17282` | fuzz write GRAY_ALPHA/16-bit il=1 3x22 tr=[none] via png chunks=[iccp] level=-1 strategy=2 filters=0x40 | exit 0 | [x] |
| 1952 | `wr\|ct=6\|bd=8\|il=0\|w=10\|h=9\|tr=invalpha\|mode=rows\|x=time\|lvl=0\|strat=2\|filt=248\|n=2\|seed=17283` | fuzz write RGBA/8-bit il=0 10x9 tr=[invalpha] via rows chunks=[time] level=0 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1953 | `wr\|ct=6\|bd=16\|il=0\|w=9\|h=3\|tr=none\|mode=png\|x=text\|lvl=9\|strat=1\|filt=64\|n=2\|seed=17284` | fuzz write RGBA/16-bit il=0 9x3 tr=[none] via png chunks=[text] level=9 strategy=1 filters=0x40 | exit 0 | [x] |
| 1954 | `wr\|ct=0\|bd=1\|il=1\|w=9\|h=13\|tr=shift+packing\|mode=image\|x=bkgd\|lvl=1\|strat=1\|filt=0\|n=2\|seed=17285` | fuzz write GRAY/1-bit il=1 9x13 tr=[shift+packing] via image chunks=[bkgd] level=1 strategy=1 filters=0x00 | exit 70; png_error: png_set_shift: invalid shift values | [x] |
| 1955 | `wr\|ct=0\|bd=2\|il=0\|w=25\|h=14\|tr=none\|mode=image\|x=trns\|lvl=5\|strat=4\|filt=16\|n=2\|seed=17286` | fuzz write GRAY/2-bit il=0 25x14 tr=[none] via image chunks=[trns] level=5 strategy=4 filters=0x10 | exit 0 | [x] |
| 1956 | `wr\|ct=0\|bd=4\|il=1\|w=39\|h=6\|tr=filler_before\|mode=png\|x=bkgd\|lvl=-1\|strat=3\|filt=56\|n=2\|seed=17287` | fuzz write GRAY/4-bit il=1 39x6 tr=[filler_before] via png chunks=[bkgd] level=-1 strategy=3 filters=0x38 | exit 70; png_error: png_set_filler is invalid for low bit depth gray output | [x] |
| 1957 | `wr\|ct=0\|bd=8\|il=1\|w=31\|h=3\|tr=none\|mode=split\|x=srgb\|lvl=5\|strat=4\|filt=248\|n=2\|seed=17288` | fuzz write GRAY/8-bit il=1 31x3 tr=[none] via split chunks=[srgb] level=5 strategy=4 filters=0xf8 | exit 0 | [x] |
| 1958 | `wr\|ct=0\|bd=16\|il=0\|w=19\|h=16\|tr=none\|mode=png\|x=unk\|lvl=0\|strat=1\|filt=128\|n=2\|seed=17289` | fuzz write GRAY/16-bit il=0 19x16 tr=[none] via png chunks=[unk] level=0 strategy=1 filters=0x80 | exit 0 | [x] |
| 1959 | `wr\|ct=2\|bd=8\|il=1\|w=33\|h=20\|tr=none\|mode=png\|x=none\|lvl=0\|strat=1\|filt=32\|n=2\|seed=17290` | fuzz write RGB/8-bit il=1 33x20 tr=[none] via png chunks=[none] level=0 strategy=1 filters=0x20 | exit 0 | [x] |
| 1960 | `wr\|ct=2\|bd=16\|il=1\|w=28\|h=3\|tr=none\|mode=image\|x=bkgd\|lvl=9\|strat=1\|filt=16\|n=2\|seed=17291` | fuzz write RGB/16-bit il=1 28x3 tr=[none] via image chunks=[bkgd] level=9 strategy=1 filters=0x10 | exit 0 | [x] |
| 1961 | `wr\|ct=3\|bd=1\|il=1\|w=31\|h=12\|tr=none\|mode=image\|x=text\|lvl=5\|strat=0\|filt=8\|n=2\|seed=17292` | fuzz write PALETTE/1-bit il=1 31x12 tr=[none] via image chunks=[text] level=5 strategy=0 filters=0x08 | exit 0 | [x] |
| 1962 | `wr\|ct=3\|bd=2\|il=1\|w=4\|h=16\|tr=none\|mode=png\|x=iccp\|lvl=1\|strat=0\|filt=128\|n=2\|seed=17293` | fuzz write PALETTE/2-bit il=1 4x16 tr=[none] via png chunks=[iccp] level=1 strategy=0 filters=0x80 | exit 0 | [x] |
| 1963 | `wr\|ct=3\|bd=4\|il=0\|w=26\|h=17\|tr=none\|mode=rows\|x=iccp\|lvl=-1\|strat=0\|filt=248\|n=2\|seed=17294` | fuzz write PALETTE/4-bit il=0 26x17 tr=[none] via rows chunks=[iccp] level=-1 strategy=0 filters=0xf8 | exit 0 | [x] |
| 1964 | `wr\|ct=3\|bd=8\|il=0\|w=25\|h=15\|tr=none\|mode=split\|x=gama\|lvl=-1\|strat=4\|filt=0\|n=2\|seed=17295` | fuzz write PALETTE/8-bit il=0 25x15 tr=[none] via split chunks=[gama] level=-1 strategy=4 filters=0x00 | exit 0 | [x] |
| 1965 | `wr\|ct=4\|bd=8\|il=1\|w=24\|h=7\|tr=none\|mode=png\|x=trns\|lvl=9\|strat=0\|filt=128\|n=2\|seed=17296` | fuzz write GRAY_ALPHA/8-bit il=1 24x7 tr=[none] via png chunks=[trns] level=9 strategy=0 filters=0x80 | exit 0 | [x] |
| 1966 | `wr\|ct=4\|bd=16\|il=1\|w=13\|h=2\|tr=none\|mode=rows\|x=sbit\|lvl=9\|strat=2\|filt=64\|n=2\|seed=17297` | fuzz write GRAY_ALPHA/16-bit il=1 13x2 tr=[none] via rows chunks=[sbit] level=9 strategy=2 filters=0x40 | exit 0 | [x] |
| 1967 | `wr\|ct=6\|bd=8\|il=1\|w=32\|h=14\|tr=none\|mode=image\|x=gamachrmtext\|lvl=-1\|strat=3\|filt=16\|n=2\|seed=17298` | fuzz write RGBA/8-bit il=1 32x14 tr=[none] via image chunks=[gamachrmtext] level=-1 strategy=3 filters=0x10 | exit 0 | [x] |
| 1968 | `wr\|ct=6\|bd=16\|il=0\|w=2\|h=2\|tr=swap16\|mode=split\|x=chrm\|lvl=0\|strat=4\|filt=16\|n=2\|seed=17299` | fuzz write RGBA/16-bit il=0 2x2 tr=[swap16] via split chunks=[chrm] level=0 strategy=4 filters=0x10 | exit 0 | [x] |
| 1969 | `wr\|ct=0\|bd=1\|il=0\|w=23\|h=7\|tr=none\|mode=image\|x=text\|lvl=5\|strat=0\|filt=16\|n=2\|seed=17300` | fuzz write GRAY/1-bit il=0 23x7 tr=[none] via image chunks=[text] level=5 strategy=0 filters=0x10 | exit 0 | [x] |
| 1970 | `wr\|ct=0\|bd=2\|il=0\|w=23\|h=19\|tr=none\|mode=split\|x=chrm\|lvl=-1\|strat=2\|filt=248\|n=2\|seed=17301` | fuzz write GRAY/2-bit il=0 23x19 tr=[none] via split chunks=[chrm] level=-1 strategy=2 filters=0xf8 | exit 0 | [x] |
| 1971 | `wr\|ct=0\|bd=4\|il=1\|w=39\|h=9\|tr=packswap\|mode=png\|x=none\|lvl=9\|strat=4\|filt=8\|n=2\|seed=17302` | fuzz write GRAY/4-bit il=1 39x9 tr=[packswap] via png chunks=[none] level=9 strategy=4 filters=0x08 | exit 0 | [x] |
| 1972 | `wr\|ct=0\|bd=8\|il=1\|w=28\|h=7\|tr=invmono\|mode=split\|x=bkgd\|lvl=1\|strat=1\|filt=128\|n=2\|seed=17303` | fuzz write GRAY/8-bit il=1 28x7 tr=[invmono] via split chunks=[bkgd] level=1 strategy=1 filters=0x80 | exit 0 | [x] |
| 1973 | `wr\|ct=0\|bd=16\|il=0\|w=19\|h=17\|tr=swap16\|mode=png\|x=unk\|lvl=5\|strat=4\|filt=32\|n=2\|seed=17304` | fuzz write GRAY/16-bit il=0 19x17 tr=[swap16] via png chunks=[unk] level=5 strategy=4 filters=0x20 | exit 0 | [x] |
| 1974 | `wr\|ct=2\|bd=8\|il=0\|w=7\|h=2\|tr=none\|mode=split\|x=text\|lvl=5\|strat=3\|filt=56\|n=2\|seed=17305` | fuzz write RGB/8-bit il=0 7x2 tr=[none] via split chunks=[text] level=5 strategy=3 filters=0x38 | exit 0 | [x] |
| 1975 | `wr\|ct=2\|bd=16\|il=0\|w=28\|h=9\|tr=none\|mode=split\|x=bkgd\|lvl=9\|strat=2\|filt=56\|n=2\|seed=17306` | fuzz write RGB/16-bit il=0 28x9 tr=[none] via split chunks=[bkgd] level=9 strategy=2 filters=0x38 | exit 0 | [x] |
| 1976 | `wr\|ct=3\|bd=1\|il=1\|w=28\|h=20\|tr=packswap\|mode=split\|x=sbit\|lvl=-1\|strat=1\|filt=16\|n=2\|seed=17307` | fuzz write PALETTE/1-bit il=1 28x20 tr=[packswap] via split chunks=[sbit] level=-1 strategy=1 filters=0x10 | exit 0 | [x] |
| 1977 | `wr\|ct=3\|bd=2\|il=1\|w=18\|h=18\|tr=none\|mode=image\|x=sbit\|lvl=0\|strat=3\|filt=16\|n=2\|seed=17308` | fuzz write PALETTE/2-bit il=1 18x18 tr=[none] via image chunks=[sbit] level=0 strategy=3 filters=0x10 | exit 0 | [x] |
| 1978 | `wr\|ct=3\|bd=4\|il=0\|w=16\|h=13\|tr=none\|mode=rows\|x=iccp\|lvl=1\|strat=0\|filt=8\|n=2\|seed=17309` | fuzz write PALETTE/4-bit il=0 16x13 tr=[none] via rows chunks=[iccp] level=1 strategy=0 filters=0x08 | exit 0 | [x] |
| 1979 | `wr\|ct=3\|bd=8\|il=0\|w=19\|h=9\|tr=none\|mode=rows\|x=time\|lvl=1\|strat=3\|filt=128\|n=2\|seed=17310` | fuzz write PALETTE/8-bit il=0 19x9 tr=[none] via rows chunks=[time] level=1 strategy=3 filters=0x80 | exit 0 | [x] |
| 1980 | `wr\|ct=4\|bd=8\|il=1\|w=10\|h=6\|tr=none\|mode=png\|x=none\|lvl=5\|strat=0\|filt=56\|n=2\|seed=17311` | fuzz write GRAY_ALPHA/8-bit il=1 10x6 tr=[none] via png chunks=[none] level=5 strategy=0 filters=0x38 | exit 0 | [x] |
| 1981 | `wr\|ct=4\|bd=16\|il=1\|w=28\|h=12\|tr=swapalpha\|mode=png\|x=text\|lvl=-1\|strat=4\|filt=16\|n=2\|seed=17312` | fuzz write GRAY_ALPHA/16-bit il=1 28x12 tr=[swapalpha] via png chunks=[text] level=-1 strategy=4 filters=0x10 | exit 0 | [x] |
| 1982 | `wr\|ct=6\|bd=8\|il=1\|w=26\|h=11\|tr=bgr\|mode=split\|x=srgb\|lvl=0\|strat=1\|filt=64\|n=2\|seed=17313` | fuzz write RGBA/8-bit il=1 26x11 tr=[bgr] via split chunks=[srgb] level=0 strategy=1 filters=0x40 | exit 0 | [x] |
| 1983 | `wr\|ct=6\|bd=16\|il=1\|w=19\|h=10\|tr=invalpha\|mode=rows\|x=gama\|lvl=1\|strat=3\|filt=0\|n=2\|seed=17314` | fuzz write RGBA/16-bit il=1 19x10 tr=[invalpha] via rows chunks=[gama] level=1 strategy=3 filters=0x00 | exit 0 | [x] |
| 1984 | `wr\|ct=0\|bd=1\|il=0\|w=19\|h=23\|tr=none\|mode=png\|x=bkgd\|lvl=5\|strat=4\|filt=56\|n=2\|seed=17315` | fuzz write GRAY/1-bit il=0 19x23 tr=[none] via png chunks=[bkgd] level=5 strategy=4 filters=0x38 | exit 0; 2 warning(s): Ignoring attempt to write bKGD chunk out-of-range for bit_depth | [x] |
| 1985 | `wr\|ct=0\|bd=2\|il=0\|w=25\|h=14\|tr=invmono\|mode=image\|x=text\|lvl=1\|strat=4\|filt=64\|n=2\|seed=17316` | fuzz write GRAY/2-bit il=0 25x14 tr=[invmono] via image chunks=[text] level=1 strategy=4 filters=0x40 | exit 0 | [x] |
| 1986 | `wr\|ct=0\|bd=4\|il=1\|w=22\|h=12\|tr=none\|mode=rows\|x=sbit\|lvl=0\|strat=2\|filt=0\|n=2\|seed=17317` | fuzz write GRAY/4-bit il=1 22x12 tr=[none] via rows chunks=[sbit] level=0 strategy=2 filters=0x00 | exit 0 | [x] |
| 1987 | `wr\|ct=0\|bd=8\|il=1\|w=2\|h=4\|tr=none\|mode=png\|x=chrm\|lvl=0\|strat=4\|filt=64\|n=2\|seed=17318` | fuzz write GRAY/8-bit il=1 2x4 tr=[none] via png chunks=[chrm] level=0 strategy=4 filters=0x40 | exit 0 | [x] |
| 1988 | `wr\|ct=0\|bd=16\|il=0\|w=11\|h=12\|tr=none\|mode=rows\|x=chrm\|lvl=9\|strat=3\|filt=8\|n=2\|seed=17319` | fuzz write GRAY/16-bit il=0 11x12 tr=[none] via rows chunks=[chrm] level=9 strategy=3 filters=0x08 | exit 0 | [x] |

## B18 — Large images (multi-buffer zlib, long-row filter selection)

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 1989 | `wr\|ct=2\|bd=8\|il=0\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 RGB/8-bit il=0 image | exit 0 | [x] |
| 1990 | `rd\|ct=2\|bd=8\|il=0\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 RGB/8-bit il=0 image | exit 0 | [x] |
| 1991 | `wr\|ct=2\|bd=8\|il=1\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 RGB/8-bit il=1 image | exit 0 | [x] |
| 1992 | `rd\|ct=2\|bd=8\|il=1\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 RGB/8-bit il=1 image | exit 0 | [x] |
| 1993 | `wr\|ct=6\|bd=16\|il=0\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 RGBA/16-bit il=0 image | exit 0 | [x] |
| 1994 | `rd\|ct=6\|bd=16\|il=0\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 RGBA/16-bit il=0 image | exit 0 | [x] |
| 1995 | `wr\|ct=6\|bd=16\|il=1\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 RGBA/16-bit il=1 image | exit 0 | [x] |
| 1996 | `rd\|ct=6\|bd=16\|il=1\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 RGBA/16-bit il=1 image | exit 0 | [x] |
| 1997 | `wr\|ct=0\|bd=1\|il=0\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 GRAY/1-bit il=0 image | exit 0 | [x] |
| 1998 | `rd\|ct=0\|bd=1\|il=0\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 GRAY/1-bit il=0 image | exit 0 | [x] |
| 1999 | `wr\|ct=0\|bd=1\|il=1\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 GRAY/1-bit il=1 image | exit 0 | [x] |
| 2000 | `rd\|ct=0\|bd=1\|il=1\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 GRAY/1-bit il=1 image | exit 0 | [x] |
| 2001 | `wr\|ct=3\|bd=8\|il=0\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 PALETTE/8-bit il=0 image | exit 0 | [x] |
| 2002 | `rd\|ct=3\|bd=8\|il=0\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 PALETTE/8-bit il=0 image | exit 0 | [x] |
| 2003 | `wr\|ct=3\|bd=8\|il=1\|w=257\|h=131\|filt=248\|n=1\|seed=18001` | write then read back a 257x131 PALETTE/8-bit il=1 image | exit 0 | [x] |
| 2004 | `rd\|ct=3\|bd=8\|il=1\|w=257\|h=131\|mode=image\|n=1\|seed=18002` | read a 257x131 PALETTE/8-bit il=1 image | exit 0 | [x] |
| 2005 | `rd\|ct=6\|bd=8\|w=1024\|h=8\|split=1\|mode=image\|n=1\|seed=18003` | read a 1024x8 RGBA image with a 1-byte IDAT split | exit 0 | [x] |
| 2006 | `prog\|ct=6\|bd=8\|w=512\|h=16\|feed=1\|seed=18004` | progressive read of a 512x16 RGBA image fed 1 byte at a time | exit 70; png_error: png_process_data_skip is not implemented in any current version of libpng | [x] |
| 2007 | `wr\|ct=6\|bd=16\|w=512\|h=64\|lvl=9\|filt=248\|n=1\|seed=18005` | write a 512x64 16-bit RGBA image at compression level 9 | exit 0 | [x] |
| 2008 | `sw\|fmt=3\|w=300\|h=200\|n=1\|seed=18006` | simplified round trip through a 300x200 RGBA image | exit 0 | [x] |

## B19 — User transform callbacks

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2009 | `ut\|side=read\|ct=0\|bd=1\|il=0\|w=19\|h=9\|seed=19001` | read GRAY/1-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2010 | `ut\|side=write\|ct=0\|bd=1\|il=0\|w=19\|h=9\|seed=19002` | write GRAY/1-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2011 | `ut\|side=read\|ct=0\|bd=1\|il=1\|w=19\|h=9\|seed=19001` | read GRAY/1-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2012 | `ut\|side=write\|ct=0\|bd=1\|il=1\|w=19\|h=9\|seed=19002` | write GRAY/1-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2013 | `ut\|side=read\|ct=0\|bd=2\|il=0\|w=19\|h=9\|seed=19001` | read GRAY/2-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2014 | `ut\|side=write\|ct=0\|bd=2\|il=0\|w=19\|h=9\|seed=19002` | write GRAY/2-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2015 | `ut\|side=read\|ct=0\|bd=2\|il=1\|w=19\|h=9\|seed=19001` | read GRAY/2-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2016 | `ut\|side=write\|ct=0\|bd=2\|il=1\|w=19\|h=9\|seed=19002` | write GRAY/2-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2017 | `ut\|side=read\|ct=0\|bd=4\|il=0\|w=19\|h=9\|seed=19001` | read GRAY/4-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2018 | `ut\|side=write\|ct=0\|bd=4\|il=0\|w=19\|h=9\|seed=19002` | write GRAY/4-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2019 | `ut\|side=read\|ct=0\|bd=4\|il=1\|w=19\|h=9\|seed=19001` | read GRAY/4-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2020 | `ut\|side=write\|ct=0\|bd=4\|il=1\|w=19\|h=9\|seed=19002` | write GRAY/4-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2021 | `ut\|side=read\|ct=0\|bd=8\|il=0\|w=19\|h=9\|seed=19001` | read GRAY/8-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2022 | `ut\|side=write\|ct=0\|bd=8\|il=0\|w=19\|h=9\|seed=19002` | write GRAY/8-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2023 | `ut\|side=read\|ct=0\|bd=8\|il=1\|w=19\|h=9\|seed=19001` | read GRAY/8-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2024 | `ut\|side=write\|ct=0\|bd=8\|il=1\|w=19\|h=9\|seed=19002` | write GRAY/8-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2025 | `ut\|side=read\|ct=0\|bd=16\|il=0\|w=19\|h=9\|seed=19001` | read GRAY/16-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2026 | `ut\|side=write\|ct=0\|bd=16\|il=0\|w=19\|h=9\|seed=19002` | write GRAY/16-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2027 | `ut\|side=read\|ct=0\|bd=16\|il=1\|w=19\|h=9\|seed=19001` | read GRAY/16-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2028 | `ut\|side=write\|ct=0\|bd=16\|il=1\|w=19\|h=9\|seed=19002` | write GRAY/16-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2029 | `ut\|side=read\|ct=2\|bd=8\|il=0\|w=19\|h=9\|seed=19001` | read RGB/8-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2030 | `ut\|side=write\|ct=2\|bd=8\|il=0\|w=19\|h=9\|seed=19002` | write RGB/8-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2031 | `ut\|side=read\|ct=2\|bd=8\|il=1\|w=19\|h=9\|seed=19001` | read RGB/8-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2032 | `ut\|side=write\|ct=2\|bd=8\|il=1\|w=19\|h=9\|seed=19002` | write RGB/8-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2033 | `ut\|side=read\|ct=2\|bd=16\|il=0\|w=19\|h=9\|seed=19001` | read RGB/16-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2034 | `ut\|side=write\|ct=2\|bd=16\|il=0\|w=19\|h=9\|seed=19002` | write RGB/16-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2035 | `ut\|side=read\|ct=2\|bd=16\|il=1\|w=19\|h=9\|seed=19001` | read RGB/16-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2036 | `ut\|side=write\|ct=2\|bd=16\|il=1\|w=19\|h=9\|seed=19002` | write RGB/16-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2037 | `ut\|side=read\|ct=3\|bd=1\|il=0\|w=19\|h=9\|seed=19001` | read PALETTE/1-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2038 | `ut\|side=write\|ct=3\|bd=1\|il=0\|w=19\|h=9\|seed=19002` | write PALETTE/1-bit il=0 through a png_set_write_user_transform_fn callback | exit 70; png_error: Invalid palette length | [x] |
| 2039 | `ut\|side=read\|ct=3\|bd=1\|il=1\|w=19\|h=9\|seed=19001` | read PALETTE/1-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2040 | `ut\|side=write\|ct=3\|bd=1\|il=1\|w=19\|h=9\|seed=19002` | write PALETTE/1-bit il=1 through a png_set_write_user_transform_fn callback | exit 70; png_error: Invalid palette length | [x] |
| 2041 | `ut\|side=read\|ct=3\|bd=2\|il=0\|w=19\|h=9\|seed=19001` | read PALETTE/2-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2042 | `ut\|side=write\|ct=3\|bd=2\|il=0\|w=19\|h=9\|seed=19002` | write PALETTE/2-bit il=0 through a png_set_write_user_transform_fn callback | exit 70; png_error: Invalid palette length | [x] |
| 2043 | `ut\|side=read\|ct=3\|bd=2\|il=1\|w=19\|h=9\|seed=19001` | read PALETTE/2-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2044 | `ut\|side=write\|ct=3\|bd=2\|il=1\|w=19\|h=9\|seed=19002` | write PALETTE/2-bit il=1 through a png_set_write_user_transform_fn callback | exit 70; png_error: Invalid palette length | [x] |
| 2045 | `ut\|side=read\|ct=3\|bd=4\|il=0\|w=19\|h=9\|seed=19001` | read PALETTE/4-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2046 | `ut\|side=write\|ct=3\|bd=4\|il=0\|w=19\|h=9\|seed=19002` | write PALETTE/4-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2047 | `ut\|side=read\|ct=3\|bd=4\|il=1\|w=19\|h=9\|seed=19001` | read PALETTE/4-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2048 | `ut\|side=write\|ct=3\|bd=4\|il=1\|w=19\|h=9\|seed=19002` | write PALETTE/4-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2049 | `ut\|side=read\|ct=3\|bd=8\|il=0\|w=19\|h=9\|seed=19001` | read PALETTE/8-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2050 | `ut\|side=write\|ct=3\|bd=8\|il=0\|w=19\|h=9\|seed=19002` | write PALETTE/8-bit il=0 through a png_set_write_user_transform_fn callback | exit 70; png_error: Wrote palette index exceeding num_palette | [x] |
| 2051 | `ut\|side=read\|ct=3\|bd=8\|il=1\|w=19\|h=9\|seed=19001` | read PALETTE/8-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2052 | `ut\|side=write\|ct=3\|bd=8\|il=1\|w=19\|h=9\|seed=19002` | write PALETTE/8-bit il=1 through a png_set_write_user_transform_fn callback | exit 70; png_error: Wrote palette index exceeding num_palette | [x] |
| 2053 | `ut\|side=read\|ct=4\|bd=8\|il=0\|w=19\|h=9\|seed=19001` | read GRAY_ALPHA/8-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2054 | `ut\|side=write\|ct=4\|bd=8\|il=0\|w=19\|h=9\|seed=19002` | write GRAY_ALPHA/8-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2055 | `ut\|side=read\|ct=4\|bd=8\|il=1\|w=19\|h=9\|seed=19001` | read GRAY_ALPHA/8-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2056 | `ut\|side=write\|ct=4\|bd=8\|il=1\|w=19\|h=9\|seed=19002` | write GRAY_ALPHA/8-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2057 | `ut\|side=read\|ct=4\|bd=16\|il=0\|w=19\|h=9\|seed=19001` | read GRAY_ALPHA/16-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2058 | `ut\|side=write\|ct=4\|bd=16\|il=0\|w=19\|h=9\|seed=19002` | write GRAY_ALPHA/16-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2059 | `ut\|side=read\|ct=4\|bd=16\|il=1\|w=19\|h=9\|seed=19001` | read GRAY_ALPHA/16-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2060 | `ut\|side=write\|ct=4\|bd=16\|il=1\|w=19\|h=9\|seed=19002` | write GRAY_ALPHA/16-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2061 | `ut\|side=read\|ct=6\|bd=8\|il=0\|w=19\|h=9\|seed=19001` | read RGBA/8-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2062 | `ut\|side=write\|ct=6\|bd=8\|il=0\|w=19\|h=9\|seed=19002` | write RGBA/8-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2063 | `ut\|side=read\|ct=6\|bd=8\|il=1\|w=19\|h=9\|seed=19001` | read RGBA/8-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2064 | `ut\|side=write\|ct=6\|bd=8\|il=1\|w=19\|h=9\|seed=19002` | write RGBA/8-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2065 | `ut\|side=read\|ct=6\|bd=16\|il=0\|w=19\|h=9\|seed=19001` | read RGBA/16-bit il=0 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2066 | `ut\|side=write\|ct=6\|bd=16\|il=0\|w=19\|h=9\|seed=19002` | write RGBA/16-bit il=0 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2067 | `ut\|side=read\|ct=6\|bd=16\|il=1\|w=19\|h=9\|seed=19001` | read RGBA/16-bit il=1 through a png_set_read_user_transform_fn callback | exit 0 | [x] |
| 2068 | `ut\|side=write\|ct=6\|bd=16\|il=1\|w=19\|h=9\|seed=19002` | write RGBA/16-bit il=1 through a png_set_write_user_transform_fn callback | exit 0 | [x] |
| 2069 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=expand\|seed=19003` | read user transform combined with expand | exit 0 | [x] |
| 2070 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=gray2rgb\|seed=19003` | read user transform combined with gray2rgb | exit 0 | [x] |
| 2071 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=strip16\|seed=19003` | read user transform combined with strip16 | exit 0 | [x] |
| 2072 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=bgr\|seed=19003` | read user transform combined with bgr | exit 0 | [x] |
| 2073 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=gamma\|seed=19003` | read user transform combined with gamma | exit 0 | [x] |
| 2074 | `ut\|side=read\|ct=6\|bd=16\|w=17\|h=7\|tr=expand16\|seed=19003` | read user transform combined with expand16 | exit 0 | [x] |
| 2075 | `ut\|side=read\|ct=2\|bd=8\|w=17\|h=7\|uti=1\|utd=8\|utc=3\|seed=19004` | read user transform with png_set_user_transform_info(depth=8, channels=3) | exit 0 | [x] |
| 2076 | `ut\|side=read\|ct=2\|bd=8\|w=17\|h=7\|uti=1\|utd=16\|utc=4\|seed=19004` | read user transform with png_set_user_transform_info(depth=16, channels=4) | exit 0 | [x] |
| 2077 | `ut\|side=read\|ct=2\|bd=8\|w=17\|h=7\|uti=1\|utd=1\|utc=1\|seed=19004` | read user transform with png_set_user_transform_info(depth=1, channels=1) | exit 0 | [x] |
| 2078 | `ut\|side=read\|ct=2\|bd=8\|w=17\|h=7\|uti=1\|utd=0\|utc=0\|seed=19004` | read user transform with png_set_user_transform_info(depth=0, channels=0) | exit 0 | [x] |

## B20 — MNG extensions and png_set_sig_bytes hand-over

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2079 | `mng\|f=filter64\|ct=2\|bd=8\|permit=0\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/8-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2080 | `mng\|f=write64\|ct=2\|bd=8\|permit=0\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/8-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2081 | `mng\|f=filter64\|ct=2\|bd=8\|permit=1\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/8-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2082 | `mng\|f=write64\|ct=2\|bd=8\|permit=1\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/8-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2083 | `mng\|f=filter64\|ct=2\|bd=8\|permit=4\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/8-bit with png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2084 | `mng\|f=write64\|ct=2\|bd=8\|permit=4\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/8-bit with png_permit_mng_features(4) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2085 | `mng\|f=filter64\|ct=2\|bd=8\|permit=5\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/8-bit with png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2086 | `mng\|f=write64\|ct=2\|bd=8\|permit=5\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/8-bit with png_permit_mng_features(5) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2087 | `mng\|f=filter64\|ct=2\|bd=16\|permit=0\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/16-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2088 | `mng\|f=write64\|ct=2\|bd=16\|permit=0\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/16-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2089 | `mng\|f=filter64\|ct=2\|bd=16\|permit=1\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/16-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2090 | `mng\|f=write64\|ct=2\|bd=16\|permit=1\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/16-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2091 | `mng\|f=filter64\|ct=2\|bd=16\|permit=4\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/16-bit with png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2092 | `mng\|f=write64\|ct=2\|bd=16\|permit=4\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/16-bit with png_permit_mng_features(4) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2093 | `mng\|f=filter64\|ct=2\|bd=16\|permit=5\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGB/16-bit with png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2094 | `mng\|f=write64\|ct=2\|bd=16\|permit=5\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGB/16-bit with png_permit_mng_features(5) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2095 | `mng\|f=filter64\|ct=6\|bd=8\|permit=0\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/8-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2096 | `mng\|f=write64\|ct=6\|bd=8\|permit=0\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/8-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2097 | `mng\|f=filter64\|ct=6\|bd=8\|permit=1\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/8-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2098 | `mng\|f=write64\|ct=6\|bd=8\|permit=1\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/8-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2099 | `mng\|f=filter64\|ct=6\|bd=8\|permit=4\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/8-bit with png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2100 | `mng\|f=write64\|ct=6\|bd=8\|permit=4\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/8-bit with png_permit_mng_features(4) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2101 | `mng\|f=filter64\|ct=6\|bd=8\|permit=5\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/8-bit with png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2102 | `mng\|f=write64\|ct=6\|bd=8\|permit=5\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/8-bit with png_permit_mng_features(5) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2103 | `mng\|f=filter64\|ct=6\|bd=16\|permit=0\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/16-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 2 warning(s): Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2104 | `mng\|f=write64\|ct=6\|bd=16\|permit=0\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/16-bit with png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2105 | `mng\|f=filter64\|ct=6\|bd=16\|permit=1\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/16-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2106 | `mng\|f=write64\|ct=6\|bd=16\|permit=1\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/16-bit with png_permit_mng_features(1) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2107 | `mng\|f=filter64\|ct=6\|bd=16\|permit=4\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/16-bit with png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2108 | `mng\|f=write64\|ct=6\|bd=16\|permit=4\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/16-bit with png_permit_mng_features(4) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2109 | `mng\|f=filter64\|ct=6\|bd=16\|permit=5\|w=13\|h=7\|seed=20001` | read MNG intrapixel (IHDR filter method 64) RGBA/16-bit with png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 3 warning(s): MNG features are not allowed in a PNG datastream / Unknown filter method in IHDR / Invalid filter method in IHDR | [x] |
| 2110 | `mng\|f=write64\|ct=6\|bd=16\|permit=5\|w=13\|h=7\|seed=20002` | write MNG intrapixel RGBA/16-bit with png_permit_mng_features(5) | exit 0; 2 warning(s): MNG features are not allowed in a PNG datastream / Invalid filter type specified | [x] |
| 2111 | `mng\|f=emptyplte\|permit=0\|seed=20003` | read a palette image with an empty PLTE, png_permit_mng_features(0) | exit 70; png_error: Invalid palette | [x] |
| 2112 | `mng\|f=emptyplte\|permit=1\|seed=20003` | read a palette image with an empty PLTE, png_permit_mng_features(1) | exit 0; 3 warning(s): MNG features are not allowed in a PNG datastream / IDAT: Read palette index exceeding num_palette | [x] |
| 2113 | `mng\|f=emptyplte\|permit=4\|seed=20003` | read a palette image with an empty PLTE, png_permit_mng_features(4) | exit 70; png_error: Invalid palette; 1 warning(s): MNG features are not allowed in a PNG datastream | [x] |
| 2114 | `mng\|f=emptyplte\|permit=5\|seed=20003` | read a palette image with an empty PLTE, png_permit_mng_features(5) | exit 0; 3 warning(s): MNG features are not allowed in a PNG datastream / IDAT: Read palette index exceeding num_palette | [x] |
| 2115 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2116 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2117 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2118 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 0 | [x] |
| 2119 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 0 | [x] |
| 2120 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 0 | [x] |
| 2121 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 0 | [x] |
| 2122 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 0 | [x] |
| 2123 | `mng\|f=filter64sig\|ct=2\|bd=8\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 0 | [x] |
| 2124 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2125 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2126 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2127 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 0 | [x] |
| 2128 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 0 | [x] |
| 2129 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 0 | [x] |
| 2130 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 0 | [x] |
| 2131 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 0 | [x] |
| 2132 | `mng\|f=filter64sig\|ct=2\|bd=16\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGB/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 0 | [x] |
| 2133 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2134 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2135 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2136 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 0 | [x] |
| 2137 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 0 | [x] |
| 2138 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 0 | [x] |
| 2139 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 0 | [x] |
| 2140 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 0 | [x] |
| 2141 | `mng\|f=filter64sig\|ct=6\|bd=8\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 0 | [x] |
| 2142 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2143 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2144 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2145 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 0 | [x] |
| 2146 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 0 | [x] |
| 2147 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 0 | [x] |
| 2148 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 0 | [x] |
| 2149 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 0 | [x] |
| 2150 | `mng\|f=filter64sig\|ct=6\|bd=16\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of RGBA/16-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 0 | [x] |
| 2151 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2152 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2153 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2154 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2155 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2156 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2157 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2158 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2159 | `mng\|f=filter64sig\|ct=0\|bd=8\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of GRAY/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2160 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=0\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2161 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=0\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2162 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=0\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(0) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2163 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=4\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2164 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=4\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2165 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=4\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(4) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2166 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=5\|skip=3\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(3) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2167 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=5\|skip=4\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(4) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2168 | `mng\|f=filter64sig\|ct=3\|bd=8\|permit=5\|skip=8\|w=13\|h=7\|seed=20004` | MNG intrapixel read of PALETTE/8-bit with png_set_sig_bytes(8) and png_permit_mng_features(5) | exit 70; png_error: Invalid IHDR data; 1 warning(s): Unknown filter method in IHDR | [x] |
| 2169 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=0\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(0) | exit 0 | [x] |
| 2170 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=1\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(1) | exit 0 | [x] |
| 2171 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=2\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(2) | exit 0 | [x] |
| 2172 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=3\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(3) | exit 0 | [x] |
| 2173 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=4\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(4) | exit 0 | [x] |
| 2174 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=5\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(5) | exit 0 | [x] |
| 2175 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=6\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(6) | exit 0 | [x] |
| 2176 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=7\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(7) | exit 0 | [x] |
| 2177 | `mng\|f=sigbytes\|ct=2\|bd=8\|skip=8\|w=11\|h=5\|seed=20005` | read a normal stream handed over with png_set_sig_bytes(8) | exit 0 | [x] |

## B21 — CRC errors x png_set_crc_action

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2178 | `crc\|chunk=IHDR\|crit=0\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=0, ancil=0) | exit 70; png_error: IHDR: CRC error | [x] |
| 2179 | `crc\|chunk=IHDR\|crit=0\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=0, ancil=2) | exit 70; png_error: IHDR: CRC error | [x] |
| 2180 | `crc\|chunk=IHDR\|crit=0\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=0, ancil=3) | exit 70; png_error: IHDR: CRC error | [x] |
| 2181 | `crc\|chunk=IHDR\|crit=0\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=0, ancil=4) | exit 70; png_error: IHDR: CRC error | [x] |
| 2182 | `crc\|chunk=IHDR\|crit=0\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=0, ancil=5) | exit 70; png_error: IHDR: CRC error | [x] |
| 2183 | `crc\|chunk=IHDR\|crit=1\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=1, ancil=0) | exit 70; png_error: IHDR: CRC error | [x] |
| 2184 | `crc\|chunk=IHDR\|crit=1\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=1, ancil=2) | exit 70; png_error: IHDR: CRC error | [x] |
| 2185 | `crc\|chunk=IHDR\|crit=1\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=1, ancil=3) | exit 70; png_error: IHDR: CRC error | [x] |
| 2186 | `crc\|chunk=IHDR\|crit=1\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=1, ancil=4) | exit 70; png_error: IHDR: CRC error | [x] |
| 2187 | `crc\|chunk=IHDR\|crit=1\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=1, ancil=5) | exit 70; png_error: IHDR: CRC error | [x] |
| 2188 | `crc\|chunk=IHDR\|crit=2\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=2, ancil=0) | exit 70; png_error: IHDR: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2189 | `crc\|chunk=IHDR\|crit=2\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=2, ancil=2) | exit 70; png_error: IHDR: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2190 | `crc\|chunk=IHDR\|crit=2\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=2, ancil=3) | exit 70; png_error: IHDR: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2191 | `crc\|chunk=IHDR\|crit=2\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=2, ancil=4) | exit 70; png_error: IHDR: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2192 | `crc\|chunk=IHDR\|crit=2\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=2, ancil=5) | exit 70; png_error: IHDR: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2193 | `crc\|chunk=IHDR\|crit=3\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=3, ancil=0) | exit 0; 1 warning(s): IHDR: CRC error | [x] |
| 2194 | `crc\|chunk=IHDR\|crit=3\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=3, ancil=2) | exit 0; 1 warning(s): IHDR: CRC error | [x] |
| 2195 | `crc\|chunk=IHDR\|crit=3\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=3, ancil=3) | exit 0; 1 warning(s): IHDR: CRC error | [x] |
| 2196 | `crc\|chunk=IHDR\|crit=3\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=3, ancil=4) | exit 0; 1 warning(s): IHDR: CRC error | [x] |
| 2197 | `crc\|chunk=IHDR\|crit=3\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=3, ancil=5) | exit 0; 1 warning(s): IHDR: CRC error | [x] |
| 2198 | `crc\|chunk=IHDR\|crit=4\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=4, ancil=0) | exit 0 | [x] |
| 2199 | `crc\|chunk=IHDR\|crit=4\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=4, ancil=2) | exit 0 | [x] |
| 2200 | `crc\|chunk=IHDR\|crit=4\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=4, ancil=3) | exit 0 | [x] |
| 2201 | `crc\|chunk=IHDR\|crit=4\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2202 | `crc\|chunk=IHDR\|crit=4\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=4, ancil=5) | exit 0 | [x] |
| 2203 | `crc\|chunk=IHDR\|crit=5\|anc=0` | CRC error in IHDR with png_set_crc_action(crit=5, ancil=0) | exit 70; png_error: IHDR: CRC error | [x] |
| 2204 | `crc\|chunk=IHDR\|crit=5\|anc=2` | CRC error in IHDR with png_set_crc_action(crit=5, ancil=2) | exit 70; png_error: IHDR: CRC error | [x] |
| 2205 | `crc\|chunk=IHDR\|crit=5\|anc=3` | CRC error in IHDR with png_set_crc_action(crit=5, ancil=3) | exit 70; png_error: IHDR: CRC error | [x] |
| 2206 | `crc\|chunk=IHDR\|crit=5\|anc=4` | CRC error in IHDR with png_set_crc_action(crit=5, ancil=4) | exit 70; png_error: IHDR: CRC error | [x] |
| 2207 | `crc\|chunk=IHDR\|crit=5\|anc=5` | CRC error in IHDR with png_set_crc_action(crit=5, ancil=5) | exit 70; png_error: IHDR: CRC error | [x] |
| 2208 | `crc\|chunk=gAMA\|crit=0\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=0, ancil=0) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2209 | `crc\|chunk=gAMA\|crit=0\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=0, ancil=2) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2210 | `crc\|chunk=gAMA\|crit=0\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=0, ancil=3) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2211 | `crc\|chunk=gAMA\|crit=0\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=0, ancil=4) | exit 0 | [x] |
| 2212 | `crc\|chunk=gAMA\|crit=0\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=0, ancil=5) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2213 | `crc\|chunk=gAMA\|crit=1\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=1, ancil=0) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2214 | `crc\|chunk=gAMA\|crit=1\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=1, ancil=2) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2215 | `crc\|chunk=gAMA\|crit=1\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=1, ancil=3) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2216 | `crc\|chunk=gAMA\|crit=1\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=1, ancil=4) | exit 0 | [x] |
| 2217 | `crc\|chunk=gAMA\|crit=1\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=1, ancil=5) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2218 | `crc\|chunk=gAMA\|crit=2\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=2, ancil=0) | exit 0; 2 warning(s): Can't discard critical data on CRC error / gAMA: CRC error | [x] |
| 2219 | `crc\|chunk=gAMA\|crit=2\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=2, ancil=2) | exit 0; 2 warning(s): Can't discard critical data on CRC error / gAMA: CRC error | [x] |
| 2220 | `crc\|chunk=gAMA\|crit=2\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=2, ancil=3) | exit 0; 2 warning(s): Can't discard critical data on CRC error / gAMA: CRC error | [x] |
| 2221 | `crc\|chunk=gAMA\|crit=2\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=2, ancil=4) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2222 | `crc\|chunk=gAMA\|crit=2\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=2, ancil=5) | exit 0; 2 warning(s): Can't discard critical data on CRC error / gAMA: CRC error | [x] |
| 2223 | `crc\|chunk=gAMA\|crit=3\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=3, ancil=0) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2224 | `crc\|chunk=gAMA\|crit=3\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=3, ancil=2) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2225 | `crc\|chunk=gAMA\|crit=3\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=3, ancil=3) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2226 | `crc\|chunk=gAMA\|crit=3\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=3, ancil=4) | exit 0 | [x] |
| 2227 | `crc\|chunk=gAMA\|crit=3\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=3, ancil=5) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2228 | `crc\|chunk=gAMA\|crit=4\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=4, ancil=0) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2229 | `crc\|chunk=gAMA\|crit=4\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=4, ancil=2) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2230 | `crc\|chunk=gAMA\|crit=4\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=4, ancil=3) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2231 | `crc\|chunk=gAMA\|crit=4\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2232 | `crc\|chunk=gAMA\|crit=4\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=4, ancil=5) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2233 | `crc\|chunk=gAMA\|crit=5\|anc=0` | CRC error in gAMA with png_set_crc_action(crit=5, ancil=0) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2234 | `crc\|chunk=gAMA\|crit=5\|anc=2` | CRC error in gAMA with png_set_crc_action(crit=5, ancil=2) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2235 | `crc\|chunk=gAMA\|crit=5\|anc=3` | CRC error in gAMA with png_set_crc_action(crit=5, ancil=3) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2236 | `crc\|chunk=gAMA\|crit=5\|anc=4` | CRC error in gAMA with png_set_crc_action(crit=5, ancil=4) | exit 0 | [x] |
| 2237 | `crc\|chunk=gAMA\|crit=5\|anc=5` | CRC error in gAMA with png_set_crc_action(crit=5, ancil=5) | exit 0; 1 warning(s): gAMA: CRC error | [x] |
| 2238 | `crc\|chunk=IDAT\|crit=0\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=0, ancil=0) | exit 70; png_error: IDAT: CRC error | [x] |
| 2239 | `crc\|chunk=IDAT\|crit=0\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=0, ancil=2) | exit 70; png_error: IDAT: CRC error | [x] |
| 2240 | `crc\|chunk=IDAT\|crit=0\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=0, ancil=3) | exit 70; png_error: IDAT: CRC error | [x] |
| 2241 | `crc\|chunk=IDAT\|crit=0\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=0, ancil=4) | exit 70; png_error: IDAT: CRC error | [x] |
| 2242 | `crc\|chunk=IDAT\|crit=0\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=0, ancil=5) | exit 70; png_error: IDAT: CRC error | [x] |
| 2243 | `crc\|chunk=IDAT\|crit=1\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=1, ancil=0) | exit 70; png_error: IDAT: CRC error | [x] |
| 2244 | `crc\|chunk=IDAT\|crit=1\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=1, ancil=2) | exit 70; png_error: IDAT: CRC error | [x] |
| 2245 | `crc\|chunk=IDAT\|crit=1\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=1, ancil=3) | exit 70; png_error: IDAT: CRC error | [x] |
| 2246 | `crc\|chunk=IDAT\|crit=1\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=1, ancil=4) | exit 70; png_error: IDAT: CRC error | [x] |
| 2247 | `crc\|chunk=IDAT\|crit=1\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=1, ancil=5) | exit 70; png_error: IDAT: CRC error | [x] |
| 2248 | `crc\|chunk=IDAT\|crit=2\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=2, ancil=0) | exit 70; png_error: IDAT: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2249 | `crc\|chunk=IDAT\|crit=2\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=2, ancil=2) | exit 70; png_error: IDAT: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2250 | `crc\|chunk=IDAT\|crit=2\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=2, ancil=3) | exit 70; png_error: IDAT: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2251 | `crc\|chunk=IDAT\|crit=2\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=2, ancil=4) | exit 70; png_error: IDAT: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2252 | `crc\|chunk=IDAT\|crit=2\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=2, ancil=5) | exit 70; png_error: IDAT: CRC error; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2253 | `crc\|chunk=IDAT\|crit=3\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=3, ancil=0) | exit 0; 1 warning(s): IDAT: CRC error | [x] |
| 2254 | `crc\|chunk=IDAT\|crit=3\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=3, ancil=2) | exit 0; 1 warning(s): IDAT: CRC error | [x] |
| 2255 | `crc\|chunk=IDAT\|crit=3\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=3, ancil=3) | exit 0; 1 warning(s): IDAT: CRC error | [x] |
| 2256 | `crc\|chunk=IDAT\|crit=3\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=3, ancil=4) | exit 0; 1 warning(s): IDAT: CRC error | [x] |
| 2257 | `crc\|chunk=IDAT\|crit=3\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=3, ancil=5) | exit 0; 1 warning(s): IDAT: CRC error | [x] |
| 2258 | `crc\|chunk=IDAT\|crit=4\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=4, ancil=0) | exit 0 | [x] |
| 2259 | `crc\|chunk=IDAT\|crit=4\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=4, ancil=2) | exit 0 | [x] |
| 2260 | `crc\|chunk=IDAT\|crit=4\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=4, ancil=3) | exit 0 | [x] |
| 2261 | `crc\|chunk=IDAT\|crit=4\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2262 | `crc\|chunk=IDAT\|crit=4\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=4, ancil=5) | exit 0 | [x] |
| 2263 | `crc\|chunk=IDAT\|crit=5\|anc=0` | CRC error in IDAT with png_set_crc_action(crit=5, ancil=0) | exit 70; png_error: IDAT: CRC error | [x] |
| 2264 | `crc\|chunk=IDAT\|crit=5\|anc=2` | CRC error in IDAT with png_set_crc_action(crit=5, ancil=2) | exit 70; png_error: IDAT: CRC error | [x] |
| 2265 | `crc\|chunk=IDAT\|crit=5\|anc=3` | CRC error in IDAT with png_set_crc_action(crit=5, ancil=3) | exit 70; png_error: IDAT: CRC error | [x] |
| 2266 | `crc\|chunk=IDAT\|crit=5\|anc=4` | CRC error in IDAT with png_set_crc_action(crit=5, ancil=4) | exit 70; png_error: IDAT: CRC error | [x] |
| 2267 | `crc\|chunk=IDAT\|crit=5\|anc=5` | CRC error in IDAT with png_set_crc_action(crit=5, ancil=5) | exit 70; png_error: IDAT: CRC error | [x] |
| 2268 | `crc\|chunk=tEXt\|crit=0\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=0, ancil=0) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2269 | `crc\|chunk=tEXt\|crit=0\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=0, ancil=2) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2270 | `crc\|chunk=tEXt\|crit=0\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=0, ancil=3) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2271 | `crc\|chunk=tEXt\|crit=0\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=0, ancil=4) | exit 0 | [x] |
| 2272 | `crc\|chunk=tEXt\|crit=0\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=0, ancil=5) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2273 | `crc\|chunk=tEXt\|crit=1\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=1, ancil=0) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2274 | `crc\|chunk=tEXt\|crit=1\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=1, ancil=2) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2275 | `crc\|chunk=tEXt\|crit=1\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=1, ancil=3) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2276 | `crc\|chunk=tEXt\|crit=1\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=1, ancil=4) | exit 0 | [x] |
| 2277 | `crc\|chunk=tEXt\|crit=1\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=1, ancil=5) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2278 | `crc\|chunk=tEXt\|crit=2\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=2, ancil=0) | exit 0; 2 warning(s): Can't discard critical data on CRC error / tEXt: CRC error | [x] |
| 2279 | `crc\|chunk=tEXt\|crit=2\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=2, ancil=2) | exit 0; 2 warning(s): Can't discard critical data on CRC error / tEXt: CRC error | [x] |
| 2280 | `crc\|chunk=tEXt\|crit=2\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=2, ancil=3) | exit 0; 2 warning(s): Can't discard critical data on CRC error / tEXt: CRC error | [x] |
| 2281 | `crc\|chunk=tEXt\|crit=2\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=2, ancil=4) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2282 | `crc\|chunk=tEXt\|crit=2\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=2, ancil=5) | exit 0; 2 warning(s): Can't discard critical data on CRC error / tEXt: CRC error | [x] |
| 2283 | `crc\|chunk=tEXt\|crit=3\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=3, ancil=0) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2284 | `crc\|chunk=tEXt\|crit=3\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=3, ancil=2) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2285 | `crc\|chunk=tEXt\|crit=3\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=3, ancil=3) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2286 | `crc\|chunk=tEXt\|crit=3\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=3, ancil=4) | exit 0 | [x] |
| 2287 | `crc\|chunk=tEXt\|crit=3\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=3, ancil=5) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2288 | `crc\|chunk=tEXt\|crit=4\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=4, ancil=0) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2289 | `crc\|chunk=tEXt\|crit=4\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=4, ancil=2) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2290 | `crc\|chunk=tEXt\|crit=4\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=4, ancil=3) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2291 | `crc\|chunk=tEXt\|crit=4\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2292 | `crc\|chunk=tEXt\|crit=4\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=4, ancil=5) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2293 | `crc\|chunk=tEXt\|crit=5\|anc=0` | CRC error in tEXt with png_set_crc_action(crit=5, ancil=0) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2294 | `crc\|chunk=tEXt\|crit=5\|anc=2` | CRC error in tEXt with png_set_crc_action(crit=5, ancil=2) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2295 | `crc\|chunk=tEXt\|crit=5\|anc=3` | CRC error in tEXt with png_set_crc_action(crit=5, ancil=3) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2296 | `crc\|chunk=tEXt\|crit=5\|anc=4` | CRC error in tEXt with png_set_crc_action(crit=5, ancil=4) | exit 0 | [x] |
| 2297 | `crc\|chunk=tEXt\|crit=5\|anc=5` | CRC error in tEXt with png_set_crc_action(crit=5, ancil=5) | exit 0; 1 warning(s): tEXt: CRC error | [x] |
| 2298 | `crc\|chunk=IEND\|crit=0\|anc=0` | CRC error in IEND with png_set_crc_action(crit=0, ancil=0) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2299 | `crc\|chunk=IEND\|crit=0\|anc=2` | CRC error in IEND with png_set_crc_action(crit=0, ancil=2) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2300 | `crc\|chunk=IEND\|crit=0\|anc=3` | CRC error in IEND with png_set_crc_action(crit=0, ancil=3) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2301 | `crc\|chunk=IEND\|crit=0\|anc=4` | CRC error in IEND with png_set_crc_action(crit=0, ancil=4) | exit 0 | [x] |
| 2302 | `crc\|chunk=IEND\|crit=0\|anc=5` | CRC error in IEND with png_set_crc_action(crit=0, ancil=5) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2303 | `crc\|chunk=IEND\|crit=1\|anc=0` | CRC error in IEND with png_set_crc_action(crit=1, ancil=0) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2304 | `crc\|chunk=IEND\|crit=1\|anc=2` | CRC error in IEND with png_set_crc_action(crit=1, ancil=2) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2305 | `crc\|chunk=IEND\|crit=1\|anc=3` | CRC error in IEND with png_set_crc_action(crit=1, ancil=3) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2306 | `crc\|chunk=IEND\|crit=1\|anc=4` | CRC error in IEND with png_set_crc_action(crit=1, ancil=4) | exit 0 | [x] |
| 2307 | `crc\|chunk=IEND\|crit=1\|anc=5` | CRC error in IEND with png_set_crc_action(crit=1, ancil=5) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2308 | `crc\|chunk=IEND\|crit=2\|anc=0` | CRC error in IEND with png_set_crc_action(crit=2, ancil=0) | exit 0; 2 warning(s): Can't discard critical data on CRC error / IEND: CRC error | [x] |
| 2309 | `crc\|chunk=IEND\|crit=2\|anc=2` | CRC error in IEND with png_set_crc_action(crit=2, ancil=2) | exit 0; 2 warning(s): Can't discard critical data on CRC error / IEND: CRC error | [x] |
| 2310 | `crc\|chunk=IEND\|crit=2\|anc=3` | CRC error in IEND with png_set_crc_action(crit=2, ancil=3) | exit 0; 2 warning(s): Can't discard critical data on CRC error / IEND: CRC error | [x] |
| 2311 | `crc\|chunk=IEND\|crit=2\|anc=4` | CRC error in IEND with png_set_crc_action(crit=2, ancil=4) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2312 | `crc\|chunk=IEND\|crit=2\|anc=5` | CRC error in IEND with png_set_crc_action(crit=2, ancil=5) | exit 0; 2 warning(s): Can't discard critical data on CRC error / IEND: CRC error | [x] |
| 2313 | `crc\|chunk=IEND\|crit=3\|anc=0` | CRC error in IEND with png_set_crc_action(crit=3, ancil=0) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2314 | `crc\|chunk=IEND\|crit=3\|anc=2` | CRC error in IEND with png_set_crc_action(crit=3, ancil=2) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2315 | `crc\|chunk=IEND\|crit=3\|anc=3` | CRC error in IEND with png_set_crc_action(crit=3, ancil=3) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2316 | `crc\|chunk=IEND\|crit=3\|anc=4` | CRC error in IEND with png_set_crc_action(crit=3, ancil=4) | exit 0 | [x] |
| 2317 | `crc\|chunk=IEND\|crit=3\|anc=5` | CRC error in IEND with png_set_crc_action(crit=3, ancil=5) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2318 | `crc\|chunk=IEND\|crit=4\|anc=0` | CRC error in IEND with png_set_crc_action(crit=4, ancil=0) | exit 0 | [x] |
| 2319 | `crc\|chunk=IEND\|crit=4\|anc=2` | CRC error in IEND with png_set_crc_action(crit=4, ancil=2) | exit 0 | [x] |
| 2320 | `crc\|chunk=IEND\|crit=4\|anc=3` | CRC error in IEND with png_set_crc_action(crit=4, ancil=3) | exit 0 | [x] |
| 2321 | `crc\|chunk=IEND\|crit=4\|anc=4` | CRC error in IEND with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2322 | `crc\|chunk=IEND\|crit=4\|anc=5` | CRC error in IEND with png_set_crc_action(crit=4, ancil=5) | exit 0 | [x] |
| 2323 | `crc\|chunk=IEND\|crit=5\|anc=0` | CRC error in IEND with png_set_crc_action(crit=5, ancil=0) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2324 | `crc\|chunk=IEND\|crit=5\|anc=2` | CRC error in IEND with png_set_crc_action(crit=5, ancil=2) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2325 | `crc\|chunk=IEND\|crit=5\|anc=3` | CRC error in IEND with png_set_crc_action(crit=5, ancil=3) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2326 | `crc\|chunk=IEND\|crit=5\|anc=4` | CRC error in IEND with png_set_crc_action(crit=5, ancil=4) | exit 0 | [x] |
| 2327 | `crc\|chunk=IEND\|crit=5\|anc=5` | CRC error in IEND with png_set_crc_action(crit=5, ancil=5) | exit 0; 1 warning(s): IEND: CRC error | [x] |
| 2328 | `crc\|chunk=none\|crit=0\|anc=0` | CRC error in none with png_set_crc_action(crit=0, ancil=0) | exit 0 | [x] |
| 2329 | `crc\|chunk=none\|crit=0\|anc=2` | CRC error in none with png_set_crc_action(crit=0, ancil=2) | exit 0 | [x] |
| 2330 | `crc\|chunk=none\|crit=0\|anc=3` | CRC error in none with png_set_crc_action(crit=0, ancil=3) | exit 0 | [x] |
| 2331 | `crc\|chunk=none\|crit=0\|anc=4` | CRC error in none with png_set_crc_action(crit=0, ancil=4) | exit 0 | [x] |
| 2332 | `crc\|chunk=none\|crit=0\|anc=5` | CRC error in none with png_set_crc_action(crit=0, ancil=5) | exit 0 | [x] |
| 2333 | `crc\|chunk=none\|crit=1\|anc=0` | CRC error in none with png_set_crc_action(crit=1, ancil=0) | exit 0 | [x] |
| 2334 | `crc\|chunk=none\|crit=1\|anc=2` | CRC error in none with png_set_crc_action(crit=1, ancil=2) | exit 0 | [x] |
| 2335 | `crc\|chunk=none\|crit=1\|anc=3` | CRC error in none with png_set_crc_action(crit=1, ancil=3) | exit 0 | [x] |
| 2336 | `crc\|chunk=none\|crit=1\|anc=4` | CRC error in none with png_set_crc_action(crit=1, ancil=4) | exit 0 | [x] |
| 2337 | `crc\|chunk=none\|crit=1\|anc=5` | CRC error in none with png_set_crc_action(crit=1, ancil=5) | exit 0 | [x] |
| 2338 | `crc\|chunk=none\|crit=2\|anc=0` | CRC error in none with png_set_crc_action(crit=2, ancil=0) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2339 | `crc\|chunk=none\|crit=2\|anc=2` | CRC error in none with png_set_crc_action(crit=2, ancil=2) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2340 | `crc\|chunk=none\|crit=2\|anc=3` | CRC error in none with png_set_crc_action(crit=2, ancil=3) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2341 | `crc\|chunk=none\|crit=2\|anc=4` | CRC error in none with png_set_crc_action(crit=2, ancil=4) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2342 | `crc\|chunk=none\|crit=2\|anc=5` | CRC error in none with png_set_crc_action(crit=2, ancil=5) | exit 0; 1 warning(s): Can't discard critical data on CRC error | [x] |
| 2343 | `crc\|chunk=none\|crit=3\|anc=0` | CRC error in none with png_set_crc_action(crit=3, ancil=0) | exit 0 | [x] |
| 2344 | `crc\|chunk=none\|crit=3\|anc=2` | CRC error in none with png_set_crc_action(crit=3, ancil=2) | exit 0 | [x] |
| 2345 | `crc\|chunk=none\|crit=3\|anc=3` | CRC error in none with png_set_crc_action(crit=3, ancil=3) | exit 0 | [x] |
| 2346 | `crc\|chunk=none\|crit=3\|anc=4` | CRC error in none with png_set_crc_action(crit=3, ancil=4) | exit 0 | [x] |
| 2347 | `crc\|chunk=none\|crit=3\|anc=5` | CRC error in none with png_set_crc_action(crit=3, ancil=5) | exit 0 | [x] |
| 2348 | `crc\|chunk=none\|crit=4\|anc=0` | CRC error in none with png_set_crc_action(crit=4, ancil=0) | exit 0 | [x] |
| 2349 | `crc\|chunk=none\|crit=4\|anc=2` | CRC error in none with png_set_crc_action(crit=4, ancil=2) | exit 0 | [x] |
| 2350 | `crc\|chunk=none\|crit=4\|anc=3` | CRC error in none with png_set_crc_action(crit=4, ancil=3) | exit 0 | [x] |
| 2351 | `crc\|chunk=none\|crit=4\|anc=4` | CRC error in none with png_set_crc_action(crit=4, ancil=4) | exit 0 | [x] |
| 2352 | `crc\|chunk=none\|crit=4\|anc=5` | CRC error in none with png_set_crc_action(crit=4, ancil=5) | exit 0 | [x] |
| 2353 | `crc\|chunk=none\|crit=5\|anc=0` | CRC error in none with png_set_crc_action(crit=5, ancil=0) | exit 0 | [x] |
| 2354 | `crc\|chunk=none\|crit=5\|anc=2` | CRC error in none with png_set_crc_action(crit=5, ancil=2) | exit 0 | [x] |
| 2355 | `crc\|chunk=none\|crit=5\|anc=3` | CRC error in none with png_set_crc_action(crit=5, ancil=3) | exit 0 | [x] |
| 2356 | `crc\|chunk=none\|crit=5\|anc=4` | CRC error in none with png_set_crc_action(crit=5, ancil=4) | exit 0 | [x] |
| 2357 | `crc\|chunk=none\|crit=5\|anc=5` | CRC error in none with png_set_crc_action(crit=5, ancil=5) | exit 0 | [x] |

## B22 — Floating-point getters

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2358 | `fpget\|seed=1` | floating-point getters (cHRM/cHRM_XYZ/cLLI/mDCV/sCAL/aspect/offset/gAMA) over randomized fixed-point inputs, seed 1 | exit 0 | [x] |
| 2359 | `fpget\|seed=2` | floating-point getters (cHRM/cHRM_XYZ/cLLI/mDCV/sCAL/aspect/offset/gAMA) over randomized fixed-point inputs, seed 2 | exit 0 | [x] |
| 2360 | `fpget\|seed=3` | floating-point getters (cHRM/cHRM_XYZ/cLLI/mDCV/sCAL/aspect/offset/gAMA) over randomized fixed-point inputs, seed 3 | exit 0 | [x] |

## B23 — stdio-based entry points (png_init_io, *_from_file, *_to_stdio)

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2361 | `fileio\|m=lowlevel\|ct=0\|bd=1\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for GRAY/1-bit | exit 0 | [x] |
| 2362 | `fileio\|m=lowlevel\|ct=0\|bd=8\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for GRAY/8-bit | exit 0 | [x] |
| 2363 | `fileio\|m=lowlevel\|ct=2\|bd=8\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for RGB/8-bit | exit 0 | [x] |
| 2364 | `fileio\|m=lowlevel\|ct=3\|bd=8\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for PALETTE/8-bit | exit 0 | [x] |
| 2365 | `fileio\|m=lowlevel\|ct=4\|bd=16\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for GRAY_ALPHA/16-bit | exit 0 | [x] |
| 2366 | `fileio\|m=lowlevel\|ct=6\|bd=8\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for RGBA/8-bit | exit 0 | [x] |
| 2367 | `fileio\|m=lowlevel\|ct=6\|bd=16\|w=15\|h=9\|seed=23001` | png_init_io round trip through a real FILE* for RGBA/16-bit | exit 0 | [x] |
| 2368 | `fileio\|m=simple\|fmt=0\|c8=0\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_GRAY, convert_to_8bit=0 | exit 0 | [x] |
| 2369 | `fileio\|m=simple\|fmt=0\|c8=1\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_GRAY, convert_to_8bit=1 | exit 0 | [x] |
| 2370 | `fileio\|m=stdio\|fmt=0\|w=15\|h=9\|seed=23003` | png_image_write_to_stdio / begin_read_from_stdio with PNG_FORMAT_GRAY | exit 0 | [x] |
| 2371 | `fileio\|m=simple\|fmt=2\|c8=0\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_RGB, convert_to_8bit=0 | exit 0 | [x] |
| 2372 | `fileio\|m=simple\|fmt=2\|c8=1\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_RGB, convert_to_8bit=1 | exit 0 | [x] |
| 2373 | `fileio\|m=stdio\|fmt=2\|w=15\|h=9\|seed=23003` | png_image_write_to_stdio / begin_read_from_stdio with PNG_FORMAT_RGB | exit 0 | [x] |
| 2374 | `fileio\|m=simple\|fmt=3\|c8=0\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_RGBA, convert_to_8bit=0 | exit 0 | [x] |
| 2375 | `fileio\|m=simple\|fmt=3\|c8=1\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_RGBA, convert_to_8bit=1 | exit 0 | [x] |
| 2376 | `fileio\|m=stdio\|fmt=3\|w=15\|h=9\|seed=23003` | png_image_write_to_stdio / begin_read_from_stdio with PNG_FORMAT_RGBA | exit 0 | [x] |
| 2377 | `fileio\|m=simple\|fmt=7\|c8=0\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_LINEAR_RGB_ALPHA, convert_to_8bit=0 | exit 0 | [x] |
| 2378 | `fileio\|m=simple\|fmt=7\|c8=1\|w=15\|h=9\|seed=23002` | png_image_write_to_file / begin_read_from_file with PNG_FORMAT_LINEAR_RGB_ALPHA, convert_to_8bit=1 | exit 0 | [x] |
| 2379 | `fileio\|m=stdio\|fmt=7\|w=15\|h=9\|seed=23003` | png_image_write_to_stdio / begin_read_from_stdio with PNG_FORMAT_LINEAR_RGB_ALPHA | exit 0 | [x] |

## B24 — png_free_data / png_data_freer / png_destroy_info_struct

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2380 | `freedata\|mask=65535` | png_free_data(PNG_FREE_ALL) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2381 | `freedata\|mask=8` | png_free_data(PNG_FREE_HIST) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2382 | `freedata\|mask=16` | png_free_data(PNG_FREE_ICCP) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2383 | `freedata\|mask=32` | png_free_data(PNG_FREE_SPLT) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2384 | `freedata\|mask=64` | png_free_data(PNG_FREE_ROWS) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2385 | `freedata\|mask=128` | png_free_data(PNG_FREE_PCAL) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2386 | `freedata\|mask=256` | png_free_data(PNG_FREE_SCAL) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2387 | `freedata\|mask=512` | png_free_data(PNG_FREE_UNKN) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2388 | `freedata\|mask=4096` | png_free_data(PNG_FREE_PLTE) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2389 | `freedata\|mask=8192` | png_free_data(PNG_FREE_TRNS) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2390 | `freedata\|mask=16384` | png_free_data(PNG_FREE_TEXT) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2391 | `freedata\|mask=32768` | png_free_data(PNG_FREE_EXIF) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2392 | `freedata\|mask=16928` | png_free_data(PNG_FREE_MUL) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |
| 2393 | `freedata\|mask=0` | png_free_data(nothing) then png_data_freer / png_set_invalid / png_destroy_info_struct | exit 0; 1 warning(s): hIST: out of place | [x] |

## B25 — Deprecated filter heuristics

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2394 | `heur\|hm=0\|nw=0\|seed=25001` | png_set_filter_heuristics(method=0, num_weights=0) then a filtered write | exit 0 | [x] |
| 2395 | `heur\|hm=0\|nw=1\|seed=25001` | png_set_filter_heuristics(method=0, num_weights=1) then a filtered write | exit 0 | [x] |
| 2396 | `heur\|hm=0\|nw=3\|seed=25001` | png_set_filter_heuristics(method=0, num_weights=3) then a filtered write | exit 0 | [x] |
| 2397 | `heur\|hm=0\|nw=5\|seed=25001` | png_set_filter_heuristics(method=0, num_weights=5) then a filtered write | exit 0 | [x] |
| 2398 | `heur\|hm=1\|nw=0\|seed=25001` | png_set_filter_heuristics(method=1, num_weights=0) then a filtered write | exit 0 | [x] |
| 2399 | `heur\|hm=1\|nw=1\|seed=25001` | png_set_filter_heuristics(method=1, num_weights=1) then a filtered write | exit 0 | [x] |
| 2400 | `heur\|hm=1\|nw=3\|seed=25001` | png_set_filter_heuristics(method=1, num_weights=3) then a filtered write | exit 0 | [x] |
| 2401 | `heur\|hm=1\|nw=5\|seed=25001` | png_set_filter_heuristics(method=1, num_weights=5) then a filtered write | exit 0 | [x] |
| 2402 | `heur\|hm=2\|nw=0\|seed=25001` | png_set_filter_heuristics(method=2, num_weights=0) then a filtered write | exit 0 | [x] |
| 2403 | `heur\|hm=2\|nw=1\|seed=25001` | png_set_filter_heuristics(method=2, num_weights=1) then a filtered write | exit 0 | [x] |
| 2404 | `heur\|hm=2\|nw=3\|seed=25001` | png_set_filter_heuristics(method=2, num_weights=3) then a filtered write | exit 0 | [x] |
| 2405 | `heur\|hm=2\|nw=5\|seed=25001` | png_set_filter_heuristics(method=2, num_weights=5) then a filtered write | exit 0 | [x] |
| 2406 | `heur\|hm=3\|nw=0\|seed=25001` | png_set_filter_heuristics(method=3, num_weights=0) then a filtered write | exit 0 | [x] |
| 2407 | `heur\|hm=3\|nw=1\|seed=25001` | png_set_filter_heuristics(method=3, num_weights=1) then a filtered write | exit 0 | [x] |
| 2408 | `heur\|hm=3\|nw=3\|seed=25001` | png_set_filter_heuristics(method=3, num_weights=3) then a filtered write | exit 0 | [x] |
| 2409 | `heur\|hm=3\|nw=5\|seed=25001` | png_set_filter_heuristics(method=3, num_weights=5) then a filtered write | exit 0 | [x] |

## B26 — B26

| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |
|---|---------------------------|--------------------------------------------|-------------------------|-----|
| 2410 | `sfuzz\|n=8\|seed=27000` | simplified-API fuzz batch 0 (8 randomized source/format combinations) | exit 0 | [x] |
| 2411 | `sfuzz\|n=8\|seed=27001` | simplified-API fuzz batch 1 (8 randomized source/format combinations) | exit 0 | [x] |
| 2412 | `sfuzz\|n=8\|seed=27002` | simplified-API fuzz batch 2 (8 randomized source/format combinations) | exit 0 | [x] |
| 2413 | `sfuzz\|n=8\|seed=27003` | simplified-API fuzz batch 3 (8 randomized source/format combinations) | exit 0 | [x] |
| 2414 | `sfuzz\|n=8\|seed=27004` | simplified-API fuzz batch 4 (8 randomized source/format combinations) | exit 0 | [x] |
| 2415 | `sfuzz\|n=8\|seed=27005` | simplified-API fuzz batch 5 (8 randomized source/format combinations) | exit 0 | [x] |
| 2416 | `sfuzz\|n=8\|seed=27006` | simplified-API fuzz batch 6 (8 randomized source/format combinations) | exit 0 | [x] |
| 2417 | `sfuzz\|n=8\|seed=27007` | simplified-API fuzz batch 7 (8 randomized source/format combinations) | exit 0 | [x] |
| 2418 | `sfuzz\|n=8\|seed=27008` | simplified-API fuzz batch 8 (8 randomized source/format combinations) | exit 0 | [x] |
| 2419 | `sfuzz\|n=8\|seed=27009` | simplified-API fuzz batch 9 (8 randomized source/format combinations) | exit 0 | [x] |
| 2420 | `sfuzz\|n=8\|seed=27010` | simplified-API fuzz batch 10 (8 randomized source/format combinations) | exit 0 | [x] |
| 2421 | `sfuzz\|n=8\|seed=27011` | simplified-API fuzz batch 11 (8 randomized source/format combinations) | exit 0 | [x] |
| 2422 | `sfuzz\|n=8\|seed=27012` | simplified-API fuzz batch 12 (8 randomized source/format combinations) | exit 0 | [x] |
| 2423 | `sfuzz\|n=8\|seed=27013` | simplified-API fuzz batch 13 (8 randomized source/format combinations) | exit 0 | [x] |
| 2424 | `sfuzz\|n=8\|seed=27014` | simplified-API fuzz batch 14 (8 randomized source/format combinations) | exit 0 | [x] |
| 2425 | `sfuzz\|n=8\|seed=27015` | simplified-API fuzz batch 15 (8 randomized source/format combinations) | exit 0 | [x] |
| 2426 | `sfuzz\|n=8\|seed=27016` | simplified-API fuzz batch 16 (8 randomized source/format combinations) | exit 0 | [x] |
| 2427 | `sfuzz\|n=8\|seed=27017` | simplified-API fuzz batch 17 (8 randomized source/format combinations) | exit 0 | [x] |
| 2428 | `sfuzz\|n=8\|seed=27018` | simplified-API fuzz batch 18 (8 randomized source/format combinations) | exit 0 | [x] |
| 2429 | `sfuzz\|n=8\|seed=27019` | simplified-API fuzz batch 19 (8 randomized source/format combinations) | exit 0 | [x] |
| 2430 | `sfuzz\|n=8\|seed=27020` | simplified-API fuzz batch 20 (8 randomized source/format combinations) | exit 0 | [x] |
| 2431 | `sfuzz\|n=8\|seed=27021` | simplified-API fuzz batch 21 (8 randomized source/format combinations) | exit 0 | [x] |
| 2432 | `sfuzz\|n=8\|seed=27022` | simplified-API fuzz batch 22 (8 randomized source/format combinations) | exit 0 | [x] |
| 2433 | `sfuzz\|n=8\|seed=27023` | simplified-API fuzz batch 23 (8 randomized source/format combinations) | exit 0 | [x] |

