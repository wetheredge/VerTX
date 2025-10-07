#!/usr/bin/env bash

set -eu

if [[ $# -ne 1 ]]; then
	echo "::error::usage: $0 <tool>"
	exit 1
fi
tool="$1"

version="$(jq --raw-output ".\"$tool\"" .config/versions.json)"
if [[ -z "$version" ]]; then
	echo "::error::'$tool' is not in versions config"
	exit 1
fi

echo "version=$version"
