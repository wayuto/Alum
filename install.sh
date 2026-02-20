#!/bin/sh

export LOCALCATION=$PWD

echo "Installing alc (Alum compiler)..."
cargo install --path .

echo "Installing almk (Alum build tool)..."
cd $LOCALCATION/alum-make
cargo install --path .

echo "Building alum-std (Alum standard library)..."
cd $LOCALCATION/alum-std/alum
almk build

echo "Installing alum-std headers..."
sudo mkdir -p /usr/local/include/alum
sudo cp headers/* /usr/local/include/alum/

echo "Building Rust runtime (libalum_std.a)..."
cd $LOCALCATION/alum-std
cargo build --release

echo "Merging static libraries..."
cd $LOCALCATION/alum-std/target/release
mkdir -p merged_objs
cd merged_objs
ar x ../libalum_std.a
ar x ../../alum/target/libalum.a
ar rcs ../libalum.a *.o
cd ..
rm -rf merged_objs

echo "Installing merged standard library..."
sudo cp libalum.a /usr/local/lib/libalum.a
