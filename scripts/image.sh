#!/usr/bin/env bash
# Image operations deliberately bypass dotenv and the development lifecycle.
set -euo pipefail
exec python3 "$(dirname "${BASH_SOURCE[0]}")/image.py" "$@"
