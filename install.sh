cargo install --path .
cd alum-std
cargo build --release
sudo cp target/release/libalum_std.a /usr/local/lib/libalum.a
sudo cp -r alum/ /usr/local