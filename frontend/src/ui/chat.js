// Jay-AI › Chat: questions about materials, a finished exam, or anything. Rust picks the sources,
// checks every quote, number and key in the reply and marks what it couldn't confirm
// (crates/jay-study/src/chat.rs); this page only shows that, all as text.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, isTauri } from '../core/bridge.js';
import { t } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

/** The answer as one paragraph; each run of unconfirmed sentences starts with its label (never color alone). */
function answerEl(sentences, showSource) {
  const p = h('p', { class: 'chat-answer' });
  let run = null;
  for (const s of sentences) {
    const chips = s.sources.map((n) => h('button', { type: 'button', class: 'chat-chip', 'aria-label': t('Source {n}', { n }), title: t('Source {n}', { n }), onClick: () => showSource(n) }, String(n)));
    if (s.status !== 'unconfirmed') {
      run = null;
      p.append(h('span', {}, s.text, ...chips), ' ');
      continue;
    }
    if (!run) {
      run = h('span', { class: 'chat-unconfirmed' });
      p.append(h('span', { class: 'badge', 'data-status': 'check' }, t('Not confirmed')), ' ', run, ' ');
    }
    run.append(s.text, ...chips, ' ');
  }
  return p;
}

function replyEl(r) {
  const confirmed = r.sentences.some((s) => s.status === 'confirmed');
  const badge = {
    material: confirmed ? ['ok', t('From your sources')] : ['check', t('Not confirmed')],
    general: ['check', t('General knowledge, no source')],
    unknown: [null, t('Not in your sources')],
  }[r.kind];
  const head = h('div', { class: 'chat-reply-head' }, h('span', { class: 'chat-avatar', html: icons.spark, 'aria-hidden': 'true' }), h('strong', {}, 'Jay-AI'), badge ? h('span', { class: 'badge', 'data-status': badge[0] }, badge[1]) : null);
  const parts = [head];
  if (r.kind === 'unknown') parts.push(h('p', { class: 'chat-banner' }, r.sources.length ? t("Your sources don't answer this. The closest they get:") : t("Nothing in the sources you chose is about this.")));
  if (r.kind === 'material' && !confirmed) parts.push(h('p', { class: 'chat-banner', 'data-tone': 'warning' }, t("Jay-AI couldn't back this answer with your sources. Read them below.")));
  let sources = null;
  // A source chip opens the sources and takes the reader to its quote.
  const showSource = (n) => {
    if (!sources) return;
    sources.open = true;
    sources.querySelector(`[data-n="${n}"]`)?.focus();
  };
  if (r.sentences.length) parts.push(answerEl(r.sentences, showSource));
  if (r.sources.length) {
    const done = [t('quotes found in the text'), t('numbers and keys checked')];
    if (r.checks.second_reading) done.push(t('second reading'));
    sources = h(
      'details',
      // Open when the reader has to judge for themselves; one click away when all is confirmed.
      { class: 'chat-sources', open: r.kind !== 'material' || r.sentences.some((s) => s.status === 'unconfirmed') },
      h('summary', {}, t('Sources ({n})', { n: r.sources.length })),
      h(
        'ol',
        {},
        r.sources.map((s) =>
          h(
            'li',
            { 'data-n': s.n, tabindex: -1 },
            h('span', { class: 'chat-source-label' }, h('span', { class: 'chat-chip', 'aria-hidden': 'true' }, String(s.n)), s.label, s.key ? h('span', { class: 'badge', 'data-status': 'ok' }, t('Key: {key}', { key: s.key })) : null),
            ...s.quotes.map((q) => h('blockquote', {}, q)),
          ),
        ),
      ),
      r.kind === 'material' ? h('p', { class: 'hint chat-checks' }, h('span', { class: 'btn-icon', html: icons.check, 'aria-hidden': 'true' }), t('Checked: {what}.', { what: done.join(' · ') }), r.checks.retried ? ` ${t('(answered again after a failed check)')}` : '') : null,
    );
    parts.push(sources);
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
    picker.querySelector('option[value=""]')?.remove();
    try {
      chat = await invoke('chat_get', { id });
      removeBtn.hidden = false;
      body.replaceChildren(conversation());
    } catch (e) {
      fail(e);
    }
  }

  /** Choosing what the conversation is about. */
  async function starter() {
    chat = null;
    // The picker shows where the student is, and the saved conversations stay one choice away
    // (with a single one, there was no way back to it).
    if (picker.options.length && !picker.querySelector('option[value=""]')) picker.prepend(h('option', { value: '' }, t('New conversation')));
    picker.value = '';
    removeBtn.hidden = true;
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
    const general = !chat.scope.attempt && !chat.scope.materials.length;
    const turn = (q, reply) => h('div', { class: 'chat-turn' }, h('p', { class: 'chat-question' }, q), reply);
    for (const x of chat.turns) log.append(turn(x.question, replyEl(x.reply)));
    const input = h('textarea', { class: 'chat-input', rows: 1, maxlength: 800, id: 'chat-input', placeholder: t('Ask Jay-AI…') });
    const grow = () => {
      input.style.height = 'auto';
      input.style.height = `${Math.min(input.scrollHeight, 160)}px`;
    };
    input.addEventListener('input', grow);
    const send = h('button', { type: 'button', class: 'btn' }, t('Send'));
    const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));
    async function ask() {
      const q = input.value.trim();
      if (!q || send.disabled) return;
      send.disabled = true;
      stop.hidden = false;
      input.value = '';
      grow();
      empty?.remove();
      const waiting = h('p', { class: 'hint chat-waiting' }, h('span', { class: 'chat-dots', 'aria-hidden': 'true' }, h('i'), h('i'), h('i')), general ? t('Jay-AI is thinking…') : t('Jay-AI is reading your sources and checking the answer…'));
      const item = turn(q, waiting);
      log.append(item);
      item.scrollIntoView({ block: 'nearest' });
      try {
        const x = await invoke('chat_ask', { id: chat.id, question: q });
        chat.turns.push(x);
        const reply = replyEl(x.reply);
        waiting.replaceWith(reply);
        reply.scrollIntoView({ block: 'nearest' });
        if (chat.turns.length === 1) refresh(chat.id);
      } catch (e) {
        waiting.replaceWith(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e)));
        input.value = q;
        grow();
      }
      send.disabled = false;
      stop.hidden = true;
      input.focus();
    }
    send.addEventListener('click', ask);
    // Enter sends; Shift+Enter is a new line.
    input.addEventListener('keydown', (e) => e.key === 'Enter' && !e.shiftKey && !e.isComposing && (e.preventDefault(), ask()));
    const empty = chat.turns.length ? null : h('p', { class: 'hint' }, t('Ask your first question below.'));
    const head = h('div', { class: 'chat-head' }, h('span', { class: 'badge' }, chat.scope.attempt ? t('An exam I took') : general ? t('Any question') : t('My materials')), chat.title ? h('h2', { class: 'chat-title' }, chat.title) : null);
    const composer = h(
      'div',
      { class: 'chat-composer' },
      h('label', { for: 'chat-input', class: 'visually-hidden' }, t('Your question')),
      input,
      h('div', { class: 'chat-composer-row' }, h('span', { class: 'hint' }, t('Enter sends · Shift+Enter makes a new line')), stop, send),
    );
    return h('div', { class: 'chat' }, head, empty, log, composer);
  }

  picker.addEventListener('change', () => picker.value && open(picker.value));
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
