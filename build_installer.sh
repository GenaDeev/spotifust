#!/bin/bash
# Shell script to build Spotifust and package it for macOS and Linux.

set -e

echo "Building Spotifust release..."
cargo build --release

if [ "$(uname)" == "Darwin" ]; then
    echo "Packaging for macOS..."
    APP_NAME="Spotifust"
    APP_DIR="${APP_NAME}.app/Contents"
    
    mkdir -p "$APP_DIR/MacOS"
    mkdir -p "$APP_DIR/Resources"
    cp target/release/spotifust "$APP_DIR/MacOS/"
    
    cat <<EOF > "$APP_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>spotifust</string>
    <key>CFBundleIdentifier</key>
    <string>com.spotifust.app</string>
    <key>CFBundleName</key>
    <string>Spotifust</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
</dict>
</plist>
EOF

    echo "Creating DMG..."
    hdiutil create -volname "${APP_NAME}" -srcfolder "${APP_NAME}.app" -ov -format UDZO "${APP_NAME}.dmg"
    echo "Done! Installer located at ${APP_NAME}.dmg"

elif [ "$(uname)" == "Linux" ]; then
    echo "Packaging for Linux..."
    tar -czvf spotifust-linux.tar.gz -C target/release spotifust
    echo "Done! Archive located at spotifust-linux.tar.gz"
fi
