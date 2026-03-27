#!/bin/bash
# Build the cdylib with main, _init, _fini exports matching the C .so
# Run from the translated_rust/ directory after `cargo build`
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Ensure rlib is built
cargo build

RLIB=$(find target/debug/deps -name 'libdriver-*.rlib' | head -1)

# Create a cc wrapper that patches the version script to export _init/_fini
WRAPPER=$(mktemp)
cat > "$WRAPPER" << 'CCWRAP'
#!/bin/bash
args=()
for arg in "$@"; do
  case "$arg" in
    -Wl,--version-script=*)
      path="${arg#-Wl,--version-script=}"
      [ -f "$path" ] && sed -i 's/local:/  _init;\n    _fini;\n\n  local:/' "$path"
      ;;
  esac
  args+=("$arg")
done
exec cc "${args[@]}"
CCWRAP
chmod +x "$WRAPPER"

rustc --edition 2021 \
  --crate-type cdylib \
  --crate-name driver \
  src/cdylib.rs \
  --extern "driver=$RLIB" \
  -L "dependency=target/debug/deps" \
  -o target/debug/libdriver.so \
  -C "linker=$WRAPPER"

rm -f "$WRAPPER"
echo "Built target/debug/libdriver.so"
nm -D target/debug/libdriver.so | grep ' T '
