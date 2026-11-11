// Flashcards (stages 5–6): decks Jay-AI writes from the content (weak topics first), reviewed with
// spaced repetition (SM-2 in Rust), and "too long" / "too short" so new cards fit the student.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { inTime } from './card-time.js';
import { invoke, listenJobs, isTauri } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short' }).format(ms);
const LENGTH = () => ({ '-2': t('very short'), '-1': t('short'), 0: t('medium'), 1: t('a bit longer'), 2: t('long') });

/**
 * Reviewing a deck: a real card on a pile, that flips to show the answer (click, Space or Enter),
 * then four colored ratings (1–4 on the keyboard). The pile gets thinner as the cards go.
 */
function review(deck, cards, onDone) {
  let i = 0;
  let flipped = false;
  let rated = 0;
  let busy = false;
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
  const rate = h('div', { class: 'card-ratings', role: 'group', 'aria-label': t('How well did you remember?'), hidden: true });
  const vote = h('div', { class: 'card-vote', hidden: true });
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
    // The lift is an animation of its own (styles/app.css .is-flipping), restarted on every flip.
    card.classList.remove('is-flipping');
    void card.offsetWidth;
    card.classList.add('is-flipping');
    card.dataset.flipped = String(flipped);
    card.setAttribute('aria-pressed', String(flipped));
    card.querySelector('.flip-back').setAttribute('aria-hidden', String(!flipped));
    card.querySelector('.flip-front').setAttribute('aria-hidden', String(flipped));
    rate.hidden = !flipped;
    vote.hidden = !flipped;
    show.hidden = flipped;
    if (flipped) rate.querySelector('[data-tone="good"]')?.focus();
  }

  function render() {
    const c = cards[i];
    if (!c) return done();
    flip(false);
    topic.textContent = c.topic;
    front.textContent = c.front;
    backQuestion.textContent = c.front;
    back.textContent = c.back;
    hints.forEach((hint, k) => (hint.textContent = inTime(c.next[k], language())));
    message.textContent = '';
    delete message.dataset.tone;
    counter.textContent = t('Card {n} of {total}', { n: i + 1, total: cards.length });
    bar.value = i;
    // The pile behind the card thins out as the deck goes.
    pile.dataset.left = String(Math.min(2, cards.length - i - 1));
    for (const b of vote.querySelectorAll('button')) b.disabled = false;
    card.classList.remove('is-dealt');
    void card.offsetWidth;
    card.classList.add('is-dealt');
    card.focus();
  }

  function done() {
    document.removeEventListener('keydown', keys);
    el.replaceChildren(
      h('div', { class: 'panel-head' }, h('h1', {}, deck.title)),
      h('div', { class: 'deck-done' },
        h('span', { class: 'card-icon', html: icons.check, 'aria-hidden': 'true' }),
        h('h2', {}, t('All reviewed for now!')),
        h('p', { class: 'hint' }, t('{n} cards reviewed. The ones you forgot come back first next time.', { n: rated })),
        h('button', { type: 'button', class: 'btn', onClick: onDone }, t('Back to the decks')),
      ),
    );
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

  async function next(quality) {
    if (busy) return;
    busy = true;
    try {
      const k = RATINGS.findIndex((r) => r[0] === quality);
      const tone = RATINGS[k][3];
      const reply = await invoke('card_review', { deck: deck.id, index: cards[i].index, quality });
      cards[i].next = reply.next; // in case the card is shown again
      ghost(tone);
      i++;
      rated++;
      render();
      message.textContent = t('Comes back in {when}.', { when: inTime(reply.due_in, language()) });
      message.dataset.tone = tone;
    } catch (e) {
      message.textContent = studyFailureText(e);
    } finally {
      busy = false;
    }
  }

  card.addEventListener('click', () => flip());
  show.addEventListener('click', () => flip(true));
  function keys(e) {
    if (!el.isConnected) return document.removeEventListener('keydown', keys);
    if (e.target.closest('input, select, textarea')) return;
    if ((e.key === ' ' || e.key === 'Enter') && (e.target === card || e.target === document.body)) {
      e.preventDefault();
      flip();
    } else if (flipped && ['1', '2', '3', '4'].includes(e.key)) {
      next(RATINGS[Number(e.key) - 1][0]);
    }
  }
  document.addEventListener('keydown', keys);

  const el = h(
    'section',
    { class: 'app-panel is-single review' },
    h('div', { class: 'panel-head' }, h('h1', {}, deck.title), h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { document.removeEventListener('keydown', keys); onDone(); } }, t('Back'))),
    h('div', { class: 'review-progress' }, counter, bar),
    pile,
    h('div', { class: 'review-actions' }, show, rate, vote, message),
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
            h('div', { class: 'material-main' }, h('h3', { class: 'material-name' }, d.title), h('p', { class: 'material-meta' }, [t('{n} cards', { n: d.cards }), d.due ? t('{n} to review', { n: d.due }) : t('nothing to review now'), date(d.created)].join(' · '))),
            h('button', { type: 'button', class: 'btn', disabled: !d.due, onClick: async () => {
              try {
                const cards = await invoke('deck_due', { id: d.id });
                stopListening.then((un) => un());
                host.replaceChildren(review(d, cards, showHome));
              } catch (e) {
                message.textContent = studyFailureText(e);
              }
            } }, t('Review')),
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
