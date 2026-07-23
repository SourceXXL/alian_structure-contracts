#!/usr/bin/env bash
# Verify WASM artefacts exist after a release build.
set -euo pipefail

WASM_DIR="target/wasm32v1-none/release"

CONTRACTS=(
  "aid_contract"
  "treasury_contract"
  "referral_contract"
  "governance_contract"
  "oracle_contract"
  "registry_contract"
)

PASS=0
FAIL=0

for CONTRACT in "${CONTRACTS[@]}"; do
  FILE="${WASM_DIR}/${CONTRACT}.wasm"
  if [[ -f "${FILE}" ]]; then
    SIZE=$(wc -c < "${FILE}")
    echo "✅  ${CONTRACT}.wasm  (${SIZE} bytes)"
    PASS=$((PASS + 1))
  else
    echo "❌  ${CONTRACT}.wasm  NOT FOUND"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
