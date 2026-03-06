#!/usr/bin/env bash
set -uo pipefail

step() {
  echo
  echo "==> $1"
}

failures=()

run_step() {
  local label="$1"
  shift
  step "$label"
  "$@"
  local rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "PASS: $label"
  else
    echo "FAIL: $label (exit $rc)"
    failures+=("$label (exit $rc)")
  fi
}

run_step "cargo fmt --check" cargo fmt --all -- --check
run_step "cargo clippy" cargo clippy --all-targets --all-features -- -D warnings
run_step "cargo test" cargo test --all-features
run_step "cargo build" cargo build --all-features
run_step "build gtpack scaffold" bash build/build_gtpack.sh
run_step "wizard dry-run smoke" bash build/wizard_dry_run_check.sh
run_step "cargo doc" cargo doc --no-deps --all-features

step "package + publish dry-run checks"
publishable_crates="$(bash ci/list_publishable_crates.sh)"
if [ -z "$publishable_crates" ]; then
  echo "No publishable crates detected."
else
  for crate in $publishable_crates; do
    echo
    echo "--- crate: $crate ---"
    if [ "${CI:-}" = "true" ]; then
      run_step "cargo package --no-verify ($crate)" cargo package --no-verify -p "$crate"
      run_step "cargo package ($crate)" cargo package -p "$crate"
      run_step "cargo publish --dry-run ($crate)" cargo publish -p "$crate" --dry-run
    else
      run_step "cargo package --no-verify ($crate)" cargo package --no-verify -p "$crate" --allow-dirty
      run_step "cargo package ($crate)" cargo package -p "$crate" --allow-dirty
      run_step "cargo publish --dry-run ($crate)" cargo publish -p "$crate" --dry-run --allow-dirty
    fi
    run_step "package asset check ($crate)" bash ci/check_package_assets.sh "$crate"
  done
fi

echo
if [ "${#failures[@]}" -eq 0 ]; then
  echo "All local checks passed."
  exit 0
fi

echo "Local checks failed in the following areas:"
for failure in "${failures[@]}"; do
  echo " - $failure"
done
exit 1
