#!/bin/bash
set -e

# Function to run on any error
handle_error() {
    echo "CI checks failed."
}

# Register the error handler
trap handle_error ERR

echo "Running All CI Checks..."

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

"$DIR/fmt.sh"
"$DIR/lint.sh"
"$DIR/test.sh"
"$DIR/build.sh"

echo "All CI checks passed."
