#!/bin/bash
cd "$(dirname "$0")/tui-runner" || {
    echo "Could not find the tui-runner folder next to this script."
    read -n 1 -s -r -p "Press any key to close..."
    exit 1
}
npm start

echo ""
read -n 1 -s -r -p "Press any key to close..."