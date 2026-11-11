// Flashcards (stages 5–6): decks Jay-AI writes from the content (weak topics first), reviewed with
// spaced repetition (SM-2 in Rust), and "too long" / "too short" so new cards fit the student.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { inTime, whenText } from './card-time.js';
import { requeueAt } from './card-queue.js';
import { stopwatch, clockText } from './stopwatch.js';
import { invoke, listenJobs, isTauri } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short' }).format(ms);
// The deck row: what is due (or when the next card comes) on one line, what the deck holds on the next.
const status = (d) =>
  d.due ? t('{n} to review', { n: d.due }) : d.next_due === null ? t('nothing to review now') : t('Nothing to review now · the next one comes {when}', { when: whenText(d.next_due, language()) });
const meta = (d) => [
  t('{n} cards', { n: d.cards }),
  d.new ? (d.new === 1 ? t('1 new') : t('{n} new', { n: d.new })) : null,
  d.learned ? (d.learned === 1 ? t('1 learned') : t('{n} learned', { n: d.learned })) : null,
  date(d.created),
].filter(Boolean).join(' · ');
const LENGTH = () => ({ '-2': t('very short'), '-1': t('short'), 0: t('medium'), 1: t('a bit longer'), 2: t('long') });

/**
 * Reviewing a deck: a real card on a pile, that flips to show the answer (click, Space or Enter),
 * then four colored ratings (1–4 on the keyboard). The pile gets thinner as the cards go.
 */
