import pathlib, re, sys
p = pathlib.Path('src/lib.rs')
pristine = pathlib.Path(__file__).resolve().parent.joinpath('lib.rs.pristine').read_text()
if sys.argv[1] == 'restore':
    p.write_text(pristine); print('RESTORED'); sys.exit(0)
old, new = sys.argv[1], sys.argv[2]
# Anchor on a real code line: 4-space indent at start of line, no '///'.
pat = re.compile(r'^(    )' + re.escape(old) + r'$', re.M)
assert pat.search(pristine), f'anchor not found as a CODE line: {old!r}'
s = pat.sub(lambda m: m.group(1) + new, pristine)
assert s != pristine
assert '///' not in ''.join(l for l in s.splitlines(True) if new in l), 'mutated a doc line!'
p.write_text(s)
print(f'MUTATED code line:\n  - {old}\n  + {new}')
