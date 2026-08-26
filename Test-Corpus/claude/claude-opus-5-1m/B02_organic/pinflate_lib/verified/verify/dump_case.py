import os, sys
os.environ["FUZZ_SEED"]=sys.argv[1]; os.environ["FUZZ_N"]=sys.argv[2]
import fuzzdiff as fz
cases = fz.build_cases()
for i in [int(x) for x in sys.argv[3].split(",")]:
    d, osz, io_, oo, lbl = cases[i]
    print(f"{i} out={osz} in_off={io_} out_off={oo} len={len(d)} data={d.hex()}")
