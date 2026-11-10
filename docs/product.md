# Photoes

Photoes is a fast, minimal, touch-friendly, AI-first photos app for Linux.

## The problem it solves

Organising photos after taking them is tedious. A phone holds thousands of unrelated photos taken at different times. Sorting them happens in short sessions, and each session ends partway. Nothing records where the last session stopped, so the next one starts by hunting for that place. It is like reorganising one giant album.

Photoes gives unsorted photos one clear place, the inbox. The user drops everything there and sorts it whenever there is time, or asks the AI to sort it.

## Principles

- **A photo library is plain folders of plain files.** Everything Photoes knows about a photo is stored in the photo's own file, its name and its folder. The library archives to any Linux system or drive by copying folders.
- **Nothing breaks on an update.** There is no library database to corrupt, migrate or rebuild.
- **Photoes works like a command-line tool.** Each operation takes files in and puts files out.
- **Nothing happens behind the user's back.** No processing starts unless the user starts it. Running work always shows its progress.

## The main view

The main view shows the library. Three tabs choose how it is arranged:

- **Date:** by when each photo or video was taken.
- **Location:** by where it was taken, from its GPS position.
- **Folders:** by the folders on disk.

Three controls sit alongside the tabs:

- **Inbox toggle:** shows only unsorted files.
- **Favourites toggle:** shows only favourited files.
- **AI Search:** finds photos and videos by what is in them: people, places, objects and landmarks.

A separate **People** view lists the people found in the library, so the user can name them. AI Search then finds people by name.

## The inbox

The inbox is the dumping area. Photos downloaded to the computer or dropped from a phone land there and count as unsorted.

The inbox accepts a wide range of image and video formats.

## Sorting

Sorting moves files out of the inbox into real folders. The user sorts by hand or asks the AI to sort.

- A folder holds one group of photos: an event such as a holiday, an activity such as a sport, or a mix of both.
- The user decides the folders. The structure stays consistent across the library.
- Sorting converts every file to one consistent format: JPEG XL for images and MP4 for video. The file keeps the date, GPS position and camera details it came with.
- Sorting analyses every file. Claude Sonnet describes the content of each image and of keyframes from each video. ElevenLabs transcribes the audio of each video. This means sorting sends images, keyframes and audio to those services.
- The analysis is stored as plain text inside the file. A sorted file carries everything needed to search for it.

## AI Search

AI Search looks through the text stored in the files. When the AI services or the internet are unavailable, it falls back to fuzzy text search over the same text.

## Editing

Selecting a file and choosing to edit opens the edit view, a panel at the side of the photo or video.

The tools are corrective. There are no gimmick effects.

- **Images and video:** crop, exposure, contrast, highlights, shadows, white balance, saturation and colour.
- **Video only:** trim and corrective audio.

The user can copy the settings of one edit and paste them onto many other photos or clips.

### How edits are stored

- Saving an edit writes a new file that holds the edited result. Copied to any other device, it shows as edited.
- The original becomes hidden. Its name gains a `.` prefix: `.IMG_1234.jxl` is the original of `IMG_1234.jxl`. A `.` prefix always means "original".
- A photo or video has at most one edit, which sits in front of its original. To keep several edits, the user duplicates the file first and edits each copy.
- Changing the settings of an edit regenerates it from the original.
- Reverting an edit removes it and unhides the original.
- Deleting an edit deletes its original with it.

These rules mean Photoes never loses track of a file.

## Expectations

### Speed

- The app opens near instantly.
- Photos and videos open instantly.
- Scrolling, scrubbing and editing never lag.
- Touch input responds immediately.
- Display, playback and editing are hardware accelerated.

### Design

- Ultra clean, in the style of Apple's current design.
- As few buttons, windows and boxes as possible.
- Every control works by touch.

### Visible, resumable work

- Sorting and re-encoding show a clear progress report. Work may run in parallel, and the user is told whenever anything is processing.
- No data is lost when work stops partway, whether it fails, the app quits or the computer sleeps. The user can resume easily.
- Work happens in view of the user. A file being processed is renamed with a `PROCESSING-` prefix before work starts on it. It is deleted only after its result is completely written to the new location. If something goes wrong, the folders show how far the work got, and Photoes picks this up when it next opens.
