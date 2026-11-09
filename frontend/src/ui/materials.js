// Materials: three tabs (Content, Exams, Answer keys); a file attached in a tab gets its kind, an
// exam picks its answer key (or attaches one), View shows the material itself, Explain asks Jay-AI. Rust reads the files (src-tauri/src/library.rs); this page only
// shows ids, names and text, always as text (never innerHTML: the text comes from untrusted files).
import { h } from './dom.js';
import { icons } from './icons.js';
import { createTabs } from './tabs.js';
import { invoke, listen, onDragOver, isTauri } from '../core/bridge.js';
import { t, language } from '../core/i18n.js';

const KINDS = () => [
  ['content', t('Content')],
  ['exam', t('Exam')],
  ['both', t('Content and exam')],
  ['key', t('Answer key')],
];

const ICON = { content: icons.book, exam: icons.exam, both: icons.cards, key: icons.check };

/** A failure from Rust (`{ code, detail }`) in the student's language. */
export function failureText(error) {
  const detail = error?.detail ?? '';
  switch (error?.code) {
    case 'too_big':
      return t('It is bigger than 200 MB.');
    case 'unsupported':
      return t("It isn't a PDF, DOCX, text file or photo.");
    case 'invalid':
      // The detail is Rust's, in English: only the password case is worth telling apart.
      return /password/i.test(detail) ? t('The PDF has a password: save a copy without it and attach that.') : t('The file is damaged or incomplete.');
    case 'not_found':
      return t("It isn't in Jay-S anymore.");
    case 'web':
      return t('Only in the app.');
    default:
      return t('Windows couldn\'t read it ({detail}).', { detail: detail || String(error) });
  }
}

export function warningText(w) {
  switch (w.code) {
    case 'page_unreadable':
      return t("Page {page} couldn't be read.", { page: w.page });
    case 'pages_cut':
      return t('Only the first {n} pages were read.', { n: w.kept });
    case 'ocr_unavailable':
      return t('Scanned pages need a Windows language with text recognition (Settings › Time & language › Language).');
    case 'text_cut':
      return t('Only the beginning was kept: the file has too much text.');
    case 'no_text_in_photo':
      return t('No text was found in this photo.');
    default:
      return w.code;
  }
}

const compact = (n) => new Intl.NumberFormat(language(), { notation: 'compact' }).format(n);

function metaLine(m) {
  const parts = [m.pages === 1 ? t('1 page') : t('{n} pages', { n: m.pages }), t('{n} letters', { n: compact(m.chars) })];
  const questions = m.detected.choice_questions + m.detected.sum_questions;
  if (questions) parts.push(questions === 1 ? t('1 question') : t('{n} questions', { n: questions }));
  if (m.ocr_pages) parts.push(t('{n} read from images', { n: m.ocr_pages }));
  return parts.join(' · ');
}

/** The tabs of Materials, each with the kinds it lists and the kind a file attached there gets. */
const TABS = () => [
  { key: 'content', label: t('Content'), kinds: ['content', 'both'], attach: t('Attach content'), icon: icons.book, empty: t('Your study content: handouts, notes, book chapters, photos of the board.') },
  { key: 'exam', label: t('Exams'), kinds: ['exam', 'both'], attach: t('Attach exams'), icon: icons.exam, empty: t('Past exams to reuse and practice: ENEM, UEPG, UEM, your school\'s.') },
  { key: 'key', label: t('Answer keys'), kinds: ['key'], attach: t('Attach answer keys'), icon: icons.check, empty: t('The answer keys of your exams. Attach one here and pick its exam, or from the exam itself.') },
];

