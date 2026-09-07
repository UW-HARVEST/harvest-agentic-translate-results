#!/usr/bin/env python3
"""Generate CONFIGS.md from the branch classes in c_src/src/lib.c."""

from pathlib import Path


BLOCKS = [
    ("block=192", "case 192"),
    ("block=576", "case 576"),
    ("block=1152", "case 1152"),
    ("block=2304", "case 2304"),
    ("block=4608", "case 4608"),
    ("block=256", "case 256"),
    ("block=512", "case 512"),
    ("block=1024", "case 1024"),
    ("block=2048", "case 2048"),
    ("block=4096", "case 4096"),
    ("block=8192", "case 8192"),
    ("block=16384", "case 16384"),
    ("block=32768", "case 32768"),
    ("block<=256, not an exact case", "default 0x06"),
    ("block>256, not an exact case", "default 0x07"),
]

RATES = [
    ("rate=882000", "case 882000"),
    ("rate=176400", "case 176400"),
    ("rate=192000", "case 192000"),
    ("rate=8000", "case 8000"),
    ("rate=16000", "case 16000"),
    ("rate=22050", "case 22050"),
    ("rate=24000", "case 24000"),
    ("rate=32000", "case 32000"),
    ("rate=44100", "case 44100"),
    ("rate=48000", "case 48000"),
    ("rate=96000", "case 96000"),
    ("rate%1000=0, rate/1000<256, not exact", "default 0x0C"),
    ("rate%1000=0, rate/1000>=256, not exact", "default no rate bits"),
    ("rate%1000!=0, rate<65536, not exact", "default 0x0D"),
    (
        "rate%1000!=0, rate>=65536, rate%10=0, rate/10<65536",
        "default 0x0E",
    ),
    (
        "rate%1000!=0, rate>=65536, rate%10=0, rate/10>=65536",
        "default no rate bits",
    ),
    (
        "rate%1000!=0, rate>=65536, rate%10!=0",
        "default no rate bits",
    ),
]

MODES = [
    ("channel_mode%4=0; channels=any u32", "independent"),
    ("channel_mode%4=1; channels ignored", "left-side"),
    ("channel_mode%4=2; channels ignored", "side-right"),
    ("channel_mode%4=3; channels ignored", "mid-side"),
]

DEPTHS = [
    ("bitdepth=8", "case 8"),
    ("bitdepth=12", "case 12"),
    ("bitdepth=16", "case 16"),
    ("bitdepth=20", "case 20"),
    ("bitdepth=24", "case 24"),
    ("bitdepth=32", "case 32"),
    ("bitdepth not in {8,12,16,20,24,32}", "default no depth bits"),
]


def main() -> None:
    output = Path(__file__).resolve().parents[1] / "CONFIGS.md"
    lines = [
        "# Configuration surface",
        "",
        "Mechanically derived from every `case`, `default`, and conditional in",
        "`../c_src/src/lib.c`. The public header exposes one entry point:",
        "`update_frame_header`. It is the lowest-level and only API.",
        "",
        "The C branches distinguish 15 block-size classes × 17 sample-rate",
        "classes × 4 channel-mode classes × 7 bit-depth classes = **7,140**",
        "meaningful configurations. `frame_header` is always overwritten; tests",
        "still randomize its incoming value. For every row, tests use repeated",
        "fixed-seed randomized representatives that preserve the stated class.",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---|----------------|--------------------------------------------|-----|",
    ]

    row = 0
    for block, block_branch in BLOCKS:
        for rate, rate_branch in RATES:
            for mode, mode_branch in MODES:
                for depth, depth_branch in DEPTHS:
                    row += 1
                    config = (
                        f"{block} ({block_branch}); {rate} ({rate_branch}); "
                        f"{mode} ({mode_branch}); {depth} ({depth_branch})"
                    )
                    lines.append(
                        f"| {row} | `update_frame_header` | {config} | [ ] |"
                    )

    if row != 7140:
        raise RuntimeError(f"expected 7140 rows, generated {row}")
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
