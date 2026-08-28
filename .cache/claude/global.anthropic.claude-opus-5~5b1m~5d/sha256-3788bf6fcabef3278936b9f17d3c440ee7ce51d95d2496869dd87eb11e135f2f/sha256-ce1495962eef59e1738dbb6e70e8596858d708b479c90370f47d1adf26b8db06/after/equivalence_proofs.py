import struct
def f32(x): return struct.unpack('f', struct.pack('f', x))[0]

print("=== swap_row_coefficients: are the two coefficients the SAME f32? ===")
a, b = 0.12739886310880, 0.12739886341072
print(f"  0.12739886310880 as f32 = {f32(a)!r}  bits={struct.pack('>f',a).hex()}")
print(f"  0.12739886341072 as f32 = {f32(b)!r}  bits={struct.pack('>f',b).hex()}")
print(f"  identical as f32? {struct.pack('f',a) == struct.pack('f',b)}")
print(f"  decimal gap {abs(a-b):.3e} vs f32 ulp near 0.127 {2**-23 * 0.125:.3e}")

print("\n=== threshold_ge_vs_gt: can (byte/255f) ever EQUAL 0.04045 exactly? ===")
hits = [v for v in range(256) if float(f32(v/255.0)) == 0.04045]
print(f"  bytes whose normalised f32 == 0.04045 exactly: {hits}")
print(f"  0.04045*255 = {0.04045*255} (not an integer -> unreachable)")
# also the apply-gamma threshold
t = 0.00313080495356037151702786377709
print(f"\n=== applyGamma threshold {t} ===")
print(f"  t*255 = {t*255}")

print("\n=== drop_tiny_matrix_terms: is 3.1113e-10*R below the f32 ulp of the sum? ===")
for base in (1.0, 0.5, 0.1, 0.01, 0.001):
    print(f"  base={base:<7} ulp={struct.unpack('f',struct.pack('f',base))[0]*2**-23:.3e}"
          f"  tiny_term_max=3.11e-10  swamped={3.11e-10 < struct.unpack('f',struct.pack('f',base))[0]*2**-24}")
print("  when G=B=0 the tiny term is the ONLY term; trace it:")
for R in (1.0,):
    g = f32(-4.486e-11*R); b = f32(3.1113e-10*R)
    print(f"    R=1 -> Green={g:.6e} Blue={b:.6e}")
    for name,v in (("Green",g),("Blue",b)):
        lin = f32(v*12.92)            # both below the applyGamma threshold
        out = f32(lin*255.0+0.5)
        print(f"      {name}: linear={lin:.6e} denorm_arg={out!r} trunc={int(out)}")
    print(f"    with term dropped: denorm_arg=0.5 trunc={int(0.5)}")
