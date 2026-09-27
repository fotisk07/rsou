check:
    @cargo fmt
    @cargo check

run *args: check
    cargo run -- {{args}}

test:
    @cargo fmt
    @cargo test
