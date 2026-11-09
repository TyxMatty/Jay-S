// Materials › View: the material as it is, the way Jay-E shows it (PDF pages drawn in Rust and served
// one by one by the jays:// protocol, only those near the screen; DOCX as a page; photos), or the
// text Jay-S extracted. Rust builds the DOCX HTML with every text escaped and only http(s)/mailto
// links (Jay-E's writer), so it's the one place innerHTML takes something from a file.
import { h } from './dom.js';
import { invoke } from '../core/bridge.js';
import { t } from '../core/i18n.js';
import { failureText } from './materials.js';

/** Where the jays:// protocol answers in the WebView2 (Windows). */
const BASE = 'http://jays.localhost';

function pdfView(m, pages) {
  let zoom = 1;
  const holder = h('div', { class: 'pdf-pages' });
  const label = h('span', { class: 'hint pdf-zoom' });
  const sheets = pages.map(([w, hgt], i) => {
    const img = h('img', { alt: t('Page {n}', { n: i + 1 }), decoding: 'async' });
    const sheet = h('div', { class: 'pdf-page', style: `aspect-ratio:${w}/${hgt}`, 'data-index': String(i) }, img, h('span', { class: 'pdf-page-number' }, String(i + 1)));
    return sheet;
  });
  holder.append(...sheets);
  const width = () => {
    // Drawn at the shown width times the screen's density, in steps of 200 px (fewer redraws).
    const px = holder.clientWidth * zoom * (window.devicePixelRatio || 1);
    return Math.min(3000, Math.max(400, Math.ceil(px / 200) * 200));
  };
  // Only pages near the screen have a picture: the others give their memory back.
  const observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        const img = e.target.querySelector('img');
        if (e.isIntersecting) {
          const src = `${BASE}/pdf/${m.id}/${e.target.dataset.index}/${width()}`;
          if (img.getAttribute('src') !== src) img.src = src;
        } else {
          img.removeAttribute('src');
        }
      }
    },
    { rootMargin: '1200px 0px' },
  );
  sheets.forEach((s) => observer.observe(s));
  function setZoom(z) {
    zoom = Math.min(3, Math.max(0.5, z));
    holder.style.setProperty('--zoom', String(zoom));
    label.textContent = `${Math.round(zoom * 100)}%`;
    // The pictures near the screen are drawn again at the new size.
    for (const s of sheets) {
      const img = s.querySelector('img');
      if (img.getAttribute('src')) img.src = `${BASE}/pdf/${m.id}/${s.dataset.index}/${width()}`;
    }
  }
  setZoom(1);
  const bar = h(
    'div',
    { class: 'button-row viewer-bar' },
    h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Zoom out'), title: t('Zoom out'), onClick: () => setZoom(zoom - 0.25) }, '−'),
    label,
    h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Zoom in'), title: t('Zoom in'), onClick: () => setZoom(zoom + 0.25) }, '+'),
    h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => setZoom(1) }, t('Fit width')),
    h('span', { class: 'hint' }, pages.length === 1 ? t('1 page') : t('{n} pages', { n: pages.length })),
  );
  const el = h('div', { class: 'viewer-pdf' }, bar, holder);
  el.dispose = () => observer.disconnect();
  return el;
}

function docView(m, html) {
  const page = h('article', { class: 'doc-page' });
  page.innerHTML = html;
  // Images come from the protocol; links never navigate the app.
  for (const img of page.querySelectorAll('img')) {
    const src = img.getAttribute('src') ?? '';
    if (/^docimg\/[0-9a-f]{32}\/\d+$/.test(src)) img.src = `${BASE}/${src}`;
    else img.remove();
  }
  page.addEventListener('click', (e) => e.target.closest('a') && e.preventDefault());
  return page;
}

export function createViewer(m, onBack) {
  const body = h('div', { class: 'viewer-body' }, h('p', { class: 'hint' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Opening…')));
  let current = null;
  const show = (view) => {
    current?.dispose?.();
    current = view;
    body.replaceChildren(view);
  };
  const mode = h(
    'div',
    { class: 'segmented viewer-mode', role: 'radiogroup', 'aria-label': t('Show') },
    h('label', {}, h('input', { type: 'radio', name: 'viewer-mode', value: 'original', checked: true }), t('Original')),
    h('label', {}, h('input', { type: 'radio', name: 'viewer-mode', value: 'text' }), t('Extracted text')),
  );

  async function original() {
    try {
      const v = await invoke('materials_view', { id: m.id });
      if (v.kind === 'pdf') show(pdfView(m, v.pages));
      else if (v.kind === 'doc') show(docView(m, v.html));
      else if (v.kind === 'image') show(h('div', { class: 'viewer-image' }, h('img', { src: `${BASE}/file/${m.id}`, alt: m.name })));
      else await text();
    } catch (e) {
      show(h('p', { class: 'hint', 'data-tone': 'error' }, failureText(e)));
    }
  }

  async function text() {
    try {
      const body = await invoke('materials_text', { id: m.id });
      // Page breaks (form feeds) as a visible line.
      show(h('pre', { class: 'material-text viewer-text', tabindex: '0' }, body.replaceAll('\f', '\n\n――――――――\n\n') || t('No text was found.')));
    } catch (e) {
      show(h('p', { class: 'hint', 'data-tone': 'error' }, failureText(e)));
    }
  }

  mode.addEventListener('change', () => (mode.querySelector('input:checked').value === 'text' ? text() : original()));
  original();
  return h(
    'section',
    { class: 'app-panel is-single viewer' },
    h('div', { class: 'panel-head' }, h('h1', { class: 'viewer-title', title: m.name }, m.name), h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { current?.dispose?.(); onBack(); } }, t('Back'))),
    mode,
    body,
  );
}
