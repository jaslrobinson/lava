#!/bin/bash
# LAVA - Live Animated Visuals for Arch
# Quick install script (builds from source)

set -e

echo "=== LAVA Installer ==="
echo ""

# Check dependencies
echo "Checking dependencies..."
for cmd in cargo pnpm node; do
    if ! command -v $cmd &>/dev/null; then
        echo "ERROR: '$cmd' not found. Install it first."
        exit 1
    fi
done

for pkg in gtk+-3.0 webkit2gtk-4.1 gtk-layer-shell-0; do
    if ! pkg-config --exists "$pkg" 2>/dev/null; then
        echo "ERROR: '$pkg' development library not found."
        echo "  Install: sudo pacman -S gtk3 webkit2gtk-4.1 gtk-layer-shell"
        exit 1
    fi
done

echo "All dependencies found."
echo ""

# Install frontend deps
echo "Installing frontend dependencies..."
pnpm install

# Build wallpaper helper
echo "Building lava-wallpaper..."
cargo build -p lava-wallpaper --release

# Build main app
echo "Building LAVA..."
npx tauri build --no-bundle

# Install
echo ""
echo "Installing to /usr/local/bin/..."
sudo install -Dm755 target/release/lava /usr/local/bin/lava
sudo install -Dm755 target/release/lava-wallpaper /usr/local/bin/lava-wallpaper

# Install frontend dist (needed by wallpaper in release mode)
echo "Installing frontend dist..."
sudo mkdir -p /usr/share/lava/dist/assets
sudo install -Dm644 dist/index.html /usr/share/lava/dist/index.html
for f in dist/assets/*; do
    sudo install -Dm644 "$f" "/usr/share/lava/dist/assets/$(basename "$f")"
done

# Desktop entry
sudo tee /usr/share/applications/lava.desktop >/dev/null <<EOF
[Desktop Entry]
Type=Application
Name=LAVA
GenericName=Live Wallpaper Engine
Comment=Live Animated Visuals for Arch
Exec=lava
Icon=lava
Categories=Utility;Graphics;
Keywords=wallpaper;live;widget;desktop;
StartupNotify=true
EOF

# Icon
if [ -f src-tauri/icons/icon.png ]; then
    sudo install -Dm644 src-tauri/icons/icon.png /usr/share/icons/hicolor/256x256/apps/lava.png
fi

echo ""
echo "=== LAVA installed successfully! ==="
echo "Run 'lava' to start."
