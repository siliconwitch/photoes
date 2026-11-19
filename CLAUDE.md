# Photoes

A fast, minimal, touch-friendly, AI-assisted photos app for Linux.

## Documents

- `docs/product.md` says what the app is and how it is used. Read it before changing behaviour. It is a fixed document. Never edit it unless the user explicitly approves that edit. It changes only when the user requests a big new feature or change.
- "Decisions" below lists decisions that have no code yet. Once code implements one, delete its entry. Keep its reason as a comment only when an agent could not work it out from the code, following the comment rule in the `technical-writing` skill.

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

## Decisions

- **Sorted images are JPEG XL.** JPEG input is transcoded losslessly, so it converts back to the exact original bytes. Other formats are re-encoded.
- **Sorted video is MP4.** H.264, HEVC and AV1 are remuxed, not re-encoded. Edits and other codecs encode to AV1 with Opus audio. Never assume a sorted `.mp4` is AV1.
- **Each sorted file embeds a 512 px preview.** The grid draws tiles from it and never decodes a full file. Embedded, not cached, so a copied library draws at once.
- **Metadata lives inside each file.** Images keep GPS, date and camera in Exif. AI text goes in XMP `dc:description`, the favourite in XMP `xmp:Rating`, and people names and edit settings in XMP. A metadata change never re-encodes pixels or streams.
- **The edit panel previews on the GPU and encodes when the panel closes.** An encode takes seconds for an image and minutes for video. Preview and encode share the same correction maths, so the file matches the preview.

### Not decided

Ask the user before choosing any of these. Do not infer an answer from the code.

- Which library decodes inbox formats when sorting. gdk-pixbuf only makes thumbnails today.
- Whether to bundle our own icons.
- Whether the map gets a dark style, and whether libshumate stays, since its label widgets make zooming stutter.
- Which fields hold the embedded preview, faces and people names, and which `xmp:Rating` value means favourite.
- Where video keeps its GPS, date and camera details. MP4 has no Exif.
- How staged work survives a crash, and how a write avoids a half-written file that looks finished.
- Which face model to use, and which AV1 encoder runs when the GPU has none.

## Commands

direnv loads the flake's dev shell through `.envrc`, so the Rust toolchain and every library are already on `PATH`. Do not run `nix develop`.

- While testing a change with the user, restart the app after each edit. Stop the running copy with `pkill -x photoes`, then start `cargo run` in the background.
- Before finishing a change, run `cargo fmt` and `cargo clippy`.
- `nix build` builds the package as users install it. It sees only git-tracked files, so run `git add` on a new file first.

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
