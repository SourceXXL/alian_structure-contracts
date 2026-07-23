#!/usr/bin/env bash
# Deploy all contracts to the target network.
# Usage: ./scripts/deploy.sh [testnet|mainnet]
set -euo pipefail

NETWORK="${1:-testnet}"
WASM_DIR="target/wasm32v1-none/release"

CONTRACTS=(
  "aid_contract"
  "treasury_contract"
  "referral_contract"
  "governance_contract"
  "oracle_contract"
  "registry_contract"
)

for CONTRACT in "${CONTRACTS[@]}"; do
  echo "Deploying ${CONTRACT} to ${NETWORK}..."
  soroban contract deploy \
    --wasm "${WASM_DIR}/${CONTRACT}.wasm" \
    --network "${NETWORK}" \
    --source admin
done
