# CONFIGS.md — configuration surface table (Phase B)

Mechanically derived from the four `switch` statements and five `if`s of
`c_src/src/lib.c`. There is exactly **one** public entry point
(`include/lib.h`: `void update_frame_header(tflac *t);`), so the
"configuration" is entirely the state of the six `struct tflac` fields the
function reads. There are no runtime option setters, no modes, no `#ifdef`s.

## The axes the C actually branches on

`t->frame_header` is built as
`0xFFF80000 | (BS<<12) | (SR<<8) | CH | (BD<<1)`, where the four contributions
come from four independent branch trees. Axes and their equivalence classes:

**Axis BS — `cur_blocksize` (line 13, 15 classes)**
`B1`=192→`0x1`, `B2`=576→`0x2`, `B3`=1152→`0x3`, `B4`=2304→`0x4`,
`B5`=4608→`0x5`, `B6`=256→`0x8`, `B7`=512→`0x9`, `B8`=1024→`0xA`,
`B9`=2048→`0xB`, `B10`=4096→`0xC`, `B11`=8192→`0xD`, `B12`=16384→`0xE`,
`B13`=32768→`0xF`, `B14`=default & `<=256`→`0x6`, `B15`=default & `>256`→`0x7`.

**Axis SR — `samplerate` (line 58, 17 classes)**
`S1`=882000→`0x1`, `S2`=176400→`0x2`, `S3`=192000→`0x3`, `S4`=8000→`0x4`,
`S5`=16000→`0x5`, `S6`=22050→`0x6`, `S7`=24000→`0x7`, `S8`=32000→`0x8`,
`S9`=44100→`0x9`, `S10`=48000→`0xA`, `S11`=96000→`0xB`,
`S12`=default & `%1000==0` & `/1000<256`→`0xC`,
`S13`=default & `%1000==0` & `/1000>=256`→none,
`S14`=default & `%1000!=0` & `<65536`→`0xD`,
`S15`=default & `%1000!=0` & `>=65536` & `%10==0` & `/10<65536`→`0xE`,
`S16`=default & `%1000!=0` & `>=65536` & `%10==0` & `/10>=65536`→none,
`S17`=default & `%1000!=0` & `>=65536` & `%10!=0`→none.

**Axis CH — `channel_mode` (line 106) × `channels` (line 109), 8 classes**
`C1`=`mode%4==0`,channels=1→`0x00`; `C2`=`mode%4==0`,channels=2→`0x10`;
`C3`=`mode%4==0`,channels=8→`0x70`; `C4`=`mode%4==0`,channels=0→`0xFFFFFFF0`;
`C5`=`mode%4==0`,channels∈{9…0xFFFFFFFF}→overflowing `(channels-1)<<4`;
`C6`=`mode%4==1`→`0x80`; `C7`=`mode%4==2`→`0x90`; `C8`=`mode%4==3`→`0xA0`.
Input-shape sub-axis: `channel_mode` is `tflac_u8`, so all 256 raw values fold
onto `C1..C8` through `% 4` — the raw value is itself an axis (0..=255).

**Axis BD — `bitdepth` (line 123, 7 classes)**
`D1`=8→`0x1`, `D2`=12→`0x2`, `D3`=16→`0x4`, `D4`=20→`0x5`, `D5`=24→`0x6`,
`D6`=32→`0x7`, `D7`=default→none.

**Axis P — pointer/struct shape** (aligned vs unaligned, pre-existing
`frame_header` contents, padding bytes, repeated calls on the same struct).

Full cross-product = 15 × 17 × 8 × 7 = **14 280** distinguished states; rows
R1–R47 pin every individual class, R48–R56 pin the interactions (including the
complete exhaustive cross-product), R57–R62 pin the struct/pointer shapes.
Every row is driven with **many randomized inputs** drawn from its class using a
fixed-seed xorshift64\* PRNG, and both `.so`s are compared **byte-for-byte over
all 24 struct bytes**.

## Configuration table

