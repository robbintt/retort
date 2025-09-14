#!/bin/bash
set -e

# Function to run on exit
cleanup() {
    if [ "$?" -ne 0 ]; then
        echo "CI checks failed."
    fi
}

# Register the cleanup function to be called on script exit
trap cleanup EXIT

echo "Running All CI Checks..."

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

"$DIR/fmt.sh"
"$DIR/lint.sh"
"$DIR/test.sh"
"$DIR/build.sh"

echo "All CI checks passed."
