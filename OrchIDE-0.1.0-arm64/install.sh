#!/bin/bash
echo "Installing OrchIDE..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP="$SCRIPT_DIR/OrchIDE.app"

if [ ! -d "$APP" ]; then
    echo "Error: OrchIDE.app not found next to this script"
    exit 1
fi

cp -r "$APP" /Applications/
xattr -cr /Applications/OrchIDE.app
chmod +x /Applications/OrchIDE.app/Contents/MacOS/orch-ide
echo "Installed! Opening OrchIDE..."
open /Applications/OrchIDE.app
