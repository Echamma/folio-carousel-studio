# Folio Carousel Studio

This is the **Linux desktop branch**. The Windows version is preserved on `main`.

Download the x86-64 AppImage or Debian package from [GitHub Releases](https://github.com/Echamma/folio-carousel-studio/releases). For the AppImage, run `chmod +x Folio-Carousel-Studio-1.1.1-x64.AppImage`, then `./Folio-Carousel-Studio-1.1.1-x64.AppImage`. If FUSE is unavailable, use `./Folio-Carousel-Studio-1.1.1-x64.AppImage --appimage-extract-and-run`. For Debian/Ubuntu, install with `sudo apt install ./Folio-Carousel-Studio-1.1.1-x64.deb`, then launch Folio from your application menu.

On Linux, `npm ci` installs development dependencies, `npm start` opens the editor, and `npm run build` creates AppImage and `.deb` packages. Use Node.js 22. `npm run build:windows` retains Windows packaging. Headless tests run with `xvfb-run -a npm test`. GitHub tests editor behavior, recovery on close, and the packaged executable on Ubuntu 22.04 before publishing Linux packages with SHA-256 checksums.

A Windows desktop app for making Instagram carousels, inspired by the collection in `D:\Figma Templates`. The existing animation studio in the parent folder is preserved.

## Run

Use the Linux AppImage or `.deb` package from Releases. Windows users can find `Folio-Carousel-Studio-1.1.1.exe` in the Windows release or use the `main` branch. No account is required to run the app; this private repository requires GitHub access to download it.

To run from source: `npm install`, then `npm start`. To rebuild the portable executable: `npm run build`.

## Create

- Select one of ten editable layouts: Introduction, Text Only, Checklist, Quote, Image Center, Image Left, Image Right, Image Compare, Thank You, and Navy Ribbon.
- **Navy ribbon carousel** starts an eight-slide square project based on `Untitled.png`: navy background, cream typography, and continuous white rough borders. The first and last slides center their headlines; the middle slides use a heading and body. Edit the sale copy to add your actual offers. Brand kit's accent color changes the cream text. Background texture switches between rough and smooth ribbons; the swipe toggle adds optional footer branding. Undo restores your previous project after starting this preset.
- Write your copy in Slide details. Add, duplicate, reorder, or delete slides using the strip below the canvas. Undo and redo restore edits and slide changes.
- Use Brand kit to change the name, Instagram handle, accent color, and font across every slide.
- Choose light or dark and square (1080 × 1080) or portrait (1080 × 1350).
- Add PNG, JPEG, or WebP photos to image layouts. Images are cropped to fill their frame and embedded in saved projects. Use photos under 15 MB; projects support up to 150 MB.
- View the 36 original Figma layout references in the template library. Selecting one applies its format, theme, and layout. The templates are editable recreations inspired by the originals; the outlined text in exported Figma SVGs is not directly editable.
- Preview the carousel and move through it with the arrow buttons or arrow keys.
- Export one slide as PNG or the entire carousel as a ZIP containing numbered full-resolution PNGs. A one-slide carousel exports directly as PNG.
- Save project writes an editable `.folio.json` file. Open resumes a saved file. The last working project also saves automatically on this device. Project files are the portable backup for your work.

Everything runs locally. No account, network, API key, or paid service is needed. Original references are bundled, so the app does not depend on the D: drive after it is built. Background texture, swipe cues, copy, images, and brand controls are editable. The app uses preset layouts rather than freeform layer editing.

Shortcuts: Ctrl+S saves a project, Ctrl+Z undoes, Ctrl+Shift+Z redoes; arrow keys select slides when a text field is not focused.

Closing the desktop window saves a recovery copy of the current project before exiting, including edits made immediately before clicking Close. The next launch restores whichever is newer: the recovery copy or the local autosave. If recovery saving fails, the app offers Cancel or Close without saving. The browser version retains its normal unsaved-changes warning.
