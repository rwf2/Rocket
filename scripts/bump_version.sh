#! /usr/bin/env bash

#
# Bumps the version number to ${1}.
#

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

if [ -z "${1}" ] ; then
  echo "Usage: $0 <new-version>"
  echo "Example: $0 0.6.1"
  exit 1
fi

function do_replace_docs() {
  sd "${1}" "${2}" $(fd -t f -e toml -E '/news/*' . "${PROJECT_ROOT}")
  sd "${1}" "${2}" $(fd -t f -e md -E '/news/*' . "${SITE_ROOT}")
}

function do_replace_all() {
  sd "${1}" "${2}" $(fd -t f -e rs . "${PROJECT_ROOT}")
  do_replace_docs "${1}" "${2}"
}

NEW_VERSION="${1}"
TODAY=$(date "+%b %d, %Y")

if $PRE_RELEASE; then
  do_replace_all "/${PHYSICAL_CODENAME}" "/${CODENAME}"
  do_replace_docs "${PHYSICAL_CODENAME}" "${CODENAME}"
else
  NEW_CODENAME="v$(echo "${NEW_VERSION}" | cut -d'.' -f1-2)"
  do_replace_all "/${VIRTUAL_CODENAME}" "/${CODENAME}"
  do_replace_all "/${CODENAME}" "/${NEW_CODENAME}"
  do_replace_docs "${VIRTUAL_CODENAME}" "${CODENAME}"
  do_replace_docs "${CODENAME}" "${NEW_CODENAME}"
fi

do_replace_all "${VERSION}" "${NEW_VERSION}"
sd "^date.*" "date = \"${TODAY}\"" "${SITE_ROOT}/index.toml"
