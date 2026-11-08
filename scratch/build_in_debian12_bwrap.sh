#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_ENV="$ROOT_DIR/build_env_debian12"
ROOTFS="$ROOT_DIR/os/build_real/rootfs"

echo "========================================================================"
echo " [BHOOT] Building Release Workspace in Debian 12 Environment (bwrap)"
echo "========================================================================"

if [ ! -f "$BUILD_ENV/usr/bin/gcc" ]; then
    echo "1. Initializing isolated Debian 12 build rootfs..."
    rm -rf "$BUILD_ENV"
    mkdir -p "$BUILD_ENV"
    cp -a "$ROOTFS"/* "$BUILD_ENV"/
    cp /etc/resolv.conf "$BUILD_ENV/etc/resolv.conf"
    echo "deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] http://deb.debian.org/debian bookworm main" > "$BUILD_ENV/etc/apt/sources.list"

    echo "2. Installing build toolchain in Debian 12 environment..."
    bwrap \
        --uid 0 --gid 0 \
        --bind "$BUILD_ENV" / \
        --dev /dev \
        --proc /proc \
        --ro-bind /etc/resolv.conf /etc/resolv.conf \
        /bin/sh -c "
            set -eu
            apt-get update -o APT::Sandbox::User=root
            apt-get install -o APT::Sandbox::User=root -y --no-install-recommends build-essential libssl-dev pkg-config
        "
fi

echo "3. Compiling Rust workspace natively against Debian 12 glibc 2.36..."
bwrap \
    --uid 0 --gid 0 \
    --bind "$BUILD_ENV" / \
    --dev /dev \
    --proc /proc \
    --ro-bind /home/bhoot/.cargo /root/.cargo \
    --ro-bind /home/bhoot/.rustup /root/.rustup \
    --bind "$ROOT_DIR" /workspace \
    /bin/sh -c "
        set -eu
        export PATH=\"/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"
        export RUSTUP_HOME=\"/root/.rustup\"
        export CARGO_HOME=\"/root/.cargo\"
        export RUSTUP_TOOLCHAIN=\"stable-x86_64-unknown-linux-gnu\"
        export CC=\"/usr/bin/gcc\"
        export CXX=\"/usr/bin/g++\"

        echo \"  Compiler: \$(gcc --version | head -n 1)\"
        echo \"  Rustc:    \$(rustc --version)\"
        echo \"  Cargo:    \$(cargo --version)\"

        cd /workspace
        CARGO_TARGET_DIR=/workspace/target_debian12 cargo build --release --workspace
    "

echo "4. Installing compiled binary to OS rootfs..."
cp "$ROOT_DIR/target_debian12/release/bhoot-desktop" "$ROOTFS/usr/local/bin/bhoot-desktop"
chmod 0755 "$ROOTFS/usr/local/bin/bhoot-desktop"

echo "5. Checking GLIBC ABI requirements..."
GLIBC_REQ=$(objdump -T "$ROOTFS/usr/local/bin/bhoot-desktop" | grep -o 'GLIBC_[0-9.]*' | sort -V | tail -n 1)
echo "  Maximum GLIBC Symbol Version Required: $GLIBC_REQ"
if [ "$GLIBC_REQ" = "GLIBC_2.38" ] || [ "$GLIBC_REQ" = "GLIBC_2.39" ] || [ "$GLIBC_REQ" = "GLIBC_2.40" ]; then
    echo "  ERROR: Incompatible GLIBC requirement ($GLIBC_REQ)!" >&2
    exit 1
fi

echo "========================================================================"
echo " [BHOOT] Native Debian 12 Build Complete: $ROOTFS/usr/local/bin/bhoot-desktop"
echo "========================================================================"
