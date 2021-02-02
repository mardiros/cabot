test:
    cargo test
    cd tests/functionals && poetry install && poetry run behave

doc:
    cargo doc --no-deps --features json
    firefox target/doc/cabot/index.html
