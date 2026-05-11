#!/bin/bash
# Remove -Zjson-target-spec from the arguments
args=()
for arg in "$@"; do
    if [ "$arg" != "-Zjson-target-spec" ]; then
        args+=("$arg")
    fi
done
exec rustc "${args[@]}"
