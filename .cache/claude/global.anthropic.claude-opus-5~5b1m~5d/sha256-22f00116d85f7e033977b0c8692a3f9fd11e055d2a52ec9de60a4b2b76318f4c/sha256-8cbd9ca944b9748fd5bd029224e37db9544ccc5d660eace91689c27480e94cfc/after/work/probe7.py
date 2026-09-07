import subprocess, shutil
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig7'
shutil.copy(SRC,BAK)
M = [
 ("Compare string: strcmp -> strncmp 1", "            if strcmp((*a).valuestring, (*b).valuestring) == 0 {\n                return TRUE;", "            if strncmp((*a).valuestring, (*b).valuestring, 1) == 0 {\n                return TRUE;"),
]
res=[]
for name, old, new in M:
    s=open(BAK).read()
    if old not in s:
        res.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    res.append((name,'detected' if p.returncode!=0 else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in res: print(f"{r:20s} {n}")
