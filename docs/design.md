# Design decisions

Each section gives a decision and the reason for it. Read the section for the module you are working on.

Three sections describe working code: "Configuration", "Interface shell" and "Packaging and release". Every other section is a decision that new code must follow.

Measurements come from the development machine: 16 threads, libjxl 0.12, a 12 MP (4000 x 3000) synthetic image, warm page cache.

## State lives in the files

There is no database and no on-disk index. Files hold all state: pixels, metadata, name and folder.

Why: a library database is the part of a photos app that corrupts, breaks on updates and needs migrations. With none, an app redesign cannot break a library, and a library copied as folders is complete.

The one exception is the list of named people in `~/.local/share/photoes/`. Each file still carries its own people names, so search works without that list.

## A cache may only cost speed

State is anything lost when deleted. A cache is rebuilt from the files or the network. Caches are allowed under these rules:

- It lives under `~/.cache`. Deleting that folder changes nothing but speed.
- Each entry is checked against its source before use. A stale or damaged entry is rebuilt, never trusted. Map tiles are the exception: nothing local can check one, so they are replaced by age.
- It has no format of ours to migrate. A changed format ignores the old entries.
- Only work the user started fills it. Opening the map is such a start, and it covers the tiles downloaded around the view.

The caches are the thumbnails, the map's tile index and the map's tile store. "Interface shell" describes each.

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

Why embedded and not cached: the preview travels with the file. A copied library, a new machine or an emptied cache draws at once, with no decode of every photo first. It adds 17 KB per file.

JPEG XL defines a preview frame for this, but libjxl 0.12 can only read one, not write one. Its other built-in route, the 1:8 progressive decode, is the 29 to 47 ms measured above.

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

Opening the Location tab starts the map, and the map needs the tiles around the view to zoom smoothly. The user ruled that downloading them is part of that user action, not background work. No new batch starts once the tab is left. A batch already queued still finishes.

## Toolkit

Rust with GTK4 and libadwaita for the app, GStreamer with VA-API for video decode and encode, and libjxl for images.

- GTK4 and GStreamer both have maintained Rust bindings. Go's are weaker.
- GTK4 has no backdrop blur. Floating controls are translucent but do not blur what is behind them. The user accepted this over Qt Quick in C++.

## Configuration

`~/.config/photoes/config` names the directories, one `key = value` per line.

```
library = ~/Pictures
inbox = ~/downloads
```

- `library` holds sorted files and `inbox` holds unsorted files. Either key may repeat.
- A leading `~/` expands to the home directory.
- The format is plain lines because it needs no parser dependency.
- A missing file means no directories. The app opens empty.

## Interface shell

This is a first draft. Favourites and search have controls but no behaviour yet.

**Modules.** `main.rs` builds the window and wires the controls. `date_view`, `location_view` and `folder_view` each own one tab. `library` decides which files are photos. `thumbnail` decodes, saves and holds their thumbnails. `map_source` fetches the tile index, adapts the map style and stores the tiles. `config` reads the config file. The views never call each other.

**Controls float over the gallery** in a `gtk::Overlay`. The glass look is translucency and shadow only, because GTK has no backdrop blur.

**Icons come from the Adwaita icon theme.** It has no inbox tray and no heart, so Inbox uses `folder-download-symbolic` and Favourites uses a star.

**The Inbox toggle is a filter.** Off shows every file. On shows only files under an `inbox` directory.

**Date view orders by file modification time**, newest first. Nothing reads the Exif date yet.

**Grid tiles are square because the thumbnails are.** A `gtk::Picture` holding a square texture is as tall as its column is wide. This needs `vscroll-policy` set to natural, and a 1 x 1 placeholder texture before the thumbnail arrives. No custom widget is involved.

**Each view sets its own column count.**

- `columns = visible width / tile size`, applied as `max-columns`.
- `min-columns` stays 1. A higher minimum sets a minimum window width and stops the window shrinking.
- The visible width comes from the scroller's horizontal `page-size`. The handler defers to idle, because columns cannot change during a layout pass.

**Pinch zooms.**

