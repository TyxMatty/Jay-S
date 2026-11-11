// "See in the original exam": the pages of the PDF a question lives on (Rust found them when it split
// the exam), drawn by the jays:// protocol only when the student opens the panel. The page sends
// nothing but the material id and a page number; Rust checks both (view.rs).
import { h } from './dom.js';
import { t } from '../core/i18n.js';
import { BASE } from './viewer.js';

/** Most pages shown at once: the ones closest to the question (the last ones of its range). */
const MAX_PAGES = 3;
/** Drawn this wide, in px: sharp on a wide window, and a phone only scales it down. */
const WIDTH = 1000;

let counter = 0;

/** `original` is `{ material, first, last }` (0-based pages) or null; returns the control or null. */
export function originalPages(original) {
  if (!original) return null;
  const last = original.last;
  const first = Math.max(original.first, last - (MAX_PAGES - 1));
  const id = `original-pages-${++counter}`;
  const panel = h('div', { class: 'original-pages', id, hidden: true });
  const button = h('button', {
    type: 'button',
    class: 'btn btn-secondary',
    'aria-expanded': 'false',
    'aria-controls': id,
    onClick: () => {
      const open = panel.hidden;
      panel.hidden = !open;
      button.setAttribute('aria-expanded', String(open));
      button.textContent = open ? t('Hide the original exam') : t('See in the original exam');
      if (open && !panel.firstChild) {
        for (let p = first; p <= last; p++) {
          panel.append(h('img', { src: `${BASE}/pdf/${original.material}/${p}/${WIDTH}`, alt: t('Page {n} of the original exam', { n: p + 1 }), loading: 'lazy', decoding: 'async' }));
        }
      }
    },
  }, t('See in the original exam'));
  return h('div', { class: 'original' }, button, panel);
}
