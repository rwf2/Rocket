#! /usr/bin/env bash
set -e

#
# Publishes the current versions of all Rocket crates to crates.io.
#

# Brings in _ROOT, _DIR, _DIRS globals.
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

if ! [ -z "$(git status --porcelain)" ]; then
  echo "There are uncommitted changes! Aborting."
  exit 1
fi

# Ensure everything passes before trying to publish.
echo ":::: Running complete test suite..."
cargo clean
bash "${SCRIPT_DIR}/test.sh" +stable --all
bash "${SCRIPT_DIR}/test.sh" +stable --all --release

# Publish all the things.
for dir in "${ALL_CRATE_ROOTS[@]}"; do
  pushd "${dir}"
  echo ":::: Publishing '${dir}'..."
  # We already checked things ourselves. Don't spend time reverifying.
  cargo publish --no-verify --allow-dirty ${@:1}
  # Give the index some time to update so the deps are there if we need them.
  sleep 5
  popd
done
