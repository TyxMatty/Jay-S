// Jay-AI › Chat: questions about materials, a finished exam, or anything. Rust picks the sources,
// checks every quote, number and key in the reply and marks what it couldn't confirm
// (crates/jay-study/src/chat.rs); this page only shows that, all as text.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, isTauri } from '../core/bridge.js';
import { t } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

function sentenceEl(s) {
  const chips = s.sources.map((n) => h('sup', { class: 'chat-chip', title: t('Source {n}', { n }) }, `[${n}]`));
  if (s.status === 'unconfirmed') {
    return h('p', { class: 'chat-sentence is-unconfirmed' }, h('span', { class: 'badge', 'data-status': 'check' }, t('Not confirmed')), ' ', s.text, ...chips);
  }
  return h('p', { class: 'chat-sentence' }, s.text, ...chips);
}

function replyEl(r) {
  const parts = [];
  if (r.kind === 'general') parts.push(h('p', { class: 'chat-banner', 'data-tone': 'warning' }, t("From Jay-AI's general knowledge, with no source: check it in a book or with your teacher.")));
  if (r.kind === 'unknown') parts.push(h('p', { class: 'chat-banner' }, r.sources.length ? t("Your sources don't answer this. The closest they get:") : t("Nothing in the sources you chose is about this.")));
  if (r.kind === 'material' && !r.sentences.some((s) => s.status === 'confirmed')) parts.push(h('p', { class: 'chat-banner', 'data-tone': 'warning' }, t("Jay-AI couldn't back this answer with your sources. Read them below.")));
  parts.push(...r.sentences.map(sentenceEl));
  if (r.sources.length) {
    parts.push(
      h(
        'ol',
        { class: 'chat-sources' },
        r.sources.map((s) =>
          h(
            'li',
            {},
            h('span', { class: 'chat-source-label' }, `[${s.n}] ${s.label}`, s.key ? h('span', { class: 'badge', 'data-status': 'ok' }, t('Key: {key}', { key: s.key })) : null),
            ...s.quotes.map((q) => h('blockquote', {}, q)),
          ),
        ),
      ),
    );
  }
  if (r.kind === 'material') {
    const done = [t('quotes found in the text'), t('numbers and keys checked')];
    if (r.checks.second_reading) done.push(t('second reading'));
    parts.push(h('p', { class: 'hint chat-checks' }, h('span', { class: 'btn-icon', html: icons.check, 'aria-hidden': 'true' }), t('Checked: {what}.', { what: done.join(' · ') }), r.checks.retried ? ` ${t('(answered again after a failed check)')}` : ''));
  }
  return h('div', { class: 'chat-reply' }, parts);
}

