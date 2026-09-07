#!/usr/bin/env bash
#
# Cloud Agent environment bootstrap for PrompTTY/Warp.
#
# Idempotent, non-interactive setup that prepares a fresh Ubuntu base image to
# build, run, and test the Rust workspace. It mirrors the Linux packages from
# script/linux/install_build_deps and install_runtime_deps, but omits the
# interactive gcloud-auth step (only needed for SSH integration tests) so it can
# run unattended.

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "${REPO_ROOT}"

echo "==> Installing apt packages"
sudo apt-get update -y
sudo apt-get install -y --no-install-recommends \
  curl git ca-certificates unzip \
  build-essential cmake pkg-config python-is-python3 \
  libssl-dev libfreetype-dev libexpat1-dev libgit2-dev libfontconfig1-dev \
  jq brotli libasound2-dev libclang-dev clang-format musl-tools \
  locales fontconfig zlib1g \
  libx11-6 libxcb1 libxi6 libxcursor1 libxkbcommon-x11-0 \
  libwayland-client0 libwayland-egl1 mesa-vulkan-drivers libegl1 \
  yaru-theme-icon

# The stock image points the `cc`/`c++` alternatives at clang, but that clang
# install cannot find the C++ standard headers or libstdc++, which breaks the
# C/C++ crates in the dependency tree (esaxx-rs, onnxruntime, minimp4, ...).
# The GNU toolchain is fully functional, so make it the default for both
# compiling and linking.
echo "==> Selecting the GNU C/C++ toolchain"
sudo update-alternatives --set cc /usr/bin/gcc
sudo update-alternatives --set c++ /usr/bin/g++

# A modern protoc is required for proto3 'optional' fields (>= 3.15). The apt
# package on older Ubuntu is too old, so install a pinned release.
PROTOC_VERSION="25.1"
if ! command -v protoc >/dev/null 2>&1 || [[ "$(protoc --version 2>/dev/null)" != "libprotoc ${PROTOC_VERSION}" ]]; then
  echo "==> Installing protoc ${PROTOC_VERSION}"
  case "$(uname -m)" in
    x86_64)  PROTOC_ZIP="protoc-${PROTOC_VERSION}-linux-x86_64.zip" ;;
    aarch64) PROTOC_ZIP="protoc-${PROTOC_VERSION}-linux-aarch_64.zip" ;;
    *)       echo "Unsupported architecture for protoc: $(uname -m)" >&2; exit 1 ;;
  esac
  curl -fsSL -o /tmp/protoc.zip \
    "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${PROTOC_ZIP}"
  sudo unzip -o /tmp/protoc.zip -d /usr/local bin/protoc 'include/*'
  rm -f /tmp/protoc.zip
fi

echo "==> Fetching Git LFS assets (ONNX models, etc.)"
git lfs install --local
git lfs pull

# Warm the build cache and validate the toolchain end-to-end by building the
# headless TUI (the runnable console front-end). Uses the same invocation as
# ./script/run-tui.
echo "==> Building the headless TUI (promptty-tui)"
cargo build -p warp_tui --bin promptty-tui --features standalone

echo "✅ Environment ready. Run the TUI with ./script/run-tui"
