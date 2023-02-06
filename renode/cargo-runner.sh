#!/usr/bin/env bash

set -euo pipefail

bin_path=$1

renode_bin=${RENODE_PATH-renode}
renode_opts=${RENODE_OPTS-}

echo "Running renode with binary '${bin_path}'"

export CARGO_RENODE_BIN_RELATIVE_PATH="${bin_path}"

${renode_bin} ${renode_opts} renode/emulate.resc

exit 0
