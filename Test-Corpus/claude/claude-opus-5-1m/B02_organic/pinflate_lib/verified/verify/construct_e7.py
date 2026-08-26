#!/usr/bin/env python3
"""Constructs an input that reaches `cp_ptr`'s assert (lib.c:95).

The reader keeps the invariant `count == -consumed (mod 8)` because every
refill adds 32 bits -- except `cp_peak_bits`' "final word" branch, which adds
`s->bits_left`. If that branch runs while `consumed % 8 != 0` the invariant
breaks, and then `cp_stored`'s `cp_read_bits(s, s->count & 7)` no longer
byte-aligns the stream, so `cp_ptr` sees `bits_left & 7 != 0`.

Only *sizes* matter for that, so the reader is simulated over a request
schedule; the bit values are filled in afterwards.
"""
import sys

ASSERT_OK = None


class Sim:
    def __init__(self, in_bytes, first_bytes):
        self.count = first_bytes * 8
        self.bits_left = in_bytes * 8
        self.word_index = 0
        self.word_count = (in_bytes - first_bytes) // 4
        last = (in_bytes - first_bytes) % 4
        if (in_bytes - first_bytes) < 0:
            # C: negative & 3 on two's complement
            last = (in_bytes - first_bytes) & 3
        self.fwa = 1 if last else 0
        self.consumed = 0
        self.fail = None

    def peak(self, n):
        if self.count < n:
            if self.word_index < self.word_count:
                self.word_index += 1
                self.count += 32
            elif self.fwa:
                self.count += self.bits_left
                self.fwa = 0

    def consume(self, n):
        if self.count < n:
            self.fail = 115
            return
        self.count -= n
        self.bits_left -= n
        self.consumed += n

    def read(self, n):
        if n > 32:
            self.fail = 123
            return
        if n < 0:
            self.fail = 124
            return
        if not self.bits_left > 0:
            self.fail = 125
            return
        if not self.count <= 64:
            self.fail = 126
            return
        if (self.bits_left + self.count) - n < 0:
            self.fail = 127
            return
        self.peak(n)
        self.consume(n)


def trial(in_bytes, first_bytes, code_lens):
    """first block: btype==1 with the given symbol code lengths (last = EOB);
    second block: btype==0.  Returns (positions, bits_left_at_cp_ptr) or None."""
    s = Sim(in_bytes, first_bytes)
    pos = {}
    s.read(1)                       # bfinal
    s.read(2)                       # btype == 1
    if s.fail:
        return None
    for cl in code_lens:            # cp_block -> cp_decode
        s.peak(16)
        s.consume(cl)
        if s.fail:
            return None
    s.read(1)                       # bfinal of block 2
    s.read(2)                       # btype == 0
    if s.fail:
        return None
    d = s.count & 7
    s.read(d)                       # cp_stored's alignment discard
    if s.fail:
        return None
    pos["len"] = s.consumed
    s.read(16)                      # LEN
    if s.fail:
        return None
    pos["nlen"] = s.consumed
    s.read(16)                      # NLEN
    if s.fail:
        return None
    pos["after"] = s.consumed
    pos["bits_left"] = s.bits_left
    pos["count"] = s.count
    pos["word_index"] = s.word_index
    return pos


def main():
    # fixed-tree code lengths: 7 (symbols 256..279), 8 (0..143, 280..287),
    # 9 (144..255)
    found = []
    for in_bytes in range(4, 40):
        for in_off in range(4):
            first_bytes = ((in_off + 3) & ~3) - in_off
            for nlit in range(0, 9):
                for cl in (8, 9):
                    code_lens = [cl] * nlit + [7]
                    r = trial(in_bytes, first_bytes, code_lens)
                    if r is None:
                        continue
                    if r["bits_left"] & 7:
                        found.append((in_bytes, in_off, nlit, cl, r))
    print(f"{len(found)} candidate shapes reach cp_ptr with bits_left & 7 != 0")
    for f in found[:12]:
        in_bytes, in_off, nlit, cl, r = f
        print(f"  in_bytes={in_bytes} in_off={in_off} nlit={nlit} codelen={cl} "
              f"len_pos={r['len']} nlen_pos={r['nlen']} bits_left={r['bits_left']} "
              f"(&7={r['bits_left'] & 7}) count={r['count']} wi={r['word_index']}")
    if not found:
        sys.exit(1)


main()
