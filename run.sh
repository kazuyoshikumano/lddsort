docker run --rm -it ^
    -v $(pwd):/usr/src/lddsort ^
    rust:latest ^
    bash -c "ln -fs /usr/src/lddsort/target/release/lddsort /usr/bin && bash"