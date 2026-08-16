#!/bin/sh
set -e

export LOCATION="$PWD"

echo "Installing alc (Alum compiler) and alc-lsp (Alum language server)..."
cargo install --path "$LOCATION"

echo "Installing almk (Alum build tool)..."
cd "$LOCATION/alum-make"
cargo install --path .

echo "Building alum-std (Alum standard library)..."
cd "$LOCATION/alum-std/alum"
almk build

echo "Installing alum-std modules..."
sudo mkdir -p /usr/local/include/alum
sudo rm -f /usr/local/include/alum/*.ah
sudo cp "$LOCATION/alum-std/alum/src/"*.al /usr/local/include/alum/

echo "Building Rust runtime (libalum_std.a)..."
cd "$LOCATION/alum-std"
cargo build --release

echo "Merging static libraries..."
cd "$LOCATION/alum-std/target/release"
mkdir -p merged_objs
cd merged_objs
ar x ../libalum_std.a

if [ -f "$LOCATION/alum-std/alum/target/libalum.a" ]; then
    ar x "$LOCATION/alum-std/alum/target/libalum.a"
fi

rm -f ../libalum.a
ar rcs ../libalum.a *.o
cd ..
rm -rf merged_objs

echo "Installing merged standard library..."
sudo cp libalum.a /usr/local/lib/libalum.a

echo "Packaging VS Code extension..."
if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
    cd "$LOCATION/alum-vscode" || {
        echo "WARNING: cannot enter $LOCATION/alum-vscode; skipping extension." >&2
    }

    if [ ! -d node_modules ]; then
        echo "Installing extension dependencies..."
        npm install || {
            echo "WARNING: npm install failed; skipping extension packaging." >&2
        }
    fi

    if [ -d node_modules ] && yes | npx vsce package --out alum-vscode-lsp.vsix; then
        echo "Extension packaged: $LOCATION/alum-vscode/alum-vscode-lsp.vsix"
        if command -v code >/dev/null 2>&1; then
            echo "Installing extension into VS Code..."
            code --install-extension alum-vscode-lsp.vsix
        else
            echo "VS Code CLI ('code') not found; install the vsix manually with:"
            echo "  code --install-extension $LOCATION/alum-vscode/alum-vscode-lsp.vsix"
        fi
    else
        echo "WARNING: vsce package failed; skipping extension packaging." >&2
    fi
else
    echo "WARNING: node/npm not found; skipping VS Code extension packaging." >&2
fi

echo "Installation completed successfully!"
