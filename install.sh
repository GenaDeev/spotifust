#!/bin/bash
set -e

# Generic script to install Spotifust on any Linux distribution.
# If run with sudo, it installs globally. If not, it installs locally in your home directory.

echo "🚀 Starting Spotifust installation..."

if [ "$EUID" -eq 0 ]; then
    echo "🔒 Running as root. Installing globally for all users..."
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/share/applications"
    ICON_DIR="/usr/share/pixmaps"
else
    echo "👤 Running as a regular user. Installing locally..."
    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"
fi

# Create directories if they don't exist
mkdir -p "$BIN_DIR"
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"

# Find the executable binary
if [ -f "spotifust" ]; then
    EXECUTABLE="spotifust"
elif [ -f "target/release/spotifust" ]; then
    EXECUTABLE="target/release/spotifust"
else
    echo "❌ Error: Could not find the 'spotifust' binary."
    echo "   Please run this script in the folder where you extracted the tar.gz or after running 'cargo build --release'."
    exit 1
fi

echo "📦 Copying binary to $BIN_DIR..."
cp "$EXECUTABLE" "$BIN_DIR/spotifust"
chmod +x "$BIN_DIR/spotifust"

# Check if local bin is in PATH
if [ "$EUID" -ne 0 ] && [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "⚠️  Warning: $BIN_DIR does not seem to be in your PATH."
    echo "   You might need to add it to your ~/.bashrc or ~/.zshrc to launch the app from the terminal."
fi

echo "🎨 Copying icon..."
if [ -f "assets/spotifust.png" ]; then
    cp "assets/spotifust.png" "$ICON_DIR/spotifust.png"
else
    echo "⚠️  Warning: 'assets/spotifust.png' not found. The application menu might lack an icon."
fi

echo "🔗 Creating desktop entry (.desktop) and registering protocol (spotifust://)..."
cat <<EOF > "$APP_DIR/spotifust.desktop"
[Desktop Entry]
Name=Spotifust
Comment=A native, fast, and light Spotify desktop client
Exec=$BIN_DIR/spotifust %u
Icon=spotifust
Terminal=false
Type=Application
Categories=AudioVideo;Audio;Player;
MimeType=x-scheme-handler/spotifust;
EOF

chmod 644 "$APP_DIR/spotifust.desktop"

# Update desktop application database
if command -v update-desktop-database >/dev/null 2>&1; then
    echo "🔄 Updating desktop application database..."
    update-desktop-database "$APP_DIR"
else
    echo "⚠️  'update-desktop-database' not found. You might need to restart your desktop environment for the custom protocol to be registered."
fi

echo "✅ Installation complete! You can now launch Spotifust."
