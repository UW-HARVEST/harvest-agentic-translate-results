import os, subprocess, sys
os.environ["FUZZ_SEED"]=sys.argv[3]; os.environ["FUZZ_N"]=sys.argv[4]
import difftest as dt, fuzzdiff as fz
if len(sys.argv) > 6:   # child: lib, idx
    cases = fz.build_cases()
    d, osz, io_, oo, lbl = cases[int(sys.argv[6])]
    lib = dt.load(sys.argv[5])
    r = dt.run(lib, d, osz, io_, oo)
    print(repr((r[0], r[2]))); sys.stdout.flush(); os._exit(0)
cases = fz.build_cases()
idxs = [int(x) for x in sys.argv[1].split(",") if x]
for i in idxs:
    res=[]
    for so in (sys.argv[2].split(":")[0], sys.argv[2].split(":")[1]):
        p = subprocess.run([sys.executable, __file__, "-", "-", sys.argv[3], sys.argv[4], so, str(i)],
                           capture_output=True, text=True)
        res.append(f"sig{-p.returncode}" if p.returncode < 0 else p.stdout.strip())
    print(f"[{i}] C={res[0]} R={res[1]} -> {'SAME' if res[0]==res[1] else 'DIFF'}")
