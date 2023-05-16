#!/usr/bin/env bash
set -euo pipefail

# allow overriding the version
VERSION=${RYE_VERSION:-latest}

REPO=mitsuhiko/rye
PLATFORM=`uname -s`
ARCH=`uname -m`

if [[ $PLATFORM == "Darwin" ]]; then
  PLATFORM="macos"
elif [[ $PLATFORM == "Linux" ]]; then
  PLATFORM="linux"
else
  echo "error: Unsupported platform $PLATFORM";
  exit 1
fi

if [[ $ARCH == armv8* ]] || [[ $ARCH == arm64* ]] || [[ $ARCH == aarch64* ]]; then
  ARCH="aarch64"
elif [[ $ARCH == i686* ]]; then
  ARCH="x86"
fi

BINARY="rye-${ARCH}-${PLATFORM}"

# Oddly enough GitHub has different URLs for latest vs specific version
if [[ $VERSION == "latest" ]]; then
  DOWNLOAD_URL=https://github.com/${REPO}/releases/latest/download/${BINARY}
else
  DOWNLOAD_URL=https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}
fi

echo "This script will automatically download and install rye (${VERSION}) for you."
if [ "x$(id -u)" == "x0" ]; then
  echo "warning: this script is running as root.  This is dangerous and unnecessary!"
fi

if ! hash curl 2> /dev/null; then
  echo "error: you do not have 'curl' installed which is required for this script."
  exit 1
fi

TEMP_FILE=`mktemp "${TMPDIR:-/tmp}/.ryeinstall.XXXXXXXX"`

cleanup() {
  rm -f "$TEMP_FILE"
}

trap cleanup EXIT
HTTP_CODE=$(curl -SL --progress-bar "$DOWNLOAD_URL" --output "$TEMP_FILE" --write-out "%{http_code}")
if [[ ${HTTP_CODE} -lt 200 || ${HTTP_CODE} -gt 299 ]]; then
  echo "error: your platform and architecture (${PLATFORM}-${ARCH}) is unsupported."
  exit 1
fi

chmod +x "$TEMP_FILE"
"$TEMP_FILE" self install
