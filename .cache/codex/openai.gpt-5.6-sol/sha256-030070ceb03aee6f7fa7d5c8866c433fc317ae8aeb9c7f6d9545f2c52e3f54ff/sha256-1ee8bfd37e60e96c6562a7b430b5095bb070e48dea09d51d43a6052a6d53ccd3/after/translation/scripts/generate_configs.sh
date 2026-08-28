#!/bin/sh
set -eu

output=${1:-CONFIGS.md}

blocks='
B01|cur_blocksize == 192
B02|cur_blocksize == 576
B03|cur_blocksize == 1152
B04|cur_blocksize == 2304
B05|cur_blocksize == 4608
B06|cur_blocksize == 256
B07|cur_blocksize == 512
B08|cur_blocksize == 1024
B09|cur_blocksize == 2048
B10|cur_blocksize == 4096
B11|cur_blocksize == 8192
B12|cur_blocksize == 16384
B13|cur_blocksize == 32768
B14|other cur_blocksize <= 256
B15|other cur_blocksize > 256
'

rates='
S01|samplerate == 882000
S02|samplerate == 176400
S03|samplerate == 192000
S04|samplerate == 8000
S05|samplerate == 16000
S06|samplerate == 22050
S07|samplerate == 24000
S08|samplerate == 32000
S09|samplerate == 44100
S10|samplerate == 48000
S11|samplerate == 96000
S12|other samplerate % 1000 == 0 and samplerate / 1000 < 256
S13|other samplerate % 1000 == 0 and samplerate / 1000 >= 256
S14|samplerate % 1000 != 0 and samplerate < 65536
S15|samplerate >= 65536 and % 1000 != 0 and % 10 == 0 and / 10 < 65536
S16|samplerate >= 65536 and % 1000 != 0 and % 10 != 0
S17|samplerate % 1000 != 0 and % 10 == 0 and samplerate / 10 >= 65536
'

modes='
M01|channel_mode % 4 == 0; channels spans u32 boundaries
M02|channel_mode % 4 == 1
M03|channel_mode % 4 == 2
M04|channel_mode % 4 == 3
'

depths='
D01|bitdepth == 8
D02|bitdepth == 12
D03|bitdepth == 16
D04|bitdepth == 20
D05|bitdepth == 24
D06|bitdepth == 32
D07|other bitdepth
'

awk -v blocks="$blocks" -v rates="$rates" -v modes="$modes" -v depths="$depths" '
BEGIN {
    print "# Configuration surface"
    print ""
    print "The rows are the mechanically derived cross-product of every distinct"
    print "control-flow class in `update_frame_header`: 15 block-size paths, 17"
    print "sample-rate paths, 4 channel-mode paths, and 7 bit-depth paths."
    print "Every row is exercised with 32 deterministic randomized inputs."
    print ""
    print "| # | entry point(s) | configuration (options set + input shape) | status |"
    print "|---|----------------|--------------------------------------------|--------|"

    nb = split(blocks, b, "\n")
    nr = split(rates, r, "\n")
    nm = split(modes, m, "\n")
    nd = split(depths, d, "\n")
    row = 0
    for (bi = 1; bi <= nb; bi++) {
        if (b[bi] == "") continue
        split(b[bi], bp, "|")
        for (ri = 1; ri <= nr; ri++) {
            if (r[ri] == "") continue
            split(r[ri], rp, "|")
            for (mi = 1; mi <= nm; mi++) {
                if (m[mi] == "") continue
                split(m[mi], mp, "|")
                for (di = 1; di <= nd; di++) {
                    if (d[di] == "") continue
                    split(d[di], dp, "|")
                    row++
                    printf "| %d | `update_frame_header` | %s: %s; %s: %s; %s: %s; %s: %s | [ ] |\n", \
                        row, bp[1], bp[2], rp[1], rp[2], mp[1], mp[2], dp[1], dp[2]
                }
            }
        }
    }
}
' > "$output"
