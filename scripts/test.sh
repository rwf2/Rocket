#!/usr/bin/env bash
set -e

# Brings in _ROOT, _DIR, _DIRS globals.
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "${SCRIPT_DIR}/config.sh"

# Add Cargo to PATH.
export PATH=${HOME}/.cargo/bin:${PATH}
export CARGO_INCREMENTAL=0
export RUSTC_BOOTSTRAP=1
CARGO="cargo"

# Checks that the versions for Cargo projects $@ all match
function check_versions_match() {
  local last_version=""
  for dir in "${@}"; do
    local cargo_toml="${dir}/Cargo.toml"
    if ! [ -f "${cargo_toml}" ]; then
      echo "Cargo configuration file '${cargo_toml}' does not exist."
      exit 1
    fi

    local version=$(grep version "${cargo_toml}" | head -n 1 | cut -d' ' -f3)
    if [ -z "${last_version}" ]; then
      last_version="${version}"
    elif ! [ "${version}" = "${last_version}" ]; then
      echo "Versions differ in '${cargo_toml}'. ${version} != ${last_version}"
      exit 1
    fi
  done
}

function check_style() {
  # Ensure there are no tabs in any file.
  local tab=$(printf '\t')
  local matches=$(git grep -PIn "${tab}" "${PROJECT_ROOT}" | grep -v 'LICENSE')
  if ! [ -z "${matches}" ]; then
    echo "Tab characters were found in the following:"
    echo "${matches}"
    exit 1
  fi

  # Ensure non-comment lines are under 100 characters.
  local n=100
  local matches=$(git grep -PIn "(?=^..{$n,}$)(?!^\s*\/\/[\/!].*$).*" '*.rs')
  if ! [ -z "${matches}" ]; then
    echo "Lines longer than $n characters were found in the following:"
    echo "${matches}"
    exit 1
  fi

  # Ensure there's no trailing whitespace.
  local matches=$(git grep -PIn "\s+$" "${PROJECT_ROOT}" | grep -v -F '.stderr:')
  if ! [ -z "${matches}" ]; then
    echo "Trailing whitespace was found in the following:"
    echo "${matches}"
    exit 1
  fi

  local pattern='tail -n 1 % | grep -q "^$" && echo %'
  local matches=$(git grep -z -Il '' | xargs -0 -P 16 -I % sh -c "${pattern}")
  if ! [ -z "${matches}" ]; then
    echo "Trailing new line(s) found in the following:"
    echo "${matches}"
    exit 1
  fi
}

function indir() {
  local dir="${1}"
  shift
  pushd "${dir}" > /dev/null 2>&1 ; $@ ; popd > /dev/null 2>&1
}

function test_contrib() {
  DB_POOLS_FEATURES=(
    deadpool_postgres
    deadpool_redis
    sqlx_mysql
    sqlx_postgres
    sqlx_sqlite
    mongodb
    diesel_mysql
    diesel_postgres
  )

  SYNC_DB_POOLS_FEATURES=(
    diesel_postgres_pool
    diesel_sqlite_pool
    diesel_mysql_pool
    postgres_pool
    sqlite_pool
    memcache_pool
  )

  DYN_TEMPLATES_FEATURES=(
    tera
    handlebars
    minijinja
  )

  WS_FEATURES=(
    tungstenite
  )

  for feature in "${DB_POOLS_FEATURES[@]}"; do
    echo ":: Building and testing db_pools [$feature]..."
    $CARGO test -p rocket_db_pools --no-default-features --features $feature $@
  done

  for feature in "${SYNC_DB_POOLS_FEATURES[@]}"; do
    echo ":: Building and testing sync_db_pools [$feature]..."
    $CARGO test -p rocket_sync_db_pools --no-default-features --features $feature $@
  done

  for feature in "${DYN_TEMPLATES_FEATURES[@]}"; do
    echo ":: Building and testing dyn_templates [$feature]..."
    $CARGO test -p rocket_dyn_templates --no-default-features --features $feature $@
  done

  for feature in "${WS_FEATURES[@]}"; do
    echo ":: Building and testing ws [$feature]..."
    $CARGO test -p rocket_ws --no-default-features --features $feature $@
  done
}

function test_core() {
  FEATURES=(
    tokio-macros
    http2
    http3-preview
    secrets
    tls
    mtls
    json
    msgpack
    uuid
    trace
  )

  echo ":: Building and checking core [no features]..."
  RUSTDOCFLAGS="-Zunstable-options --no-run" \
    indir "${CORE_LIB_ROOT}" $CARGO test --no-default-features $@

  for feature in "${FEATURES[@]}"; do
    echo ":: Building and checking core [${feature}]..."
    RUSTDOCFLAGS="-Zunstable-options --no-run" \
      indir "${CORE_LIB_ROOT}" $CARGO test --no-default-features --features "${feature}" $@
  done
}

function test_examples() {
  # Cargo compiles Rocket once with the `secrets` feature enabled, so when run
  # in production, we need a secret key or tests will fail needlessly. We test
  # in core that secret key failing/not failing works as expected, but here we
  # provide a valid secret_key so tests don't fail.
  echo ":: Building and testing examples..."
  indir "${EXAMPLES_DIR}" $CARGO update
  ROCKET_SECRET_KEY="itlYmFR2vYKrOmFhupMIn/hyB6lYCCTXz4yaQX89XVg=" \
    indir "${EXAMPLES_DIR}" $CARGO test --all $@
}

