#!/usr/bin/env bash
set -euo pipefail

version="$(node -p "require('./package.json').version")"
if [ -z "${version}" ]; then
  echo "package.json version is empty" >&2
  exit 1
fi

if [ "${RELEASE_TYPE}" = "alpha" ]; then
  latest_alpha="$(git tag --list "v${version}-alpha.*" --sort=-v:refname | head -n 1 || true)"
  if [ -z "${latest_alpha}" ]; then
    alpha_num=1
    previous_tag="$(git tag --list 'v*' --sort=-v:refname | grep -v -- '-alpha' | head -n 1 || true)"
  else
    alpha_num="$(echo "${latest_alpha}" | sed 's/.*-alpha\.//')"
    alpha_num="$((alpha_num + 1))"
    previous_tag="${latest_alpha}"
  fi
  tag="v${version}-alpha.${alpha_num}"
  prerelease="true"
else
  previous_tag="$(git tag --list 'v*' --sort=-v:refname | grep -v -- '-alpha' | head -n 1 || true)"
  tag="v${version}"
  prerelease="false"
fi

{
  echo "version=${version}"
  echo "tag=${tag}"
  echo "name=${tag}"
  echo "prerelease=${prerelease}"
  echo "previous_tag=${previous_tag:-}"
} >> "${GITHUB_OUTPUT}"