export function createChatPanel() {
  const message = h('p', { class: 'hint', 'data-tone': 'error', role: 'alert', hidden: true });
  const fail = (e) => {
    message.textContent = studyFailureText(e);
    message.hidden = false;
  };
  const picker = h('select', { class: 'input', 'aria-label': t('Conversation') });
  const removeBtn = h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete this conversation'), title: t('Delete this conversation'), html: icons.trash });
  const newBtn = h('button', { type: 'button', class: 'btn btn-secondary' }, t('New conversation'));
  const body = h('div', { class: 'chat-body' });
  let chat = null;

  async function refresh(select) {
    const list = await invoke('chats_list').catch(() => []);
    picker.replaceChildren(...list.map((c) => h('option', { value: c.id, selected: c.id === select }, c.title || t('New conversation'))));
    picker.hidden = removeBtn.hidden = list.length === 0;
    return list;
  }

  async function open(id) {
    message.hidden = true;
    try {
      chat = await invoke('chat_get', { id });
      body.replaceChildren(conversation());
    } catch (e) {
      fail(e);
    }
  }

  /** Choosing what the conversation is about. */
  async function starter() {
    chat = null;
    const [materials, attempts] = await Promise.all([invoke('materials_list').catch(() => []), invoke('attempts_list').catch(() => [])]);
    const finished = attempts.filter((a) => a.finished);
    const radio = (value, label, checked) => h('label', {}, h('input', { type: 'radio', name: 'chat-about', value, checked }), label);
    const about = h('div', { class: 'segmented', role: 'radiogroup', 'aria-label': t('About') }, radio('general', t('Any question'), true), radio('materials', t('My materials')), radio('attempt', t('An exam I took')));
    const boxes = materials.filter((m) => m.kind !== 'key').map((m) => h('label', { class: 'check-row' }, h('input', { type: 'checkbox', value: m.id }), h('span', {}, m.name)));
    const materialsField = h('div', { class: 'chat-pick', hidden: true }, boxes.length ? boxes : h('p', { class: 'hint' }, t('Attach materials first (Materials section).')));
    const attempt = h('select', { class: 'input', 'aria-label': t('Exam') }, finished.map((a) => h('option', { value: a.id }, a.title)));
    const attemptField = h('div', { class: 'chat-pick', hidden: true }, finished.length ? attempt : h('p', { class: 'hint' }, t('Finish an exam or mock test first.')));
    const note = h('p', { class: 'hint' });
    const notes = {
      general: t("Jay-AI answers from what it knows and says so: no source to check it against."),
      materials: t('Jay-AI answers only from these materials, quoting them; what it can\'t back is marked.'),
      attempt: t('Ask about any question ("why is 12 wrong?"): Jay-AI sees the official key, your marks and your result.'),
    };
    const kind = () => about.querySelector('input:checked').value;
    const sync = () => {
      materialsField.hidden = kind() !== 'materials';
      attemptField.hidden = kind() !== 'attempt';
      note.textContent = notes[kind()];
    };
    about.addEventListener('change', sync);
    sync();
    const start = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.spark }), t('Start'));
    start.addEventListener('click', async () => {
      const scope = { materials: [], attempt: null };
      if (kind() === 'materials') {
        scope.materials = boxes.map((b) => b.querySelector('input')).filter((i) => i.checked).map((i) => i.value);
        if (!scope.materials.length) return fail({ code: 'empty' });
      } else if (kind() === 'attempt') {
        if (!attempt.value) return fail({ code: 'empty' });
        scope.attempt = attempt.value;
      }
      message.hidden = true;
      try {
        const c = await invoke('chat_new', { scope });
        await refresh(c.id);
        await open(c.id);
      } catch (e) {
        fail(e);
      }
    });
    body.replaceChildren(h('fieldset', {}, cardTitle(icons.spark, t('What do you want to talk about?')), about, materialsField, attemptField, note, h('div', { class: 'button-row' }, start)));
  }

  function conversation() {
    const log = h('div', { class: 'chat-log', 'aria-live': 'polite' });
    const turn = (q, reply) => h('div', { class: 'chat-turn' }, h('p', { class: 'chat-question' }, q), reply);
    for (const x of chat.turns) log.append(turn(x.question, replyEl(x.reply)));
    const input = h('textarea', { class: 'input chat-input', rows: 2, maxlength: 800, id: 'chat-input', placeholder: t('Ask Jay-AI…') });
    const send = h('button', { type: 'button', class: 'btn' }, t('Send'));
    const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));
    async function ask() {
      const q = input.value.trim();
      if (!q || send.disabled) return;
      send.disabled = true;
      stop.hidden = false;
      input.value = '';
      empty?.remove();
      const waiting = h('p', { class: 'hint' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Jay-AI is reading your sources and checking the answer…'));
      const item = turn(q, waiting);
      log.append(item);
      item.scrollIntoView({ block: 'nearest' });
      try {
        const x = await invoke('chat_ask', { id: chat.id, question: q });
        chat.turns.push(x);
        waiting.replaceWith(replyEl(x.reply));
        if (chat.turns.length === 1) refresh(chat.id);
      } catch (e) {
        waiting.replaceWith(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e)));
        input.value = q;
      }
      send.disabled = false;
      stop.hidden = true;
      input.focus();
    }
    send.addEventListener('click', ask);
    // Enter sends; Shift+Enter is a new line.
    input.addEventListener('keydown', (e) => e.key === 'Enter' && !e.shiftKey && !e.isComposing && (e.preventDefault(), ask()));
    const empty = chat.turns.length ? null : h('p', { class: 'hint' }, t('Ask your first question below.'));
    return h('div', { class: 'chat' }, chat.title ? h('h2', { class: 'chat-title' }, chat.title) : null, empty, log, h('label', { for: 'chat-input', class: 'visually-hidden' }, t('Your question')), input, h('div', { class: 'button-row' }, send, stop));
  }

  picker.addEventListener('change', () => open(picker.value));
  newBtn.addEventListener('click', starter);
  removeBtn.addEventListener('click', async () => {
    if (!chat || !confirm(t('Delete this conversation?'))) return;
    try {
      await invoke('chat_delete', { id: chat.id });
      const list = await refresh(null);
      if (list.length) open(list[0].id);
      else starter();
    } catch (e) {
      fail(e);
    }
  });

  refresh(null).then((list) => (list.length ? open(list[0].id) : starter()));
  return h('div', { class: 'chat-panel' }, h('div', { class: 'button-row chat-bar' }, picker, removeBtn, newBtn), message, body);
}
