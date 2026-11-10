# Photoes

A fast, minimal, touch-friendly, AI-first photos app for Linux.

## Documents

- `docs/product.md` says what the app is and how it is used. Read it before changing behaviour. It is a fixed document. Never edit it unless the user explicitly approves that edit. It changes only when the user requests a big new feature or change.
- `docs/design.md` records the technical decisions and the reason for each. Read the relevant section before working on a module. Update it when a decision changes. Keep it short: reasons only, never a log of the work.

## Commands

```sh
nix develop          # shell with the Rust toolchain and every library
cargo run
cargo clippy
cargo fmt
nix build            # the package, as users install it
```

When adding or removing a system library, change `buildInputs` in `flake.nix` and `depends` in `aur/PKGBUILD` together.
