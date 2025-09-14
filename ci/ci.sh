#!/bin/bash
set -e
echo "Running All CI Checks..."

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

"$DIR/fmt.sh"
"$DIR/lint.sh"
"$DIR/test.sh"
"$DIR/build.sh"

echo "All CI checks passed."
