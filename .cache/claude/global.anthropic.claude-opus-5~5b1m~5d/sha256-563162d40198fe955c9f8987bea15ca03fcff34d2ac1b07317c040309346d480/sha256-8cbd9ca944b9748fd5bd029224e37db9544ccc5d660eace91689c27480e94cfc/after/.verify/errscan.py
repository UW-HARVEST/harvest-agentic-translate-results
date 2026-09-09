import re,glob,os
pat = re.compile(r'\b(png_error|png_chunk_error|png_benign_error|png_app_error|png_warning|png_app_warning|png_chunk_warning|png_chunk_report|png_fixed_error|png_longjmp)\s*\(')
funcpat = re.compile(r'^([A-Za-z_][A-Za-z0-9_ \*]*?)\s*\(')
rows=[]
for f in sorted(glob.glob('c_src/src/*.c')):
    lines=open(f, encoding='utf8', errors='replace').read().split('\n')
    cur='?'
    for i,l in enumerate(lines):
        if l and (l[0].isalpha() or l[0]=='_') and '(' in l:
            m=funcpat.match(l)
            if m and not l.rstrip().endswith(';'):
                cur=m.group(1).split()[-1].lstrip('*')
        mm=pat.search(l)
        if mm:
            txt=' '.join(x.strip() for x in lines[i:i+3])
            msg=re.search(r'"((?:[^"\\]|\\.)*)"', txt)
            rows.append((os.path.basename(f), i+1, cur, mm.group(1), msg.group(1) if msg else ''))
print(len(rows))
with open('.verify/errsites.tsv','w') as o:
    for r in rows: o.write('\t'.join(map(str,r))+'\n')
