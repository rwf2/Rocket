#!/usr/bin/env bash
set -e

# Symlinks all of the tests in this directory with those in sibling
# `ui-fail-stable` and `ui-fail-nightly` directories.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

stable="${SCRIPT_DIR}/../ui-fail-stable"
nightly="${SCRIPT_DIR}/../ui-fail-nightly"
anchor="$(basename ${SCRIPT_DIR})"

echo ":: Synchronizing..."
echo "   stable: ${stable}"
echo "   nightly: ${nightly}"
echo "   anchor: ${anchor}"

for dir in "${stable}" "${nightly}"; do
  find "${dir}" -type l -delete

  for file in "${SCRIPT_DIR}"/*.rs; do
    ln -s "../${anchor}/$(basename $file)" "${dir}/"
  done
done
