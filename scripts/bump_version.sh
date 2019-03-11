#! /usr/bin/env bash

#
# Bumps the version number from <current> to <next> on all libraries.
#

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

if [ -z "${1}" ] || [ -z "${2}" ]; then
  echo "Usage: $0 <current> <next>"
  echo "Example: $0 0.1.1 0.1.2"
  exit 1
fi

if ! git grep -c "${1}" > /dev/null; then
  echo "The version '${1}' doesn't appear to be correct."
  echo "Exiting."
  exit 1
fi

function major() {
  echo "${1}" | cut -d'.' -f1-2
}

function do_replace() {
  find "${PROJECT_ROOT}" -name "*.rs" | xargs sed -i.bak "s/${1}/${2}/g"
  find "${PROJECT_ROOT}" -name "*.toml" | xargs sed -i.bak "s/${1}/${2}/g"
  find "${SITE_ROOT}" -name "*.md" | xargs sed -i.bak "s/${1}/${2}/g"
  sed -i.bak "s/${1}/${2}/g" "${SCRIPT_DIR}/config.sh"
  sed -i.bak "s/${1}/${2}/g" "${PROJECT_ROOT}/README.md"
}

do_replace "v$(major ${1})" "v$(major ${2})"
do_replace "${1}" "${2}"

today=$(date "+%b %d, %Y")
sed -i.bak "s/^date.*/date = \"$today\"/" "${SITE_ROOT}/index.toml"

find ${PROJECT_ROOT} -name "*.bak" | xargs rm
