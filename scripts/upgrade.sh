#!/usr/bin/env bash
# Upgrade a deployed contract via governance authorisation.
# Usage: ./scripts/upgrade.sh <contract_id> <wasm_path>
set -euo pipefail

CONTRACT_ID="${1:?contract_id required}"
WASM_PATH="${2:?wasm_path required}"

soroban contract upload \
  --wasm "${WASM_PATH}" \
  --network testnet \
  --source admin

soroban contract invoke \
  --id "${CONTRACT_ID}" \
  --network testnet \
  --source admin \
  -- upgrade --new-wasm-hash "$(soroban contract hash --wasm "${WASM_PATH}")"