function materialCard(m, { onDeleted, onError, onExplain, onView, onChanged, keys }) {
  const select = h(
    'select',
    { class: 'input material-kind', 'aria-label': t('What it is') },
    KINDS().map(([value, label]) => h('option', { value, selected: m.kind === value }, label)),
  );
  const icon = h('span', { class: 'card-icon', html: ICON[m.kind], 'aria-hidden': 'true' });
  const check = h('span', { class: 'badge', 'data-status': 'check', hidden: m.detected.sure || m.kind !== m.detected.kind }, t('Check this'));
  const isExam = m.kind === 'exam' || m.kind === 'both';
  const card = h(
    'li',
    { class: 'material-card', 'data-kind': m.kind },
    h(
      'div',
      { class: 'material-row' },
      icon,
      h(
        'div',
        { class: 'material-main' },
        h('h3', { class: 'material-name', title: m.name }, m.name),
        h('p', { class: 'material-meta' }, metaLine(m)),
        m.warnings.map((w) => h('p', { class: 'hint', 'data-tone': 'warning' }, warningText(w))),
      ),
      check,
      select,
      h(
        'div',
        { class: 'material-actions' },
        h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => onView(m) }, t('View')),
        m.kind === 'key' ? null : h('button', { type: 'button', class: 'btn', onClick: () => onExplain(m) }, h('span', { class: 'btn-icon', html: icons.spark }), t('Explain')),
        h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete {name}', { name: m.name }), title: t('Delete'), html: icons.trash, onClick: remove }),
      ),
    ),
    isExam ? keyRow() : null,
  );

  /** An exam's answer key: automatic, none, one of the keys, or a file attached right here. */
  function keyRow() {
    const pick = h('select', { class: 'input', id: `key-${m.id}` });
    const note = h('span', { class: 'hint' });
    const attachKey = h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => invoke('materials_pick', { attach: { kind: 'key', exam: m.id } }).catch((e) => onError(failureText(e))) }, h('span', { class: 'btn-icon', html: icons.attach }), t('Attach its answer key…'));
    async function load() {
      try {
        const info = await invoke('materials_key', { id: m.id });
        const found = keys().find((k) => k.id === info.key)?.name;
        pick.replaceChildren(
          h('option', { value: 'auto', selected: info.choice.mode === 'auto' }, found && info.choice.mode === 'auto' ? t('Automatic: {name}', { name: found }) : t('Automatic')),
          h('option', { value: 'none', selected: info.choice.mode === 'none' }, t('None: Jay-AI answers')),
          ...keys().map((k) => h('option', { value: k.id, selected: info.choice.mode === 'material' && info.choice.id === k.id }, k.name)),
        );
        note.textContent = info.key ? '' : t('Without a key, Jay-AI answers the questions you reuse (marked as such).');
      } catch (e) {
        note.textContent = failureText(e);
      }
    }
    pick.addEventListener('change', async () => {
      const key = pick.value === 'auto' ? { mode: 'auto' } : pick.value === 'none' ? { mode: 'none' } : { mode: 'material', id: pick.value };
      try {
        await invoke('materials_set_key', { id: m.id, key });
        load();
      } catch (e) {
        onError(failureText(e));
      }
    });
    load();
    const row = h('div', { class: 'material-key' }, h('label', { for: `key-${m.id}`, class: 'field-label' }, t('Answer key')), pick, attachKey, note);
    return row;
  }

  select.addEventListener('change', async () => {
    const previous = m.kind;
    try {
      Object.assign(m, await invoke('materials_set_kind', { id: m.id, kind: select.value }));
      onChanged();
    } catch (e) {
      select.value = previous;
      onError(failureText(e));
    }
  });

  async function remove() {
    if (!confirm(t('Delete "{name}" from Jay-S? Your own file stays where it is.', { name: m.name }))) return;
    try {
      await invoke('materials_delete', { id: m.id });
      onDeleted(m);
    } catch (e) {
      onError(failureText(e));
    }
  }

  return card;
}

