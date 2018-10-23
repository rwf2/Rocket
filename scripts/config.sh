# Simply sets up a few useful variables.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

function relative() {
  local full_path="${SCRIPT_DIR}/../${1}"

  if [ -d "${full_path}" ]; then
    # Try to use readlink as a fallback to readpath for cross-platform compat.
    if command -v realpath >/dev/null 2>&1; then
      realpath "${full_path}"
    elif ! (readlink -f 2>&1 | grep illegal > /dev/null); then
      readlink -f "${full_path}"
    else
      echo "Rocket's scripts require 'realpath' or 'readlink -f' support." >&2
      echo "Install realpath or GNU readlink via your package manager." >&2
      echo "Aborting." >&2
      exit 1
    fi
  else
    # when the directory doesn't exist, fallback to this.
    echo "${full_path}"
  fi
}

ROCKET_VERSION="0.4.0-dev"
CURRENT_RELEASE=false

# Root of workspace-like directories.
PROJECT_ROOT=$(relative "") || exit $?
CORE_ROOT=$(relative "core") || exit $?
CONTRIB_ROOT=$(relative "contrib") || exit $?
SITE_ROOT=$(relative "site") || exit $?

# Root of project-like directories.
CORE_LIB_ROOT=$(relative "core/lib") || exit $?
CORE_CODEGEN_ROOT=$(relative "core/codegen") || exit $?
CORE_HTTP_ROOT=$(relative "core/http") || exit $?
CONTRIB_LIB_ROOT=$(relative "contrib/lib") || exit $?
CONTRIB_CODEGEN_ROOT=$(relative "contrib/codegen") || exit $?

# Root of infrastructure directories.
EXAMPLES_DIR=$(relative "examples") || exit $?
DOC_DIR=$(relative "target/doc") || exit $?

ALL_PROJECT_DIRS=(
    "${CORE_LIB_ROOT}"
    "${CORE_CODEGEN_ROOT}"
    "${CORE_HTTP_ROOT}"
    "${CONTRIB_LIB_ROOT}"
    "${CONTRIB_CODEGEN_ROOT}"
)

if [ "${1}" = "-p" ]; then
  echo "SCRIPT_DIR: ${SCRIPT_DIR}"
  echo "PROJECT_ROOT: ${PROJECT_ROOT}"
  echo "CORE_ROOT: ${CORE_ROOT}"
  echo "CONTRIB_ROOT: ${CONTRIB_ROOT}"
  echo "SITE_ROOT: ${SITE_ROOT}"
  echo "CORE_LIB_ROOT: ${CORE_LIB_ROOT}"
  echo "CORE_CODEGEN_ROOT: ${CORE_CODEGEN_ROOT}"
  echo "CORE_HTTP_ROOT: ${CORE_HTTP_ROOT}"
  echo "CONTRIB_LIB_ROOT: ${CONTRIB_LIB_ROOT}"
  echo "CONTRIB_CODEGEN_ROOT: ${CONTRIB_CODEGEN_ROOT}"
  echo "EXAMPLES_DIR: ${EXAMPLES_DIR}"
  echo "DOC_DIR: ${DOC_DIR}"
  echo "ALL_PROJECT_DIRS: ${ALL_PROJECT_DIRS[*]}"
fi
