#! /usr/bin/env bash

SCRIPT_PATH=$(cd "$(dirname "$0")" ; pwd -P)
DATABASE_URL="${SCRIPT_PATH}/db/db.sqlite"

rm -f "${DATABASE_URL}"
