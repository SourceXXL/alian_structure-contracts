#!/usr/bin/env bash
# Initialise all deployed contracts.
# Usage: ./scripts/initialize.sh <admin_address> <aid_id> <treasury_id> <referral_id> <governance_id> <oracle_id> <registry_id>
set -euo pipefail

ADMIN="${1:?admin_address required}"
AID_ID="${2:?aid_contract_id required}"
TREASURY_ID="${3:?treasury_contract_id required}"
REFERRAL_ID="${4:?referral_contract_id required}"
GOVERNANCE_ID="${5:?governance_contract_id required}"
ORACLE_ID="${6:?oracle_contract_id required}"
REGISTRY_ID="${7:?registry_contract_id required}"

for ID in "${AID_ID}" "${TREASURY_ID}" "${REFERRAL_ID}" "${GOVERNANCE_ID}" "${ORACLE_ID}" "${REGISTRY_ID}"; do
  echo "Initialising contract ${ID}..."
  soroban contract invoke \
    --id "${ID}" \
    --network testnet \
    --source admin \
    -- initialize --admin "${ADMIN}"
done
