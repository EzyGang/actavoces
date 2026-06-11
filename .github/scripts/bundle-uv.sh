#!/usr/bin/env bash
set -euo pipefail

asset_url="https://github.com/astral-sh/uv/releases/latest/download/${UV_ASSET}"
checksum_url="${asset_url}.sha256"
archive_path=".release/uv/${UV_ASSET}"
checksum_path="${archive_path}.sha256"
extract_path=".release/uv/extracted"
runtime_path="src-tauri/resources/runtime/uv"

mkdir -p ".release/uv" "${extract_path}" "${runtime_path}"
curl -fsSL "${asset_url}" -o "${archive_path}"
curl -fsSL "${checksum_url}" -o "${checksum_path}"

expected_hash="$(grep -Eo '[a-fA-F0-9]{64}' "${checksum_path}" | head -n 1 | tr 'A-F' 'a-f')"
if command -v sha256sum >/dev/null 2>&1; then
  actual_hash="$(sha256sum "${archive_path}" | awk '{print $1}')"
else
  actual_hash="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
fi

if [ "${expected_hash}" != "${actual_hash}" ]; then
  echo "uv checksum mismatch for ${UV_ASSET}" >&2
  exit 1
fi

tar -xzf "${archive_path}" -C "${extract_path}"
binary_path="$(find "${extract_path}" -type f -name "${UV_BINARY}" | head -n 1)"

if [ -z "${binary_path}" ]; then
  echo "Unable to find extracted uv binary: ${UV_BINARY}" >&2
  exit 1
fi

cp "${binary_path}" "${runtime_path}/${UV_BINARY}"
chmod +x "${runtime_path}/${UV_BINARY}"
"${runtime_path}/${UV_BINARY}" --version
