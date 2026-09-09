import os, shutil, subprocess, sys, json

ROOT = "$HARVEST_WORKDIR"
SRC  = os.path.join(ROOT, "translation")

MUTANTS = {
  # name: (file, old, new, description)
  "lz4_hash4_byU16": ("src/lz4.rs",
      "(sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 1))",
      "(sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 2))",
      "byU16 hash shift off by one -> different match choices for srcSize < 64KB"),
  "lz4_hash5": ("src/lz4.rs",
      "let prime5bytes: u64 = 889523592379u64;",
      "let prime5bytes: u64 = 889523592377u64;",
      "byU32/byPtr hash prime changed -> different match choices for large inputs"),
  "xxh32_prime": ("src/xxhash.rs",
      "const PRIME32_2: u32 = 2246822519u32;",
      "const PRIME32_2: u32 = 2246822521u32;",
      "XXH32 round prime changed -> every XXH32 value differs"),
  "hc_pattern_analysis": ("src/lz4hc.rs",
      "let pattern_analysis: c_int = if max_nb_attempts > 128 { 1 } else { 0 };",
      "let pattern_analysis: c_int = if max_nb_attempts >= 128 { 1 } else { 0 };",
      "HC patternAnalysis threshold >128 -> >=128, changes level 8 output"),
  "frame_header_checksum": ("src/lz4frame.rs",
      "    (xxh >> 8) as u8",
      "    (xxh >> 7) as u8",
      "frame header checksum byte shift -> every frame header differs"),
  "file_maxwrite_256k": ("src/lz4file.rs",
      "                (**lz4f_write).maxWriteSize = 256 * 1024;",
      "                (**lz4f_write).maxWriteSize = 255 * 1024;",
      "lz4file writeOpen chunking for 256KB blocks -> different block boundaries in the produced file"),
  "file_srcbuf_tiny": ("src/lz4file.rs",
      "                (**lz4f_read).srcBufMaxSize = 256 * 1024;",
      "                (**lz4f_read).srcBufMaxSize = 1;",
      "lz4file read buffer for 256KB blocks reduced to ONE byte -- probes whether srcBufMaxSize is observable at all"),
  "file_srcbuf_256k": ("src/lz4file.rs",
      "                (**lz4f_read).srcBufMaxSize = 256 * 1024;",
      "                (**lz4f_read).srcBufMaxSize = 255 * 1024;",
      "lz4file read buffer for 256KB blocks is one KiB short"),
}

name = sys.argv[1]
f, old, new, desc = MUTANTS[name]
d = os.path.join(ROOT, "work", "mut_" + name)
if os.path.exists(d):
    shutil.rmtree(d)
os.makedirs(d)
shutil.copytree(os.path.join(SRC, "src"), os.path.join(d, "src"))
shutil.copy(os.path.join(SRC, "Cargo.toml"), d)
shutil.copy(os.path.join(SRC, "Cargo.lock"), d)
p = os.path.join(d, f)
s = open(p).read()
n = s.count(old)
if n != 1:
    print("MUTATION-TARGET-NOT-UNIQUE count=%d for %s" % (n, name)); sys.exit(2)
open(p, "w").write(s.replace(old, new))
print("mutant %-22s : %s" % (name, desc))
print(d)
