// Files dragged over the window, from any section: three zones (Content, Exam, Answer key) and the
// one under the cursor is what the files become. Rust keeps the dropped paths; the page only says
// which zone they fell on (materials_drop), so it never names a path.
import { h } from './dom.js';
import { icons } from './icons.js';
import { t } from '../core/i18n.js';
import { invoke, isTauri } from '../core/bridge.js';

/** The kind of the last drop, for the Materials panel that opens because of it. */
export const drops = { kind: null, examFor: () => null, onDrop: null };

const ZONES = () => [
  { kind: 'content', label: t('Content'), icon: icons.book },
  { kind: 'exam', label: t('Exams'), icon: icons.exam },
  { kind: 'key', label: t('Answer keys'), icon: icons.check },
];

/** Mounts the overlay; `openMaterials(kind)` shows Materials on that tab after a drop. */
export async function mountDropZones(root, openMaterials) {
  if (!isTauri) return;
  const zones = ZONES().map((z) => ({ ...z, el: h('div', { class: 'drop-zone', 'data-kind': z.kind }, h('span', { class: 'drop-zone-icon', html: z.icon, 'aria-hidden': 'true' }), h('span', {}, z.label)) }));
  const overlay = h('div', { class: 'drop-overlay', hidden: true, 'aria-hidden': 'true' }, h('p', { class: 'drop-title' }, t('Drop to attach as…')), h('div', { class: 'drop-zones' }, zones.map((z) => z.el)));
  root.append(overlay);

  // The position comes in physical pixels of the webview.
  const zoneAt = (p) => {
    const x = p.x / devicePixelRatio;
    const y = p.y / devicePixelRatio;
    return zones.find((z) => {
      const r = z.el.getBoundingClientRect();
      return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
    });
  };
  const light = (zone) => zones.forEach((z) => z.el.classList.toggle('is-over', z === zone));

  const { getCurrentWebview } = await import('@tauri-apps/api/webview');
  await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === 'enter' || payload.type === 'over') {
      overlay.hidden = false;
      light(zoneAt(payload.position));
    } else if (payload.type === 'drop') {
      const zone = zoneAt(payload.position);
      overlay.hidden = true;
      light(null);
      // Outside every zone (between them): nothing is attached and Rust's list is cleared.
      const kind = zone?.kind ?? null;
      if (kind) {
        drops.kind = kind;
        openMaterials(kind);
      }
      invoke('materials_drop', { attach: kind ? { kind, exam: kind === 'key' ? drops.examFor() : null } : null }).catch(() => {});
    } else {
      overlay.hidden = true;
      light(null);
    }
  });
}