- Date view scales the tile size continuously between 48 and 512 px, starting at 160.
- Folder view steps through six levels, three levels per doubling of finger distance. A level change resets the list factory, which rebuilds every row.

| Folder level | Layout | Icon size |
|---|---|---|
| 0, 1, 2 | One column of rows | 24, 40, 64 px |
| 3, 4, 5 | Grid of icons | 96, 144, 224 px |

**Folder view lists only folders that contain photos.** It is derived from the scanned photo paths, not from the disk. The first entry `..` goes up. No back button is needed.

**Thumbnails are a stopgap for unsorted files.** Sorted files will use their embedded preview.

- gdk-pixbuf decodes a thumbnail that fits in 512 x 512 px on a worker thread. HEIC comes from libheif's gdk-pixbuf loader. Decoding and saving took 100 to 180 ms per photo on the user's photos.
- Decoding runs on its own thread pool, half the processors wide. `gio::spawn_blocking` is not used, because it shares GLib's task threads with libshumate's tile reads. With it, a first launch on 727 photos left the map without a single tile for over 6 s. With the separate pool the map drew while the photos decoded.
- Each thumbnail is saved to `~/.cache/thumbnails/x-large`, following the freedesktop thumbnail standard. The file name is the MD5 of the photo's `file://` address. The PNG records that address and the photo's modification time, and is used only while that time matches the photo.
- The size is the standard's 512 px, not its 256 px, because the user works on a 5K display at scale 2. A saved thumbnail is 220 to 350 KB, 196 MB for 726 photos.
- Why that folder and not our own: the standard is the format, so there is none of ours to maintain, and file managers read and write the same entries.
- The standard wants private files, so the folders are mode 700 and the files 600. `glib::file_set_contents_full` writes each file whole or not at all.
- A tile shows the centre square of the thumbnail. A 4:3 photo gives a 384 px square, which at scale 2 is sharp up to a 192 px tile. The standard's next size is 1024 px.
- `thumbnail::Store` holds the textures decoded this session, so a tile scrolled back, a tab switch and a map pin reuse them. `main` makes one store and hands it to every view. It holds 256 MB of textures, about 580 photos, and drops the ones unused for longest.
- Measured on the user's 727 photos with a long scroll into unseen rows: tiles kept arriving for 1.1 s after the scroll with nothing saved, and were already drawn when it ended with thumbnails saved or held.
- Unbinding a tile drops its `thumbnail::Request`, which cancels a decode that has not started. Without this, a fast scroll queues hundreds of dead decodes.
- A file counts as a photo when gdk-pixbuf has a loader for its extension. Video is not shown yet.

**The map is libshumate drawing OpenFreeMap vector tiles.**

