const { app, BrowserWindow } = require('electron');
const path = require('node:path');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');

process.env.FOLIO_TEST = '1';
app.setPath('userData', path.join(app.getPath('temp'), 'folio-layers-test-' + process.pid));
require('../main.cjs');
const timeout = setTimeout(() => { console.error('Layer editor test timed out'); app.exit(1); }, 30000);

function mouse(win, type, x, y, dragging = false) { win.webContents.sendInputEvent({ type, x: Math.round(x), y: Math.round(y), button: 'left', clickCount: 1, modifiers: dragging ? ['leftButtonDown'] : [] }); }

app.whenReady().then(async () => {
  try {
    const win = BrowserWindow.getAllWindows()[0];
    if (win.webContents.isLoading()) await new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    win.show();
    win.focus();
    const before = await win.webContents.executeJavaScript(`(async () => {
      await window.FolioReady;
      project = defaults(); selected = 0; selectedLayerId = null; syncControls(); await paint();
      addLayerText(); await paint();
      const layer = current().layers[0]; const rect = document.querySelector('#overlay-editor .layer-hit').getBoundingClientRect();
      return { x: layer.x, y: layer.y, width: layer.width, height: layer.height, rect: rect.toJSON() };
    })()`);
    const centerX = before.rect.left + before.rect.width / 2;
    const centerY = before.rect.top + before.rect.height / 2;
    mouse(win, 'mouseMove', centerX, centerY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseDown', centerX, centerY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseMove', centerX + 35, centerY + 25, true);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseUp', centerX + 35, centerY + 25);
    await new Promise(resolve => setTimeout(resolve, 200));
    const moved = await win.webContents.executeJavaScript(`(() => {
      const layer = current().layers[0]; const corner = document.querySelector('.layer-hit .layer-resize').getBoundingClientRect();
      return { x: layer.x, y: layer.y, width: layer.width, height: layer.height, corner: corner.toJSON() };
    })()`);
    assert(moved.x > before.x && moved.y > before.y, 'Dragging text moves it on the slide');
    const cornerX = moved.corner.left + moved.corner.width / 2;
    const cornerY = moved.corner.top + moved.corner.height / 2;
    mouse(win, 'mouseMove', cornerX, cornerY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseDown', cornerX, cornerY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseMove', cornerX + 35, cornerY + 25, true);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseUp', cornerX + 35, cornerY + 25);
    await new Promise(resolve => setTimeout(resolve, 200));
    const resized = await win.webContents.executeJavaScript(`(() => {
      const layer = current().layers[0];
      return { width: layer.width, height: layer.height, selected: selectedLayerId === layer.id };
    })()`);
    assert(resized.width > moved.width && resized.height > moved.height, 'Dragging corner resizes text box');
    assert(resized.selected, 'Text stays selected after resizing');
    const imageBefore = await win.webContents.executeJavaScript(`(async () => {
      const sample = document.createElement('canvas'); sample.width = 160; sample.height = 80;
      sample.getContext('2d').fillRect(0,0,160,80);
      const blob = await new Promise(resolve => sample.toBlob(resolve,'image/png'));
      await addLayerImage(new File([blob], 'extra.png', { type: 'image/png' })); await paint();
      const layer = current().layers[1]; const rect = document.querySelectorAll('.layer-hit .layer-resize')[1].getBoundingClientRect();
      return { width: layer.width, height: layer.height, rect: rect.toJSON() };
    })()`);
    const imageX = imageBefore.rect.left + imageBefore.rect.width / 2;
    const imageY = imageBefore.rect.top + imageBefore.rect.height / 2;
    mouse(win, 'mouseMove', imageX, imageY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseDown', imageX, imageY);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseMove', imageX + 30, imageY + 15, true);
    await new Promise(resolve => setTimeout(resolve, 60));
    mouse(win, 'mouseUp', imageX + 30, imageY + 15);
    await new Promise(resolve => setTimeout(resolve, 200));
    const imageAfter = await win.webContents.executeJavaScript(`(() => { const layer = current().layers[1]; return { width: layer.width, height: layer.height }; })()`);
    assert(imageAfter.width > imageBefore.width && imageAfter.height > imageBefore.height, 'Dragging corner resizes an added image');
    assert(Math.abs(imageAfter.width / imageAfter.height - imageBefore.width / imageBefore.height) < .02, 'Image keeps its aspect ratio while resizing');
    await fs.mkdir(path.join(__dirname, '..', 'test-output'), { recursive: true });
    await win.webContents.executeJavaScript(`new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))`);
    await fs.writeFile(path.join(__dirname, '..', 'test-output', 'layers-ui.png'), (await win.webContents.capturePage()).toPNG());
    console.log('PASS: Canvas text moves and resizes; extra image resizes without distortion');
    clearTimeout(timeout); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
});
