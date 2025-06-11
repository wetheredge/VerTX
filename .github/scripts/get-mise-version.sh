#!/usr/bin/env bash

set -eu

if [[ $# -ne 1 ]]; then
	echo "::error::usage: $0 <tool>"
	exit 1
fi
tool="$1"

version="$(sed -En "s/.*\b$tool\"?\s*=\s*\"([^\"]+)\".*/\1/p" .config/mise.toml)"
if [[ -z "$version" ]]; then
	echo "::error::'$tool' is not in mise config"
	exit 1
fi

echo "version=$version"