- Vector tiles are used because the app draws the labels, so it can choose their language. Image tiles have each country's own language baked in.
- Labels use the first system language from `glib::language_names()`, then any Latin-script name, then the local name. The tiles carry names in about 80 languages.
- `src/map_style.json` is OpenFreeMap's Positron style, unmodified, so a newer copy can replace it. `src/map_style.LICENSE.md` is its licence and must ship with it. `map_source` adapts the style in memory each time it loads.
- **Few labels are drawn**, because libshumate builds a widget for each label and the full set makes zooming stutter. Kept, in the style's own ranking: countries, cities, towns, main roads, streets, then lakes and rivers. Dropped: seas, oceans, bays, streams, canals, states, villages, suburbs and other small places, footpath names, road numbers and airports. `map_source` drops the layers and narrows the water layers by `class`.
- **Names appear in the order someone zooming in looks for them.** Countries show from the start, cities from zoom 7 and towns from zoom 9, then main roads from 12 and streets from 15 as the style has them. The style itself shows cities from zoom 3 and towns from zoom 6, while many countries are still in view. At zoom 7 a 2560 px wide map spans 14 degrees, about one large country.
- Measured at 5K over a city, zoom 9.5 to 12.6: the frame time lost to slow frames fell from 390 to 540 ms to 170 to 190 ms, and the worst frame from 100 to 150 ms to 50 ms. With no labels at all it was 110 to 160 ms. At world zoom the trim changes little, because the labels there are the kept ones.
- The app fetches `https://tiles.openfreemap.org/planet`, the tile index, to learn the tile address. OpenFreeMap changes that address weekly, and libshumate cannot follow the link in the style itself.
- The index is saved as `~/.cache/photoes/tile-index.json`. The map draws from the saved index at once, then fetches a fresh one. It switches source only when the fetched address differs, and saves the new index after the source builds.
- The style writes `["linear", 1]`, which libshumate rejects. `map_source` rewrites it to `["linear"]`.
- The fetch and the tiles start when the Location tab opens. That is the user action that allows the network call.
- Offline, a saved index still draws the tiles in the store, with pins. With no saved index the map is blank with no pins, because pins need the tile source for their projection. The fetch retries the next time the tab opens.
- **The app keeps its own tile store** in `~/.cache/photoes/tiles/<zoom>/<x>/<y>.pbf`. `map_source::TileStore` is a libshumate data source that reads a tile from there and downloads it when it is missing. libshumate's own store is fixed at 100 MB and shared with every libshumate app. A minute of zooming at 5K downloaded 54 MB, so it kept dropping tiles and downloading them again.
- Tiles are stored by number, not under the weekly tile address. A new address therefore downloads nothing again, and neighbouring tiles can come from different weeks.
- Past 2 GB the tiles downloaded longest ago are deleted, once per launch on a worker thread. That is also the only refresh: a stored tile is never checked against the server.
- `TileStore` overrides `start_request`. The `libshumate` crate 0.8.1's default version completes the request with no data source attached, and libshumate then fails every tile with `assertion 'SHUMATE_IS_DATA_SOURCE (self)' failed`.
- The renderer still builds libshumate's own downloader from the style before the store replaces it. That leaves an empty folder in `~/.cache/shumate`.
- **The map downloads tiles ahead of the user.** When the view has rested for 300 ms it fetches the view one zoom level out, then the current level with half a view more on every side, then the view one level in. Zooming out uncovers the most unloaded map, so it goes first.
- Six of these downloads run at a time, to go easy on a free tile server. They wait while libshumate is downloading a tile it needs to draw, and a move of the view drops the ones still waiting. Started straight away, they shared the connection with the visible tiles and delayed them by seconds.
- Measured at 5K from an empty store: a zoom level never seen before was fully drawn 185 to 1461 ms after the zoom stopped without these downloads, and between 32 ms before and 90 ms after with them.
- libshumate's `map-loaded` signal is not used as the trigger. It did not fire for every load.
- Drawing is not the slow part. At 5K, tiles already stored were all drawn 8 to 250 ms after the view stopped moving. Tiles that had to download took 280 to 1170 ms. World-level tiles are large: the 16 tiles of zoom 2 are 14 MB.
- The map widget's background is Positron's land colour. Tiles not yet loaded then look like empty map, not like holes in a dark window.
- Zooming is not smooth and the app cannot fix it. In a headless 5K run a wheel zoom drew a frame every 30 to 50 ms where 17 ms is smooth. A profile of those frames shows libshumate building and placing its labels, each of which is a widget. Without labels the slow frames halved. Pins and the view limit made no difference.
- `location_view` queues a layout of the base map layer two frames after setting a source. Without it the map stayed blank until the next pan in 4 of 6 first launches, and in 0 of 12 with it. The cause, read from libshumate 1.7's `should_defer`: it takes the jump from no source to the current view for a fast pan and holds the tiles back, and its one retry can see the same frame time and keep holding. The layout must come a frame later than that retry. A layout from an idle handler came too early in 2 of 5 launches of the release build. The `v1_4` feature of the `libshumate` crate exists only to reach that layer.
- Vector tiles are 512 px, twice the size of image tiles, so every zoom level shows twice the scale.
- **The view never leaves the world.** Zooming out stops when the world is as wide or as tall as the map, whichever comes first. Panning stops at the poles and at the east and west edges. Past any of them, libshumate repeats the world as a grid.
- libshumate's own minimum zoom is a whole number and resets when a source is set, and it has no pan limits. So `location_view` corrects the zoom and the position each time the viewport reports a change. A `gtk::DrawingArea` laid over the map supplies the size changes, which the map does not report.
- libshumate wraps a pan past the east or west edge round to the far side of the world. `location_view` remembers which half of the world the view was in and stops it at that edge. Stopping at the nearer edge instead would throw a long drag across the map.
- At the limit a wheel turn is stopped before libshumate sees it, and a zoom animation that reaches the limit is stopped from an idle handler. Otherwise libshumate goes on moving the map towards where a further zoom would centre it.
- **Two fingers on a touchpad pan the map, and a pinch zooms.** libshumate alone zooms on any scroll. `location_view` takes scroll events measured in pixels, which come from a touchpad, and leaves wheel clicks to libshumate. The pan has no glide after the fingers lift.
- The pan multiplies the touchpad's distance by 2.5. GTK's scrolled windows use the same factor (`MAGIC_SCROLL_FACTOR`), so the map moves as far as the photo grid for the same stroke.
- libshumate's scroll controller has its kinetic flag removed. With it, the end of every two-finger pan started libshumate's 200 ms zoom glide, which reset the map's position each frame. A second stroke or a pinch inside that time was partly thrown away: three quick strokes moved the map 720 px of 900.
- Tiles stop at zoom 14. libshumate enlarges them beyond that, and street level still draws.
- The map only pans and zooms. libshumate has no switch for rotation, so `location_view` finds the map's `GestureRotate` controller and sets its propagation phase to none. Removing the controller or resetting the angle would fight libshumate's own handlers.