export function createMaterialsPanel() {
  const message = h('p', { class: 'hint', 'data-tone': 'error', role: 'alert', hidden: true });
  const note = h('p', { class: 'hint', role: 'status' });
  const dropHint = h('div', { class: 'drop-hint', hidden: true, 'aria-hidden': 'true' }, t('Drop to attach'));
  const attach = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.attach }), t('Attach files'));
  // In Answer keys: the exam the key being attached belongs to.
  const forExam = h('select', { class: 'input for-exam', 'aria-label': t('For the exam') });
  const forExamField = h('label', { class: 'for-exam-field', hidden: true }, h('span', { class: 'field-label' }, t('For the exam')), forExam);
  /** Every material, as listed. */
  let all = [];
  /** token → the card shown while that file is read (in the tab it was attached from). */
  const reading = new Map();

  const showError = (text) => {
    message.textContent = text;
    message.hidden = false;
  };
  const keys = () => all.filter((m) => m.kind === 'key');

  const panes = TABS().map((tab) => {
    const list = h('ul', { class: 'material-list', 'aria-live': 'polite' });
    const empty = h('div', { class: 'empty-state' }, h('span', { class: 'card-icon', html: tab.icon, 'aria-hidden': 'true' }), h('p', {}, tab.empty), h('p', { class: 'hint' }, isTauri ? t('Or drop the files on the window.') : t('Only in the app.')));
    const panel = h('div', { class: 'material-pane' }, empty, list);
    return { ...tab, list, empty, panel };
  });

  const handlers = {
    onDeleted: (m) => {
      all = all.filter((x) => x.id !== m.id);
      render();
    },
    onError: showError,
    onChanged: () => render(),
    keys,
    // The explanation and the viewer take the section's place; Back puts the same list back.
    onExplain: async (m) => {
      const { createExplainView } = await import('./explain.js');
      host.replaceChildren(createExplainView(m, () => host.replaceChildren(section)));
    },
    onView: async (m) => {
      const { createViewer } = await import('./viewer.js');
      host.replaceChildren(createViewer(m, () => host.replaceChildren(section)));
    },
  };

  function render() {
    for (const pane of panes) {
      const mine = all.filter((m) => pane.kinds.includes(m.kind));
      const pending = [...pane.list.querySelectorAll('.is-reading, .is-error')];
      pane.list.replaceChildren(...pending, ...mine.map((m) => materialCard(m, handlers)));
      pane.empty.hidden = pane.list.children.length > 0;
      tabLabels[pane.key].textContent = mine.length ? String(mine.length) : '';
    }
    forExam.replaceChildren(h('option', { value: '' }, t('Find it by itself')), ...all.filter((m) => m.kind === 'exam' || m.kind === 'both').map((m) => h('option', { value: m.id }, m.name)));
    tellRust();
  }

  const tabLabels = {};
  const tabs = createTabs({
    idPrefix: 'materials',
    label: t('Kinds of material'),
    className: 'material-tabs',
    items: panes.map((p) => {
      tabLabels[p.key] = h('span', { class: 'tab-count' });
      return { key: p.key, label: h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: p.icon }), p.label, tabLabels[p.key]), panel: p.panel };
    }),
    onSelect: (key) => {
      const tab = panes.find((p) => p.key === key);
      attach.lastChild.textContent = tab.attach;
      forExamField.hidden = key !== 'key';
      note.textContent = '';
      tellRust();
    },
  });

  /** What a file attached now becomes: the tab's kind (and, for a key, its exam). */
  function attachChoice() {
    const kind = tabs.current ?? 'content';
    return { kind, exam: kind === 'key' ? forExam.value || null : null };
  }
  // Files dropped on the window go where the student is looking.
  const tellRust = () => isTauri && invoke('materials_drop_target', { attach: attachChoice() }).catch(() => {});
  forExam.addEventListener('change', tellRust);

  attach.addEventListener('click', () => {
    message.hidden = true;
    invoke('materials_pick', { attach: attachChoice() }).catch((e) => showError(failureText(e)));
  });

  const current = () => panes.find((p) => p.key === tabs.current) ?? panes[0];

  const listening = listen('materials-import', (e) => {
    if (e.state === 'reading') {
      const card = h('li', { class: 'material-card is-reading' }, h('div', { class: 'material-row' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), h('p', { class: 'material-main' }, t('Reading {name}…', { name: e.name }))));
      reading.set(e.token, card);
      current().list.prepend(card);
      current().empty.hidden = true;
      return;
    }
    const card = reading.get(e.token);
    reading.delete(e.token);
    if (e.state === 'done') {
      card?.remove();
      all = [e.material, ...all.filter((m) => m.id !== e.material.id)];
      render();
      // Attached from another tab's kind (a key from an exam's card): say where it went.
      const home = panes.find((p) => p.kinds.includes(e.material.kind));
      if (home && home.key !== tabs.current) note.textContent = t('"{name}" is in {tab}.', { name: e.material.name, tab: home.label });
    } else {
      const error = h(
        'li',
        { class: 'material-card is-error' },
        h(
          'div',
          { class: 'material-row' },
          h('p', { class: 'material-main' }, t("Couldn't read {name}: {reason}", { name: e.name, reason: failureText(e.error) })),
          h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Close'), title: t('Close'), html: icons.close, onClick: (ev) => (ev.currentTarget.closest('li').remove(), render()) }),
        ),
      );
      if (card) card.replaceWith(error);
      else current().list.prepend(error);
    }
  });
  onDragOver((over) => (dropHint.hidden = !over));

  if (isTauri) {
    // After the listener: listing also attaches the files Jay-S was opened with.
    listening
      .then(() => invoke('materials_list'))
      .then((materials) => {
        all = materials;
        render();
      })
      .catch((e) => showError(failureText(e)));
  }

  tabs.select('content');
  const section = h(
    'section',
    { class: 'app-panel is-single materials-panel' },
    h('div', { class: 'panel-head' }, h('h1', {}, t('Materials')), h('div', { class: 'attach-group' }, forExamField, attach)),
    tabs.list,
    message,
    note,
    ...panes.map((p) => p.panel),
    dropHint,
  );
  const host = h('div', { class: 'lazy-panel' }, section);
  return host;
}
