build:
  cargo build --release

run:
  export RUST_LOG=holly=debug
  cargo run

deploy: build
  sudo service holly restart

screenshot:
  python3 -c "import holly; holly.HollyClient().screenshot()"

log:
  python3 -c "import holly; holly.HollyClient().html()"


