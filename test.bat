docker run --rm -it ^
    -v %~dp0:/usr/src/lddsort ^
    -w /usr/src/lddsort ^
    rust:latest ^
    bash -c "cargo build --release && /usr/src/lddsort/target/release/lddsort ../../lib"