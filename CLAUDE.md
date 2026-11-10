# Photoes

A fast, minimal, touch-friendly, AI-first photos app for Linux.

## Principles

- **Work like a CLI tool.** Every operation is a pipeline: files in, files out.
- **Files hold all state.** A photo's state is its pixels, its metadata, its name and its folder. There is no database and no on-disk index. The app derives every view from the files at launch, so a design change never needs a migration.
- **No hidden work.** No processing starts without a user action. Every running job shows progress. Jobs may run in parallel.
- **Interrupted work leaves a visible trail.** See "File names carry job state" below.
- **Fast.** The app opens near instantly. Images open, scrub and edit without lag. Decode, preview and encode use the GPU.
- **Minimal interface.** Clean Apple-style design with few buttons, windows and boxes. Every control works by touch.

## Stack

- Rust, GTK4 and libadwaita for the app.
- GStreamer with VA-API for video decode and encode.
- libjxl for images.
- Claude Sonnet describes images and video keyframes.
- ElevenLabs Scribe transcribes video audio.
- A local face model through ONNX Runtime groups people.

## Views

- Three tabs: Date, Location (GPS) and Folders (as stored on disk).
- Inbox toggle: shows unsorted files.
- Favourites toggle: shows favourited files.
- AI Search: searches the text stored in the files. Without an API or network it falls back to fuzzy search.
- People: lists the identified people so the user can name them.

## Formats

The inbox accepts a wide range of formats. Sorting converts each file to one of two formats.

| | Sorted format | Conversion |
|---|---|---|
| Image | JPEG XL (`.jxl`) | JPEG transcodes losslessly. Other formats are re-encoded. |
| Video | MP4 (`.mp4`) | H.264, HEVC and AV1 are remuxed untouched. Edits and other codecs encode to AV1 with Opus audio. |

Metadata lives in the file:

| Data | Field |
|---|---|
| GPS, date, camera | Exif |
| AI description and transcript | XMP `dc:description` |
| Favourite | XMP `xmp:Rating` |
| People names | XMP, and `~/.local/share/photoes/` for the list of named people |
| Edit settings | XMP of the edit |
| Grid preview | A 512 px image embedded at sort time, so the grid never decodes a full file |

## File names carry job state

- `PROCESSING-` prefix: the file is in a pipeline. The app renames the source before work starts. The result also carries the prefix while it is being written, so a half-written file never looks finished. The app removes the prefix from the complete result, then deletes the source. On launch, the app finds every `PROCESSING-` file and offers to resume.
- `.` prefix: the file is the original of an edit. `.IMG_1234.jxl` is the original of `IMG_1234.jxl`.

## Editing

- An edit is a new file holding the edited pixels. Any other device or app shows it as edited.
- A file has at most one edit. To make a second edit, the user duplicates the file first.
- Changing the settings of an edit regenerates it from the original. The edit panel previews on the GPU while a setting moves. Leaving the panel re-encodes the edit.
- Reverting an edit deletes it and unhides the original.
- Deleting an edit deletes its original too.
- Image and video: crop, exposure, contrast, highlights, shadows, white balance and saturation. Video adds trim and corrective audio. No gimmick effects.
- The user copies edit settings from one file and pastes them onto many.

## Sorting

Sorting moves files from the inbox into real folders, such as one folder per holiday or activity. The user decides the folders. The structure stays consistent, so the library archives to any Linux system or drive as plain folders.

## Commands

```sh
nix develop          # shell with the Rust toolchain and every library
cargo run
cargo clippy
cargo fmt
nix build            # the package, as users install it
```

## Releasing

1. Set `version` in `Cargo.toml`.
2. Push a `vX.Y.Z` tag. `.github/workflows/aur.yml` stamps `aur/PKGBUILD` with the version and checksum, then pushes it to the AUR.

Keep `buildInputs` in `flake.nix` and `depends` in `aur/PKGBUILD` in step.
