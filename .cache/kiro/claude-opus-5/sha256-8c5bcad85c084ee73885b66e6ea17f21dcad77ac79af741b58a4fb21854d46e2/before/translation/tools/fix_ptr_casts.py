#!/usr/bin/env python3
"""Rewrite `*p as <int>` to `**p as <int>` for `&mut PCRE2_SPTR`-style params."""
import re
import glob

PTR_PARAM = re.compile(
    r'(\w+)\s*:\s*&mut\s+(PCRE2_SPTR|PCRE2_UCHAR\s*\*|\*const\s+\w+|\*mut\s+\w+)')
INT = r'(?:u8|u16|u32|u64|c_int|i32|usize)'

total = 0
for path in sorted(glob.glob('src/*.rs')):
    lines = open(path).read().split('\n')
    fn_starts = [i for i, l in enumerate(lines)
                 if re.match(r'\s*(pub(\(crate\))?\s+)?(unsafe\s+)?fn\s+\w+', l)]
    fn_starts.append(len(lines))
    changed = 0
    for k in range(len(fn_starts) - 1):
        start, end = fn_starts[k], fn_starts[k + 1]
        sig = '\n'.join(lines[start:end])
        brace = sig.find('{')
        if brace > 0:
            sig = sig[:brace]
        names = set(m.group(1) for m in PTR_PARAM.finditer(sig))
        if not names:
            continue
        for i in range(start, end):
            for name in names:
                pat = re.compile(r'(?<![*\w.])\*(' + re.escape(name) +
                                 r')(\s+as\s+' + INT + r'\b)')
                new = pat.sub(r'**\1\2', lines[i])
                if new != lines[i]:
                    changed += new.count('**' + name) - lines[i].count('**' + name)
                    lines[i] = new
    if changed:
        open(path, 'w').write('\n'.join(lines))
        print(f"{path}: fixed {changed} site(s)")
        total += changed
print("total", total)
