#!/usr/bin/env bash
# Demo script — requires a running diet-optimizer instance and curl.
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Health check"
curl -s "${BASE_URL}/health" | jq .

echo
echo "==> Optimize (percentages, no dairy)"
curl -s -X POST "${BASE_URL}/api/v1/optimize-diet" \
  -H 'Content-Type: application/json' \
  -d @"${SCRIPT_DIR}/request-percentages.json" | jq .

echo
echo "==> Optimize (grams, high protein)"
curl -s -X POST "${BASE_URL}/api/v1/optimize-diet" \
  -H 'Content-Type: application/json' \
  -d @"${SCRIPT_DIR}/request-grams.json" | jq .

echo
echo "==> Optimize (vegan-friendly exclusions)"
curl -s -X POST "${BASE_URL}/api/v1/optimize-diet" \
  -H 'Content-Type: application/json' \
  -d @"${SCRIPT_DIR}/request-vegan.json" | jq .

echo
echo "==> Validation error (negative calories)"
curl -s -w "\nHTTP %{http_code}\n" -X POST "${BASE_URL}/api/v1/optimize-diet" \
  -H 'Content-Type: application/json' \
  -d '{"target_calories": -1, "macro_targets": {"type": "grams", "carbs_g": 1, "protein_g": 1, "fat_g": 1}}' | jq . 2>/dev/null || true
