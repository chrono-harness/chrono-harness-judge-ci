#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-ci/Cargo.toml
product=crates/judge-ci/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/judge-ci-tests/Cargo.toml
cargo test --locked --manifest-path crates/judge-ci-tests/Cargo.toml
coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

verifier=$(command -v "${VERUS:-verus}") || coverage_failure "registered verifier was not found"

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "$verifier" "$2" ||
    coverage_failure "$1: verifier failed"
}

admit_proof() {
  local operation=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "$verifier" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$operation: required proof result was not admitted"
}

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/verified_collection.rs src/result_types.rs src/collect.rs src/verified_consumption.rs \
  src/consumption.rs src/lib.rs ||
  coverage_failure "native library differs from required sources"

src=crates/judge-ci/src
tests=crates/judge-ci-tests/tests
include=$tests/../../judge-ci/src
collection_operation=verify.ci-collection
collection_tests_operation=verify.ci-collection-tests
preparation_operation=verify.ci-preparation
preparation_tests_operation=verify.ci-preparation-tests

prove "$collection_operation" "$src/verified_collection.rs"
prove "$collection_tests_operation" "$tests/verified_collection.rs"
prove "$preparation_operation" "$src/verified_consumption.rs"
prove "$preparation_tests_operation" "$tests/verified_consumption.rs"

admit_proof "$collection_operation" "$src/verified_collection.rs" \
  "$src/result_types.rs"
admit_proof "$collection_tests_operation" "$tests/verified_collection.rs" \
  "$include/verified_collection.rs" "$include/result_types.rs" "$tests/support/expect.rs"
admit_proof "$preparation_operation" "$src/verified_consumption.rs"
admit_proof "$preparation_tests_operation" "$tests/verified_consumption.rs" \
  "$include/verified_consumption.rs"
