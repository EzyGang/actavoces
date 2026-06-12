#!/usr/bin/env bash
set -euo pipefail

asset_url="${FFMPEG_URL:-https://github.com/Tyrrrz/FFmpegBin/releases/latest/download/${FFMPEG_ASSET}}"
archive_path=".release/ffmpeg/${FFMPEG_ASSET}"
extract_path=".release/ffmpeg/extracted"
runtime_path="src-tauri/resources/runtime/ffmpeg/${RUST_TARGET}"

mkdir -p ".release/ffmpeg" "${extract_path}" "${runtime_path}"
curl -fsSL "${asset_url}" -o "${archive_path}"

case "${archive_path}" in
  *.zip)
    unzip -q "${archive_path}" -d "${extract_path}"
    ;;
  *.tar.xz | *.txz)
    tar -xJf "${archive_path}" -C "${extract_path}"
    ;;
  *.tar.gz | *.tgz)
    tar -xzf "${archive_path}" -C "${extract_path}"
    ;;
  *)
    echo "Unsupported FFmpeg archive format: ${archive_path}" >&2
    exit 1
    ;;
esac

binary_path="$(find "${extract_path}" -type f -name "${FFMPEG_BINARY}" | head -n 1)"

if [ -z "${binary_path}" ]; then
  echo "Unable to find extracted FFmpeg binary: ${FFMPEG_BINARY}" >&2
  exit 1
fi

cp "${binary_path}" "${runtime_path}/${FFMPEG_BINARY}"
chmod +x "${runtime_path}/${FFMPEG_BINARY}"
"${runtime_path}/${FFMPEG_BINARY}" -version