**The launch scan reads each photo's GPS position from Exif** with the `kamadak-exif` crate, which parses JPEG, HEIC and PNG. Scanning 727 photos, Exif included, took 34 ms with a warm cache.

**Photos appear on the map as pins that cluster.**

- Photos in the same 96 px square of the map share one pin. The square grid is fixed to the world at the whole-number zoom level, so pins do not shuffle while panning.
- A pin shows the first photo of its square and a count when it holds more than one. Zooming in splits a square, and the first photo keeps its pin.
- Pins exist only within 2048 px of the map centre, which covers a 4K window. This bounds the number of thumbnails decoded.
- A hidden map makes no pins. They are built when the Location tab opens.
- Pins in neighbouring squares can overlap.

## Packaging and release

- **`flake.nix` uses `rustPlatform.buildRustPackage` and only the `nixpkgs` input.** Fewer inputs to keep current.
- **`wrapGAppsHook4` is required.** It gives the installed binary its GTK and GStreamer plugin paths.
- **Nix builds only git-tracked files.** Run `git add` on a new file before `nix build`.
- **`flake.nix` and `aur/PKGBUILD` list libjxl and GStreamer before the code uses them.** The stack is decided, and the dev shell needs them.
- **`flake.nix` builds its own gdk-pixbuf `loaders.cache`.** gdk-pixbuf reads one cache file, and nixpkgs ships one per package. Without the merged file, HEIC does not load.
- **glib-networking is a dependency.** It provides TLS. Without it the map is blank.
- **libsoup is a direct dependency.** libshumate already uses it. The app uses it once, to fetch the tile index.
- **libshumate must be 1.4 or newer.** The `v1_4` crate feature links `shumate_simple_map_get_base_map_layer`.
- **The wrapper adds the Adwaita icon path itself.** `wrapGAppsHook4` does not, and a desktop without GNOME then shows missing-icon marks.
- **The dev shell's `shellHook` repeats those three settings.** `cargo run` starts the unwrapped binary.
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

- Which library decodes inbox formats in the sort pipeline. gdk-pixbuf is only the draft's thumbnailer.
- Whether to bundle our own icons, for a proper inbox icon and one look on every desktop.
- Whether the map gets a dark style. Positron is light only.
- Whether libshumate stays as the map. Its labels make zooming stutter, and trimming them removed about half of that. MapLibre Native draws on the GPU. Whether it has a usable GTK 4 widget is unchecked.
- Which field holds the embedded preview.
- Which `xmp:Rating` value means favourite.
- Which XMP field holds people names.
- Which face model to use. Its licence must allow redistribution.
- How AI Search uses Sonnet.
- How the AI chooses folders when it sorts.
- Which AV1 encoder runs when the GPU has none.
