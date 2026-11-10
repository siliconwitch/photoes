# Design decisions

Each section gives a decision and the reason for it. Read the section for the module you are working on.

As of `v0.1.0` only "Packaging and release" describes working code. The crate is an empty window. Every other section is a decision that new code must follow.

Measurements come from the development machine: 16 threads, libjxl 0.12, a 12 MP (4000 x 3000) synthetic image, warm page cache.

## State lives in the files

There is no database, no on-disk index and no thumbnail cache. Files hold all state: pixels, metadata, name and folder.

Why: a library database is the part of a photos app that corrupts, breaks on updates and needs migrations. With none, an app redesign cannot break a library, and a library copied as folders is complete.

The one exception is the list of named people in `~/.local/share/photoes/`. Each file still carries its own people names, so search works without that list.

## The app scans the library at launch

Date view, Location view and search need metadata from every file. The app reads it at launch into memory and searches in memory.

Why this is fast enough: listing 20,000 files took 3.6 ms, and opening each and reading its first 2 KB took 46 ms, or 2.3 µs per file. That scales to about 0.12 s for 50,000 files.

Consequences:

- Write the metadata at the front of each file, so the scan reads a few KB and never the whole file.
- Folders view needs only a directory listing. It must not wait for the scan.
- The cold-cache cost is unmeasured. The estimate is seconds on NVMe and minutes on a spinning disk. The scan must show progress.

## Each sorted file embeds a 512 px preview

The grid draws tiles from a preview embedded in each file at sort time. It never decodes a full file.

Why: decoding the 12 MP image for a tile took 29 to 47 ms on 4 threads, even with libjxl's 1:8 progressive decode. A screen of tiles then takes about a second, and fast scrolling shows blanks. A 512 x 384 preview encoded at distance 2 was 17 KB and decoded in 4.4 ms on one thread.

Why embedded and not cached: a cache is state outside the files.

## Formats

| Decision | Reason |
|---|---|
| Sorted images are JPEG XL. | Its container holds Exif and XMP beside the pixels, and it compresses well. |
| JPEG input is transcoded losslessly. | The JXL is smaller (17% in one test) and converts back to the exact original bytes. Nothing is lost. |
| Other image formats are re-encoded. | They have no lossless path into JXL. |
| Sorted video is MP4. | MP4 carries XMP, and players everywhere read it. |
| H.264, HEVC and AV1 video is remuxed, not re-encoded. | Re-encoding is lossy and takes minutes per clip. |
| Edits and other codecs encode to AV1 with Opus audio. | Both are royalty free and compress well. |

Sorted video therefore has mixed codecs. Never assume a sorted `.mp4` is AV1.

## Metadata fields

| Data | Field | Reason |
|---|---|---|
| GPS, date, camera | Exif | The fields the camera already wrote. |
| AI description and transcript | XMP `dc:description` | A standard field, so other tools can read and search it. |
| Favourite | XMP `xmp:Rating` | A standard field that other photo apps show. |
| Edit settings | XMP of the edit | The edit panel reopens them, and copy and paste reads them. No state outside the file. |
| People names | XMP | Search by name works on a copied library. |

A metadata change rewrites metadata only. It must never re-encode pixels or streams.

## File names carry job state

There is no job journal. The file names are the journal, and the user can read them in any file manager.

Every pipeline follows this order, including the re-encode of an edit:

1. Rename the source to `PROCESSING-<name>`.
2. Write the result as `PROCESSING-<result name>`.
3. Rename the result to its final name.
4. Delete the source.

Why the result carries the prefix too: a crash during step 2 would otherwise leave a half-written file that looks finished.

At launch, each `PROCESSING-` file means one of these:

| Found | Meaning | Recovery |
|---|---|---|
| Source only | Stopped before or during step 2. | Run the pipeline again for that file. |
| Source and `PROCESSING-` result | Stopped during step 2. | Discard the partial result and run again. |
| Source and final result | Stopped before step 4. | Delete the source. |

