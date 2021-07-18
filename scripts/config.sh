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

function future_date() {
  local days_in_future=`[[ -z "$1" ]] && echo "0" || echo "$1"`
  if date -v+1d +%Y-%m-%d > /dev/null 2>&1; then
    echo $(date -v+${days_in_future}d +%Y-%m-%d)
  elif date -d "+1 day" > /dev/null 2>&1; then
    echo $(date '+%Y-%m-%d' -d "+${days_in_future} days")
  else
    echo "Error: need a 'date' cmd that accepts -v (BSD) or -d (GNU)"
    exit 1
  fi
}

# Root of workspace-like directories.
PROJECT_ROOT=$(relative "") || exit $?
CORE_ROOT=$(relative "core") || exit $?
CONTRIB_ROOT=$(relative "contrib") || exit $?
SITE_ROOT=$(relative "site") || exit $?
BENCHMARKS_ROOT=$(relative "benchmarks") || exit $?
FUZZ_ROOT=$(relative "core/lib/fuzz") || exit $?

# Root of project-like directories.
CORE_LIB_ROOT=$(relative "core/lib") || exit $?
CORE_CODEGEN_ROOT=$(relative "core/codegen") || exit $?
CORE_HTTP_ROOT=$(relative "core/http") || exit $?
GUIDE_TESTS_ROOT=$(relative "site/tests") || exit $?

# Root of infrastructure directories.
EXAMPLES_DIR=$(relative "examples") || exit $?
DOC_DIR=$(relative "target/doc") || exit $?

# Versioning information. These are changed as versions change.
VERSION=$(git grep -h "^version" "${CORE_LIB_ROOT}" | head -n 1 | cut -d '"' -f2)
MAJOR_VERSION=$(echo "${VERSION}" | cut -d'.' -f1-2)
VIRTUAL_CODENAME="$(git branch --show-current)"
PHYSICAL_CODENAME="v${MAJOR_VERSION}"
CURRENT_RELEASE=true
PRE_RELEASE=true

# A generated codename for this version. Use the git branch for pre-releases.
case $PRE_RELEASE in
  true)
    CODENAME="${VIRTUAL_CODENAME}"
    DOC_VERSION="${VERSION}-$(future_date)"
    ;;
  false)
    CODENAME="${PHYSICAL_CODENAME}"
    DOC_VERSION="${VERSION}"
    ;;
esac

CORE_CRATE_ROOTS=(
    "${CORE_HTTP_ROOT}"
    "${CORE_CODEGEN_ROOT}"
    "${CORE_LIB_ROOT}"
)

CONTRIB_SYNC_DB_POOLS_CRATE_ROOTS=(
    "${CONTRIB_ROOT}/sync_db_pools/lib"
    "${CONTRIB_ROOT}/sync_db_pools/codegen"
)

CONTRIB_DB_POOLS_CRATE_ROOTS=(
    "${CONTRIB_ROOT}/db_pools/lib"
    "${CONTRIB_ROOT}/db_pools/codegen"
)

ALL_CRATE_ROOTS=(
    "${CORE_HTTP_ROOT}"
    "${CORE_CODEGEN_ROOT}"
    "${CORE_LIB_ROOT}"
    "${CONTRIB_ROOT}/sync_db_pools/codegen"
    "${CONTRIB_ROOT}/sync_db_pools/lib"
    "${CONTRIB_ROOT}/dyn_templates"
)

function print_environment() {
  echo "  VERSION: ${VERSION}"
  echo "  MAJOR_VERSION: ${MAJOR_VERSION}"
  echo "  CODENAME: ${CODENAME}"
  echo "  DOC_VERSION: ${DOC_VERSION}"
  echo "  CURRENT_RELEASE: ${CURRENT_RELEASE}"
  echo "  PRE_RELEASE: ${PRE_RELEASE}"
  echo "  SCRIPT_DIR: ${SCRIPT_DIR}"
  echo "  PROJECT_ROOT: ${PROJECT_ROOT}"
  echo "  CORE_ROOT: ${CORE_ROOT}"
  echo "  CONTRIB_ROOT: ${CONTRIB_ROOT}"
  echo "  SITE_ROOT: ${SITE_ROOT}"
  echo "  BENCHMARKS_ROOT: ${BENCHMARKS_ROOT}"
  echo "  CORE_LIB_ROOT: ${CORE_LIB_ROOT}"
  echo "  CORE_CODEGEN_ROOT: ${CORE_CODEGEN_ROOT}"
  echo "  CORE_HTTP_ROOT: ${CORE_HTTP_ROOT}"
  echo "  GUIDE_TESTS_ROOT: ${GUIDE_TESTS_ROOT}"
  echo "  EXAMPLES_DIR: ${EXAMPLES_DIR}"
  echo "  DOC_DIR: ${DOC_DIR}"
  echo "  ALL_CRATE_ROOTS: ${ALL_CRATE_ROOTS[*]}"
  echo "  date(): $(future_date)"
}

if [ "${1}" = "-p" ]; then
  print_environment
fi
