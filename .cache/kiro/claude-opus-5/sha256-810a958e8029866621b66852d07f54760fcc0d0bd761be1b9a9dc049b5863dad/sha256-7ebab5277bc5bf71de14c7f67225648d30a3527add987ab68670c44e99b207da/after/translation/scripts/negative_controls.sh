#!/usr/bin/env bash
# Negative controls: deliberately break the Rust translation in one spot at a
# time and confirm the differential tests notice.  Every mutation is reverted.
set -u
cd "$(dirname "$0")/.." || exit 1

backup=$(mktemp -d)
cp -r src "$backup/src"
restore() { rm -rf src; cp -r "$backup/src" src; }
trap 'restore; rm -rf "$backup"' EXIT

fail=0
mutate() {
  local name="$1" file="$2" from="$3" to="$4" tests="$5"
  restore
  if ! grep -qF -- "$from" "$file"; then
    echo "SKIP  $name (pattern not found in $file)"
    return
  fi
  python3 - "$file" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
open(p, "w").write(s.replace(a, b, 1))
PY
  rm -f target/release/libdriver.so
  if timeout 600 cargo test $tests > /tmp/negctl.log 2>&1; then
    echo "BAD   $name -- tests still PASSED (blind spot)"
    case "$name" in *"expected no-op"*) ;; *) fail=1 ;; esac
  else
    echo "GOOD  $name -- detected"
  fi
}

mutate "log prefix [INFO]->[info]" src/logger.rs \
  'write_entry(b"[INFO] ", message)' 'write_entry(b"[info] ", message)' \
  '--test level1_logger'

# NOTE: `strncpy(dst, src, 256)` is *equivalent* to `strncpy(dst, src, 255)`
# here, because add_task immediately does `dst[255] = '\\0'` either way.  Kept as
# a documented no-op control: it is expected to report BAD.
mutate "strncpy length 255->256 (expected no-op)" src/task_manager.rs \
  'strncpy(desc, description, 256 - 1)' 'strncpy(desc, description, 256)' \
  '--test level2_task_manager'

mutate "default MAX_TASKS 10->11" src/task_manager.rs \
  'None => 10,' 'None => 11,' \
  '--test level2_task_manager'

mutate "atoi saturation removed" src/cutil.rs \
  'as_long as c_int' 'as_long.clamp(i32::MIN as i64, i32::MAX as i64) as c_int' \
  '--test level2_task_manager'

mutate "print_tasks index i+1 -> i" src/task_manager.rs \
  'i.wrapping_add(1).to_string()' 'i.to_string()' \
  '--test level2_task_manager'

mutate "driver EXIT_FAILURE 1->2" src/driver.rs \
  'const EXIT_FAILURE: c_int = 1;' 'const EXIT_FAILURE: c_int = 2;' \
  '--test level3_driver'

mutate "driver priority starts at 0" src/driver.rs \
  'let mut priority: c_int = 1;' 'let mut priority: c_int = 0;' \
  '--test level3_driver'

mutate "driver skips trailing segment" src/driver.rs \
  'while *start != 0 {' 'while *start != 0 && *start != b(0) {' \
  '--test level3_driver'

mutate "logger open truncates instead of appends" src/logger.rs \
  'OpenOptions::new().append(true).create(true).open(path).ok()' \
  'OpenOptions::new().write(true).truncate(true).create(true).open(path).ok()' \
  '--test level1_logger'

# --- stdio buffering mode (glibc _IO_file_doallocate) ----------------------

mutate "line buffering never enabled" src/stdio_stream.rs \
  'if md.mode() & S_IFMT == S_IFCHR && unsafe { isatty(file.as_raw_fd()) } != 0 {' \
  'if false {' \
  '--test level7_line_buffered'

mutate "line buffering always enabled" src/stdio_stream.rs \
  'if md.mode() & S_IFMT == S_IFCHR && unsafe { isatty(file.as_raw_fd()) } != 0 {' \
  'if true {' \
  '--test level1_logger --test level4_process_exit'

mutate "buffer size ignores st_blksize" src/stdio_stream.rs \
  '            if blksize > 0 && blksize < BUFSIZ {
                capacity = blksize;
            }' '' \
  '--test level1_logger'

mutate "line flush stops at first newline" src/stdio_stream.rs \
  'match self.buf.iter().rposition(|&b| b == b'"'"'\n'"'"') {' \
  'match self.buf.iter().position(|&b| b == b'"'"'\n'"'"') {' \
  '--test level7_line_buffered'

mutate "no_mangle removed from print_tasks" src/task_manager.rs \
  '#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks' \
  'pub unsafe extern "C" fn print_tasks' \
  '--test level2_task_manager'

restore
rm -f target/release/libdriver.so
echo "--- rebuilding pristine ---"
timeout 600 cargo build --release --lib > /dev/null 2>&1 || { echo "pristine rebuild FAILED"; exit 1; }
exit $fail
