'use strict';
const $ = id => document.getElementById(id);
const layerList = document.createElement('div'); layerList.id = 'layer-list'; layerList.className = 'layer-list'; $('layer-controls').before(layerList);
const layerOrder = document.createElement('div'); layerOrder.className = 'layer-actions';
for (const [id, label] of [['layer-back', 'Send backward'], ['layer-front', 'Bring forward']]) { const button = document.createElement('button'); button.id = id; button.type = 'button'; button.className = 'outline'; button.textContent = label; layerOrder.append(button); }
$('remove-layer').before(layerOrder);
['project-name', 'eyebrow', 'title', 'body', 'layer-text'].forEach(id => $(id).dir = 'auto');
const { layouts, referenceNames, render } = FolioRender;
const clone = value => JSON.parse(JSON.stringify(value));
const createSlide = (layout = 'text', overrides = {}) => ({ id: crypto.randomUUID(), layout, eyebrow: 'THE CREATIVE NOTES', title: 'One idea.\nA little possibility.', body: 'Share something useful. Make it simple, make it thoughtful, make it yours.', items: ['Start with a clear idea', 'Make every slide count', 'Finish with a next step'], image: '', image2: '', layers: [], titleSize: 88, texture: true, swipe: true, ...overrides });
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
let project = defaults(); let selected = 0; let selectedLayerId = null; let previewIndex = 0; let db; let toastTimer; let saveTimer; let changeTimer; let renderToken = 0; let projectPickerToken = 0; let savedSnapshot; let busy = false; let featuredProjects = []; let savedProjectsCache = []; let projectFolder = null; let saveQueue = Promise.resolve(); let renderedImages = {};
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
async function persistSnapshot(snapshot) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction('projects', 'readwrite'); const store = tx.objectStore('projects');
    store.put(snapshot, 'current');
    if (snapshot.libraryId) store.put(snapshot, 'library:' + snapshot.libraryId);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error || new Error('Could not save this project.'));
    tx.onabort = () => reject(tx.error || new Error('Could not save this project.'));
  });
}
async function saveNow() {
  clearTimeout(saveTimer);
  if (!db && !projectFolder) { $('save-status').textContent = 'Use Save project to keep your work'; return; }
  project.savedAt = Date.now(); const snapshot = clone(project); const folder = projectFolder;
  saveQueue = saveQueue.catch(() => {}).then(async () => {
    if (db) await persistSnapshot(snapshot);
    if (folder && window.desktop?.saveFolderProject) {
      const saved = await desktop.saveFolderProject(snapshot);
      snapshot.folderFileName = saved.fileName;
      if (project.libraryId === snapshot.libraryId && projectFolder === folder) project.folderFileName = saved.fileName;
      if (db) await persistSnapshot(snapshot);
    }
    if (project.libraryId === snapshot.libraryId) {
      savedSnapshot = JSON.stringify(snapshot);
      $('save-status').textContent = folder ? 'Saved in project folder' : 'Saved on this device';
    }
    renderSavedProjects().catch(() => {});
  });
  return saveQueue;
}
function scheduleSave() {
  $('save-status').textContent = 'Saving on this device…'; clearTimeout(saveTimer);
  saveTimer = setTimeout(() => saveNow().catch(() => { $('save-status').textContent = 'Could not save to the project folder'; }), 350);
}
async function loadImage(src) {
  if (!src) return null;
  if (!imageCache.has(src)) imageCache.set(src, new Promise(resolve => { const img = new Image(); img.onload = () => resolve(img); img.onerror = () => resolve(null); img.src = src; }));
  return imageCache.get(src);
}
async function allImages(snapshot = project) {
  const sources = [...new Set(snapshot.slides.flatMap(s => [s.image, s.image2, ...(s.layers || []).filter(layer => layer.type === 'image').map(layer => layer.src)]).filter(Boolean))];
  const result = {}; await Promise.all(sources.map(async src => { result[src] = await loadImage(src); })); return result;
}
async function paint() {
  const token = ++renderToken; const images = await allImages(); if (token !== renderToken) return;
  renderedImages = images;
  const result = render($('artboard'), current(), project, selected, images);
  renderLayerHandles();
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
const clamp = (number, low, high) => Math.min(Math.max(number, low), high);
function selectedLayer() { return (current().layers || []).find(layer => layer.id === selectedLayerId); }
function syncLayerControls() {
  const layer = selectedLayer();
  layerList.replaceChildren();
  (current().layers || []).forEach((item, index) => {
    const button = document.createElement('button'); button.type = 'button';
    button.className = 'layer-list-item' + (item.id === selectedLayerId ? ' active' : '');
    button.textContent = item.type === 'image' ? `▣ Image ${index + 1}` : `T ${item.text.slice(0, 32) || 'Empty text'}`;
    button.setAttribute('aria-pressed', String(item.id === selectedLayerId));
    button.onclick = () => { selectedLayerId = item.id; syncLayerControls(); };
    layerList.append(button);
  });
  $('layer-controls').hidden = !layer;
  const position = (current().layers || []).findIndex(item => item.id === selectedLayerId);
  $('layer-back').disabled = position <= 0;
  $('layer-front').disabled = position < 0 || position === current().layers.length - 1;
  if (layer) {
    const isText = layer.type === 'text';
    for (const id of ['layer-text-label', 'layer-size-label', 'layer-color-label']) $(id).hidden = !isText;
    if (isText) {
      if ($('layer-text').value !== layer.text) $('layer-text').value = layer.text;
      $('layer-size').value = layer.fontSize;
      $('layer-size-value').textContent = layer.fontSize + ' px';
      $('layer-color').value = layer.color;
    }
  }
  document.querySelectorAll('#overlay-editor .layer-hit').forEach(element => element.classList.toggle('selected', element.dataset.id === selectedLayerId));
}
function positionLayerHandle(element, layer) {
  const height = project.format === 'square' ? 1080 : 1350;
  element.style.left = `${layer.x / 1080 * 100}%`;
  element.style.top = `${layer.y / height * 100}%`;
  element.style.width = `${layer.width / 1080 * 100}%`;
  element.style.height = `${layer.height / height * 100}%`;
}
function renderLayerHandles() {
  const editor = $('overlay-editor'); editor.replaceChildren();
  for (const layer of current().layers || []) {
    const element = document.createElement('div'); element.className = 'layer-hit'; element.dataset.id = layer.id;
    element.tabIndex = 0; element.setAttribute('role', 'button');
    element.setAttribute('aria-label', layer.type === 'image' ? 'Move or resize image' : 'Move or resize text');
    positionLayerHandle(element, layer);
    const corner = document.createElement('span'); corner.className = 'layer-resize'; corner.title = 'Drag to resize';
    element.append(corner);
    element.onpointerdown = event => {
      if (event.button !== 0) return;
      event.preventDefault(); event.stopPropagation();
      selectedLayerId = layer.id; syncLayerControls();
      const resizing = event.target === corner;
      const start = { ...layer, pointerX: event.clientX, pointerY: event.clientY };
      let changed = false;
      element.setPointerCapture(event.pointerId);
      element.onpointermove = move => {
        if (!element.hasPointerCapture(move.pointerId)) return;
        const rect = $('artboard').getBoundingClientRect();
        const dx = (move.clientX - start.pointerX) * 1080 / rect.width;
        const dy = (move.clientY - start.pointerY) * (project.format === 'square' ? 1080 : 1350) / rect.height;
        if (Math.abs(dx) + Math.abs(dy) < 1) return;
        if (!changed) { record(); changed = true; }
        if (resizing) {
          if (layer.type === 'image') {
            const factor = 1 + (Math.abs(dx / start.width) > Math.abs(dy / start.height) ? dx / start.width : dy / start.height);
            const bounded = clamp(factor, Math.max(40 / start.width, 40 / start.height), Math.min((1080 - start.x) / start.width, ((project.format === 'square' ? 1080 : 1350) - start.y) / start.height));
            layer.width = Math.round(start.width * bounded); layer.height = Math.round(start.height * bounded);
          } else {
            layer.width = Math.round(clamp(start.width + dx, 50, 1080 - start.x));
            layer.height = Math.round(clamp(start.height + dy, 40, (project.format === 'square' ? 1080 : 1350) - start.y));
          }
        } else {
          layer.x = Math.round(clamp(start.x + dx, 0, 1080 - layer.width));
          layer.y = Math.round(clamp(start.y + dy, 0, (project.format === 'square' ? 1080 : 1350) - layer.height));
        }
        positionLayerHandle(element, layer);
        render($('artboard'), current(), project, selected, renderedImages);
      };
      element.onpointerup = element.onpointercancel = end => {
        if (element.hasPointerCapture(end.pointerId)) element.releasePointerCapture(end.pointerId);
        element.onpointermove = element.onpointerup = element.onpointercancel = null;
        if (changed) { finishChange(); syncControls(); }
      };
    };
    element.ondblclick = () => { selectedLayerId = layer.id; syncLayerControls(); if (layer.type === 'text') $('layer-text').focus(); };
    editor.append(element);
  }
  syncLayerControls();
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
    const active = button.dataset.id === (projectFolder ? project.folderFileName : project.libraryId);
    button.classList.toggle('active', active); button.setAttribute('aria-pressed', String(active));
  });
  syncLayerControls();
}
function selectSlide(i) { clearTimeout(changeTimer); changeTimer = null; selected = i; selectedLayerId = null; syncControls(); paint(); }
function addSlide() { if (project.slides.length >= 20) return toast('A carousel can contain up to 20 slides.'); edit(() => { const slide = current().layout === 'navy-ribbon' ? createSlide('navy-ribbon', { eyebrow: '', title: 'Your next idea', body: 'Add a new chapter to your story.', titleSize: 64, swipe: false }) : createSlide(); project.slides.splice(selected + 1, 0, slide); selected++; }); }
function undo(redo = false) {
  const source = redo ? future : history; const target = redo ? history : future;
  if (!source.length) return; clearTimeout(changeTimer); changeTimer = null; target.push(JSON.stringify({ project, selected })); const state = JSON.parse(source.pop()); project = state.project; selected = state.selected; selectedLayerId = null; syncControls(); finishChange();
}
function validate(value) {
  if (!value || value.version !== 1 || !Array.isArray(value.slides) || value.slides.length < 1 || value.slides.length > 20 || !['portrait', 'square'].includes(value.format) || !['light', 'dark'].includes(value.theme)) throw new Error('This is not a supported Folio project.');
  if (value.direction !== undefined && !['ltr', 'rtl'].includes(value.direction)) throw new Error('Invalid project text direction.');
  if (!/^#[\da-f]{6}$/i.test(value.accent) || !['Arial', 'Georgia', 'Verdana', 'Trebuchet MS'].includes(value.font)) throw new Error('Project brand settings are invalid.');
  for (const key of ['name', 'brand', 'handle']) if (typeof value[key] !== 'string' || value[key].length > 100) throw new Error('Invalid project text.');
  value.slides.forEach(s => {
    if (!layouts.some(l => l.id === s.layout) || !Number.isFinite(s.titleSize) || s.titleSize < 44 || s.titleSize > 120) throw new Error('Invalid slide layout.');
    for (const key of ['eyebrow', 'title', 'body']) if (typeof s[key] !== 'string' || s[key].length > 1500) throw new Error('Invalid slide text.');
    if (!Array.isArray(s.items) || s.items.length > 5 || s.items.some(t => typeof t !== 'string' || t.length > 1000)) throw new Error('Invalid checklist.');
    for (const key of ['image', 'image2']) if (typeof s[key] !== 'string' || (s[key] && !/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/.test(s[key]))) throw new Error('Invalid slide image.');
    if (s.layers === undefined) s.layers = [];
    if (!Array.isArray(s.layers) || s.layers.length > 30) throw new Error('Invalid slide elements.');
    s.layers.forEach(layer => {
      if (!layer || !['image', 'text'].includes(layer.type)) throw new Error('Invalid slide element.');
      for (const key of ['x', 'y', 'width', 'height']) if (!Number.isFinite(layer[key])) throw new Error('Invalid element position.');
      if (layer.x < 0 || layer.y < 0 || layer.width < 20 || layer.height < 20 || layer.x + layer.width > 1080.01 || layer.y + layer.height > (value.format === 'square' ? 1080.01 : 1350.01)) throw new Error('Element is outside the slide.');
      if (layer.type === 'image' && (typeof layer.src !== 'string' || !/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/.test(layer.src))) throw new Error('Invalid element image.');
      if (layer.type === 'text' && (typeof layer.text !== 'string' || layer.text.length > 500 || !/^#[\da-f]{6}$/i.test(layer.color) || !Number.isFinite(layer.fontSize) || layer.fontSize < 24 || layer.fontSize > 180)) throw new Error('Invalid element text.');
      layer.id = crypto.randomUUID();
    });
    if (typeof s.texture !== 'boolean' || typeof s.swipe !== 'boolean') throw new Error('Invalid slide options.');
    s.id = crypto.randomUUID();
  });
  return value;
}
async function adopt(value) { const next = validate(value); record(); project = next; selected = 0; selectedLayerId = null; imageCache.clear(); syncControls(); finishChange(); toast('Project opened. Pick up where you left off.'); }
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
async function legacyProjects() {
  if (!db) return [];
  const values = await new Promise((resolve, reject) => {
    const request = db.transaction('projects').objectStore('projects').getAll();
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  const byId = new Map();
  values.filter(value => value && value.libraryId && Array.isArray(value.slides)).forEach(value => {
    const previous = byId.get(value.libraryId);
    if (!previous || (value.savedAt || 0) >= (previous.savedAt || 0)) byId.set(value.libraryId, value);
  });
  return [...byId.values()];
}
async function migrateProjectsToFolder() {
  if (!projectFolder || !window.desktop?.listFolderProjects) return;
  const existing = await desktop.listFolderProjects();
  const byId = new Map(existing.filter(item => item.libraryId).map(item => [item.libraryId, item]));
  for (const value of await legacyProjects()) {
    const match = byId.get(value.libraryId);
    if (match && match.savedAt >= (value.savedAt || 0)) continue;
    const copy = clone(value);
    copy.folderFileName = match ? match.fileName : '';
    const saved = await desktop.saveFolderProject(copy);
    if (project.libraryId === copy.libraryId) project.folderFileName = saved.fileName;
    byId.set(copy.libraryId, { fileName: saved.fileName, libraryId: copy.libraryId, savedAt: copy.savedAt || 0 });
  }
}
function updateFolderLabel() {
  const label = projectFolder || (window.desktop ? 'Choose a folder with Open' : 'Saved in this browser');
  $('projects-folder-path').textContent = label; $('projects-folder-path').title = label;
  $('project-dialog-folder-path').textContent = label; $('project-dialog-folder-path').title = label;
}
async function chooseProjectFolder() {
  if (!window.desktop?.chooseProjectFolder) return true;
  await saveNow();
  const previous = projectFolder;
  const chosen = await desktop.chooseProjectFolder();
  if (!chosen) return false;
  projectFolder = chosen;
  if (previous !== chosen) {
    project.folderFileName = '';
    await migrateProjectsToFolder();
  }
  updateFolderLabel();
  await renderSavedProjects();
  return true;
}
async function ensureProjectFolder() {
  if (projectFolder || !window.desktop?.chooseProjectFolder) return true;
  return chooseProjectFolder();
}
async function renderSavedProjects() {
  if (!db && !projectFolder) return;
  updateFolderLabel();
  const featuredOrder = new Map(featuredProjects.map((entry, index) => [entry.slug, index]));
  let entries;
  try { entries = projectFolder && window.desktop?.listFolderProjects ? await desktop.listFolderProjects() : await legacyProjects(); }
  catch (_) {
    savedProjectsCache = [];
    const list = $('projects-list'); list.replaceChildren();
    const message = document.createElement('p'); message.className = 'collection-intro'; message.textContent = 'Your saved folder is temporarily unavailable. Folio will use it again when it returns.'; list.append(message);
    $('projects-count').textContent = 'Folder unavailable';
    if ($('project-dialog').open) renderProjectPicker();
    return;
  }
  const projects = entries.sort((a, b) => {
    const ai = featuredOrder.has(a.libraryId) ? featuredOrder.get(a.libraryId) : Infinity;
    const bi = featuredOrder.has(b.libraryId) ? featuredOrder.get(b.libraryId) : Infinity;
    return ai - bi || (b.savedAt || 0) - (a.savedAt || 0);
  });
  savedProjectsCache = projects;
  const list = $('projects-list'); list.replaceChildren();
  projects.forEach((entry, index) => {
    const button = document.createElement('button'); button.type = 'button'; button.className = 'collection-project'; button.dataset.id = projectFolder ? entry.fileName : entry.libraryId;
    button.setAttribute('aria-label', `Open ${entry.name}`);
    const number = document.createElement('span'); number.className = 'collection-number'; number.textContent = String(index + 1).padStart(2, '0');
    const label = document.createElement('span'); label.className = 'collection-label'; label.dir = 'auto'; label.textContent = entry.name || 'Untitled carousel';
    const count = document.createElement('small'); count.textContent = `${Array.isArray(entry.slides) ? entry.slides.length : entry.slides} slides`;
    button.append(number, label, count); button.onclick = () => openSavedProject(button.dataset.id); list.append(button);
  });
  $('projects-count').textContent = `${projects.length} project${projects.length === 1 ? '' : 's'}`;
  syncControls();
  if ($('project-dialog').open) renderProjectPicker();
}
function renderProjectPicker() {
  const token = ++projectPickerToken;
  const query = $('project-search').value.trim().toLocaleLowerCase();
  const list = $('project-picker-list'); list.replaceChildren();
  const matches = savedProjectsCache.filter(entry => (entry.name || '').toLocaleLowerCase().includes(query));
  $('project-picker-count').textContent = `${matches.length} project${matches.length === 1 ? '' : 's'}`;
  matches.forEach(entry => {
    const button = document.createElement('button'); button.type = 'button'; button.className = 'project-tile'; button.dataset.id = projectFolder ? entry.fileName : entry.libraryId;
    button.setAttribute('aria-label', `Open ${entry.name}`);
    const cover = document.createElement('span'); cover.className = 'project-tile-cover';
    const canvas = document.createElement('canvas'); canvas.setAttribute('aria-hidden', 'true');
    cover.append(canvas);
    const info = document.createElement('span'); info.className = 'project-tile-info';
    const label = document.createElement('strong'); label.dir = 'auto'; label.textContent = entry.name || 'Untitled carousel';
    const count = document.createElement('small'); count.textContent = `${Array.isArray(entry.slides) ? entry.slides.length : entry.slides} slides`;
    info.append(label, count);
    const action = document.createElement('span'); action.className = 'project-tile-action'; action.setAttribute('aria-hidden', 'true'); action.textContent = 'Open ↗';
    button.append(cover, info, action);
    button.onclick = async () => { if (await openSavedProject(button.dataset.id)) $('project-dialog').close(); };
    list.append(button);
    const value = projectFolder && window.desktop?.openFolderProject ? desktop.openFolderProject(entry.fileName) : Promise.resolve(entry);
    Promise.resolve(value).then(async snapshot => {
      if (token !== projectPickerToken || !button.isConnected || !snapshot?.slides?.length) return;
      const images = await allImages(snapshot);
      if (token !== projectPickerToken || !button.isConnected) return;
      render(canvas, snapshot.slides[0], snapshot, 0, images);
      cover.classList.add('ready');
      cover.style.aspectRatio = snapshot.format === 'square' ? '1' : '4 / 5';
    }).catch(() => {
      if (button.isConnected) cover.classList.add('unavailable');
    });
  });
  if (!list.childElementCount) {
    const empty = document.createElement('p'); empty.className = 'collection-intro'; empty.textContent = 'No saved projects match your search.'; list.append(empty);
  }
}
async function showProjectPicker() {
  try {
    if (!await ensureProjectFolder()) return;
    await renderSavedProjects();
    $('project-search').value = '';
    renderProjectPicker();
    $('project-dialog').showModal();
    $('project-search').focus();
  } catch (error) { toast('Could not show saved projects: ' + error.message); }
}
async function openSavedProject(id) {
  try {
    await saveNow();
    const value = projectFolder && window.desktop?.openFolderProject ? await desktop.openFolderProject(id) : await savedLibraryProject(id);
    if (!value) throw new Error('This saved project could not be found.');
    if (!value.libraryId) value.libraryId = crypto.randomUUID();
    await adopt(value);
    await saveNow();
    switchTab('projects');
    await renderSavedProjects();
    return true;
  } catch (error) { toast('Could not open project: ' + error.message); return false; }
}
async function createProject(value) {
  try {
    await saveNow();
    if (!await ensureProjectFolder()) return;
    value.libraryId = crypto.randomUUID();
    value.folderFileName = '';
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
    if (!await ensureProjectFolder()) return;
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
  if (!await ensureProjectFolder()) return;
  value.libraryId = crypto.randomUUID();
  value.folderFileName = '';
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
async function addLayerImage(file, x, y) {
  if (!file) return;
  const targetId = current().id;
  try {
    if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type) || file.size > 15 * 1024 * 1024) throw new Error('Choose a PNG, JPEG, or WebP under 15 MB.');
    const src = await new Promise((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(reader.result); reader.onerror = () => reject(new Error('Could not read image.')); reader.readAsDataURL(file); });
    const image = await loadImage(src);
    if (!image) throw new Error('This image cannot be read.');
    if (JSON.stringify(project).length + src.length > 150 * 1024 * 1024) throw new Error('Project reached the 150 MB limit. Use a smaller image.');
    const target = project.slides.find(slide => slide.id === targetId); if (!target) return;
    if ((target.layers || []).length >= 30) throw new Error('A slide can have up to 30 extra elements.');
    const scale = 440 / Math.max(image.width, image.height);
    const width = Math.max(40, Math.round(image.width * scale));
    const height = Math.max(40, Math.round(image.height * scale));
    const layer = { id: crypto.randomUUID(), type: 'image', src, x: Math.round(clamp(x ?? (1080 - width) / 2, 0, 1080 - width)), y: Math.round(clamp(y ?? ((project.format === 'square' ? 1080 : 1350) - height) / 2, 0, (project.format === 'square' ? 1080 : 1350) - height)), width, height };
    edit(() => { target.layers ||= []; target.layers.push(layer); selectedLayerId = layer.id; });
    toast('Image added. Drag it to move or use the corner to resize.');
  } catch (error) { toast(error.message); }
}
function addLayerText() {
  if ((current().layers || []).length >= 30) return toast('A slide can have up to 30 extra elements.');
  const layer = { id: crypto.randomUUID(), type: 'text', text: 'Your text here', color: project.theme === 'dark' ? '#f9dec9' : '#20211f', fontSize: 64, x: 140, y: project.format === 'square' ? 410 : 520, width: 650, height: 180 };
  edit(() => { current().layers ||= []; current().layers.push(layer); selectedLayerId = layer.id; });
  $('layer-text').focus(); $('layer-text').select();
}
function removeSelectedLayer() {
  if (!selectedLayer()) return;
  edit(() => { current().layers = current().layers.filter(layer => layer.id !== selectedLayerId); selectedLayerId = null; });
}
function moveSelectedLayer(direction) {
  const position = (current().layers || []).findIndex(layer => layer.id === selectedLayerId);
  const next = position + direction;
  if (position < 0 || next < 0 || next >= current().layers.length) return;
  edit(() => { const [layer] = current().layers.splice(position, 1); current().layers.splice(next, 0, layer); });
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
$('format').onchange = () => edit(() => { project.format = $('format').value; const height = project.format === 'square' ? 1080 : 1350; project.slides.forEach(slide => (slide.layers || []).forEach(layer => { if (layer.height > height) { const scale = height / layer.height; layer.height = height; if (layer.type === 'image') layer.width = Math.round(layer.width * scale); } layer.x = Math.min(layer.x, 1080 - layer.width); layer.y = Math.min(layer.y, height - layer.height); })); });
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
$('add-layer-image').onclick = () => $('layer-image-file').click();
$('layer-image-file').onchange = () => { addLayerImage($('layer-image-file').files[0]); $('layer-image-file').value = ''; };
$('add-layer-text').onclick = addLayerText;
$('remove-layer').onclick = removeSelectedLayer;
$('layer-back').onclick = () => moveSelectedLayer(-1);
$('layer-front').onclick = () => moveSelectedLayer(1);
$('layer-text').oninput = () => { const layer = selectedLayer(); if (!layer || layer.type !== 'text') return; markTextEdit(); layer.text = $('layer-text').value; finishChange(); };
$('layer-size').oninput = () => { const layer = selectedLayer(); if (!layer || layer.type !== 'text') return; markTextEdit(); layer.fontSize = Number($('layer-size').value); $('layer-size-value').textContent = layer.fontSize + ' px'; finishChange(); };
$('layer-color').oninput = () => { const layer = selectedLayer(); if (!layer || layer.type !== 'text') return; markTextEdit(); layer.color = $('layer-color').value; finishChange(); };
$('artboard').onclick = () => { selectedLayerId = null; syncLayerControls(); };
document.querySelector('.canvas-wrap').addEventListener('dragover', event => { if (event.dataTransfer.types.includes('Files')) { event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; } });
document.querySelector('.canvas-wrap').addEventListener('drop', event => {
  event.preventDefault();
  const file = [...event.dataTransfer.files].find(item => item.type.startsWith('image/'));
  if (!file) return;
  const rect = $('artboard').getBoundingClientRect();
  addLayerImage(file, (event.clientX - rect.left) * 1080 / rect.width, (event.clientY - rect.top) * (project.format === 'square' ? 1080 : 1350) / rect.height);
});
document.addEventListener('paste', event => {
  if (/INPUT|TEXTAREA/.test(event.target.tagName)) return;
  const file = [...(event.clipboardData?.files || [])].find(item => item.type.startsWith('image/'));
  if (file) { event.preventDefault(); addLayerImage(file); }
});
$('save-project').onclick = saveProject;
$('save-project-file').onclick = saveProjectFile;
$('new-project').onclick = () => createProject({ ...defaults(), name: 'Untitled carousel' });
$('navy-starter').onclick = startNavy;
$('open-project').onclick = showProjectPicker;
$('change-project-folder').onclick = () => chooseProjectFolder().catch(error => toast('Could not change folder: ' + error.message));
$('project-dialog-change-folder').onclick = () => chooseProjectFolder().catch(error => toast('Could not change folder: ' + error.message));
$('project-search').oninput = renderProjectPicker;
$('project-dialog-import').onclick = () => { $('project-dialog').close(); $('import-project').click(); };
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
  if ($('project-dialog').open) return;
  if ($('reference-dialog').open) return;
  if (selectedLayer() && (event.key === 'Delete' || event.key === 'Backspace')) { event.preventDefault(); removeSelectedLayer(); return; }
  if (selectedLayer() && event.key === 'Escape') { selectedLayerId = null; syncLayerControls(); return; }
  if (selectedLayer() && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
    event.preventDefault(); const layer = selectedLayer();
    edit(() => { const step = event.shiftKey ? 20 : 5; layer.x = clamp(layer.x + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0), 0, 1080 - layer.width); layer.y = clamp(layer.y + (event.key === 'ArrowDown' ? step : event.key === 'ArrowUp' ? -step : 0), 0, (project.format === 'square' ? 1080 : 1350) - layer.height); });
    return;
  }
  if (event.key === 'ArrowLeft' && selected > 0) selectSlide(selected - 1);
  if (event.key === 'ArrowRight' && selected < project.slides.length - 1) selectSlide(selected + 1);
});
window.FolioSession = { snapshot: () => clone(project) };
window.addEventListener('beforeunload', event => { if (!window.desktop && savedSnapshot !== JSON.stringify(project)) { event.preventDefault(); event.returnValue = ''; } });
window.FolioReady = (async () => {
  try {
    if (window.desktop) {
      projectFolder = await desktop.getProjectFolder();
    }
    let saved = null;
    try { saved = await openDB(); } catch (_) { db = undefined; }
    if (db) await seedFunTechProjects().catch(() => {});
    if (window.desktop) {
      const recovery = await desktop.loadRecovery();
      if (recovery && (!saved || (recovery.savedAt || 0) > (saved.savedAt || 0))) saved = recovery;
    }
    if (!saved && featuredProjects.length) saved = await savedLibraryProject(featuredProjects[0].slug);
    if (projectFolder && saved?.libraryId) {
      try {
        const files = await desktop.listFolderProjects();
        const match = files.find(item => item.fileName === saved.folderFileName) || files.find(item => item.libraryId === saved.libraryId);
        if (match) {
          if (Math.max(match.savedAt, match.modifiedAt) >= (saved.savedAt || 0)) saved = await desktop.openFolderProject(match.fileName);
          else saved.folderFileName = match.fileName;
        }
      } catch (_) { /* Keep the recovered editor state until the saved folder returns. */ }
    }
    if (saved) { project = validate(saved); savedSnapshot = JSON.stringify(project); }
  } catch (_) { $('save-status').textContent = 'Use Save project to keep your work'; }
  syncControls(); await paint(); await renderSavedProjects(); scheduleSave();
})();
