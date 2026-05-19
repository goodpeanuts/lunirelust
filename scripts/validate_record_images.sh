#!/usr/bin/env bash
# Validate record image folder structure under private/images/record/
# Rules:
#   1. Each folder {ID}/ must contain {ID}.jpg (main image)
#   2. Numbered images {ID}_N.jpg must start from _0 and be contiguous
#   3. No files with names that don't match the pattern

set -euo pipefail

RECORD_DIR="${1:-subm/luna/data/assets/private/images/record}"
OUTPUT_FILE="${2:-invalid_record_images.txt}"

if [ ! -d "$RECORD_DIR" ]; then
  echo "Error: directory not found: $RECORD_DIR" >&2
  exit 1
fi

total=0
invalid=0

> "$OUTPUT_FILE"

for dir in "$RECORD_DIR"/*/; do
  total=$((total + 1))
  id=$(basename "$dir")
  reasons=()

  # Rule 1: main image must exist
  if [ ! -f "${dir}${id}.jpg" ]; then
    reasons+=("missing main image: ${id}.jpg")
  fi

  # Collect all files and check naming patterns
  numbered_indices=()
  has_extra=false

  for file in "$dir"*; do
    [ -f "$file" ] || continue
    fname=$(basename "$file")

    if [ "$fname" = "${id}.jpg" ]; then
      # main image, already checked above
      :
    elif [[ "$fname" =~ ^${id}_([0-9]+)\.jpg$ ]]; then
      numbered_indices+=("${BASH_REMATCH[1]}")
    else
      has_extra=true
      reasons+=("unexpected file: ${fname}")
    fi
  done

  # Rule 2 & 3: numbered series validation
  if [ ${#numbered_indices[@]} -gt 0 ]; then
    # Must start from 0
    has_zero=false
    for idx in "${numbered_indices[@]}"; do
      if [ "$idx" = "0" ]; then
        has_zero=true
        break
      fi
    done
    if [ "$has_zero" = false ]; then
      reasons+=("numbered series does not start from _0")
    fi

    # Must be contiguous: sort indices, check no gaps
    sorted=($(printf '%s\n' "${numbered_indices[@]}" | sort -n))
    max=${sorted[-1]}
    expected_count=$((max + 1))
    if [ "${#sorted[@]}" -ne "$expected_count" ]; then
      # Find which indices are missing
      declare -A present
      for idx in "${sorted[@]}"; do
        present[$idx]=1
      done
      missing=()
      for ((i = 0; i <= max; i++)); do
        if [ -z "${present[$i]:-}" ]; then
          missing+=("_${i}")
        fi
      done
      reasons+=("missing numbered images: ${missing[*]}")
      unset present
    fi
  fi

  # Report if any rule violated
  if [ ${#reasons[@]} -gt 0 ]; then
    invalid=$((invalid + 1))
    echo "[INVALID] $id"
    for r in "${reasons[@]}"; do
      echo "  - $r"
    done
    echo "$id" >> "$OUTPUT_FILE"
  fi
done

echo ""
echo "Done. Scanned: $total, Invalid: $invalid"
echo "Invalid IDs saved to: $OUTPUT_FILE"