function test_default() {
  echo ":: Building and testing core libraries..."
  indir "${PROJECT_ROOT}" $CARGO test --all --all-features $@

  echo ":: Checking benchmarks..."
  indir "${BENCHMARKS_ROOT}" $CARGO update
  indir "${BENCHMARKS_ROOT}" $CARGO check --benches --all-features $@

  case "$OSTYPE" in
      darwin* | linux*)
          echo ":: Checking testbench..."
          indir "${TESTBENCH_ROOT}" $CARGO update
          indir "${TESTBENCH_ROOT}" $CARGO check $@

          echo ":: Checking fuzzers..."
          indir "${FUZZ_ROOT}" $CARGO update
          indir "${FUZZ_ROOT}" $CARGO check --all --all-features $@
          ;;
      *) echo ":: Skipping testbench, fuzzers [$OSTYPE]" ;;
  esac
}

function test_ui() {
  echo ":: Testing compile-time UI output..."
  indir "${PROJECT_ROOT}" $CARGO test --test ui-fail --all --all-features -- --ignored $@
}

function run_benchmarks() {
  echo ":: Running benchmarks..."
  indir "${BENCHMARKS_ROOT}" $CARGO update
  indir "${BENCHMARKS_ROOT}" $CARGO bench $@
}

function run_testbench() {
  echo ":: Running testbench..."
  indir "${TESTBENCH_ROOT}" $CARGO update
  indir "${TESTBENCH_ROOT}" $CARGO run $@
}

# The kind of test we'll be running.
TEST_KIND="default"
KINDS=("default" "all" "core" "contrib" "examples" "benchmarks" "testbench" "ui")

function print_help() {
  echo "USAGE:"
  echo "  $0 [+<TOOLCHAIN>] [--help|-h] [--<TEST>]"
  echo ""
  echo "OPTIONS:"
  echo "  +<TOOLCHAIN>   Forwarded to Cargo to select toolchain."
  echo "  --help, -h     Print this help message and exit."
  echo "  --<TEST>       Run the specified test suite."
  echo "                 (Run without --<TEST> to run default tests.)"

  echo ""
  echo "AVAILABLE <TEST> OPTIONS:"
  for kind in "${KINDS[@]}"; do
    echo "  ${kind}"
  done

  echo ""
  echo "EXAMPLES:"
  echo "  $0                     # Run default tests on current toolchain."
  echo "  $0 +stable --all       # Run all tests on stable toolchain."
  echo "  $0 --ui                # Run UI tests on current toolchain."
}

if [[ $1 == +* ]]; then
  CARGO="$CARGO $1"
  shift
fi

if [[ $1 == "--help" ]] || [[ $1 == "-h" ]]; then
  print_help
  exit 0
fi

if [[ " ${KINDS[@]} " =~ " ${1#"--"} " ]]; then
  TEST_KIND=${1#"--"}
  shift
fi

echo ":: Preparing. Environment is..."
print_environment
echo "  CARGO: $CARGO"
echo "  EXTRA FLAGS: $@"

echo ":: Ensuring core crate versions match..."
check_versions_match "${CORE_CRATE_ROOTS[@]}"

echo ":: Ensuring contrib sync_db_pools versions match..."
check_versions_match "${CONTRIB_SYNC_DB_POOLS_CRATE_ROOTS[@]}"

echo ":: Ensuring contrib db_pools versions match..."
check_versions_match "${CONTRIB_DB_POOLS_CRATE_ROOTS[@]}"

echo ":: Ensuring minimum style requirements are met..."
check_style

echo ":: Updating dependencies..."
if ! $CARGO update ; then
  echo "   WARNING: Update failed! Proceeding with possibly outdated deps..."
fi

case $TEST_KIND in
  core) test_core $@ ;;
  contrib) test_contrib $@ ;;
  examples) test_examples $@ ;;
  default) test_default $@ ;;
  benchmarks) run_benchmarks $@ ;;
  testbench) run_testbench $@ ;;
  ui) test_ui $@ ;;
  all)
    test_default $@ & default=$!
    test_examples $@ & examples=$!
    test_core $@ & core=$!
    test_contrib $@ & contrib=$!
    run_testbench $@ & testbench=$!
    test_ui $@ & ui=$!

    failures=()
    if ! wait $default ; then failures+=("DEFAULT"); fi
    if ! wait $examples ; then failures+=("EXAMPLES"); fi
    if ! wait $core ; then failures+=("CORE"); fi
    if ! wait $contrib ; then failures+=("CONTRIB"); fi
    if ! wait $testbench ; then failures+=("TESTBENCH"); fi
    if ! wait $ui ; then failures+=("UI"); fi

    if [ ${#failures[@]} -ne 0 ]; then
      tput setaf 1;
      echo -e "\n!!! ${#failures[@]} TEST SUITE FAILURE(S) !!!"
      for failure in "${failures[@]}"; do
        echo "    :: ${failure}"
      done

      tput sgr0
      exit ${#failures[@]}
    fi

    ;;
esac
