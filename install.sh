#!/bin/sh

export LOCALCATION=$PWD

echo "Installing alc..."
cargo install --path .

echo "Installing alum-std..."
cd $LOCALCATION/alum-std
cargo build --release
sudo cp target/release/libalum_std.a /usr/local/lib/libalum_std.a
sudo ln -sf /usr/local/lib/libalum_std.a /usr/local/lib/libalum.a
sudo mkdir -p /usr/local/include/alum
sudo cp alum/*.al /usr/local/include/alum/

echo "Installing almk..."
cd $LOCALCATION/alum-make
cargo install --path .