function review(deck, cards, onDone, { again = false, practice = false } = {}) {
  // The session queue: a forgotten card is put back in it (`again`), so the total grows.
  const queue = cards.map((card) => ({ card, again }));
  const since = Date.now(); // a new review() is a new session: the summary counts from here
  let pos = 0;
  let last = null; // the last rating, if it can still be undone: { pos, insertedAt }
  let flipped = false;
  let busy = false;
  let finished = false; // the summary is showing: the clock stays stopped
  // The session clock only runs while the window and this screen are visible.
  const clock = stopwatch();
  let shownAt = 0;
  let thinkMs = null; // set on the first flip of the card shown
  const clockLabel = t('Time in this session');
  const clockValue = h('span', {}, '0:00');
  const clockEl = h('span', { class: 'review-clock', title: clockLabel, 'aria-label': clockLabel }, h('span', { class: 'review-clock-icon', html: icons.timer, 'aria-hidden': 'true' }), clockValue);
  const topic = h('span', { class: 'flip-topic' });
  const front = h('p', { class: 'card-front' });
  const backQuestion = h('p', { class: 'card-back-question' });
  const back = h('p', { class: 'card-back' });
  const card = h(
    'div',
    { class: 'flip-card', role: 'button', tabindex: '0', 'aria-pressed': 'false' },
    h(
      'div',
      { class: 'flip-inner' },
      h('div', { class: 'flip-face flip-front' }, topic, front, h('span', { class: 'flip-hint', 'aria-hidden': 'true' }, t('Click or press Space to flip'))),
      h('div', { class: 'flip-face flip-back', 'aria-hidden': 'true' }, h('span', { class: 'flip-topic' }, t('Answer')), backQuestion, back),
    ),
  );
  const pile = h('div', { class: 'card-pile' }, card);
  const counter = h('p', { class: 'card-counter' });
  const bar = h('progress', { class: 'ai-progress', max: cards.length, value: 0 });
  const message = h('p', { class: 'hint', role: 'status' });
  const show = h('button', { type: 'button', class: 'btn' }, t('Show the answer'));
  const newUndoButton = () => h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => undo() }, h('span', { class: 'btn-icon', html: icons.undo, 'aria-hidden': 'true' }), t('Undo'));
  const undoButton = newUndoButton();
  const summaryUndo = newUndoButton(); // the same step back, offered on the summary too
  const canUndo = (yes) => (undoButton.hidden = summaryUndo.hidden = !yes);
  const rate = h('div', { class: 'card-ratings', role: 'group', 'aria-label': t('How well did you remember?'), hidden: true });
  const vote = h('div', { class: 'card-vote', hidden: true });
  const nextCard = h('button', { type: 'button', class: 'btn', hidden: true, onClick: () => advance() }, t('Next card'));
  // Color and a symbol and a word: never color alone.
  const RATINGS = [
    [1, () => h('span', { class: 'rating-symbol', html: icons.close, 'aria-hidden': 'true' }), t('Forgot'), 'again'],
    [3, () => h('span', { class: 'rating-symbol', html: icons.wave, 'aria-hidden': 'true' }), t('Hard'), 'hard'],
    [4, () => h('span', { class: 'rating-symbol', html: icons.check, 'aria-hidden': 'true' }), t('Good'), 'good'],
    [5, () => h('span', { class: 'rating-symbol', html: icons.star, 'aria-hidden': 'true' }), t('Easy'), 'easy'],
  ];
  const hints = RATINGS.map(() => h('span', { class: 'rating-hint' }));
  rate.append(
    ...RATINGS.map(([q, symbol, label, tone], k) =>
      h('button', { type: 'button', class: 'rating', 'data-tone': tone, onClick: () => next(q) },
        symbol(),
        h('span', { class: 'rating-label' }, label),
        hints[k],
        h('kbd', { 'aria-hidden': 'true' }, String(k + 1)),
      ),
    ),
  );
  const voteButton = (tooLong, label) =>
    h('button', { type: 'button', class: 'chip', onClick: async (e) => {
      const button = e.currentTarget;
      try {
        const p = await invoke('card_vote', { tooLong });
        button.disabled = true;
        message.textContent = t('Got it: new cards will be {length}.', { length: LENGTH()[p.card_length] });
      } catch (err) {
        message.textContent = studyFailureText(err);
      }
    } }, label);
  vote.append(h('span', { class: 'hint' }, t('This card was:')), voteButton(true, t('Too long')), voteButton(false, t('Too short')));

  function flip(to = !flipped) {
    if (to === flipped) return;
    flipped = to;
    if (to && thinkMs === null) thinkMs = clock.elapsed() - shownAt;
    // The lift is an animation of its own (styles/app.css .is-flipping), restarted on every flip.
    card.classList.remove('is-flipping');
    void card.offsetWidth;
    card.classList.add('is-flipping');
    card.dataset.flipped = String(flipped);
    card.setAttribute('aria-pressed', String(flipped));
    card.querySelector('.flip-back').setAttribute('aria-hidden', String(!flipped));
    card.querySelector('.flip-front').setAttribute('aria-hidden', String(flipped));
    rate.hidden = vote.hidden = practice || !flipped;
    nextCard.hidden = !practice || !flipped;
    show.hidden = flipped;
    if (flipped) (practice ? nextCard : rate.querySelector('[data-tone="good"]'))?.focus();
  }

  function render() {
    const c = queue[pos]?.card;
    if (!c) return done();
    flip(false);
    shownAt = clock.elapsed();
    thinkMs = null;
    topic.textContent = c.topic;
    front.textContent = c.front;
    backQuestion.textContent = c.front;
    back.textContent = c.back;
    hints.forEach((hint, k) => (hint.textContent = inTime(c.next[k], language())));
    message.textContent = '';
    delete message.dataset.tone;
    counter.textContent = t('{n} left', { n: queue.length - pos });
    bar.max = queue.length;
    bar.value = pos;
    // The pile behind the card thins out as the deck goes.
    pile.dataset.left = String(Math.min(2, queue.length - pos - 1));
    for (const b of vote.querySelectorAll('button')) b.disabled = false;
    card.classList.remove('is-dealt');
    void card.offsetWidth;
    card.classList.add('is-dealt');
    card.focus();
  }

  // The summary: every number comes from what Rust saved (the deck's log and cards).
  async function done() {
    finished = true;
    clock.pause();
    reviewBody.hidden = true;
    summaryBody.hidden = false;
    const back = h('button', { type: 'button', class: 'btn', onClick: () => { stop(); onDone(); } }, t('Back to the decks'));
    if (practice) {
      // Nothing was saved, so there are no numbers to show.
      summaryBody.replaceChildren(h('span', { class: 'card-icon', html: icons.check, 'aria-hidden': 'true' }), h('h2', {}, t('Pass done.')), h('p', {}, t("The cards' schedule did not change.")), h('div', { class: 'button-row' }, back));
      return;
    }
    summaryBody.replaceChildren(h('span', { class: 'card-icon', html: icons.check, 'aria-hidden': 'true' }), h('h2', {}, t('All reviewed for now!')));
    const buttons = (...extra) => h('div', { class: 'button-row' }, back, ...extra, summaryUndo);
    let s;
    try {
      s = await invoke('deck_session', { id: deck.id, since });
    } catch (e) {
      if (finished) summaryBody.append(h('p', { class: 'hint', role: 'status' }, studyFailureText(e)), buttons());
      return;
    }
    if (!finished) return; // undone while the summary was loading
    const reviews = s.counts.reduce((a, b) => a + b, 0);
    const word = (k) => RATINGS[k][2];
    const tally = RATINGS.map((r, k) => `${word(k)} ${s.counts[k]}`).join(', ');
    summaryBody.append(
      h('div', { class: 'session-bar', role: 'img', 'aria-label': tally }, ...RATINGS.map((r, k) => {
        const seg = h('span', { class: 'session-seg', 'data-tone': r[3], hidden: !s.counts[k] });
        seg.style.flexGrow = String(s.counts[k]);
        return seg;
      })),
      h('ul', { class: 'session-legend' }, ...RATINGS.map((r, k) => s.counts[k] ? h('li', { 'data-tone': r[3] }, r[1](), `${s.counts[k]} ${word(k)}`) : null).filter(Boolean)),
    );
    if (reviews) summaryBody.append(h('p', { class: 'hint' }, t('{time} on the cards · {avg} per rating', { time: clockText(s.ms), avg: new Intl.NumberFormat(language(), { style: 'unit', unit: 'second', unitDisplay: 'short' }).format(Math.round(s.ms / reviews / 1000)) })));
    if (s.forgotten.length) {
      summaryBody.append(
        h('div', { class: 'session-forgot' },
          h('h3', {}, t('You forgot:')),
          h('ul', {}, ...s.forgotten.slice(0, 5).map((c) => h('li', {}, c.front))),
          s.forgotten.length > 5 ? h('p', { class: 'hint' }, t('and {n} more', { n: s.forgotten.length - 5 })) : null,
        ),
      );
    }
    if (s.due_now > 0) summaryBody.append(h('p', {}, s.due_now === 1 ? t('1 card is still due now.') : t('{n} cards are still due now.', { n: s.due_now })));
    else if (s.next_due !== null) summaryBody.append(h('p', {}, s.next_count === 1 ? t('Next review: {when}, 1 card.', { when: whenText(s.next_due, language()) }) : t('Next review: {when}, {n} cards.', { when: whenText(s.next_due, language()), n: s.next_count })));
    summaryBody.append(buttons(s.forgotten.length ? h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { stop(); el.replaceWith(review(deck, s.forgotten, onDone, { again: true })); } }, t('Review the forgotten ones now')) : null));
  }

  // The card just rated leaves in the rating's tone (a clone, so the next card is dealt at once).
  function ghost(tone) {
    if (document.documentElement.dataset.motion === 'reduced') return;
    const g = card.cloneNode(true);
    g.classList.remove('is-dealt', 'is-flipping');
    g.classList.add('flip-ghost');
    g.dataset.out = tone;
    g.setAttribute('aria-hidden', 'true');
    g.setAttribute('inert', '');
    g.removeAttribute('tabindex');
    pile.append(g);
    g.addEventListener('animationend', () => g.remove());
    setTimeout(() => g.remove(), 400);
  }

  // Practice: the schedule stays untouched, so no card_review, card_undo or deck_session call.
  function advance() {
    if (!flipped) return;
    pos++;
    render();
  }

  async function next(quality) {
    if (busy) return;
    busy = true;
    try {
      const k = RATINGS.findIndex((r) => r[0] === quality);
      const tone = RATINGS[k][3];
      const entry = queue[pos];
      const reply = await invoke('card_review', { deck: deck.id, index: entry.card.index, quality, again: entry.again, ms: Math.round(thinkMs ?? 0) });
      entry.card.next = reply.next; // in case the card is shown again
      let insertedAt = -1;
      if (quality === 1) {
        insertedAt = requeueAt(pos, queue.length);
        queue.splice(insertedAt, 0, { card: entry.card, again: true });
      }
      last = { pos, insertedAt };
      canUndo(true);
      ghost(tone);
      pos++;
      render();
      message.textContent = quality === 1 ? t('Comes back in this session, in a few cards.') : t('Comes back in {when}.', { when: inTime(reply.due_in, language()) });
      message.dataset.tone = tone;
    } catch (e) {
      message.textContent = studyFailureText(e);
    } finally {
      busy = false;
    }
  }

  // One step back: the Rust side puts the deck back as it was before the last rating.
  async function undo() {
    if (busy || !last) return;
    busy = true;
    try {
      const reply = await invoke('card_undo');
      if (last.insertedAt >= 0) queue.splice(last.insertedAt, 1);
      pos = last.pos;
      queue[pos].card.next = reply.next;
      last = null;
      canUndo(false);
      finished = false;
      reviewBody.hidden = false;
      summaryBody.hidden = true;
      render();
      message.textContent = t('Rating undone.');
    } catch (e) {
      last = null;
      canUndo(false);
      message.textContent = studyFailureText(e);
    } finally {
      busy = false;
    }
  }

  card.addEventListener('click', () => flip());
  show.addEventListener('click', () => flip(true));
  function keys(e) {
    if (!el.isConnected) return document.removeEventListener('keydown', keys);
    if (e.target.closest('input, select, textarea, [contenteditable]')) return;
    if (!practice && (e.key === 'Backspace' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z'))) {
      e.preventDefault();
      undo();
    } else if (finished) {
      // the summary has no card to flip or rate
    } else if (practice && flipped && (e.key === 'Enter' || e.key === 'ArrowRight')) {
      e.preventDefault();
      advance();
    } else if ((e.key === ' ' || e.key === 'Enter') && (e.target === card || e.target === document.body)) {
      e.preventDefault();
      flip();
    } else if (!practice && flipped && ['1', '2', '3', '4'].includes(e.key)) {
      next(RATINGS[Number(e.key) - 1][0]);
    }
  }
  document.addEventListener('keydown', keys);

  function tick() {
    if (!el.isConnected) return stop();
    if (finished || document.hidden || el.offsetParent === null) clock.pause();
    else clock.resume();
    clockValue.textContent = clockText(clock.elapsed());
  }
  const ticker = setInterval(tick, 1000);
  document.addEventListener('visibilitychange', tick);
  function stop() {
    clearInterval(ticker);
    document.removeEventListener('visibilitychange', tick);
    document.removeEventListener('keydown', keys);
  }

  const reviewBody = h('div', {}, h('div', { class: 'review-progress' }, counter, practice ? h('span', { class: 'badge' }, t('Practice: the schedule does not change')) : null, bar, clockEl), pile, h('div', { class: 'review-actions' }, show, rate, vote, nextCard, message, undoButton));
  const summaryBody = h('div', { class: 'deck-done session-summary', hidden: true });
  const el = h(
    'section',
    { class: 'app-panel is-single review' },
    h('div', { class: 'panel-head' }, h('h1', {}, deck.title), h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { stop(); onDone(); } }, t('Back'))),
    reviewBody,
    summaryBody,
  );
  render();
  return el;
}