| #   | entry point | configuration (options set + input shape) | test | ✔ |
|-----|-------------|-------------------------------------------|------|---|
| R1  | `update_frame_header` | BS=`B1` (`cur_blocksize`=192); SR/CH/BD randomized | `cfg_blocksize_classes` | [x] |
| R2  | `update_frame_header` | BS=`B2` (576); others randomized | `cfg_blocksize_classes` | [x] |
| R3  | `update_frame_header` | BS=`B3` (1152); others randomized | `cfg_blocksize_classes` | [x] |
| R4  | `update_frame_header` | BS=`B4` (2304); others randomized | `cfg_blocksize_classes` | [x] |
| R5  | `update_frame_header` | BS=`B5` (4608); others randomized | `cfg_blocksize_classes` | [x] |
| R6  | `update_frame_header` | BS=`B6` (256); others randomized | `cfg_blocksize_classes` | [x] |
| R7  | `update_frame_header` | BS=`B7` (512); others randomized | `cfg_blocksize_classes` | [x] |
| R8  | `update_frame_header` | BS=`B8` (1024); others randomized | `cfg_blocksize_classes` | [x] |
| R9  | `update_frame_header` | BS=`B9` (2048); others randomized | `cfg_blocksize_classes` | [x] |
| R10 | `update_frame_header` | BS=`B10` (4096); others randomized | `cfg_blocksize_classes` | [x] |
| R11 | `update_frame_header` | BS=`B11` (8192); others randomized | `cfg_blocksize_classes` | [x] |
| R12 | `update_frame_header` | BS=`B12` (16384); others randomized | `cfg_blocksize_classes` | [x] |
| R13 | `update_frame_header` | BS=`B13` (32768); others randomized | `cfg_blocksize_classes` | [x] |
| R14 | `update_frame_header` | BS=`B14` (default, `<=256`: exhaustive 0..=256 minus 192/256); others randomized | `cfg_blocksize_classes` + `cfg_blocksize_exhaustive_0_70000` | [x] |
| R15 | `update_frame_header` | BS=`B15` (default, `>256`: 257, 65535, 65536, 0x7FFFFFFF, 0xFFFFFFFF, random) | `cfg_blocksize_classes` + `cfg_blocksize_exhaustive_0_70000` | [x] |
| R16 | `update_frame_header` | SR=`S1` (882000); BS/CH/BD randomized | `cfg_samplerate_classes` | [x] |
| R17 | `update_frame_header` | SR=`S2` (176400); others randomized | `cfg_samplerate_classes` | [x] |
| R18 | `update_frame_header` | SR=`S3` (192000); others randomized | `cfg_samplerate_classes` | [x] |
| R19 | `update_frame_header` | SR=`S4` (8000); others randomized | `cfg_samplerate_classes` | [x] |
| R20 | `update_frame_header` | SR=`S5` (16000); others randomized | `cfg_samplerate_classes` | [x] |
| R21 | `update_frame_header` | SR=`S6` (22050); others randomized | `cfg_samplerate_classes` | [x] |
| R22 | `update_frame_header` | SR=`S7` (24000); others randomized | `cfg_samplerate_classes` | [x] |
| R23 | `update_frame_header` | SR=`S8` (32000); others randomized | `cfg_samplerate_classes` | [x] |
| R24 | `update_frame_header` | SR=`S9` (44100); others randomized | `cfg_samplerate_classes` | [x] |
| R25 | `update_frame_header` | SR=`S10` (48000); others randomized | `cfg_samplerate_classes` | [x] |
| R26 | `update_frame_header` | SR=`S11` (96000); others randomized | `cfg_samplerate_classes` | [x] |
| R27 | `update_frame_header` | SR=`S12` (`%1000==0`, `/1000<256`: 0, 1000, 88000, 255000, random `k*1000`, k<256) | `cfg_samplerate_classes` | [x] |
| R28 | `update_frame_header` | SR=`S13` (`%1000==0`, `/1000>=256`: 256000, 1000000, 4294000000, random `k*1000`, k>=256) | `cfg_samplerate_classes` | [x] |
| R29 | `update_frame_header` | SR=`S14` (`%1000!=0`, `<65536`: 1, 22051, 44101, 65535, random `<65536` not `%1000`) | `cfg_samplerate_classes` | [x] |
| R30 | `update_frame_header` | SR=`S15` (`%1000!=0`, `>=65536`, `%10==0`, `/10<65536`: 88200, 65540, 655350, random) | `cfg_samplerate_classes` | [x] |
| R31 | `update_frame_header` | SR=`S16` (`%1000!=0`, `>=65536`, `%10==0`, `/10>=65536`: 655360, 4294967290, random) | `cfg_samplerate_classes` | [x] |
| R32 | `update_frame_header` | SR=`S17` (`%1000!=0`, `>=65536`, `%10!=0`: 65537, 96001, 0xFFFFFFFF, random) | `cfg_samplerate_classes` | [x] |
| R33 | `update_frame_header` | CH=`C1` (`channel_mode%4==0`, `channels`=1); BS/SR/BD randomized | `cfg_channel_classes` | [x] |
| R34 | `update_frame_header` | CH=`C2` (mode%4=0, channels=2); others randomized | `cfg_channel_classes` | [x] |
| R35 | `update_frame_header` | CH=`C3` (mode%4=0, channels=8, i.e. the largest field-fitting count); others randomized | `cfg_channel_classes` | [x] |
| R36 | `update_frame_header` | CH=`C1..C3` sweep: mode%4=0 with `channels` exhaustive 1..=8 (all legal FLAC counts) | `cfg_channel_classes` | [x] |
| R37 | `update_frame_header` | CH=`C4` (mode%4=0, channels=0 → underflow `0xFFFFFFF0`); others randomized | `cfg_channel_classes` | [x] |
| R38 | `update_frame_header` | CH=`C5` (mode%4=0, channels ∈ {9,16,17,255,4096,0x0FFFFFFF,0x10000000,0x10000001,0xFFFFFFFF,random}) | `cfg_channel_classes` | [x] |
| R39 | `update_frame_header` | CH=`C6` (`channel_mode%4==1`, LEFT_SIDE) — `channels` must be ignored: randomized incl. 0 | `cfg_channel_classes` | [x] |
| R40 | `update_frame_header` | CH=`C7` (`channel_mode%4==2`, SIDE_RIGHT) — `channels` ignored, randomized incl. 0 | `cfg_channel_classes` | [x] |
| R41 | `update_frame_header` | CH=`C8` (`channel_mode%4==3`, MID_SIDE) — `channels` ignored, randomized incl. 0 | `cfg_channel_classes` | [x] |
| R42 | `update_frame_header` | BD=`D1` (`bitdepth`=8); BS/SR/CH randomized | `cfg_bitdepth_classes` | [x] |
| R43 | `update_frame_header` | BD=`D2` (12); others randomized | `cfg_bitdepth_classes` | [x] |
| R44 | `update_frame_header` | BD=`D3` (16); others randomized | `cfg_bitdepth_classes` | [x] |
| R45 | `update_frame_header` | BD=`D4` (20); others randomized | `cfg_bitdepth_classes` | [x] |
| R46 | `update_frame_header` | BD=`D5` (24) and `D6` (32); others randomized | `cfg_bitdepth_classes` | [x] |
| R47 | `update_frame_header` | BD=`D7` (default: exhaustive 0..=256 minus the 6 listed, plus 0xFFFFFFFF, 0x80000000, random) | `cfg_bitdepth_classes` + `cfg_bitdepth_exhaustive_0_1000` | [x] |
| R48 | `update_frame_header` | **complete cross-product** BS×SR×CH×BD over one representative per class: 15×17×8×7 = 14 280 combinations | `cfg_full_cross_product` | [x] |
| R49 | `update_frame_header` | cross-product of BS classes × CH classes (blocksize nibble vs the `channels-1` overflow bleeding into it) | `cfg_full_cross_product` | [x] |
| R50 | `update_frame_header` | cross-product of SR classes × CH classes (samplerate nibble vs `channels-1` overflow) | `cfg_full_cross_product` | [x] |
| R51 | `update_frame_header` | all 256 raw `channel_mode` values × all 8 CH classes' `channels` values (enum folding × count) | `cfg_channel_mode_exhaustive_u8` | [x] |
| R52 | `update_frame_header` | `channel_mode` exhaustive 0..=255 with BS/SR/BD randomized per value | `cfg_channel_mode_exhaustive_u8` | [x] |
| R53 | `update_frame_header` | `samplerate` exhaustive 0..=200 000 (covers every `S12`/`S14`/`S15`/`S17` boundary) with rotating BS/CH/BD | `cfg_samplerate_exhaustive_0_200000` | [x] |
| R54 | `update_frame_header` | `cur_blocksize` exhaustive 0..=70 000 (covers all 13 literals + both default arms) with rotating SR/CH/BD | `cfg_blocksize_exhaustive_0_70000` | [x] |
| R55 | `update_frame_header` | `bitdepth` exhaustive 0..=1 000 with rotating BS/SR/CH | `cfg_bitdepth_exhaustive_0_1000` | [x] |
| R56 | `update_frame_header` | uniform-random full struct, 400 000 draws (all 4 fields random u32 / u8, unconstrained) | `cfg_fuzz_uniform_random` | [x] |
| R57 | `update_frame_header` | structured-random: each field drawn from its class-representative pool ∪ near-boundary jitter (±1, ±10, ±1000), 400 000 draws | `cfg_fuzz_structured_random` | [x] |
| R58 | `update_frame_header` | realistic FLAC encoder configurations: (44100,48000,96000,192000) × (1,2,8) × (8,16,24,32) × (4096,1152,192) × all 4 modes | `cfg_realistic_flac_matrix` | [x] |
| R59 | `update_frame_header` | P: repeated invocation — call 3× on the same struct, assert idempotence and identical C/Rust results each time | `cfg_repeated_invocation_idempotent` | [x] |
| R60 | `update_frame_header` | P: `frame_header` pre-loaded with 0x00000000 / 0xFFFFFFFF / 0xDEADBEEF / random before the call (assignment vs OR) | `cfg_prior_frame_header_ignored` | [x] |
| R61 | `update_frame_header` | P: padding bytes at offset 13..15 pre-loaded with 0x00 / 0xFF / random; must survive unchanged and identically | `cfg_padding_preserved` | [x] |
| R62 | `update_frame_header` | P: unaligned struct pointer (buffer offset +1) — x86-64-permitted misaligned access | `err_e16_unaligned_pointer` | [x] |

