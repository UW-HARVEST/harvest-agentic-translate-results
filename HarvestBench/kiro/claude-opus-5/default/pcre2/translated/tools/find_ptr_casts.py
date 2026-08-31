#!/usr/bin/env python3
"""Flag suspicious single dereferences of double-indirection parameters.

`&mut PCRE2_SPTR` (i.e. `&mut *const u8`) needs `**p` to read the pointed-to
code unit; `*p as u32` silently compiles as a pointer-to-integer cast and is
almost always a translation bug.
"""
import re
import sys
import glob

PTR_PARAM = re.compile(
    r'(\w+)\s*:\s*&mut\s+(PCRE2_SPTR|PCRE2_UCHAR\s*\*|\*const\s+\w+|\*mut\s+\w+)')

problems = []

for path in sorted(glob.glob('src/*.rs')):
    src = open(path).read()
    lines = src.split('\n')
    # Find function spans
    fn_starts = [i for i, l in enumerate(lines)
                 if re.match(r'\s*(pub(\(crate\))?\s+)?(unsafe\s+)?fn\s+\w+', l)]
    fn_starts.append(len(lines))
    for k in range(len(fn_starts) - 1):
        start, end = fn_starts[k], fn_starts[k + 1]
        body = '\n'.join(lines[start:end])
        # Parameter list: up to the first '{' after the signature
        brace = body.find('{')
        sig = body[:brace] if brace > 0 else body
        names = set(m.group(1) for m in PTR_PARAM.finditer(sig))
        if not names:
            continue
        for name in names:
            # Reading through the reference should be **name, not *name
            for m in re.finditer(
                    r'(?<![*\w.])\*' + re.escape(name) +
                    r'\s*(as\s+(?:u8|u16|u32|u64|c_int|i32|usize)\b|==|!=|<=|>=)',
                    body):
                off = body[:m.start()].count('\n')
                problems.append((path, start + off + 1, name,
                                 lines[start + off].strip()))

for p, ln, name, text in problems:
    print(f"{p}:{ln}: suspicious `*{name}`: {text}")
print(f"\n{len(problems)} suspicious site(s)", file=sys.stderr)
