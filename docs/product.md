# Photoes

Photoes is a fast, minimal, touch-friendly, AI-assisted photos app for Linux.

In this document a photo means an image or a video file.

## The problem it solves

Cleaning up a phone's photo library by hand, in spare moments such as a flight, goes wrong in three ways:

- Scrolling through thousands of photos and deleting junk, it is easy to miss a photo or delete one worth keeping.
- A session stops whenever attention drifts, and nothing records where it stopped. The library ends up patchy, with some stretches cleaned and others full of junk.
- There is no way to work through the photos of one event as a batch. Those batches need the most thought, so they get skipped for later.

Photoes keeps unsorted photos in one place, the inbox. Whatever is still in the inbox is still to sort, so a stopped session resumes where it left off. An AI agent proposes what to keep, what to delete and which album each photo belongs in. The user approves every change. Albums live in the user's own storage, such as a cloud drive, rather than Apple's.

Photoes keeps the iOS Photos features worth having: browsing by date and by location, finding photos by the people in them, and quick crop, rotate and trim.

## Libraries and albums

- An album is a folder that holds one group of photos: an event such as a holiday, an activity such as a sport, or a mix of both.
- A library is a location where albums are stored. The `library` key in the config file lists one or more libraries.
- The app shows the photos of every library mixed together, with no grouping by library. Only the Folders tab separates them: its top level shows each library side by side.

## Principles

- **A library is plain folders of plain files.** Everything Photoes knows about a photo is stored in the photo's own file, its name and its album. A library archives to any Linux system or drive by copying folders.
- **Nothing breaks on an update.** There is no library database to corrupt or migrate. Outside the libraries and the inbox, Photoes keeps only its config file, its log and caches such as thumbnails and downloaded map tiles. Deleting a cache costs only the time to rebuild it.
- **Every setting is in the config file.** `~/.config/photoes/config` is plain text, and the app keeps no setting anywhere else.
- **Photoes works like a command-line tool.** Each operation takes files in and puts files out.
- **Nothing happens behind the user's back.** No processing starts unless the user starts it. Running work always shows its progress.

## The main view

The main view shows the photos. Three tabs choose how they are arranged:

- **Date:** by when each photo was taken.
- **Location:** by where it was taken, from its GPS position.
- **Folders:** by library, then by album.

Four controls sit alongside the tabs:

- **Inbox toggle:** on shows only unsorted photos in the inbox. Off shows only sorted photos.
- **Favourites toggle:** on shows only favourited photos. Off shows every photo.
- **Search:** fuzzy text search over the AI text and people's names stored in each sorted photo. It needs no AI service or internet connection.
- **Status light:** shows what the app is doing. See "Visible, durable work".

Opening a photo shows the editor view, one photo at a time. It shows the photo's AI text and lets the user change it, and it opens the edit panel. The same view reviews staged photos before a sort.

## The inbox

The inbox is the dumping area. Photos downloaded to the computer or dropped from a phone land there and count as unsorted.

The inbox accepts a wide range of image and video formats.

## Sorting

Sorting moves photos out of the inbox into albums. Nothing is sorted or deleted until the user confirms it.

1. The user turns on the Inbox toggle and selects some photos.
2. Optionally, the user types some context about the photos, such as "Sam's birthday".
3. The user asks the agent to sort them.
4. For each photo the agent picks an album and the library that holds it, and writes the AI text. It may rotate a photo it thinks is sideways. It never crops or trims.
5. The agent then marks for deletion any photo it thinks is not worth keeping, such as a duplicate, a blurry shot or one with better similar shots. The photo is already prepared as in step 4, so keeping it costs nothing more.
6. The photos are now staged. The grid shows a staged icon on each one, and a deleted icon on each one marked for deletion.
7. The user reviews the photos one by one in the editor view. The user can change the album, the library and the AI text, and can crop, rotate and trim. The user can also mark a staged photo for deletion, or stage a photo marked for deletion.
8. The user confirms. Staged photos move into their albums, and photos marked for deletion are deleted.

Sorting by hand uses the same staging and confirmation, in either of two ways:

- Select photos, then mark them for deletion or type context saying where they belong. The agent stages them from that.
- Open a photo in the editor view and fill in the album, the library and the AI text. This stages the photo without the agent.

Rules for every sort:

- Album names follow one consistent pattern across the libraries.
- Sorting converts every photo to one consistent format: JPEG XL for images and MP4 for video. The file keeps the date, GPS position and camera details it came with.
- The agent sends each image to Claude Sonnet, and keyframes from each video. ElevenLabs transcribes the audio of each video. Sorting is the only step that uses AI.
- The AI text is stored as plain text inside the file, so a sorted photo carries everything needed to search for it.

## People

Photoes finds photos by the people in them without a list of people stored outside the files.

- Sorting runs a local face model on each photo. No face data goes to an AI service.
- Each face's position, name and fingerprint are stored in the photo's file. A fingerprint is a list of numbers the face model computes, and faces of the same person get similar fingerprints. The file also records which model made it.
- The People view groups similar faces from every library. It is rebuilt from the files at launch.
- Naming a group writes the name into every file in it. The user can merge and split groups, because the model gets some faces wrong.
- Fingerprints from different models cannot be compared. After a model change, the user starts a pass that computes new fingerprints from the stored face positions. Names stay with the faces, so none are lost.

## Editing

The edit panel opens at the side of the editor view.

The tools are corrective. There are no gimmick effects.

- **Images and video:** crop, rotate, exposure, contrast, highlights, shadows, white balance, saturation and colour.
- **Video only:** trim and corrective audio.

The user can copy the settings of one edit and paste them onto many other photos.

### How edits are stored

- Saving an edit writes a new file that holds the edited result. Copied to any other device, it shows as edited.
- The original becomes hidden. Its name gains a `.` prefix: `.IMG_1234.jxl` is the original of `IMG_1234.jxl`. A `.` prefix always means "original".
- A photo has at most one edit, which sits in front of its original. To keep several edits, the user duplicates the file first and edits each copy.
- Changing the settings of an edit regenerates it from the original.
- Reverting an edit removes it and unhides the original.
- Deleting an edit deletes its original with it.

These rules mean Photoes never loses track of a file.

## Expectations

### Speed

- The app opens near instantly.
- Photos open instantly.
- Scrolling, scrubbing and editing never lag.
- Touch input responds immediately.
- Display, playback and editing are hardware accelerated.

### Design

- Ultra clean, in the style of Apple's current design.
- As few buttons, windows and boxes as possible.
- Every control works by touch.

### Visible, durable work

- Sorting and re-encoding show a clear progress report. Work may run in parallel.
- The status light is always visible in a corner of the window. It is green when the app is idle, amber while work runs and red after an error.
- Clicking the status light or pressing its hotkey opens the log. The log records every action Photoes takes and every error, so any change to a file can be traced. Entries expire after 7 days by default.
- No work is lost when it stops partway, whether it fails, the app quits or the computer sleeps. Staged photos reopen as they were left, with their albums, AI text, rotations and deletion marks.
- A sorted photo's original stays in the inbox until its sorted copy is completely written to disk, and only then is deleted. An edit's original gains its `.` prefix only after the edit is completely written. A crash at worst leaves both the original and its result.
