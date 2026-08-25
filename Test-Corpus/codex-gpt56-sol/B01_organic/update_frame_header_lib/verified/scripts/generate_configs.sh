#!/bin/sh
set -eu

output=${1:-CONFIGS.md}
mark=${2:- }

blocks='
exact 192
exact 576
exact 1152
exact 2304
exact 4608
exact 256
exact 512
exact 1024
exact 2048
exact 4096
exact 8192
exact 16384
exact 32768
default <=256 excluding 192 and 256
default >256 excluding explicit cases
'

rates='
exact 882000
exact 176400
exact 192000
exact 8000
exact 16000
exact 22050
exact 24000
exact 32000
exact 44100
exact 48000
exact 96000
default divisible by 1000 with quotient <256, excluding exact cases
default divisible by 1000 with quotient >=256, excluding exact cases
default not divisible by 1000 and <65536
default not divisible by 1000, >=65536, divisible by 10, quotient <65536
default not divisible by 1000, divisible by 10, quotient >=65536
default not divisible by 1000, >=65536, and not divisible by 10
'

channels='
channel_mode % 4 = 0; channels is any u32
channel_mode % 4 = 1; channels ignored
channel_mode % 4 = 2; channels ignored
channel_mode % 4 = 3; channels ignored
'

depths='
exact 8
exact 12
exact 16
exact 20
exact 24
exact 32
default excluding explicit cases
'

{
    printf '%s\n' '# Configuration Surface'
    printf '%s\n' ''
    printf '%s\n' '## Build-Time Configurations'
    printf '%s\n' ''
    printf '%s\n' '`Cargo.toml` has no `[features]` table and CMake declares no options or'
    printf '%s\n' 'conditional sources. The power set of Rust features therefore has one member:'
    printf '%s\n' 'the empty set. The default and `--no-default-features` builds select that same'
    printf '%s\n' 'configuration.'
    printf '%s\n' ''
    printf '%s\n' '| # | Rust features | CMake options | [ ] |'
    printf '%s\n' '|---|---------------|---------------|-----|'
    printf '| B1 | empty set | none | [%s] |\n' "$mark"
    printf '%s\n' ''
    printf '%s\n' '## Runtime Configurations'
    printf '%s\n' ''
    printf '%s\n' 'The sole public entry point is `update_frame_header`. Each row is one member'
    printf '%s\n' 'of the mechanically derived cross-product of every branch equivalence class:'
    printf '%s\n' '15 block-size classes x 17 sample-rate classes x 4 channel-mode classes x'
    printf '%s\n' '7 bit-depth classes = 7,140 rows. Within every row, tests randomize values'
    printf '%s\n' 'inside non-singleton classes, `channels`, the incoming `frame_header`, and'
    printf '%s\n' 'all struct padding bytes using a fixed seed.'
    printf '%s\n' ''
    printf '%s\n' '| # | entry point(s) | configuration (options set + input shape) | [ ] |'
    printf '%s\n' '|---|----------------|-------------------------------------------|-----|'

    row=1
    old_ifs=$IFS
    IFS='
'
    for block in $blocks; do
        for rate in $rates; do
            for channel in $channels; do
                for depth in $depths; do
                    printf '| C%d | `update_frame_header` | block: %s; rate: %s; %s; depth: %s | [%s] |\n' \
                        "$row" "$block" "$rate" "$channel" "$depth" "$mark"
                    row=$((row + 1))
                done
            done
        done
    done
    IFS=$old_ifs
} > "$output"
