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
            # guard: nearest preceding line containing if/else/case within 8 lines
            guard=''
            for j in range(i-1, max(-1,i-9), -1):
                s=lines[j].strip()
                if s.startswith(('if','else if','case','default','while','for')) or s.endswith('||') or s.endswith('&&'):
                    # collect the if statement possibly multiline
                    k=j; buf=[]
                    while k<i and len(buf)<5:
                        buf.append(lines[k].strip()); k+=1
                    guard=' '.join(buf); break
            guard=re.sub(r'\s+',' ',guard)[:200]
            rows.append((os.path.basename(f), i+1, cur, mm.group(1), (msg.group(1) if msg else ''), guard))
with open('.verify/errsites.tsv','w') as o:
    for r in rows: o.write('\t'.join(map(str,r))+'\n')
print(len(rows))
