# Folio Carousel Studio

A Windows desktop app for making Instagram carousels, inspired by the collection in `D:\Figma Templates`. The existing animation studio in the parent folder is preserved.

## Run

Open `dist/Folio-Carousel-Studio-1.2.1.exe`. This is a portable app; installation and an account are not required.

To run from source: `npm install`, then `npm start`. To rebuild the portable executable: `npm run build`.

## Create

- Select one of ten editable layouts: Introduction, Text Only, Checklist, Quote, Image Center, Image Left, Image Right, Image Compare, Thank You, and Navy Ribbon.
- **Projects** opens with ten editable six-slide Fun Tech Facts carousels in the navy ribbon style. Click any project to edit it. The collection is seeded into local project storage once, so your edits remain yours on later launches.
- **New** creates a new project in Projects. **Save project** (or Ctrl+S) saves the current project there. **Open** shows a searchable picker for saved projects. The app also saves changes automatically, including when switching between projects.
- **Open a project file** imports a `.folio.json` file into Projects. **Save a copy to file** writes a portable `.folio.json` backup.
- **Navy ribbon carousel** in Templates starts an eight-slide square project based on `Untitled.png`: navy background, cream typography, and continuous white rough borders. The first and last slides center their headlines; the middle slides use a heading and body.
- Write your copy in Slide details. Add, duplicate, reorder, or delete slides using the strip below the canvas. Undo and redo restore edits and slide changes.
- Use Brand kit to change the name, Instagram handle, accent color, and font across every slide.
- Choose light or dark and square (1080 × 1080) or portrait (1080 × 1350).
- Add PNG, JPEG, or WebP photos to image layouts. Images are cropped to fill their frame and embedded in saved projects. Use photos under 15 MB; projects support up to 150 MB.
- View the 36 original Figma layout references in the template library. Selecting one applies its format, theme, and layout. The templates are editable recreations inspired by the originals; the outlined text in exported Figma SVGs is not directly editable.
- Preview the carousel and move through it with the arrow buttons or arrow keys.
- Export one slide as PNG or the entire carousel as a ZIP containing numbered full-resolution PNGs. A one-slide carousel exports directly as PNG.
- Every project is stored locally in the app. Export a project file when you want a portable backup or need to move it to another device.

Everything runs locally. No account, network, API key, or paid service is needed. Original references are bundled, so the app does not depend on the D: drive after it is built. Background texture, swipe cues, copy, images, and brand controls are editable. The app uses preset layouts rather than freeform layer editing.

Shortcuts: Ctrl+S saves the current project in Projects, Ctrl+Z undoes, Ctrl+Shift+Z redoes; arrow keys select slides when a text field is not focused.

Closing the desktop window saves a recovery copy of the current project before exiting, including edits made immediately before clicking Close. The next launch restores whichever is newer: the recovery copy or the local autosave. If recovery saving fails, the app offers Cancel or Close without saving. The browser version retains its normal unsaved-changes warning.
