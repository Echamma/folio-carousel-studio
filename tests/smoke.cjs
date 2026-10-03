const { app, BrowserWindow, dialog } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const JSZip = require('jszip');
process.env.FOLIO_TEST = '1';
const output = path.join(__dirname, '..', 'test-output');
const folder = path.join(output, 'smoke-project-folder-' + process.pid);
const files = ['project.json', 'carousel.zip', 'single.png', 'navy-project.json', 'navy-carousel.zip'];
app.setPath('userData', path.join(output, 'test-profile-' + process.pid));
let saveIndex = 0;
dialog.showSaveDialog = async () => ({ canceled: false, filePath: path.join(output, files[saveIndex++]) });
dialog.showOpenDialog = async (_, options) => ({ canceled: false, filePaths: [options?.properties?.includes('openDirectory') ? folder : path.join(output, 'project.json')] });
require('../main.cjs');
const timeout = setTimeout(() => { console.error('Desktop test timed out'); app.exit(1); }, 60000);
app.whenReady().then(async () => {
  try {
    await fs.mkdir(output, { recursive: true });
    await fs.mkdir(folder, { recursive: true });
    const win = BrowserWindow.getAllWindows()[0];
    if (win.webContents.isLoading()) await new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    const results = await win.webContents.executeJavaScript(`(async () => {
      const checks = [];
      const assert = (value, message) => { if (!value) throw new Error(message); checks.push(message); };
      await window.FolioReady;
      project = defaults(); selected = 0; await paint(); syncControls();
      assert(project.slides.length === 5, 'Default five-slide project loads');
      const before = JSON.stringify(project); addSlide(); assert(project.slides.length === 6, 'Add slide'); undo(); assert(JSON.stringify(project) === before, 'Undo slide insertion'); undo(true); assert(project.slides.length === 6, 'Redo slide insertion'); undo();
      $('title').value = 'Native desktop check'; $('title').dispatchEvent(new Event('input')); assert(current().title === 'Native desktop check', 'Text editing updates project'); undo(); assert(current().title === 'Good design.\\nGreat stories.', 'Undo text edit');
      $('duplicate').click(); assert(project.slides.length === 6 && selected === 1, 'Duplicate selects new slide'); $('move-right').click(); assert(selected === 2, 'Reorder slide'); $('delete-slide').click(); assert(project.slides.length === 5, 'Delete slide'); undo(); assert(project.slides.length === 6, 'Undo deletion');
      project = defaults(); selected = 0; history.length = 0; future.length = 0;
      let renders = 0;
      for (const format of ['square', 'portrait']) for (const theme of ['light', 'dark']) for (const layout of layouts) {
        const canvas = document.createElement('canvas'); render(canvas, createSlide(layout.id), { ...project, format, theme }, 0);
        assert(canvas.width === 1080 && canvas.height === (format === 'square' ? 1080 : 1350), format + ' ' + theme + ' ' + layout.name + ' renders'); renders++;
      }
      const ctx = document.createElement('canvas').getContext('2d'); ctx.font = '30px Arial'; const lines = FolioRender.wrap(ctx, 'a'.repeat(300), 100); assert(lines.every(line=>ctx.measureText(line).width <= 100), 'Long words wrap inside frame');
      let invalidRejected = false; try { validate({...project,slides:[]}); } catch { invalidRejected = true; } assert(invalidRejected, 'Invalid project rejected');
      const malicious = clone(project); malicious.slides[0].image = 'https://example.com/image.png'; invalidRejected = false; try { validate(malicious); } catch { invalidRejected = true; } assert(invalidRejected, 'Remote image URLs rejected');
      const refs = [];
      for (const format of ['Portrait','Square']) for (const theme of ['Light','Dark']) for (const section of referenceNames) {
        const img = new Image(); refs.push(new Promise((resolve,reject)=>{ img.onload=resolve; img.onerror=()=>reject(new Error('Missing reference: '+img.src)); img.src='assets/references/'+encodeURIComponent('Size='+format+', Theme='+theme+', Sections='+section+'.svg'); }));
      }
      await Promise.all(refs); assert(refs.length === 36, 'All 36 bundled Figma SVGs load');
      const photo = document.createElement('canvas'); photo.width = 400; photo.height = 300; const pc = photo.getContext('2d'); pc.fillStyle = '#e65443'; pc.fillRect(0,0,400,300);
      const blob = await new Promise(resolve=>photo.toBlob(resolve,'image/png')); const file = new File([blob], 'test-photo.png',{type:'image/png'}); const transfer = new DataTransfer(); transfer.items.add(file); $('image-file').files = transfer.files;
      current().layout = 'image-center'; await uploadImage($('image-file'),'image'); assert(current().image.startsWith('data:image/png;base64,'), 'Uploaded photo embedded');
      const overlayPhoto = document.createElement('canvas'); overlayPhoto.width = 100; overlayPhoto.height = 100; const oc = overlayPhoto.getContext('2d'); oc.fillStyle = '#12ab34'; oc.fillRect(0,0,100,100);
      const overlayBlob = await new Promise(resolve => overlayPhoto.toBlob(resolve,'image/png'));
      await addLayerImage(new File([overlayBlob], 'overlay.png', { type: 'image/png' }));
      assert(current().layers.length === 1 && current().layers[0].type === 'image', 'Extra image can be added over any layout');
      const overlayCanvas = document.createElement('canvas'); render(overlayCanvas,current(),project,0,await allImages());
      const imageLayer = current().layers[0];
      assert([...overlayCanvas.getContext('2d').getImageData(imageLayer.x+5,imageLayer.y+5,1,1).data].join(',') === '18,171,52,255', 'Extra image appears in exported render');
      addLayerText(); assert(current().layers.length === 2 && current().layers[1].type === 'text', 'Movable text can be added');
      $('layer-text').value = 'Place this anywhere'; $('layer-text').dispatchEvent(new Event('input'));
      $('layer-size').value = '80'; $('layer-size').dispatchEvent(new Event('input'));
      assert(current().layers[1].text === 'Place this anywhere' && current().layers[1].fontSize === 80, 'Free text and size remain editable');
      assert($('layer-list').children.length === 2, 'Extra elements appear in the layer list');
      $('layer-list').children[0].click(); assert(selectedLayerId === imageLayer.id, 'Layer list selects an image behind other elements');
      $('layer-front').click(); assert(current().layers[1].id === imageLayer.id, 'Selected element can move in front');
      $('layer-back').click(); assert(current().layers[0].id === imageLayer.id, 'Selected element can move behind');
      $('layer-list').children[1].click(); assert(selectedLayerId === current().layers[1].id, 'Layer list selects text for editing');
      const savedLayerProject = clone(project); validate(savedLayerProject);
      await desktop.saveProject(clone(project)); const opened = await desktop.openProject(); assert(opened.slides[0].image === project.slides[0].image && opened.slides[0].layers.length === 2, 'Native save/open preserves base image and movable elements'); validate(opened);
      await exportSlides(false); await exportSlides(true);
      project = defaults(); selected = 0; syncControls(); await paint(); scheduleSave();
      await new Promise(resolve=>setTimeout(resolve,600)); assert($('save-status').textContent === 'Saved on this device', 'Autosave completes');
      assert($('slide-count').textContent === '5 slides' && $('slide-caption').textContent === 'Slide 01', 'Restored project has consistent controls');
      await startNavy(); assert(project.slides.length === 8 && project.format === 'square' && project.slides.every(s=>s.layout === 'navy-ribbon'), 'Navy starter creates eight editable square slides');
      addSlide(); assert(project.slides.length === 9 && current().layout === 'navy-ribbon', 'Additional navy slides keep the selected style'); undo(); assert(project.slides.length === 8 && selected === 0, 'Undo adding a navy slide');
      const navyCanvas = document.createElement('canvas'); render(navyCanvas, current(), project, 0);
      const navyCtx = navyCanvas.getContext('2d'); assert(Array.from(navyCtx.getImageData(0,0,1,1).data).join(',') === '60,65,93,255', 'Navy background matches the reference');
      const arabicSlide = createSlide('navy-ribbon', { title: 'مرحباً بكم', body: 'نص عربي', texture: false, swipe: false });
      const arabicCanvas = document.createElement('canvas'); render(arabicCanvas, arabicSlide, { ...project, direction: 'rtl', slides: [arabicSlide] }, 0);
      const arabicCtx = arabicCanvas.getContext('2d');
      const titleInk = (from, to) => { let count = 0; for (let x = from; x < to; x += 4) for (let y = 350; y < 480; y += 4) if (arabicCtx.getImageData(x,y,1,1).data[0] > 150) count++; return count; };
      assert(titleInk(700, 990) > titleInk(90, 380) + 10, 'Arabic slide text aligns to the right');
      const firstRender = navyCanvas.toDataURL(); render(navyCanvas,current(),project,0); assert(navyCanvas.toDataURL() === firstRender, 'Rough border renders consistently between preview and export');
      $('title').value = 'Your next idea'; $('title').dispatchEvent(new Event('input')); assert(current().title === 'Your next idea', 'Navy headline remains editable'); undo(); assert(current().title === 'What Is A\\nQR Code?', 'Navy headline undo');
      const savedNavy = clone(project); validate(savedNavy); assert(savedNavy.slides.length === 8, 'Navy project validates for reopening');
      await desktop.saveProject(savedNavy); await exportSlides(false);
      const sheet = document.createElement('canvas'); sheet.width = 2160; sheet.height = 1080; const sheetCtx = sheet.getContext('2d');
      for (let i=0;i<8;i++) { const canvas=document.createElement('canvas'); render(canvas,project.slides[i],project,i); sheetCtx.drawImage(canvas,(i%4)*540,Math.floor(i/4)*540,540,540); }
      const navySheet = sheet.toDataURL('image/png');
      const contact = document.createElement('canvas'); contact.width=8640; contact.height=1080; const contactCtx=contact.getContext('2d');
      for (let i=0;i<8;i++) { const canvas=document.createElement('canvas'); render(canvas,project.slides[i],project,i); contactCtx.drawImage(canvas,i*1080,0); }
      const navyPanorama=contact.toDataURL('image/png');
      undo(); // The starter is also a reversible project change.
      return { checks, renders, navySheet, navyPanorama };
    })()`);
    const zip = await JSZip.loadAsync(await fs.readFile(path.join(output, 'carousel.zip')));
    const pngs = Object.keys(zip.files); assert.equal(pngs.length, 5);
    for (const name of pngs) {
      const png = await zip.file(name).async('nodebuffer'); assert.equal(png.readUInt32BE(16), 1080); assert.equal(png.readUInt32BE(20), 1350);
    }
    const single = await fs.readFile(path.join(output, 'single.png')); assert.equal(single.readUInt32BE(16), 1080); assert.equal(single.readUInt32BE(20), 1350);
    const navyZip = await JSZip.loadAsync(await fs.readFile(path.join(output, 'navy-carousel.zip')));
    assert.equal(Object.keys(navyZip.files).length,8);
    for (const file of Object.values(navyZip.files)) { const png=await file.async('nodebuffer'); assert.equal(png.readUInt32BE(16),1080); assert.equal(png.readUInt32BE(20),1080); }
    await fs.writeFile(path.join(output,'navy-preview.png'),Buffer.from(results.navySheet.split(',')[1],'base64'));
    await fs.writeFile(path.join(output,'navy-panorama.png'),Buffer.from(results.navyPanorama.split(',')[1],'base64'));
    delete results.navySheet; delete results.navyPanorama;
    const shot = await win.webContents.capturePage(); await fs.writeFile(path.join(output, 'desktop-preview.png'), shot.toPNG());
    await fs.writeFile(path.join(output, 'results.json'), JSON.stringify({ ...results, exportChecks: ['ZIP has 5 PNGs', 'All exports 1080 × 1350', 'Single PNG export passes'] }, null, 2));
    console.log('PASS: ' + results.checks.length + ' desktop checks; ' + results.renders + ' layout combinations; native project roundtrip; portrait exports + eight navy slides at 1080 × 1080.');
    clearTimeout(timeout); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
});
