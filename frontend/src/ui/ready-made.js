// The "Ready-made exams" tab of Exams: the catalogue of official exams (crates/jay-study/src/catalog.rs).
// The page only sends an entry's `id`; the address, the checks and the answer key live in Rust.
// A download is the one study job (kind "catalog"); its events carry no entry id, so the row that
// started it is remembered here.
import { h } from './dom.js';
import { icons } from './icons.js';
import { invoke, listenJobs, isTauri } from '../core/bridge.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

const mb = (bytes) => new Intl.NumberFormat(language(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(bytes / 1e6);
const failureText = (e) => (e?.code === 'cancelled' ? t('Stopped. Downloading again continues where it left off.') : studyFailureText(e));

function meta(item) {
  return [
    t('{n} questions', { n: item.questions }),
    item.style === 'uepg' ? t('sum') : item.style.toUpperCase(),
    item.official_key ? t('official answer key') : null,
    t('{size} MB', { size: mb(item.size) }),
  ].filter(Boolean).join(' · ');
}

/** `onOpen(examId)` starts the exam; `onChange()` runs when a download added an exam. */
export function createReadyMade({ onOpen, onChange }) {
  const filters = h('div', { class: 'ready-made-filters' });
  const list = h('ul', { class: 'material-list' });
  const loadError = h('p', { class: 'hint', 'data-tone': 'error', role: 'alert' });
  let items = [];
  let family = null;
  let year = null;
  /** The entry being downloaded, its percentage's node, and whether any download is running. */
  let active = null;
  let percentNode = null;
  let running = false;
  /** entry id → what went wrong, in words. */
  const errors = new Map();

  const percent = (done, total) => t('Downloading… {n}%', { n: total ? Math.min(100, Math.floor((done / total) * 100)) : 0 });

  function row(item) {
    const busy = active === item.id;
    let actions;
    if (item.state === 'ready') {
      actions = [h('span', { class: 'badge', 'data-status': 'ok' }, t('Ready')), h('button', { type: 'button', class: 'btn', onClick: () => onOpen(item.exam) }, t('Take the exam'))];
    } else if (busy) {
      percentNode = h('span', { class: 'hint', role: 'status', 'aria-live': 'polite' }, percent(0, 0));
      actions = [percentNode, h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => invoke('job_cancel') }, t('Stop'))];
    } else {
      actions = [h('button', { type: 'button', class: 'btn', disabled: running, onClick: () => get(item.id) }, h('span', { class: 'btn-icon', html: icons.download }), t('Download'))];
    }
    return h('li', { class: 'material-card' }, h('div', { class: 'material-row' },
      h('span', { class: 'card-icon', html: icons.exam, 'aria-hidden': 'true' }),
      h('div', { class: 'material-main' },
        h('h3', { class: 'material-name' }, item.title),
        h('p', { class: 'material-meta' }, meta(item)),
        errors.has(item.id) ? h('p', { class: 'hint', 'data-tone': 'error', role: 'alert' }, errors.get(item.id)) : null,
      ),
      h('div', { class: 'material-actions' }, actions),
    ));
  }

  function render() {
    const families = [...new Set(items.map((i) => i.family))];
    if (!families.includes(family)) family = families[0] ?? null;
    const years = [...new Set(items.filter((i) => i.family === family).map((i) => i.year))].sort((a, b) => b - a);
    if (!years.includes(year)) year = years[0] ?? null;
    const select = h('select', { class: 'input ready-made-year', 'aria-label': t('Year'), onChange: (e) => ((year = Number(e.target.value)), render()) }, years.map((y) => h('option', { value: y, selected: y === year }, String(y))));
    filters.replaceChildren(
      ...families.map((f) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(f === family), onClick: () => ((family = f), render()) }, f)),
      select,
    );
    percentNode = null;
    const shown = items.filter((i) => i.family === family && i.year === year);
    filters.hidden = !items.length;
    list.replaceChildren(...(shown.length ? shown.map(row) : loadError.textContent ? [] : [h('li', { class: 'empty-state' }, h('p', {}, t('No ready-made exam for this choice.')))]));
  }

  async function load() {
    try {
      items = await invoke('catalog_list');
      loadError.textContent = '';
    } catch {
      items = [];
      loadError.textContent = t("Couldn't load the ready-made exams.");
    }
    render();
  }

  async function get(id) {
    errors.delete(id);
    active = id;
    running = true;
    render();
    try {
      await invoke('catalog_get', { id });
    } catch (e) {
      // Busy (another job) or anything else that stopped it before it began.
      active = null;
      running = false;
      errors.set(id, failureText(e));
      render();
    }
  }

  // shortcut: a download that began before this screen was rebuilt shows no row (its events carry no
  // entry id) and only disables "Download"; upgrade if the job events learn to name the entry.
  const stopListening = listenJobs(async (e) => {
    if (e.kind !== 'catalog') return;
    if (e.state === 'progress') {
      if (!running) ((running = true), render());
      if (e.step.step === 'downloading' && percentNode) percentNode.textContent = percent(e.step.done, e.step.total);
      return;
    }
    if (e.state === 'error' && active) errors.set(active, failureText(e.error));
    active = null;
    running = false;
    await load();
    if (e.state === 'done') onChange();
  });

  const el = h('div', { class: 'ready-made' });
  if (isTauri) {
    el.append(h('p', { class: 'hint' }, t("The exam is downloaded from the university's website when you choose it; nothing of yours is sent.")), filters, loadError, list);
    load();
  } else {
    el.append(h('p', { class: 'hint' }, t('Only in the app.')));
  }
  el.dispose = () => stopListening.then((un) => un());
  return el;
}
