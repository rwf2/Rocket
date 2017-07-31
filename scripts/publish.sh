#! /usr/bin/env bash
set -e

#
# Publishes the current versions of core, contrib, and codegen to crates.io.
#

# Brings in: ROOT_DIR, EXAMPLES_DIR, LIB_DIR, CODEGEN_DIR, CONTRIB_DIR, DOC_DIR
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

if ! [ -z "$(git status --porcelain)" ]; then
  echo "There are uncommited changes! Aborting."
  exit 1
fi

# Ensure everything passes before trying to publish.
echo ":::: Running test suite..."
cargo clean
bash "${SCRIPT_DIR}/test.sh"

# Temporarily remove the dependency on codegen from core so crates.io verifies.
sed -i.bak 's/rocket_codegen.*//' "${LIB_DIR}/Cargo.toml"

# Publish all the things.
for dir in "${LIB_DIR}" "${CODEGEN_DIR}" "${CONTRIB_DIR}"; do
  pushd "${dir}"
  echo ":::: Publishing '${dir}..."
  # We already checked things ourselves. Don't spend time reverifying.
  cargo publish --no-verify --allow-dirty
  popd
done

# Restore the original core Cargo.toml.
mv "${LIB_DIR}/Cargo.toml.bak" "${LIB_DIR}/Cargo.toml"
