import sys, subprocess, shutil, os, re
CASES = [
 ("rem_euclid instead of C truncating % for param3",
  "wrapping_add(param3 % 10)", "wrapping_add(param3.rem_euclid(10))"),
 ("rem_euclid instead of C truncating % for param4",
  "confuse_types(state, param4 % 4)", "confuse_types(state, param4.rem_euclid(4))"),
 ("zero-extend negative capacity (loses malloc failure)",
  "malloc(capacity as usize)", "malloc(capacity as u32 as usize)"),
 ("uint mask 0xFF -> 0xFFF in confuse_types op2",
  "(s.data.uint_val() & 0xFF) as c_int", "(s.data.uint_val() & 0xFFF) as c_int"),
 ("mode weight 3 -> 4 in confusion",
  "(s.flags.mode() as c_int).wrapping_mul(3)", "(s.flags.mode() as c_int).wrapping_mul(4)"),
 ("process_buffer off-by-one on remaining",
  "remaining -= consumed;", "remaining = remaining.saturating_sub(consumed.max(2));"),
 ("float multiply in f64 instead of f32",
  "s.data.float_val() * 100.0f32", "(s.data.float_val() as f64 * 100.0f64) as f32"),
 ("byte sum without sign extension",
  "(s.data.byte(0) as c_int).wrapping_add(s.data.byte(1) as c_int)",
  "((s.data.byte(0) as u8) as c_int).wrapping_add((s.data.byte(1) as u8) as c_int)"),
 ("initial status 15 -> 14",
  "s.flags.set_status(15);", "s.flags.set_status(14);"),
 ("snprintf given capacity-1",
  "capacity as usize,\n            c\"State:%d:Mode:%d\"",
  "(capacity as usize).saturating_sub(1),\n            c\"State:%d:Mode:%d\""),
]
src = open("translation/src/lib.rs").read()
for name, old, new in CASES:
    if old not in src:
        print(f"MUTATION [{name}]: PATTERN NOT FOUND -- skipped"); continue
    shutil.rmtree("/tmp/mut", ignore_errors=True); os.makedirs("/tmp/mut")
    shutil.copytree("translation/src", "/tmp/mut/src")
    for f in ("Cargo.toml", "Cargo.lock"):
        shutil.copy(f"translation/{f}", f"/tmp/mut/{f}")
    open("/tmp/mut/src/lib.rs","w").write(src.replace(old, new, 1))
    b = subprocess.run(["cargo","build","--release"], cwd="/tmp/mut",
                       capture_output=True, text=True, timeout=300)
    if b.returncode != 0:
        print(f"MUTATION [{name}]: BUILD FAILED"); continue
    env = dict(os.environ, RUST_SO_PATH="/tmp/mut/target/release/libconfusion_lib.so")
    t = subprocess.run(["cargo","test","--release","--tests"], cwd="translation",
                       capture_output=True, text=True, env=env, timeout=600)
    n = len(re.findall(r"^test .* FAILED", t.stdout, re.M))
    print(f"MUTATION [{name}]: {n} failing tests")