Recovery needs the user's go-ahead, because no work starts without a user action.

## Edits

- **The edit file holds baked pixels.** Other devices and apps show the edit without knowing Photoes.
- **A `.` prefix marks the original.** Linux tools hide dotfiles, so the original disappears from view but travels with its folder. `.IMG_1234.jxl` beside `IMG_1234.jxl` means the second is an edit.
- **One edit per original.** A fixed one-to-one pairing by name needs no bookkeeping. Several edits need duplicated files.
- **Every settings change regenerates the edit from the original.** Editing an edit would compound lossy encodes.
- **The edit panel previews on the GPU and encodes on leaving the panel.** Encoding a 12 MP JXL ran at 10 MP/s on 4 threads, about 1.2 s per image. Video takes minutes. Neither can follow a slider.
- **Preview and encode use the same correction maths.** The baked file must match what the panel showed.
- **Deleting an edit deletes its original.** An orphaned hidden original would be a file the app has lost track of.

## AI

- **Claude Sonnet describes images and video keyframes. ElevenLabs Scribe transcribes video audio.** The user chose both services.
- **Faces are grouped by a local model through ONNX Runtime.** Sonnet does not recognise faces and returns no face embeddings, so it cannot build the People list.
- **Search has an offline path.** Fuzzy matching over the stored text needs no service.

## Work is visible

No pipeline, scan or network call starts without a user action, and each shows progress while it runs. Parallel work is fine. A silent background worker is a bug, even a helpful one.

## Toolkit

Rust with GTK4 and libadwaita for the app, GStreamer with VA-API for video decode and encode, and libjxl for images.

- GTK4 and GStreamer both have maintained Rust bindings. Go's are weaker.
- GTK4 has no backdrop blur. The design is therefore clean and flat, not translucent glass. The user accepted this over Qt Quick in C++.

## Packaging and release

- **`flake.nix` uses `rustPlatform.buildRustPackage` and only the `nixpkgs` input.** Fewer inputs to keep current.
- **`wrapGAppsHook4` is required.** It gives the installed binary its GTK and GStreamer plugin paths.
- **Nix builds only git-tracked files.** Run `git add` on a new file before `nix build`.
- **`flake.nix` and `aur/PKGBUILD` list libjxl and GStreamer before the code uses them.** The stack is decided, and the dev shell needs them.
- **`aur/PKGBUILD` is a template.** The workflow stamps `pkgver` from the tag and replaces `sha256sums=('SKIP')`. Never edit the copy on the AUR.
- **The workflow runs in `archlinux:base-devel`.** `updpkgsums` and `makepkg` exist only on Arch.
- **The workflow creates a `builder` user.** `makepkg` refuses to run as root.
- **Every SSH path in the workflow is explicit.** GitHub sets `$HOME` to `/github/home` inside a job container, but `ssh` takes the home directory from the password database. `~/.ssh` would point to two different places.
- **The AUR host key is pinned in the workflow.** It was checked against the fingerprint published on aur.archlinux.org, `SHA256:RFzBCUItH9LZS0cKB5UE6ceAYhBD5C8GeOBip8Z11+4`, on 2026-11-10. If the AUR rotates its key, the push fails until the line is updated.
- **The push targets `master`.** The AUR accepts no other branch.
- **The workflow publishes but does not build.** A `PKGBUILD` that fails to build still publishes. As of `v0.1.0` it has never been built on Arch.
- **The app ID is `io.github.siliconwitch.Photoes`.** It follows the convention for projects hosted on GitHub.
- **The licence is GPL-3.0-or-later.** Forks stay open.

## Not decided

Ask the user before choosing any of these. Do not infer an answer from the code.

- Where the library and the inbox live on disk.
- Which field holds the embedded preview.
- Which `xmp:Rating` value means favourite.
- Which XMP field holds people names.
- Which face model to use. Its licence must allow redistribution.
- How AI Search uses Sonnet.
- How the AI chooses folders when it sorts.
- Which AV1 encoder runs when the GPU has none.
