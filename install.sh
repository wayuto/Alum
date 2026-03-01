#!/bin/sh
set -e

export LOCATION="$PWD"

echo "Installing alc (Alum compiler)..."
cargo install --path "$LOCATION"

echo "Installing almk (Alum build tool)..."
cd "$LOCATION/alum-make"
cargo install --path .

echo "Building alum-std (Alum standard library)..."
cd "$LOCATION/alum-std/alum"
almk build

echo "Installing alum-std headers..."
sudo mkdir -p /usr/local/include/alum
sudo cp "$LOCATION/alum-std/alum/headers/"*.al /usr/local/include/alum/

echo "Building Rust runtime (libalum_std.a)..."
cd "$LOCATION/alum-std"
cargo build --release

echo "Merging static libraries..."
cd "$LOCATION/alum-std/target/release"
mkdir -p merged_objs
cd merged_objs
ar x ../libalum_std.a

# 检查 alum 目标库是否存在
if [ -f "$LOCATION/alum-std/alum/target/libalum.a" ]; then
    ar x "$LOCATION/alum-std/alum/target/libalum.a"
fi

ar rcs ../libalum.a *.o
cd ..
rm -rf merged_objs

echo "Installing merged standard library..."
sudo cp libalum.a /usr/local/lib/libalum.a

echo "Installation completed successfully!"
