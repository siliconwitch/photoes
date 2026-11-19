# Photoes

A fast, minimal, touch-friendly, AI-assisted photos app for Linux.

## Documents

- `docs/product.md` says what the app is and how it is used. Read it before changing behaviour. It is a fixed document. Never edit it unless the user explicitly approves that edit. It changes only when the user requests a big new feature or change.
- `docs/design.md` records the technical decisions and the reason for each. Read the relevant section before working on a module. Update it when a decision changes. Keep it short: reasons only, never a log of the work.

## Glossary

Use these terms in code, comments, docs and the interface. Never use a term from the Avoid column for the same thing.

| Term | Meaning | Avoid |
|---|---|---|
| photo | An image or a video file. | picture, media, item, asset |
| image, video | One kind of photo, when only that kind is meant. | |
| library | A location where albums are stored. There can be several, and the config file's `library` key lists every one. | collection, root, gallery |
| album | A folder in a library that holds one group of photos. | event, collection, folder (for an album) |
| inbox | The location of unsorted photos. | import folder, unsorted folder |
| sorted, unsorted | In an album, or still in the inbox. | imported, organised |
| sort | Move photos from the inbox into albums, after the user confirms. | import, ingest, organise |
| agent | The AI that prepares photos for a sort. | assistant, bot |
| context | Text the user types to tell the agent about the photos. | prompt, hint |
| AI text | The description and transcript stored in a photo's file. | caption, tags, analysis |
| staged | Prepared for a sort, by the agent or by hand, and waiting for the user to confirm. | pending, draft, queued |
| marked for deletion | Proposed for deletion and waiting for the user to confirm. Nothing is deleted yet. | deleted (before confirmation) |
| editor view | The view that shows one photo, its AI text and the edit panel. | photo view, detail view, viewer, lightbox |
| edit | The file that holds an edited result. | version, copy |
| original | The hidden `.`-prefixed file an edit was made from. | master, source, backup |
| favourite | A photo the user has favourited. | liked, starred |
| search | Fuzzy text search over the AI text and people's names. | AI search, smart search |
| fingerprint | The numbers a face model computes for one face. | embedding, face vector, face ID |
| status light | The corner indicator of the app's state. | indicator, badge, spinner |
| log | The record of every action and error, opened from the status light. | history, journal, activity |
| cache | Data under `~/.cache` that rebuilds itself when deleted. | database |
| config file | `~/.config/photoes/config`. | settings, preferences |

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
