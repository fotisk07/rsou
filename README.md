# Sou (Rust)

A personal accounting CLI, rewritten in Rust from the Python version of Sou.
The repository and Cargo package are named `rsou`; the executable is `sou`.

## Install

With Rust and Cargo installed, run from this repository:

```sh
cargo install --path .
sou --help
```

Cargo installs `sou` into `~/.cargo/bin` by default. Ensure that directory is on
`PATH`. Run `cargo install --path .` again after pulling updates to rebuild and
replace the installed executable.

### Switching from Python Sou

If Python Sou was installed with `uv tool install`, preserve its launcher before
replacing it with a link to the Rust executable:

```sh
# Run once, before replacing the Python launcher; sou-python must not exist.
ln -s "$(readlink -f ~/.local/bin/sou)" ~/.local/bin/sou-python
ln -sfn ~/.cargo/bin/sou ~/.local/bin/sou
```

`sou` now runs Rust; `sou-python` still runs Python. Keep the Python uv tool
installed for `sou-python` to work. Reinstalling or upgrading that tool may
replace the `sou` link; recreate the link afterward if necessary.

## Usage

```sh
sou --help
sou --version
sou check
```

The default journal is `journal.sou`; use `-j PATH` to select another file.

## Development

```sh
cargo run -- --help
cargo test
```