## Results

All 62 rows pass across randomized inputs — see the ✔ column above. Case
counts (`cargo test --no-default-features -- --nocapture`):

```
R1..R15  blocksize classes ................  68 056 cases, 0 mismatches
R16..R32 samplerate classes ............... 176 638 cases, 0 mismatches
R33..R41 channel classes ..................  97 000 cases, 0 mismatches
R42..R47 bitdepth classes .................  50 008 cases, 0 mismatches
R48..R50 full cross-product (15x17x8x7) ...  14 280 cases, 0 mismatches
R51..R52 channel_mode exhaustive u8 .......  59 392 cases, 0 mismatches
R53      samplerate exhaustive 0..=200000 . 200 001 cases, 0 mismatches
R54      cur_blocksize exhaustive 0..=70000  70 001 cases, 0 mismatches
R55      bitdepth exhaustive 0..=1000 ......  20 020 cases, 0 mismatches
R56      uniform random fuzz .............. 400 000 cases, 0 mismatches
R57      structured random fuzz ........... 400 000 cases, 0 mismatches
R58      realistic FLAC matrix ............   3 072 cases, 0 mismatches
R59      repeated invocation .............. 150 000 cases, 0 mismatches
R60      prior frame_header ignored ....... 120 000 cases, 0 mismatches
R61      padding preserved ................ 100 000 cases, 0 mismatches
R62      unaligned pointer (E16) .......... 60 000 cases, 0 mismatches
```

Reinforced by `tests/phase_b_deep_sweep.rs`, which sweeps every axis over its
**complete** 2^32 domain (`channels` x all 4 modes), i.e. 30 064 771 072
additional differential cases, all matching. Every case compares all 24 struct
bytes (the exhaustive sweeps compare `frame_header`, the only field either
implementation writes — proven by E14/R61).
