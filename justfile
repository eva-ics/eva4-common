all:
  @just -l

test:
  cargo test --features full
  CLIPPY_EXTRA_LINTS="-D warnings" clippy --features full
