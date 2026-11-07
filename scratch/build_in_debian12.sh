#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_ENV="$ROOT_DIR/build_env_debian12"

echo "========================================================================"
echo " [BHOOT] Building Release Workspace Inside Isolated Debian 12 Environment"
echo "========================================================================"

mkdir -p "$BUILD_ENV/root/.cargo" "$BUILD_ENV/root/.rustup" "$BUILD_ENV/workspace" "$BUILD_ENV/dev" "$BUILD_ENV/proc"

unshare -U -p -f -r -m sh -c "
    mount -t proc proc '$BUILD_ENV/proc'
    mount --bind /dev '$BUILD_ENV/dev'
    mount --bind /home/bhoot/.cargo '$BUILD_ENV/root/.cargo'
    mount --bind /home/bhoot/.rustup '$BUILD_ENV/root/.rustup'
    mount --bind '$ROOT_DIR' '$BUILD_ENV/workspace'

    chroot '$BUILD_ENV' sh -c '
        set -eu
        export PATH=\"/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"
        export RUSTUP_HOME=\"/root/.rustup\"
        export CARGO_HOME=\"/root/.cargo\"
        export RUSTUP_TOOLCHAIN=\"stable-x86_64-unknown-linux-gnu\"
        export CC=\"/usr/bin/gcc\"
        export CXX=\"/usr/bin/g++\"

        echo \"Inside Debian 12 Chroot:\"
        gcc --version | head -n 1
        rustc --version
        cargo --version

        cd /workspace
        echo \"Cleaning previous debian12 target...\"
        rm -rf target_debian12

        echo \"Building workspace in --release mode against Debian 12 native libraries...\"
        CARGO_TARGET_DIR=/workspace/target_debian12 cargo build --release --workspace

        echo \"Workspace build completed successfully.\"
    '

    umount -l '$BUILD_ENV/workspace' 2>/dev/null || true
    umount -l '$BUILD_ENV/root/.rustup' 2>/dev/null || true
    umount -l '$BUILD_ENV/root/.cargo' 2>/dev/null || true
    umount -l '$BUILD_ENV/dev' 2>/dev/null || true
    umount -l '$BUILD_ENV/proc' 2>/dev/null || true
"

echo "========================================================================"
echo " [BHOOT] Debian 12 Native Compilation Complete"
echo "========================================================================"

