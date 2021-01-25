# RustPivot Implant

Requires the rustup/cargo to build the binary
First edit the 'port' and 'ip' variables in src/main.rs to match the backend
address configured for server.
Then build with: 
cargo build --release --bin implant

Note: it will build for whatever operating system it is compiled on unless
specified otherwise via the --target flag
