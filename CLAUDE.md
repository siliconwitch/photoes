# Photoes

A fast, minimal, touch-friendly, AI-first photos app for Linux.

## Documents

- `docs/product.md` says what the app is and how it is used. Read it before changing behaviour. It is a fixed document. Never edit it unless the user explicitly approves that edit. It changes only when the user requests a big new feature or change.
- `docs/design.md` records the technical decisions and the reason for each. Read the relevant section before working on a module. Update it when a decision changes. Keep it short: reasons only, never a log of the work.

## Commands

direnv loads the flake's dev shell through `.envrc`, so the Rust toolchain and every library are already on `PATH`. Do not run `nix develop`.

- While testing a change with the user, restart the app after each edit. Stop the running copy with `pkill -x photoes`, then start `cargo run` in the background.
- Before finishing a change, run `cargo fmt` and `cargo clippy`.
- `nix build` builds the package as users install it.

When adding or removing a system library, change `buildInputs` in `flake.nix` and `depends` in `aur/PKGBUILD` together.

## Testing the inbox

`scratch/` is ignored by git and holds a disposable library for testing the sort workflow:

- `originals/` holds test photos and videos. Never write to it.
- `inbox/` and `sorted/` are what the app reads. `~/.config/photoes/config` points `inbox` and `library` at them.
- `undated/` holds JPEGs with no embedded date, for testing how sorting dates them.

Reset the test library before each test run. The script empties `inbox/` and `sorted/`, then copies `originals/` into `inbox/`:

```sh
scratch/reset
```
