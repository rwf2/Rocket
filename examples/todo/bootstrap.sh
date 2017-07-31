#! /usr/bin/env bash

SCRIPT_PATH=$(cd "$(dirname "$0")" ; pwd -P)
DATABASE_URL="${SCRIPT_PATH}/db/db.sql"

pushd "${SCRIPT_PATH}" > /dev/null
  # clear an existing database
  rm -f "${DATABASE_URL}"

  # install the diesel CLI tools if they're not installed
  if ! command -v diesel >/dev/null 2>&1; then
    cargo install diesel_cli --no-default-features --features=sqlite > /dev/null
  fi

  # create db/db.sql
  diesel migration --database-url="${DATABASE_URL}" run > /dev/null
popd > /dev/null

echo "export DATABASE_URL=\"${DATABASE_URL}\""
