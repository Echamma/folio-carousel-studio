(function (root) {
  'use strict';
  const layouts = [
    ['introduction', 'Introduction', '↗', 'Make a first impression'],
    ['text', 'Text only', 'T', 'Give your idea some space'],
    ['checklist', 'Checklist', '✓', 'Make the next steps clear'],
    ['quote', 'Quote', '“', 'A thought worth saving'],
    ['image-center', 'Image center', '▣', 'Let your image do the talking'],
    ['image-left', 'Image left', '◧', 'A little balance goes a long way'],
    ['image-right', 'Image right', '◨', 'Show and tell'],
    ['compare', 'Image compare', '⊞', 'Two sides of a story'],
    ['thank-you', 'Thank you', '♡', 'Keep the conversation going'],
    ['navy-ribbon', 'Navy ribbon', '≋', 'A continuous story, inspired by Untitled.png']
  ].map(([id, name, icon, description]) => ({ id, name, icon, description }));
  const referenceNames = ['Introduction', 'Text Only', 'Checklist', 'Quote', 'Image Center', 'Image Left', 'Image Right', 'Image Compare', 'Thank You'];
  const palettes = { light: { bg: '#fff2ed', fg: '#20211f', muted: '#6c665f' }, dark: { bg: '#151716', fg: '#faf8f1', muted: '#b6b5ac' } };
  function wrap(ctx, value, width) {
    const lines = [];
    for (const paragraph of String(value || '').split('\n')) {
      let line = '';
      for (const word of paragraph.split(/\s+/).filter(Boolean)) {
        if (ctx.measureText(word).width > width) {
          if (line) { lines.push(line); line = ''; }
          for (const char of word) {
            if (line && ctx.measureText(line + char).width > width) { lines.push(line); line = ''; }
            line += char;
          }
        } else if (line && ctx.measureText(line + ' ' + word).width > width) { lines.push(line); line = word; }
        else line += (line ? ' ' : '') + word;
      }
      lines.push(line);
    }
    return lines;
  }
  function text(ctx, value, x, y, width, size, color, font, weight = 400, maxHeight = Infinity) {
    let actual = size;
    let lines;
    do {
      ctx.font = `${weight} ${actual}px "${font}"`;
      lines = wrap(ctx, value, width);
      if (lines.length * actual * 1.18 <= maxHeight || actual <= 16) break;
      actual -= 2;
    } while (true);
    ctx.fillStyle = color;
    ctx.textBaseline = 'top';
    const visible = lines.slice(0, Math.max(1, Math.floor(maxHeight / (actual * 1.18))));
    visible.forEach((line, i) => ctx.fillText(line, x, y + i * actual * 1.18));
    return { height: visible.length * actual * 1.18, reduced: actual < size || visible.length < lines.length };
  }
  function rounded(ctx, x, y, w, h, radius, color) {
    ctx.fillStyle = color; ctx.beginPath(); ctx.roundRect(x, y, w, h, radius); ctx.fill();
  }
  function picture(ctx, image, x, y, w, h, p, accent) {
    ctx.save(); ctx.beginPath(); ctx.roundRect(x, y, w, h, 22); ctx.clip();
    if (image) {
      const scale = Math.max(w / image.width, h / image.height);
      ctx.drawImage(image, x + (w - image.width * scale) / 2, y + (h - image.height * scale) / 2, image.width * scale, image.height * scale);
    } else {
      ctx.fillStyle = p.fg + '0c'; ctx.fillRect(x, y, w, h);
      ctx.fillStyle = accent + '28'; ctx.beginPath(); ctx.arc(x + w * .78, y + h * .25, Math.min(w, h) * .23, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = accent + '38'; ctx.beginPath(); ctx.moveTo(x, y + h); ctx.lineTo(x + w * .4, y + h * .4); ctx.lineTo(x + w * .75, y + h); ctx.fill();
      ctx.fillStyle = accent + '20'; ctx.beginPath(); ctx.moveTo(x + w * .3, y + h); ctx.lineTo(x + w * .75, y + h * .52); ctx.lineTo(x + w, y + h); ctx.fill();
      text(ctx, 'YOUR IMAGE HERE', x + 25, y + h - 46, w - 50, 18, p.muted, 'Arial', 600);
    }
    ctx.restore();
  }
  function render(canvas, slide, project, index, images = {}) {
    const h = project.format === 'square' ? 1080 : 1350;
    canvas.width = 1080; canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (slide.layout === 'navy-ribbon') return renderNavy(ctx, slide, project, index, h);
    const p = palettes[project.theme]; const accent = project.accent; const font = project.font;
    const margin = 88; const width = 904; const bottom = h - 190;
    ctx.fillStyle = p.bg; ctx.fillRect(0, 0, 1080, h);
    if (slide.texture) {
      ctx.strokeStyle = p.fg + '05'; ctx.lineWidth = 1;
      for (let x = -h; x < 1080; x += 24) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x + h, h); ctx.stroke(); }
      ctx.fillStyle = accent + '06';
      for (let x = 18; x < 1080; x += 28) for (let y = 18; y < h; y += 28) { ctx.beginPath(); ctx.arc(x, y, 1.2, 0, Math.PI * 2); ctx.fill(); }
    }
    ctx.fillStyle = accent; ctx.fillRect(0, 0, 1080 * (index + 1) / project.slides.length, 8);
    let reduced = false;
    const draw = (...args) => { const result = text(ctx, ...args); reduced ||= result.reduced; return result.height; };
    draw(slide.eyebrow.toUpperCase(), margin, 87, width - 180, 22, accent, font, 700, 60);
    if (slide.swipe) text(ctx, `${String(index + 1).padStart(2, '0')} / ${String(project.slides.length).padStart(2, '0')}`, 880, 88, 140, 22, p.muted, font, 400);
    const title = slide.title; const body = slide.body; const size = slide.titleSize;
    if (slide.layout === 'introduction') {
      const titleY = h === 1350 ? 290 : 215;
      const titleHeight = draw(title, margin, titleY, width, size + 18, p.fg, font, 700, h * .36);
      draw(body, margin + 4, titleY + titleHeight + 34, width - 100, 32, p.muted, font, 400, bottom - titleY - titleHeight - 45);
      rounded(ctx, margin, bottom - 30, 216, 52, 26, accent);
      text(ctx, 'A LITTLE INSPIRATION', margin + 18, bottom - 12, 180, 14, p.bg, font, 700);
      ctx.strokeStyle = accent; ctx.lineWidth = 4; ctx.beginPath(); ctx.arc(897, bottom - 38, 56, .15, Math.PI * 1.85); ctx.stroke();
      text(ctx, '↗', 854, bottom - 86, 130, 83, accent, 'Arial', 400);
    } else if (slide.layout === 'quote') {
      const y = h === 1350 ? 275 : 210; const boxH = bottom - y - 30;
      rounded(ctx, margin, y, width, boxH, 30, accent + '12');
      text(ctx, '“', margin + 48, y + 25, 180, 150, accent, 'Georgia', 700);
      draw(title, margin + 54, y + 146, width - 108, size * .78, accent, font, 700, boxH - 230);
      draw(body, margin + 54, y + boxH - 65, width - 108, 24, p.muted, font, 400, 55);
    } else if (slide.layout === 'checklist') {
      const th = draw(title, margin, 200, width, size * .86, p.fg, font, 700, 185);
      const bh = draw(body, margin, 220 + th, width, 28, p.muted, font, 400, 105);
      const y = 255 + th + bh; const items = slide.items.slice(0, 5);
      const gap = 16; const rowH = Math.min(120, (bottom - y - gap * (items.length - 1)) / Math.max(1, items.length));
      items.forEach((item, i) => {
        const ry = y + i * (rowH + gap); rounded(ctx, margin, ry, width, rowH, 16, accent + '12');
        rounded(ctx, margin + 27, ry + (rowH - 42) / 2, 42, 42, 21, accent);
        text(ctx, '✓', margin + 34, ry + (rowH - 30) / 2, 35, 27, p.bg, 'Arial', 700);
        draw(item, margin + 95, ry + Math.max(12, (rowH - 48) / 2), width - 125, 38, p.fg, font, 600, rowH - 20);
      });
    } else if (slide.layout === 'image-left' || slide.layout === 'image-right') {
      const left = slide.layout === 'image-left'; const y = 238;
      picture(ctx, images[slide.image], left ? margin : 571, y, 421, bottom - y, p, accent);
      const tx = left ? 558 : margin;
      const th = draw(title, tx, y + 18, 420, size * .73, p.fg, font, 700, (bottom - y) * .52);
      draw(body, tx, y + th + 50, 420, 28, p.muted, font, 400, bottom - y - th - 55);
    } else if (slide.layout === 'image-center' || slide.layout === 'compare') {
      const th = draw(title, margin, 191, width, size * .84, p.fg, font, 700, 175);
      const y = 231 + th; const imageH = bottom - y - 110;
      if (slide.layout === 'compare') {
        picture(ctx, images[slide.image], margin, y, 440, imageH, p, accent);
        picture(ctx, images[slide.image2], 552, y, 440, imageH, p, accent);
      } else picture(ctx, images[slide.image], margin, y, width, imageH, p, accent);
      draw(body, margin, y + imageH + 26, width, 28, p.muted, font, 400, 90);
    } else if (slide.layout === 'thank-you') {
      text(ctx, '✳', 462, h * .23, 200, 132, accent, 'Arial', 400);
      const y = h * .4; const th = draw(title, margin, y, width, size + 8, p.fg, font, 700, 260);
      draw(body, margin + 4, y + th + 32, width - 60, 32, p.muted, font, 400, bottom - y - th - 45);
    } else {
      const th = draw(title, margin, 235, width, size, p.fg, font, 700, 310);
      rounded(ctx, margin, 267 + th, 80, 6, 3, accent);
      draw(body, margin, 307 + th, width, 37, p.muted, font, 400, bottom - th - 320);
    }
    text(ctx, project.brand, margin, h - 123, 650, 25, p.fg, font, 700, 34);
    text(ctx, project.handle, margin, h - 86, 650, 20, p.muted, font, 400, 28);
    if (slide.swipe) {
      text(ctx, index === project.slides.length - 1 ? 'SAVE & SHARE' : 'SWIPE FOR MORE', 734, h - 102, 200, 16, p.muted, font, 600);
      rounded(ctx, 936, h - 122, 56, 56, 28, accent);
      text(ctx, index === project.slides.length - 1 ? '♡' : '→', 950, h - 111, 50, 29, p.bg, 'Arial', 400);
    }
    return { reduced };
  }
  // Use world coordinates so the torn ribbons continue across adjacent exports.
  function ribbonNoise(x, seed) {
    let n = Math.imul((x | 0) ^ seed, 0x45d9f3b);
    n = Math.imul(n ^ (n >>> 16), 0x45d9f3b);
    return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
  }
  function ribbon(ctx, y, index, seed) {
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    for (let x = -4; x <= 1084; x += 2) {
      const world = index * 1080 + x;
      const edge = y - 24 - ribbonNoise(world, seed) * 15 + ribbonNoise(Math.floor(world / 14), seed + 1) * 10;
      if (x === -4) ctx.moveTo(x, edge); else ctx.lineTo(x, edge);
    }
    for (let x = 1084; x >= -4; x -= 2) {
      const world = index * 1080 + x;
      ctx.lineTo(x, y + 24 + ribbonNoise(world, seed + 2) * 18 - ribbonNoise(Math.floor(world / 18), seed + 3) * 12);
    }
    ctx.closePath(); ctx.fill();
    for (let x = -4; x < 1084; x += 3) {
      const world = index * 1080 + x;
      for (const direction of [-1, 1]) {
        const offset = 25 + ribbonNoise(world, seed + 4) * 38;
        const radius = 1.5 + ribbonNoise(world, seed + 5) * 3.8;
        ctx.beginPath(); ctx.arc(x, y + direction * offset, radius, 0, Math.PI * 2); ctx.fill();
      }
    }
  }
  function renderNavy(ctx, slide, project, index, height) {
    const cream = project.accent; const top = height * .17; const bottom = height * .81;
    ctx.fillStyle = '#3c415d'; ctx.fillRect(0, 0, 1080, height);
    if (slide.texture) { ribbon(ctx, top, index, 87); ribbon(ctx, bottom, index, 321); }
    else { ctx.fillStyle = '#fff'; ctx.fillRect(0, top - 24, 1080, 48); ctx.fillRect(0, bottom - 24, 1080, 48); }
    let reduced = false;
    const titleSize = slide.titleSize;
    const centered = !slide.body.trim() && !slide.eyebrow.trim();
    if (centered) {
      let actual = titleSize; let lines;
      do { ctx.font = '700 ' + actual + 'px "' + project.font + '"'; lines = wrap(ctx, slide.title, 830); if (lines.length * actual * 1.18 <= bottom - top - 190 || actual <= 18) break; actual -= 2; } while (true);
      reduced = actual < titleSize;
      ctx.fillStyle = cream; ctx.textBaseline = 'top';
      const y = (top + bottom - lines.length * actual * 1.18) / 2;
      lines.forEach((line, i) => ctx.fillText(line, (1080 - ctx.measureText(line).width) / 2, y + i * actual * 1.18));
    } else {
      const x = 88; const width = 900; let y = top + 164;
      if (slide.eyebrow) { const eyebrow = text(ctx, slide.eyebrow, x, y - 64, width, 22, cream, project.font, 700, 55); reduced ||= eyebrow.reduced; }
      const title = text(ctx, slide.title, x, y, width, titleSize, cream, project.font, 700, (bottom - y - 100) * .5);
      reduced ||= title.reduced; y += title.height + 78;
      const body = text(ctx, slide.body, x, y, width, 34, cream, project.font, 600, bottom - y - 85); reduced ||= body.reduced;
    }
    if (slide.swipe) {
      text(ctx, project.brand + (project.handle ? '  ' + project.handle : ''), 88, height - 95, 720, 20, cream, project.font, 600, 32);
      text(ctx, (index + 1) + ' / ' + project.slides.length + '  →', 880, height - 95, 150, 20, cream, project.font, 600, 32);
    }
    return { reduced };
  }
  const api = { layouts, referenceNames, wrap, render, ribbonNoise };
  if (typeof module !== 'undefined') module.exports = api;
  else root.FolioRender = api;
})(typeof window !== 'undefined' ? window : this);
