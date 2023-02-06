#!/usr/bin/env bash

set -euo pipefail

renode_bin=${RENODE_PATH-renode}

bin_path=$1

echo "Running renode with binary '${bin_path}'"

export CARGO_RENODE_BIN_RELATIVE_PATH="${bin_path}"

${renode_bin} renode/emulate.resc

exit 0
