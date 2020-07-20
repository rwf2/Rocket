#! /usr/bin/env bash
set -e

#
# Publishes the current versions of all Rocket crates to crates.io.
#

# Brings in _ROOT, _DIR, _DIRS globals.
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

function strip_dev_dependencies() {
  perl -i.bak -p0e 's/\[dev-dependencies\].*//smg' "${1}/Cargo.toml"
}

function restore_dev_dependencies() {
  mv "${1}/Cargo.toml.bak" "${1}/Cargo.toml"
}

if ! [ -z "$(git status --porcelain)" ]; then
  echo "There are uncommitted changes! Aborting."
  exit 1
fi

# Ensure everything passes before trying to publish.
echo ":::: Running test suite..."
cargo clean
bash "${SCRIPT_DIR}/test.sh"
bash "${SCRIPT_DIR}/test.sh" --contrib
bash "${SCRIPT_DIR}/test.sh" --release

# Temporarily remove dev-dependencies so crates.io verifies.
echo ":::: Stripping [dev-dependencies]..."
for dir in "${ALL_PROJECT_DIRS[@]}"; do
  strip_dev_dependencies "${dir}"
done

# Publish all the things.
for dir in "${ALL_PROJECT_DIRS[@]}"; do
  pushd "${dir}"
  echo ":::: Publishing '${dir}'..."
  # We already checked things ourselves. Don't spend time reverifying.
  cargo publish --no-verify --allow-dirty ${@:1}
  popd
done

# Restore dev-dependencies.
echo ":::: Restoring [dev-dependencies]..."
for dir in "${ALL_PROJECT_DIRS[@]}"; do
  restore_dev_dependencies "${dir}"
done
