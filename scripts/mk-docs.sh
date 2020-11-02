#!/bin/bash
set -e

#
# Builds the rustdocs for all of the libraries.
#

# Brings in: PROJECT_ROOT, EXAMPLES_DIR, LIB_DIR, CODEGEN_DIR, CONTRIB_DIR, DOC_DIR
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

if [ "${1}" != "-d" ]; then
  # We need to clean-up beforehand so we don't get all of the dependencies.
  echo ":::: Cleaning up before documenting..."
  cargo clean
  cargo update
fi

# Generate the rustdocs for all of the crates.
echo ":::: Generating the docs..."
pushd "${PROJECT_ROOT}" > /dev/null 2>&1
RUSTDOCFLAGS="-Z unstable-options --crate-version ${DOC_VERSION}" \
  cargo doc -p rocket -p rocket_contrib --no-deps --all-features
popd > /dev/null 2>&1

# Blank index, for redirection.
touch "${DOC_DIR}/index.html"
