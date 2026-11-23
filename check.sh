#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-ci/Cargo.toml
cargo build --locked --manifest-path crates/judge-ci/Cargo.toml
cargo fmt --check --manifest-path crates/judge-ci-tests/Cargo.toml
cargo test --locked --manifest-path crates/judge-ci-tests/Cargo.toml
prove() {
  local label=$1 root=$2
  shift 2
  local output="$scratch/$label.rlib"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    "$@" --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$scratch/proof.json" || { cat "$scratch/proof.json" >&2; return 1; }
  cat "$scratch/proof.json"
  jq -e '.verus.version == "0.2026.04.24.f8e1704" and
    .verus.commit == "f8e17045c4ac17a02e566b8997070f2eecc255be" and
    (."verification-results" | .success == true and ."is-verifying-entire-crate" == true and
    ."encountered-error" == false and ."encountered-vir-error" == false and
    .errors == 0 and .verified > 0)' "$scratch/proof.json" >/dev/null
  test -s "$output" && test -s "$output.d"
}

prove verify.ci-collection crates/judge-ci/src/verified_collection.rs

prove verify.ci-collection-tests crates/judge-ci-tests/tests/verified_collection.rs

# The remaining cross-product contract tests use the exact Cargo input revisions.
identifiers_manifest=$(cargo metadata --locked --manifest-path crates/judge-ci/Cargo.toml --format-version 1 |
  jq -er '[.packages[] | select(.name == "chrono-identifiers") | .manifest_path] | select(length == 1) | first')
prove libchrono_identifiers "$(dirname "$identifiers_manifest")/src/lib.rs" \
  --crate-name chrono_identifiers --export "$scratch/chrono_identifiers.vir"

cargo metadata --locked --format-version 1 --manifest-path crates/judge-ci/Cargo.toml > "$scratch/packages.json"
export CHRONO_VERUS_SOURCE_VERIFIER="$scratch/source_verifier.rs"
jq -er '[.packages[] | select(.name == "chrono-harness") | .manifest_path | sub("Cargo.toml$"; "src/verified_verifier.rs")] | select(length == 1) | "#[path = " + (first | @json) + "] pub mod source_verifier;"' "$scratch/packages.json" > "$CHRONO_VERUS_SOURCE_VERIFIER"
# The three concrete source-plan contract cases still exercise the pinned routes proof.
cargo metadata --locked --format-version 1 --manifest-path crates/judge-ci-tests/Cargo.toml > "$scratch/test-packages.json"
export CHRONO_VERUS_CONTINUITY_PLAN="$scratch/continuity_plan.rs"
jq -er '[.packages[] | select(.name == "chrono-judge-routes") | .manifest_path | sub("Cargo.toml$"; "src/lib.rs")] | select(length == 1) | "#[path = " + (first | @json) + "] mod continuity_plan;"' "$scratch/test-packages.json" > "$CHRONO_VERUS_CONTINUITY_PLAN"
prove verify.product-contract-tests crates/judge-ci-tests/tests/product_contracts.rs \
  --extern "chrono_identifiers=$scratch/libchrono_identifiers.rlib" \
  --import "chrono_identifiers=$scratch/chrono_identifiers.vir"
