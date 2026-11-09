// Materials (stage 1): attach content and exams, see what Jay-S read and what it thinks each one is,
// change it, read the text, delete. Rust reads the files (src-tauri/src/library.rs); this page only
// shows ids, names and text, always as text (never innerHTML: the text comes from untrusted files).
import { h } from './dom.js';
import { icons } from './icons.js';
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

function materialCard(m, { onDeleted, onError, onExplain }) {
  const select = h(
    'select',
    { class: 'input material-kind', 'aria-label': t('What it is') },
    KINDS().map(([value, label]) => h('option', { value, selected: m.kind === value }, label)),
  );
  const icon = h('span', { class: 'card-icon', html: ICON[m.kind], 'aria-hidden': 'true' });
  const check = h('span', { class: 'badge', 'data-status': 'check', hidden: m.detected.sure || m.kind !== m.detected.kind }, t('Check this'));
  const text = h('pre', { class: 'material-text', hidden: true, tabindex: '0' });
  const textButton = h('button', { type: 'button', class: 'btn btn-secondary', 'aria-expanded': 'false' }, t('Show text'));
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
        m.kind === 'key' ? null : h('button', { type: 'button', class: 'btn', onClick: () => onExplain(m) }, h('span', { class: 'btn-icon', html: icons.spark }), t('Explain')),
        textButton,
        h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete {name}', { name: m.name }), title: t('Delete'), html: icons.trash, onClick: remove }),
      ),
    ),
    text,
  );

  select.addEventListener('change', async () => {
    const previous = m.kind;
    try {
      Object.assign(m, await invoke('materials_set_kind', { id: m.id, kind: select.value }));
      card.dataset.kind = m.kind;
      icon.innerHTML = ICON[m.kind];
      check.hidden = true;
    } catch (e) {
      select.value = previous;
      onError(failureText(e));
    }
  });

  textButton.addEventListener('click', async () => {
    const open = text.hidden;
    if (open && !text.textContent) {
      try {
        const body = await invoke('materials_text', { id: m.id });
        // Page breaks (form feeds) as a visible line.
        text.textContent = body.replaceAll('\f', '\n\n――――――――\n\n') || t('No text was found.');
      } catch (e) {
        return onError(failureText(e));
      }
    }
    text.hidden = !open;
    textButton.setAttribute('aria-expanded', String(open));
    textButton.textContent = open ? t('Hide text') : t('Show text');
  });

  async function remove() {
    if (!confirm(t('Delete "{name}" from Jay-S? Your own file stays where it is.', { name: m.name }))) return;
    try {
      await invoke('materials_delete', { id: m.id });
      card.remove();
      onDeleted();
    } catch (e) {
      onError(failureText(e));
    }
  }

  return card;
}

export function createMaterialsPanel() {
  const list = h('ul', { class: 'material-list', 'aria-live': 'polite' });
  const message = h('p', { class: 'hint', 'data-tone': 'error', role: 'alert', hidden: true });
  const empty = h(
    'div',
    { class: 'empty-state' },
    h('span', { class: 'card-icon', html: icons.attach, 'aria-hidden': 'true' }),
    h('p', {}, t('Attach your content and your exams (PDF, DOCX, text or photos). Jay-S tells which is which.')),
    h('p', { class: 'hint' }, isTauri ? t('Or drop the files on the window.') : t('Only in the app.')),
  );
  const dropHint = h('div', { class: 'drop-hint', hidden: true, 'aria-hidden': 'true' }, t('Drop to attach'));
  const attach = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.attach }), t('Attach files'));

  const showError = (text) => {
    message.textContent = text;
    message.hidden = false;
  };
  const refreshEmpty = () => (empty.hidden = list.children.length > 0);
  const handlers = {
    onDeleted: refreshEmpty,
    onError: showError,
    // The explanation takes the section's place; Back puts the same list back (its state kept).
    onExplain: async (m) => {
      const { createExplainView } = await import('./explain.js');
      host.replaceChildren(createExplainView(m, () => host.replaceChildren(section)));
    },
  };
  /** token → the card shown while that file is read. */
  const reading = new Map();

  attach.addEventListener('click', () => {
    message.hidden = true;
    invoke('materials_pick').catch((e) => showError(failureText(e)));
  });

  const listening = listen('materials-import', (e) => {
    if (e.state === 'reading') {
      const card = h('li', { class: 'material-card is-reading' }, h('div', { class: 'material-row' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), h('p', { class: 'material-main' }, t('Reading {name}…', { name: e.name }))));
      reading.set(e.token, card);
      list.prepend(card);
    } else {
      const card = reading.get(e.token);
      reading.delete(e.token);
      const next =
        e.state === 'done'
          ? materialCard(e.material, handlers)
          : h(
              'li',
              { class: 'material-card is-error' },
              h(
                'div',
                { class: 'material-row' },
                h('p', { class: 'material-main' }, t("Couldn't read {name}: {reason}", { name: e.name, reason: failureText(e.error) })),
                h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Close'), title: t('Close'), html: icons.close, onClick: (ev) => (ev.currentTarget.closest('li').remove(), refreshEmpty()) }),
              ),
            );
      if (card) card.replaceWith(next);
      else list.prepend(next);
    }
    refreshEmpty();
  });
  onDragOver((over) => (dropHint.hidden = !over));

  if (isTauri) {
    // After the listener: listing also attaches the files Jay-S was opened with.
    listening
      .then(() => invoke('materials_list'))
      .then((materials) => {
        list.append(...materials.map((m) => materialCard(m, handlers)));
        refreshEmpty();
      })
      .catch((e) => showError(failureText(e)));
  }

  const section = h(
    'section',
    { class: 'app-panel is-single materials-panel' },
    h('div', { class: 'panel-head' }, h('h1', {}, t('Materials')), attach),
    message,
    empty,
    list,
    dropHint,
  );
  const host = h('div', { class: 'lazy-panel' }, section);
  return host;
}
