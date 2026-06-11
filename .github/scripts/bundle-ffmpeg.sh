#!/usr/bin/env bash
set -euo pipefail

asset_url="https://github.com/Tyrrrz/FFmpegBin/releases/latest/download/${FFMPEG_ASSET}"
archive_path=".release/ffmpeg/${FFMPEG_ASSET}"
extract_path=".release/ffmpeg/extracted"
runtime_path="src-tauri/resources/runtime/ffmpeg/${RUST_TARGET}"

mkdir -p ".release/ffmpeg" "${extract_path}" "${runtime_path}"
curl -fsSL "${asset_url}" -o "${archive_path}"

unzip -q "${archive_path}" -d "${extract_path}"
binary_path="$(find "${extract_path}" -type f -name "${FFMPEG_BINARY}" | head -n 1)"

if [ -z "${binary_path}" ]; then
  echo "Unable to find extracted FFmpeg binary: ${FFMPEG_BINARY}" >&2
  exit 1
fi

cp "${binary_path}" "${runtime_path}/${FFMPEG_BINARY}"
chmod +x "${runtime_path}/${FFMPEG_BINARY}"
"${runtime_path}/${FFMPEG_BINARY}" -version
