#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# -ne 1 || $1 != --native ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
scratch_root=$(pwd -P)/.chrono-harness/state/native-check
mkdir -p "$scratch_root"
scratch=$(mktemp -d "$scratch_root/check.XXXXXXXX")
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

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/verified_text.rs src/verified_collection.rs src/result_types.rs src/route_wire.rs src/project_execution.rs src/source_binding.rs src/files.rs src/collect.rs src/units.rs src/originals.rs src/verified_consumption.rs \
  src/consumption.rs src/lib.rs src/requirements.rs src/observed.rs ||
  coverage_failure "native library differs from required sources"

src=crates/judge-ci/src
tests=crates/judge-ci-tests/tests
include=$tests/../../judge-ci/src
collection_operation=verify.ci-collection
collection_tests_operation=verify.ci-collection-tests
preparation_operation=verify.ci-preparation
preparation_tests_operation=verify.ci-preparation-tests

chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/admission-proofs/$collection_operation.rlib" "$src/verified_collection.rs" \
  "$src/verified_text.rs" "$src/main.rs" "$src/result_types.rs" "$src/project_execution.rs" "$src/files.rs" "$src/source_binding.rs" "$src/route_wire.rs" "$src/collect.rs" "$src/units.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/admission-proofs/$collection_tests_operation.rlib" "$tests/verified_collection.rs" \
  "$include/verified_collection.rs" "$include/verified_text.rs" "$include/main.rs" "$include/result_types.rs" "$include/project_execution.rs" "$include/files.rs" "$include/source_binding.rs" "$include/route_wire.rs" "$include/collect.rs" "$include/units.rs" "$tests/support/expect.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/admission-proofs/$preparation_operation.rlib" "$src/verified_consumption.rs" "$src/verified_text.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/admission-proofs/$preparation_tests_operation.rlib" "$tests/verified_consumption.rs" \
  "$include/verified_consumption.rs" "$include/verified_text.rs"