export function createFlashcardsPanel() {
  const host = h('div', { class: 'lazy-panel' });
  const showHome = () => host.replaceChildren(home());

  function home() {
    const title = h('input', { class: 'input', id: 'deck-title', value: t('My cards'), maxlength: 120 });
    const count = h('input', { class: 'input', type: 'number', id: 'deck-count', min: 1, max: 300, value: 20 });
    const contentList = h('div', { class: 'check-list', role: 'group', 'aria-label': t('Content') });
    const progress = h('progress', { class: 'ai-progress', max: 1, value: 0, hidden: true });
    const message = h('p', { class: 'hint', role: 'status' });
    const profileLine = h('p', { class: 'hint' });
    const build = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.spark }), t('Make cards with Jay-AI'));
    const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));
    const list = h('ul', { class: 'material-list' });

    build.addEventListener('click', async () => {
      const materials = [...contentList.querySelectorAll('input:checked')].map((i) => i.value);
      if (!materials.length) return (message.textContent = t('Pick some content.'));
      if (!aiReady()) return (message.textContent = t('Cards need Jay-AI: download it in the Jay-AI section.'));
      try {
        await invoke('deck_build', { title: title.value.trim() || t('My cards'), materials, count: Number(count.value) || 20 });
        build.disabled = true;
        stop.hidden = false;
        progress.hidden = false;
        progress.removeAttribute('value');
        message.textContent = t('Jay-AI is writing the cards…');
      } catch (e) {
        message.textContent = studyFailureText(e);
      }
    });

    const stopListening = listenJobs((e) => {
      if (e.kind !== 'deck') return;
      if (e.state === 'progress') {
        build.disabled = true;
        stop.hidden = false;
        progress.hidden = false;
        progress.max = e.step.total || 1;
        progress.value = e.step.done;
        message.textContent = t('Jay-AI is writing the cards… {done} of {total}', { done: e.step.done, total: e.step.total });
        return;
      }
      build.disabled = false;
      stop.hidden = true;
      progress.hidden = true;
      message.textContent = e.state === 'done' ? t('Ready!') : studyFailureText(e.error);
      load();
    });

    async function load() {
      if (!isTauri) return;
      const [decks, materials, profile] = await Promise.all([invoke('decks_list'), invoke('materials_list'), invoke('profile_get')]);
      const contents = materials.filter((m) => m.kind === 'content' || m.kind === 'both');
      contentList.replaceChildren(...(contents.length ? contents.map((m) => h('label', { class: 'check-row' }, h('input', { type: 'checkbox', value: m.id, checked: true }), m.name)) : [h('p', { class: 'hint' }, t('No content yet: attach it in Materials.'))]));
      const weak = Object.entries(profile.topics).filter(([, [p, n]]) => n >= 2 && p / n < 0.7).map(([topic]) => topic);
      profileLine.textContent = [t('Your cards are {length}.', { length: LENGTH()[profile.card_length] }), weak.length ? t('Topics to reinforce first: {topics}.', { topics: weak.slice(0, 5).join(', ') }) : ''].filter(Boolean).join(' ');
      list.replaceChildren(
        ...decks.map((d) =>
          h('li', { class: 'material-card deck-card' }, h('div', { class: 'material-row' },
            h('span', { class: 'deck-thumb', 'aria-hidden': 'true' }, h('span', {}, String(d.cards))),
            h('div', { class: 'material-main' }, h('h3', { class: 'material-name' }, d.title), h('p', { class: 'material-meta deck-status', 'data-due': String(Boolean(d.due)) }, status(d)), h('p', { class: 'material-meta' }, meta(d))),
            h('button', { type: 'button', class: d.due ? 'btn' : 'btn btn-secondary', title: d.due ? null : t('A pass through every card that does not change when they come back.'), onClick: async () => {
              try {
                const cards = await invoke('deck_due', { id: d.id, all: !d.due });
                stopListening.then((un) => un());
                host.replaceChildren(review(d, cards, showHome, { practice: !d.due }));
              } catch (e) {
                message.textContent = studyFailureText(e);
              }
            } }, d.due ? t('Review') : t('Study anyway')),
            h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete {name}', { name: d.title }), title: t('Delete'), html: icons.trash, onClick: async () => {
              if (!confirm(t('Delete the deck "{name}"?', { name: d.title }))) return;
              await invoke('deck_delete', { id: d.id }).catch((e) => (message.textContent = studyFailureText(e)));
              load();
            } }),
          )),
        ),
      );
    }

    load();
    return h(
      'section',
      { class: 'app-panel is-single' },
      h('h1', {}, t('Flashcards')),
      h(
        'fieldset',
        {},
        cardTitle(icons.cards, t('New deck')),
        h('p', { class: 'hint' }, t('Cards from your content that adapt to you: short and direct or longer, and more of what you missed.')),
        profileLine,
        h('div', { class: 'field' }, h('label', { for: 'deck-title' }, t('Name')), title),
        h('div', { class: 'field' }, h('span', { class: 'field-label' }, t('Content')), contentList),
        h('div', { class: 'field' }, h('label', { for: 'deck-count' }, t('How many cards')), count),
        progress,
        message,
        h('div', { class: 'button-row' }, build, stop),
      ),
      list,
    );
  }
  showHome();
  return host;
}
