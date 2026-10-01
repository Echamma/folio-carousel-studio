'use strict';
const $ = id => document.getElementById(id);
const { layouts, referenceNames, render } = FolioRender;
const clone = value => JSON.parse(JSON.stringify(value));
const createSlide = (layout = 'text', overrides = {}) => ({ id: crypto.randomUUID(), layout, eyebrow: 'THE CREATIVE NOTES', title: 'One idea.\nA little possibility.', body: 'Share something useful. Make it simple, make it thoughtful, make it yours.', items: ['Start with a clear idea', 'Make every slide count', 'Finish with a next step'], image: '', image2: '', titleSize: 88, texture: true, swipe: true, ...overrides });
const defaults = () => ({ version: 1, name: 'The art of a good carousel', format: 'portrait', theme: 'light', accent: '#e65443', brand: 'YOUR STUDIO', handle: '@yourstudio', font: 'Arial', slides: [
  createSlide('introduction', { eyebrow: 'A NOTE FOR THE CREATORS', title: 'Good design.\nGreat stories.', body: 'Turn your next big idea into something\nworth swiping for.' }),
  createSlide('text', { title: 'Start with\nsomething real.', body: 'The best carousels begin with an idea your audience cares about.\n\nTeach something. Share a perspective. Tell a story only you can tell.' }),
  createSlide('checklist', { title: 'Make every\nslide count.', body: 'A few small things make a big difference.' }),
  createSlide('quote', { eyebrow: 'A THOUGHT WORTH SAVING', title: 'Great things are done by a series of small things brought together.', body: '— Vincent van Gogh' }),
  createSlide('thank-you', { eyebrow: 'THAT’S A WRAP', title: 'Good ideas\nare better shared.', body: 'Save this for your next creative day.\nFollow along for a little more inspiration.' })
] });
const navyProject = () => ({ version: 1, name: 'QR Code — Navy Ribbon', format: 'square', theme: 'dark', accent: '#f3ddc1', brand: 'YOUR STUDIO', handle: '@yourstudio', font: 'Arial', slides: [
  ['What Is A\nQR Code?', ''],
  ['CONNECT:', 'It is a physical copy-paste button for your camera.'],
  ['Free', 'Anyone can generate a custom code for free.'],
  ['Universal', 'Originally invented in 1994 to track car parts, they now seamlessly connect you to Wi-Fi networks, digital menus, and contactless payments.'],
  ['Sale:', 'Share your early-bird offer for artists, small shops, and restaurants.'],
  ['Sale:', 'Add your launch promotion here. Give your audience a clear reason to get started.'],
  ['Sale:', 'Add your subscription offer, price, and terms here.'],
  ['Fill out the\nform in our\nBio', '']
].map(([title, body]) => createSlide('navy-ribbon', { eyebrow: '', title, body, titleSize: body ? 64 : 98, swipe: false })) });
let project = defaults(); let selected = 0; let previewIndex = 0; let db; let toastTimer; let saveTimer; let changeTimer; let renderToken = 0; let savedSnapshot; let busy = false; let featuredProjects = [];
const history = []; const future = []; const imageCache = new Map();
function toast(message) { $('toast').textContent = message; $('toast').classList.add('visible'); clearTimeout(toastTimer); toastTimer = setTimeout(() => $('toast').classList.remove('visible'), 3500); }
function current() { return project.slides[selected]; }
function record() { clearTimeout(changeTimer); changeTimer = null; history.push(JSON.stringify({ project, selected })); if (history.length > 40) history.shift(); future.length = 0; }
function finishChange() { scheduleSave(); paint(); }
function edit(action) { record(); action(); finishChange(); syncControls(); }
function markTextEdit() { if (!changeTimer) record(); clearTimeout(changeTimer); changeTimer = setTimeout(() => { changeTimer = null; }, 600); }
async function openDB() {
  db = await new Promise((resolve, reject) => { const req = indexedDB.open('folio-studio', 1); req.onupgradeneeded = () => req.result.createObjectStore('projects'); req.onsuccess = () => resolve(req.result); req.onerror = () => reject(req.error); });
  return new Promise((resolve, reject) => { const req = db.transaction('projects').objectStore('projects').get('current'); req.onsuccess = () => resolve(req.result); req.onerror = () => reject(req.error); });
}
async function saveNow() {
  clearTimeout(saveTimer);
  if (!db) { $('save-status').textContent = 'Use Save project to keep your work'; return; }
  project.savedAt = Date.now(); const snapshot = clone(project);
  await new Promise((resolve, reject) => {
    const tx = db.transaction('projects', 'readwrite'); const store = tx.objectStore('projects');
    store.put(snapshot, 'current');
    if (snapshot.libraryId) store.put(snapshot, 'library:' + snapshot.libraryId);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error || new Error('Could not save this project.'));
    tx.onabort = () => reject(tx.error || new Error('Could not save this project.'));
  });
  savedSnapshot = JSON.stringify(snapshot); $('save-status').textContent = 'Saved on this device';
  renderSavedProjects().catch(() => {});
}
function scheduleSave() {
  $('save-status').textContent = 'Saving on this device…'; clearTimeout(saveTimer);
  saveTimer = setTimeout(() => saveNow().catch(() => { $('save-status').textContent = 'Autosave unavailable · save a project file'; }), 350);
}
async function loadImage(src) {
  if (!src) return null;
  if (!imageCache.has(src)) imageCache.set(src, new Promise(resolve => { const img = new Image(); img.onload = () => resolve(img); img.onerror = () => resolve(null); img.src = src; }));
  return imageCache.get(src);
}
async function allImages(snapshot = project) {
  const sources = [...new Set(snapshot.slides.flatMap(s => [s.image, s.image2]).filter(Boolean))];
  const result = {}; await Promise.all(sources.map(async src => { result[src] = await loadImage(src); })); return result;
}
async function paint() {
  const token = ++renderToken; const images = await allImages(); if (token !== renderToken) return;
  const result = render($('artboard'), current(), project, selected, images);
  $('overflow-warning').hidden = !result.reduced;
  document.querySelector('.canvas-wrap').style.aspectRatio = project.format === 'square' ? '1' : '4/5';
  $('height-caption').textContent = project.format === 'square' ? '1080 px' : '1350 px';
  $('dimensions').textContent = project.format === 'square' ? '1:1 square' : '4:5 portrait';
  $('slide-strip').replaceChildren();
  project.slides.forEach((slide, i) => {
    const button = document.createElement('button'); button.className = 'slide-thumb' + (i === selected ? ' active' : ''); button.setAttribute('aria-label', `Select slide ${i + 1}: ${slide.title.replace(/\n/g, ' ')}`); button.setAttribute('aria-pressed', String(i === selected));
    const canvas = document.createElement('canvas'); render(canvas, slide, project, i, images); button.append(canvas);
    const label = document.createElement('span'); label.textContent = String(i + 1).padStart(2, '0') + (i === selected ? ' · Editing' : ''); button.append(label); button.onclick = () => selectSlide(i); $('slide-strip').append(button);
  });
  const add = document.createElement('button'); add.className = 'add-thumb'; add.textContent = '+'; add.setAttribute('aria-label', 'Add slide'); add.onclick = addSlide; add.disabled = project.slides.length >= 20; $('slide-strip').append(add);
  $('template-grid').replaceChildren();
  layouts.forEach(layout => {
    const button = document.createElement('button'); button.className = 'template-card' + (layout.id === current().layout ? ' active' : ''); button.setAttribute('aria-label', `Use ${layout.name} layout`); button.setAttribute('aria-pressed', String(layout.id === current().layout));
    const canvas = document.createElement('canvas');
    const navy = layout.id === 'navy-ribbon';
    const sample = createSlide(layout.id, { eyebrow: navy ? '' : 'YOUR STORY', title: navy ? 'What Is A\nQR Code?' : layout.id === 'quote' ? 'A thought worth sharing.' : layout.id === 'thank-you' ? 'Thank you.' : layout.name === 'Introduction' ? 'Your next\nbig idea.' : 'Title space', body: navy ? '' : 'A little room for your ideas.', titleSize: 104, swipe: !navy, items: ['First idea', 'Second idea', 'Third idea'] });
    render(canvas, sample, { ...project, accent: navy ? '#f3ddc1' : project.accent, format: 'square', brand: 'YOUR STUDIO', handle: '@yourstudio', slides: [sample] }, 0);
    button.append(canvas); const label = document.createElement('span'); label.textContent = layout.name; button.append(label); button.onclick = () => edit(() => { current().layout = layout.id; if (navy) { project.theme = 'dark'; project.accent = '#f3ddc1'; current().eyebrow = ''; current().swipe = false; } }); $('template-grid').append(button);
  });
  $('undo').disabled = !history.length; $('redo').disabled = !future.length;
}
function syncControls() {
  const s = current(); const layout = layouts.find(l => l.id === s.layout);
  $('project-name').value = project.name; $('format').value = project.format;
  ['brand-name', 'brand-handle', 'brand-accent', 'brand-font'].forEach((id, i) => $(id).value = project[['brand', 'handle', 'accent', 'font'][i]]);
  $('accent-value').textContent = project.accent.toUpperCase();
  ['eyebrow', 'title', 'body'].forEach(id => $(id).value = s[id]); $('items').value = s.items.join('\n');
  $('title-size').value = s.titleSize; $('size-value').textContent = s.titleSize + ' px'; $('texture').checked = s.texture; $('swipe').checked = s.swipe;
  $('slide-caption').textContent = 'Slide ' + String(selected + 1).padStart(2, '0'); $('slide-count').textContent = project.slides.length + ' slides';
  $('inspector-number').textContent = `${String(selected + 1).padStart(2, '0')} / ${String(project.slides.length).padStart(2, '0')}`;
  $('layout-caption').textContent = layout.name; $('layout-name').textContent = layout.name; $('layout-icon').textContent = layout.icon; document.querySelector('.inspector-layout small').textContent = layout.description;
  $('light-theme').classList.toggle('selected', project.theme === 'light'); $('dark-theme').classList.toggle('selected', project.theme === 'dark');
  $('items-label').hidden = s.layout !== 'checklist'; $('image-controls').hidden = !s.layout.includes('image') && s.layout !== 'compare'; $('image2-label').hidden = s.layout !== 'compare';
  $('delete-slide').disabled = project.slides.length === 1; $('add-slide').disabled = project.slides.length >= 20; $('duplicate').disabled = project.slides.length >= 20;
  $('move-left').disabled = selected === 0; $('move-right').disabled = selected === project.slides.length - 1;
  document.querySelectorAll('#projects-list button[data-id]').forEach(button => {
    const active = button.dataset.id === project.libraryId;
    button.classList.toggle('active', active); button.setAttribute('aria-pressed', String(active));
  });
}
function selectSlide(i) { clearTimeout(changeTimer); changeTimer = null; selected = i; syncControls(); paint(); }
function addSlide() { if (project.slides.length >= 20) return toast('A carousel can contain up to 20 slides.'); edit(() => { const slide = current().layout === 'navy-ribbon' ? createSlide('navy-ribbon', { eyebrow: '', title: 'Your next idea', body: 'Add a new chapter to your story.', titleSize: 64, swipe: false }) : createSlide(); project.slides.splice(selected + 1, 0, slide); selected++; }); }
function undo(redo = false) {
  const source = redo ? future : history; const target = redo ? history : future;
  if (!source.length) return; clearTimeout(changeTimer); changeTimer = null; target.push(JSON.stringify({ project, selected })); const state = JSON.parse(source.pop()); project = state.project; selected = state.selected; syncControls(); finishChange();
}
function validate(value) {
  if (!value || value.version !== 1 || !Array.isArray(value.slides) || value.slides.length < 1 || value.slides.length > 20 || !['portrait', 'square'].includes(value.format) || !['light', 'dark'].includes(value.theme)) throw new Error('This is not a supported Folio project.');
  if (!/^#[\da-f]{6}$/i.test(value.accent) || !['Arial', 'Georgia', 'Verdana', 'Trebuchet MS'].includes(value.font)) throw new Error('Project brand settings are invalid.');
  for (const key of ['name', 'brand', 'handle']) if (typeof value[key] !== 'string' || value[key].length > 100) throw new Error('Invalid project text.');
  value.slides.forEach(s => {
    if (!layouts.some(l => l.id === s.layout) || !Number.isFinite(s.titleSize) || s.titleSize < 44 || s.titleSize > 120) throw new Error('Invalid slide layout.');
    for (const key of ['eyebrow', 'title', 'body']) if (typeof s[key] !== 'string' || s[key].length > 1500) throw new Error('Invalid slide text.');
    if (!Array.isArray(s.items) || s.items.length > 5 || s.items.some(t => typeof t !== 'string' || t.length > 1000)) throw new Error('Invalid checklist.');
    for (const key of ['image', 'image2']) if (typeof s[key] !== 'string' || (s[key] && !/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/.test(s[key]))) throw new Error('Invalid slide image.');
    if (typeof s.texture !== 'boolean' || typeof s.swipe !== 'boolean') throw new Error('Invalid slide options.');
    s.id = crypto.randomUUID();
  });
  return value;
}
async function adopt(value) { const next = validate(value); record(); project = next; selected = 0; imageCache.clear(); syncControls(); finishChange(); toast('Project opened. Pick up where you left off.'); }
async function savedLibraryProject(id) {
  if (!db) return null;
  return new Promise((resolve, reject) => {
    const request = db.transaction('projects').objectStore('projects').get('library:' + id);
    request.onsuccess = () => resolve(request.result || null); request.onerror = () => reject(request.error);
  });
}
async function saveLibraryEntry(value) {
  if (!db) throw new Error('Project storage is unavailable.');
  return new Promise((resolve, reject) => {
    const tx = db.transaction('projects', 'readwrite'); tx.objectStore('projects').put(value, 'library:' + value.libraryId);
    tx.oncomplete = resolve; tx.onerror = () => reject(tx.error); tx.onabort = () => reject(tx.error);
  });
}
async function seedFunTechProjects() {
  if (!window.desktop?.listFunTechProjects || !db) return;
  featuredProjects = await desktop.listFunTechProjects();
  for (const entry of featuredProjects) {
    if (await savedLibraryProject(entry.slug)) continue;
    const value = await desktop.openFunTechProject(entry.slug);
    value.libraryId = entry.slug;
    value.savedAt = Date.now();
    await saveLibraryEntry(value);
  }
}
async function renderSavedProjects() {
  if (!db) return;
  const values = await new Promise((resolve, reject) => {
    const request = db.transaction('projects').objectStore('projects').getAll();
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  const byId = new Map();
  values.filter(value => value && value.libraryId && Array.isArray(value.slides)).forEach(value => {
    const previous = byId.get(value.libraryId);
    if (!previous || (value.savedAt || 0) >= (previous.savedAt || 0)) byId.set(value.libraryId, value);
  });
  const featuredOrder = new Map(featuredProjects.map((entry, index) => [entry.slug, index]));
  const projects = [...byId.values()].sort((a, b) => {
    const ai = featuredOrder.has(a.libraryId) ? featuredOrder.get(a.libraryId) : Infinity;
    const bi = featuredOrder.has(b.libraryId) ? featuredOrder.get(b.libraryId) : Infinity;
    return ai - bi || (b.savedAt || 0) - (a.savedAt || 0);
  });
  const list = $('projects-list'); list.replaceChildren();
  projects.forEach((entry, index) => {
    const button = document.createElement('button'); button.type = 'button'; button.className = 'collection-project'; button.dataset.id = entry.libraryId;
    button.setAttribute('aria-label', `Open ${entry.name}`);
    const number = document.createElement('span'); number.className = 'collection-number'; number.textContent = String(index + 1).padStart(2, '0');
    const label = document.createElement('span'); label.className = 'collection-label'; label.textContent = entry.name || 'Untitled carousel';
    const count = document.createElement('small'); count.textContent = `${entry.slides.length} slides`;
    button.append(number, label, count); button.onclick = () => openSavedProject(entry.libraryId); list.append(button);
  });
  $('projects-count').textContent = `${projects.length} project${projects.length === 1 ? '' : 's'}`;
  syncControls();
}
async function openSavedProject(id) {
  try {
    await saveNow();
    const value = await savedLibraryProject(id);
    if (!value) throw new Error('This saved project could not be found.');
    await adopt(value);
    await saveNow();
    switchTab('projects');
    await renderSavedProjects();
  } catch (error) { toast('Could not open project: ' + error.message); }
}
async function createProject(value) {
  try {
    await saveNow();
    value.libraryId = crypto.randomUUID();
    await adopt(value);
    await saveNow();
    switchTab('projects');
    toast('New project saved in Projects.');
  } catch (error) { toast('Could not create project: ' + error.message); }
}
function download(blob, name) { const link = document.createElement('a'); const url = URL.createObjectURL(blob); link.href = url; link.download = name; link.click(); setTimeout(() => URL.revokeObjectURL(url), 3000); }
function safeFilename(name) { return (name || 'carousel').replace(/[<>:"/\\|?*\x00-\x1f]/g, '-').slice(0, 100); }
async function saveProject() {
  try {
    if (!project.libraryId) project.libraryId = crypto.randomUUID();
    await saveNow(); await renderSavedProjects(); switchTab('projects'); toast('Project saved in Projects.');
  } catch (error) { toast('Could not save: ' + error.message); }
}
async function saveProjectFile() {
  const snapshot = clone(project);
  try { if (window.desktop) { if (await desktop.saveProject(snapshot)) toast('Project file saved.'); }
    else { download(new Blob([JSON.stringify(snapshot, null, 2)], { type: 'application/json' }), safeFilename(snapshot.name) + '.folio.json'); toast('Project file downloaded.'); }
  } catch (error) { toast('Could not save file: ' + error.message); }
}
async function importProjectFile(value) {
  value.libraryId = crypto.randomUUID();
  await saveNow(); await adopt(value); await saveNow(); switchTab('projects'); toast('Project imported and saved in Projects.');
}
async function exportSlides(single = false) {
  if (busy) return; busy = true; $('export').disabled = true; $('export-slide').disabled = true; $('export').textContent = 'Exporting…';
  try {
    const snapshot = clone(project); const images = await allImages(snapshot); const indexes = single ? [selected] : snapshot.slides.map((_, i) => i);
    const missing = Object.values(images).some(img => !img); if (missing) throw new Error('An image could not be loaded. Replace it before exporting.');
    const slides = indexes.map(i => { const canvas = document.createElement('canvas'); render(canvas, snapshot.slides[i], snapshot, i, images); return canvas.toDataURL('image/png'); });
    const name = safeFilename(snapshot.name) + (single ? `-${String(selected + 1).padStart(2, '0')}` : '');
    if (window.desktop) { if (await desktop.exportSlides({ name, slides })) toast(single ? 'Your slide is ready.' : `${slides.length} full-resolution slides exported.`); }
    else if (slides.length === 1) { download(await (await fetch(slides[0])).blob(), name + '.png'); toast('Your slide is ready.'); }
    else { const zip = new JSZip(); slides.forEach((src, i) => zip.file(`${String(i + 1).padStart(2, '0')}-${name}.png`, src.split(',')[1], { base64: true })); download(await zip.generateAsync({ type: 'blob' }), name + '.zip'); toast(`${slides.length} slides exported.`); }
  } catch (error) { toast('Export failed: ' + error.message); }
  finally { busy = false; $('export').disabled = false; $('export-slide').disabled = false; $('export').innerHTML = 'Export carousel <span>↗</span>'; }
}
async function uploadImage(input, key) {
  const file = input.files[0]; if (!file) return;
  const targetId = current().id;
  try {
    if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type) || file.size > 15 * 1024 * 1024) throw new Error('Choose a PNG, JPEG, or WebP under 15 MB.');
    const data = await new Promise((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(reader.result); reader.onerror = () => reject(new Error('Could not read image.')); reader.readAsDataURL(file); });
    if (!await loadImage(data)) throw new Error('This image cannot be read.');
    if (JSON.stringify(project).length + data.length > 150 * 1024 * 1024) throw new Error('Project reached the 150 MB limit. Use smaller images.');
    const target = project.slides.find(s => s.id === targetId); if (!target) return;
    edit(() => { target[key] = data; }); toast('Image added.');
  } catch (error) { toast(error.message); } finally { input.value = ''; }
}
function references() {
  $('reference-grid').replaceChildren();
  layouts.forEach((layout, i) => {
    if (layout.id === 'navy-ribbon') return;
    const button = document.createElement('button'); const image = document.createElement('img'); image.loading = 'lazy'; image.alt = `${layout.name} Figma reference`;
    image.src = 'assets/references/' + encodeURIComponent(`Size=${$('ref-format').value}, Theme=${$('ref-theme').value}, Sections=${referenceNames[i]}.svg`);
    button.append(image, document.createTextNode(layout.name + ' ↗'));
    button.onclick = () => { edit(() => { current().layout = layout.id; project.format = $('ref-format').value.toLowerCase(); project.theme = $('ref-theme').value.toLowerCase(); }); $('reference-dialog').close(); toast(layout.name + ' layout applied.'); }; $('reference-grid').append(button);
  });
  const navy = document.createElement('button'); navy.className = 'panoramic-reference';
  const image = document.createElement('img'); image.src = 'assets/references/Untitled.png'; image.alt = 'Original navy panoramic carousel';
  navy.append(image, document.createTextNode('Navy ribbon · start the 8-slide carousel ↗'));
  navy.onclick = () => { startNavy(); $('reference-dialog').close(); }; $('reference-grid').append(navy);
}
function startNavy() { return createProject(navyProject()); }
async function showPreview() { render($('preview-canvas'), project.slides[previewIndex], project, previewIndex, await allImages()); $('preview-count').textContent = `${previewIndex + 1} / ${project.slides.length}`; $('preview-prev').disabled = previewIndex === 0; $('preview-next').disabled = previewIndex === project.slides.length - 1; }
['eyebrow', 'title', 'body'].forEach(id => $(id).addEventListener('input', () => { markTextEdit(); current()[id] = $(id).value; finishChange(); }));
$('items').addEventListener('input', () => { markTextEdit(); current().items = $('items').value.split('\n').filter(s => s.trim()).slice(0, 5); finishChange(); });
$('project-name').addEventListener('input', () => { markTextEdit(); project.name = $('project-name').value; scheduleSave(); });
[['brand-name', 'brand'], ['brand-handle', 'handle'], ['brand-accent', 'accent'], ['brand-font', 'font']].forEach(([id, key]) => $(id).addEventListener('input', () => { markTextEdit(); project[key] = $(id).value; $('accent-value').textContent = project.accent.toUpperCase(); finishChange(); }));
$('format').onchange = () => edit(() => { project.format = $('format').value; });
['light', 'dark'].forEach(theme => $(theme + '-theme').onclick = () => edit(() => { project.theme = theme; }));
$('title-size').oninput = () => { markTextEdit(); current().titleSize = Number($('title-size').value); $('size-value').textContent = current().titleSize + ' px'; finishChange(); };
['texture', 'swipe'].forEach(id => $(id).onchange = () => edit(() => { current()[id] = $(id).checked; }));
$('add-slide').onclick = addSlide;
$('duplicate').onclick = () => { if (project.slides.length >= 20) return; edit(() => { const copy = clone(current()); copy.id = crypto.randomUUID(); project.slides.splice(selected + 1, 0, copy); selected++; }); };
$('delete-slide').onclick = () => { if (project.slides.length === 1) return; edit(() => { project.slides.splice(selected, 1); selected = Math.min(selected, project.slides.length - 1); }); toast('Slide deleted. Use Undo to restore it.'); };
[['move-left', -1], ['move-right', 1]].forEach(([id, direction]) => $(id).onclick = () => edit(() => { const next = selected + direction; if (next < 0 || next >= project.slides.length) return; const slide = project.slides.splice(selected, 1)[0]; project.slides.splice(next, 0, slide); selected = next; }));
$('undo').onclick = () => undo(); $('redo').onclick = () => undo(true);
$('image-file').onchange = () => uploadImage($('image-file'), 'image'); $('image2-file').onchange = () => uploadImage($('image2-file'), 'image2');
$('remove-image').onclick = () => edit(() => { current().image = ''; current().image2 = ''; });
$('save-project').onclick = saveProject;
$('save-project-file').onclick = saveProjectFile;
$('new-project').onclick = () => createProject({ ...defaults(), name: 'Untitled carousel' });
$('navy-starter').onclick = startNavy;
$('open-project').onclick = () => { switchTab('projects'); $('projects-list').scrollIntoView({ block: 'nearest' }); };
$('import-project').onclick = async () => { try { if (window.desktop) { const value = await desktop.openProject(); if (value) await importProjectFile(value); } else $('project-file').click(); } catch (error) { toast('Could not open file: ' + error.message); } };
$('project-file').onchange = async () => { const file = $('project-file').files[0]; if (!file) return; try { if (file.size > 150 * 1024 * 1024) throw new Error('Project exceeds 150 MB.'); await importProjectFile(JSON.parse(await file.text())); } catch (error) { toast('Could not open: ' + error.message); } finally { $('project-file').value = ''; } };
$('export').onclick = () => exportSlides(); $('export-slide').onclick = () => exportSlides(true);
$('references').onclick = () => { $('ref-format').value = project.format === 'square' ? 'Square' : 'Portrait'; $('ref-theme').value = project.theme === 'dark' ? 'Dark' : 'Light'; references(); $('reference-dialog').showModal(); };
$('ref-format').onchange = references; $('ref-theme').onchange = references;
$('preview').onclick = async () => { previewIndex = selected; await showPreview(); $('preview-dialog').showModal(); };
$('preview-prev').onclick = () => { if (previewIndex > 0) { previewIndex--; showPreview(); } }; $('preview-next').onclick = () => { if (previewIndex < project.slides.length - 1) { previewIndex++; showPreview(); } };
document.querySelectorAll('[data-close]').forEach(button => button.onclick = () => $(button.dataset.close).close());
function switchTab(tab) { ['projects', 'templates', 'brand'].forEach(name => { $(name + '-panel').hidden = name !== tab; $(name + '-tab').classList.toggle('active', name === tab); $(name + '-tab').setAttribute('aria-selected', String(name === tab)); }); }
['projects', 'templates', 'brand'].forEach(tab => $(tab + '-tab').onclick = () => switchTab(tab));
document.addEventListener('keydown', event => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') { event.preventDefault(); saveProject(); }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') { event.preventDefault(); undo(event.shiftKey); }
  if (/INPUT|TEXTAREA|SELECT/.test(event.target.tagName)) return;
  if ($('preview-dialog').open) { if (event.key === 'ArrowLeft') $('preview-prev').click(); if (event.key === 'ArrowRight') $('preview-next').click(); return; }
  if ($('reference-dialog').open) return;
  if (event.key === 'ArrowLeft' && selected > 0) selectSlide(selected - 1);
  if (event.key === 'ArrowRight' && selected < project.slides.length - 1) selectSlide(selected + 1);
});
window.FolioSession = { snapshot: () => clone(project) };
window.addEventListener('beforeunload', event => { if (!window.desktop && savedSnapshot !== JSON.stringify(project)) { event.preventDefault(); event.returnValue = ''; } });
(async () => { try { let saved = await openDB(); await seedFunTechProjects(); if (window.desktop) { const recovery = await desktop.loadRecovery(); if (recovery && (!saved || (recovery.savedAt || 0) > (saved.savedAt || 0))) saved = recovery; } if (!saved && featuredProjects.length) saved = await savedLibraryProject(featuredProjects[0].slug); if (saved) { project = validate(saved); savedSnapshot = JSON.stringify(project); } } catch (_) { $('save-status').textContent = 'Use Save project to keep your work'; } syncControls(); await paint(); await renderSavedProjects(); scheduleSave(); })();
