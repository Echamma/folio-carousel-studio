# Folio Carousel Studio

This is the **Linux desktop branch**. The Windows version is preserved on `main`.

Download the x86-64 AppImage or Debian package from [GitHub Releases](https://github.com/Echamma/folio-carousel-studio/releases). On Linux, make the AppImage executable with `chmod +x`, then run it. If FUSE is unavailable, add `--appimage-extract-and-run`. For Debian/Ubuntu, install the `.deb` with `sudo apt install ./Folio-Carousel-Studio-1.4.0-amd64.deb`, then launch Folio from your application menu.

On Linux, `npm ci` installs development dependencies, `npm start` opens the editor, and `npm run build` creates AppImage and `.deb` packages. Use Node.js 22. `npm run build:windows` retains Windows packaging. Headless tests run with `xvfb-run -a npm test`. GitHub tests editor behavior, recovery on close, and the packaged executable on Ubuntu 22.04 before publishing Linux packages with SHA-256 checksums.

A Windows desktop app for making Instagram carousels, inspired by the collection in `D:\Figma Templates`. The existing animation studio in the parent folder is preserved.

## Run

Use the Linux AppImage or `.deb` package from Releases. Windows users can run `Folio-Carousel-Studio-1.4.0.exe` from `dist` after building this branch. No account is required to run the app; this private repository requires GitHub access to download it.

To run from source: `npm install`, then `npm start`. To rebuild the Windows portable executable: `npm run build:windows`. On Linux, use `npm run build`.

## Create

- Select one of ten editable layouts: Introduction, Text Only, Checklist, Quote, Image Center, Image Left, Image Right, Image Compare, Thank You, and Navy Ribbon.
- **Open** asks you to choose a project folder the first time. Folio remembers it and shows a searchable visual gallery of `.folio.json` files, with a cover preview for each project. Use **Change folder** in Projects or in the gallery to select another folder.
- The ten editable six-slide Fun Tech Facts carousels are added to the chosen folder on first selection, along with any projects previously saved in the app. Existing files with newer saved content are kept.
- **New** creates a new project file in the chosen folder. **Save project** (or Ctrl+S) and autosave update that file. The app also keeps a local recovery copy.
- **Open a project file** imports a `.folio.json` file into Projects. **Save a copy to file** writes a portable `.folio.json` backup.
- **Navy ribbon carousel** in Templates starts an eight-slide square project based on `Untitled.png`: navy background, cream typography, and continuous white rough borders. The first and last slides center their headlines; the middle slides use a heading and body.
- Write your copy in Slide details. Add, duplicate, reorder, or delete slides using the strip below the canvas. Undo and redo restore edits and slide changes.
- Use Brand kit to change the name, Instagram handle, accent color, and font across every slide.
- Choose light or dark and square (1080 × 1080) or portrait (1080 × 1350).
- Add PNG, JPEG, or WebP photos to image layouts. Images are cropped to fill their frame and embedded in saved projects. Use photos under 15 MB; projects support up to 150 MB.
- View the 36 original Figma layout references in the template library. Selecting one applies its format, theme, and layout. The templates are editable recreations inspired by the originals; the outlined text in exported Figma SVGs is not directly editable.
- Preview the carousel and move through it with the arrow buttons or arrow keys.
- Export one slide as PNG or the entire carousel as a ZIP containing numbered full-resolution PNGs. A one-slide carousel exports directly as PNG.
- Your editable project files live in the selected folder. **Save a copy to file** creates an additional portable backup when you want one.

Everything runs locally. No account, network, API key, or paid service is needed. Original references are bundled, so the app does not depend on the D: drive after it is built. Background texture, swipe cues, copy, images, and brand controls are editable. The app uses preset layouts rather than freeform layer editing.

Shortcuts: Ctrl+S saves the current project in Projects, Ctrl+Z undoes, Ctrl+Shift+Z redoes; arrow keys select slides when a text field is not focused.

Closing the desktop window saves a recovery copy of the current project before exiting, including edits made immediately before clicking Close. The next launch restores whichever is newer: the recovery copy or the local autosave. If recovery saving fails, the app offers Cancel or Close without saving. The browser version retains its normal unsaved-changes warning.
