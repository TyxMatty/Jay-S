// Jay-AI › Chat: questions about materials, a finished exam, or anything. Rust picks the sources,
// checks every quote, number and key in the reply and marks what it couldn't confirm
// (crates/jay-study/src/chat.rs); this page only shows that, all as text.
import { h } from './dom.js';
import { icons } from './icons.js';
import { invoke, isTauri } from '../core/bridge.js';
import { t } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

/** The answer as one paragraph, or a list when asked; each run of unconfirmed sentences starts with its label (never color alone). */
function answerEl(sentences, showSource, list) {
  // Not `chat-list`: that is the conversations sidebar.
  const p = h(list ? 'ul' : 'p', { class: list ? 'chat-answer chat-answer-list' : 'chat-answer' });
  let run = null;
  for (const s of sentences) {
    const chips = s.sources.map((n) => h('button', { type: 'button', class: 'chat-chip', 'aria-label': t('Source {n}', { n }), title: t('Source {n}', { n }), onClick: () => showSource(n) }, String(n)));
    // A list item holds one sentence; a paragraph runs them together.
    const into = (...nodes) => (list ? p.append(h('li', {}, ...nodes)) : p.append(...nodes, ' '));
    if (s.status !== 'unconfirmed') {
      run = null;
      into(h('span', {}, s.text, ...chips));
      continue;
    }
    if (list) {
      into(h('span', { class: 'badge', 'data-status': 'check' }, t('Not confirmed')), ' ', h('span', { class: 'chat-unconfirmed' }, s.text, ...chips));
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
  // From the sources or from the app's guide: both are checked against quotes.
  const grounded = r.kind === 'material' || r.kind === 'app';
  const badge = {
    material: confirmed ? ['ok', t('From your sources')] : ['check', t('Not confirmed')],
    general: ['check', t('General knowledge, no source')],
    unknown: [null, t('Not in your sources')],
    app: ['ok', t('From the Jay-S guide')],
  }[r.kind];
  const head = h('div', { class: 'chat-reply-head' }, h('span', { class: 'chat-avatar', html: icons.spark, 'aria-hidden': 'true' }), h('strong', {}, 'Jay-AI'), badge ? h('span', { class: 'badge', 'data-status': badge[0] }, badge[1]) : null);
  const parts = [head];
  // Unknown with plain sentences and no source: Jay-AI answered from what it knows, not the material.
  const outside = r.sentences.length > 0 && !r.sources.length && r.sentences.every((s) => s.status !== 'unconfirmed' && !s.sources.length);
  if (r.kind === 'unknown') parts.push(h('p', { class: 'chat-banner' }, outside ? t("Nothing in your sources is about this. This is Jay-AI's general knowledge, with no source: check it.") : r.sources.length ? t("Your sources don't answer this. The closest they get:") : t("Nothing in the sources you chose is about this.")));
  if (grounded && !confirmed) parts.push(h('p', { class: 'chat-banner', 'data-tone': 'warning' }, t("Jay-AI couldn't back this answer with your sources. Read them below.")));
  let sources = null;
  // A source chip opens the sources and takes the reader to its quote.
  const showSource = (n) => {
    if (!sources) return;
    sources.open = true;
    sources.querySelector(`[data-n="${n}"]`)?.focus();
  };
  if (r.sentences.length) parts.push(answerEl(r.sentences, showSource, r.list === true));
  if (r.sources.length) {
    const done = [t('quotes found in the text'), t('numbers and keys checked')];
    if (r.checks.second_reading) done.push(t('second reading'));
    sources = h(
      'details',
      // Open when the reader has to judge for themselves; one click away when all is confirmed.
      { class: 'chat-sources', open: !grounded || r.sentences.some((s) => s.status === 'unconfirmed') },
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
      grounded ? h('p', { class: 'hint chat-checks' }, h('span', { class: 'btn-icon', html: icons.check, 'aria-hidden': 'true' }), t('Checked: {what}.', { what: done.join(' · ') }), r.checks.retried ? ` ${t('(answered again after a failed check)')}` : '') : null,
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
  const newBtn = h('button', { type: 'button', class: 'btn btn-secondary' }, h('span', { class: 'btn-icon', html: icons.plus }), t('New conversation'));
  const list = h('ul', { class: 'chat-list' });
  const body = h('div', { class: 'chat-body' });
  let chat = null;

  /** The list shows where the student is. */
  const mark = () => {
    for (const li of list.children) li.firstChild.setAttribute('aria-current', String(li.dataset.id === chat?.id));
  };

  async function refresh() {
    const chats = await invoke('chats_list').catch(() => []);
    list.replaceChildren(
      ...chats.map((c) =>
        h(
          'li',
          { 'data-id': c.id },
          h('button', { type: 'button', class: 'chat-item', title: c.title || null, onClick: () => open(c.id) }, c.title || t('New conversation')),
          h('button', { type: 'button', class: 'icon-btn chat-item-remove', 'aria-label': t('Delete this conversation'), title: t('Delete this conversation'), html: icons.trash, onClick: () => remove(c.id) }),
        ),
      ),
    );
    list.hidden = chats.length === 0;
    mark();
    return chats;
  }

  /** `first`: a question to ask as soon as the conversation is on screen. */
  async function open(id, first) {
    message.hidden = true;
    try {
      chat = await invoke('chat_get', { id });
      mark();
      body.replaceChildren(conversation(first));
    } catch (e) {
      fail(e);
    }
  }

  async function remove(id) {
    if (!confirm(t('Delete this conversation?'))) return;
    try {
      await invoke('chat_delete', { id });
      await refresh();
      // Another conversation was deleted: the one on screen stays.
      if (id === chat?.id) starter();
    } catch (e) {
      fail(e);
    }
  }

  /** The pill: the question, what goes with it, the send button. Enter sends; Shift+Enter is a new line. */
  function composer(ask, ...extra) {
    const input = h('textarea', { class: 'chat-input', rows: 1, maxlength: 800, id: 'chat-input', placeholder: t('Ask Jay-AI…') });
    const grow = () => {
      input.style.height = 'auto';
      // Empty, or not on screen yet (scrollHeight 0): its natural one line. A height computed then
      // cut the field in half.
      input.style.height = input.value && input.scrollHeight ? `${Math.min(input.scrollHeight, 160)}px` : '';
    };
    input.addEventListener('input', grow);
    input.addEventListener('keydown', (e) => e.key === 'Enter' && !e.shiftKey && !e.isComposing && (e.preventDefault(), ask()));
    const send = h('button', { type: 'button', class: 'chat-send', 'aria-label': t('Send'), title: t('Send'), html: icons.send, onClick: () => ask() });
    return { input, grow, send, el: h('div', { class: 'chat-composer' }, h('label', { for: 'chat-input', class: 'visually-hidden' }, t('Your question')), input, ...extra, send) };
  }

  /** A new conversation: the question first, and under it what it is about. It is created on send. */
  async function starter() {
    chat = null;
    mark();
    message.hidden = true;
    const [materials, attempts] = await Promise.all([invoke('materials_list').catch(() => []), invoke('attempts_list').catch(() => [])]);
    const finished = attempts.filter((a) => a.finished);
    const radio = (value, label, checked) => h('label', {}, h('input', { type: 'radio', name: 'chat-about', value, checked }), label);
    const about = h('div', { class: 'chat-scope', role: 'radiogroup', 'aria-label': t('About') }, radio('general', t('Any question'), true), radio('materials', t('My materials')), radio('attempt', t('An exam I took')));
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
    const box = composer(async () => {
      const q = box.input.value.trim();
      if (!q || box.send.disabled) return;
      const scope = { materials: [], attempt: null };
      if (kind() === 'materials') {
        scope.materials = boxes.map((b) => b.querySelector('input')).filter((i) => i.checked).map((i) => i.value);
        if (!scope.materials.length) return fail({ code: 'empty' });
      } else if (kind() === 'attempt') {
        if (!attempt.value) return fail({ code: 'empty' });
        scope.attempt = attempt.value;
      }
      message.hidden = true;
      box.send.disabled = true;
      try {
        const c = await invoke('chat_new', { scope });
        await refresh();
        await open(c.id, q);
      } catch (e) {
        fail(e);
      }
      box.send.disabled = false;
    });
    box.send.disabled = !isTauri;
    body.replaceChildren(h('div', { class: 'chat-hero' }, h('h2', { class: 'chat-greeting' }, t('Where do we start?')), box.el, about, materialsField, attemptField, note));
    // A long conversation leaves the panel scrolled: the new one starts at its top.
    body.closest('.app-panels')?.scrollTo({ top: 0 });
    box.input.focus({ preventScroll: true });
  }

  function conversation(first) {
    // The conversation this screen was made for: `chat` changes if the student opens another one
    // while an answer is on its way.
    const mine = chat;
    const log = h('div', { class: 'chat-log', 'aria-live': 'polite' });
    const general = !mine.scope.attempt && !mine.scope.materials.length;
    const turn = (q, reply) => h('div', { class: 'chat-turn' }, h('p', { class: 'chat-question' }, q), reply);
    for (const x of mine.turns) log.append(turn(x.question, replyEl(x.reply)));
    const stop = h('button', { type: 'button', class: 'chat-send chat-stop', hidden: true, 'aria-label': t('Stop'), title: t('Stop'), onClick: () => invoke('job_cancel') });
    const { input, grow, send, el } = composer(ask, stop);
    async function ask() {
      const q = input.value.trim();
      if (!q || send.disabled) return;
      send.disabled = true;
      stop.hidden = false;
      input.value = '';
      grow();
      const waiting = h('p', { class: 'hint chat-waiting' }, h('span', { class: 'chat-dots', 'aria-hidden': 'true' }, h('i'), h('i'), h('i')), general ? t('Jay-AI is thinking…') : t('Jay-AI is reading your sources and checking the answer…'));
      const item = turn(q, waiting);
      log.append(item);
      item.scrollIntoView({ block: 'nearest' });
      try {
        const x = await invoke('chat_ask', { id: mine.id, question: q });
        mine.turns.push(x);
        const reply = replyEl(x.reply);
        waiting.replaceWith(reply);
        reply.scrollIntoView({ block: 'nearest' });
        if (mine.turns.length === 1) refresh();
      } catch (e) {
        waiting.replaceWith(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e)));
        input.value = q;
        grow();
      }
      send.disabled = false;
      stop.hidden = true;
      input.focus();
    }
    const head = h('div', { class: 'chat-head' }, h('span', { class: 'badge' }, mine.scope.attempt ? t('An exam I took') : general ? t('Any question') : t('My materials')), mine.title ? h('h2', { class: 'chat-title' }, mine.title) : null);
    if (first) {
      input.value = first;
      ask();
    }
    // The pill and its hint stay in reach while the conversation scrolls.
    return h('div', { class: 'chat' }, head, log, h('div', { class: 'chat-dock' }, el, h('p', { class: 'hint' }, t('Enter sends · Shift+Enter makes a new line'))));
  }

  newBtn.addEventListener('click', starter);
  refresh();
  starter();
  return h('div', { class: 'chat-panel' }, h('nav', { class: 'chat-side', 'aria-label': t('Conversations') }, newBtn, list), h('div', { class: 'chat-main' }, message, body));
}
