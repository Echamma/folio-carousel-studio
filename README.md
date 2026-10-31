# Folio Carousel Studio

A Windows desktop app for making Instagram carousels, inspired by the collection in `D:\Figma Templates`. The existing animation studio in the parent folder is preserved.

## Run

Open `dist/Folio-Carousel-Studio-1.5.2.exe`. This is a portable app; installation and an account are not required. The project folder you choose is remembered across launches, including when a synced folder is temporarily unavailable.

To run from source: `npm install`, then `npm start`. To rebuild the portable executable: `npm run build`.

Folio also runs inside Canopy, next to Forma and StreamCuts. The editor's main-process side lives in `app-host.cjs`, which `main.cjs` and Canopy both create. Inside a Canopy space, Folio's project folder is the space's `Folio` folder (Change folder is hidden), exports go straight to the space's `Files` instead of a save dialog, and pictures sent from the space become movable image layers on the current slide. The Fun Tech samples stay in the standalone library.

## Create

- Select one of ten editable layouts: Introduction, Text Only, Checklist, Quote, Image Center, Image Left, Image Right, Image Compare, Thank You, and Navy Ribbon.
- **Open** asks you to choose a project folder the first time. Folio remembers it and shows a searchable visual gallery of `.folio.json` files, with a cover preview for each project. Use **Change folder** in Projects or in the gallery to select another folder.
- The ten editable six-slide Fun Tech Facts carousels are added to the chosen folder on first selection, along with any projects previously saved in the app. Existing files with newer saved content are kept.
- **New** creates a new project file in the chosen folder. **Save project** (or Ctrl+S) and autosave update that file. The app also keeps a local recovery copy.
- **Open a project file** imports a `.folio.json` file into Projects. **Save a copy to file** writes a portable `.folio.json` backup.
- **Navy ribbon carousel** in Templates starts an eight-slide square project based on `Untitled.png`: navy background, cream typography, and continuous white rough borders. The first and last slides center their headlines; the middle slides use a heading and body.
- Write your copy in Slide details. Add, duplicate, reorder, or delete slides using the strip below the canvas. Undo and redo restore edits and slide changes.
- Arabic and other right-to-left text is aligned correctly in slides and editing fields. Projects can set `direction` to `rtl` so numbers and contact details stay aligned with the Arabic layout.
- Use Brand kit to change the name, Instagram handle, accent color, and font across every slide.
- Choose light or dark and square (1080 × 1080) or portrait (1080 × 1350).
- Add PNG, JPEG, or WebP photos to image layouts. Images are cropped to fill their frame and embedded in saved projects. Use photos under 15 MB; projects support up to 150 MB.
- Add extra PNG, JPEG, or WebP images to any slide with **Add image**, by dropping a file on the canvas, or by pasting an image. Use **Add text** for movable captions. Drag an element to position it; drag its lower-right handle to resize it. Use the element list to select overlapping items or change their stacking order. Selected text can be edited, recolored, and resized in Slide details. These elements appear in saved projects, previews, and exports.
- View the 36 original Figma layout references in the template library. Selecting one applies its format, theme, and layout. The templates are editable recreations inspired by the originals; the outlined text in exported Figma SVGs is not directly editable.
- Preview the carousel and move through it with the arrow buttons or arrow keys.
- Export one slide as PNG or the entire carousel as a ZIP containing numbered full-resolution PNGs. A one-slide carousel exports directly as PNG.
- Your editable project files live in the selected folder. **Save a copy to file** creates an additional portable backup when you want one.

Everything runs locally. No account, network, API key, or paid service is needed. Original references are bundled, so the app does not depend on the D: drive after it is built. Background texture, swipe cues, copy, images, and brand controls are editable. The app uses preset layouts rather than freeform layer editing.

Shortcuts: Ctrl+S saves the current project in Projects, Ctrl+Z undoes, Ctrl+Shift+Z redoes; arrow keys select slides when a text field is not focused.

Closing the desktop window saves a recovery copy of the current project before exiting, including edits made immediately before clicking Close. The next launch restores whichever is newer: the recovery copy or the local autosave. If recovery saving fails, the app offers Cancel or Close without saving. The browser version retains its normal unsaved-changes warning.
