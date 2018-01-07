#! /usr/bin/env bash

#
# Bumps the version number from <current> to <next> on all libraries.
#

if [ -z ${1} ] || [ -z ${2} ]; then
  echo "Usage: $0 <current> <next>"
  echo "Example: $0 0.1.1 0.1.2"
  exit 1
fi

today=$(date "+%b %d, %Y")

find . -name "*.toml" | xargs sed -i.bak "s/${1}/${2}/g"
find site/ -name "*.md" | xargs sed -i.bak "s/${1}/${2}/g"
sed -i.bak "s/^date.*/date = \"$today\"/" site/index.toml
find . -name "*.bak" | xargs rm
