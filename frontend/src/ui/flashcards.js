// Flashcards (stages 5–6): decks Jay-AI writes from the content (weak topics first), reviewed with
// spaced repetition (SM-2 in Rust), and "too long" / "too short" so new cards fit the student.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, listen, isTauri } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short' }).format(ms);
const LENGTH = () => ({ '-2': t('very short'), '-1': t('short'), 0: t('medium'), 1: t('a bit longer'), 2: t('long') });

function review(deck, cards, onDone) {
  let i = 0;
  const front = h('p', { class: 'card-front' });
  const back = h('p', { class: 'card-back', hidden: true });
  const meta = h('p', { class: 'hint' });
  const message = h('p', { class: 'hint', role: 'status' });
  const show = h('button', { type: 'button', class: 'btn' }, t('Show the answer'));
  const rate = h('div', { class: 'button-row', hidden: true });
  const vote = h('div', { class: 'button-row card-vote', hidden: true });
  const RATINGS = [
    [1, t('Forgot')],
    [3, t('Hard')],
    [4, t('Good')],
    [5, t('Easy')],
  ];
  rate.append(...RATINGS.map(([q, label]) => h('button', { type: 'button', class: q === 1 ? 'btn btn-secondary' : 'btn', onClick: () => next(q) }, label)));
  const voteButton = (tooLong, label) =>
    h('button', { type: 'button', class: 'btn btn-secondary', onClick: async (e) => {
      try {
        const p = await invoke('card_vote', { tooLong });
        e.currentTarget.disabled = true;
        message.textContent = t('Got it: new cards will be {length}.', { length: LENGTH()[p.card_length] });
      } catch (err) {
        message.textContent = studyFailureText(err);
      }
    } }, label);
  vote.append(voteButton(true, t('Too long')), voteButton(false, t('Too short')));

  function render() {
    const c = cards[i];
    if (!c) return onDone();
    front.textContent = c.front;
    back.textContent = c.back;
    back.hidden = true;
    rate.hidden = true;
    vote.hidden = true;
    show.hidden = false;
    message.textContent = '';
    meta.textContent = `${t('Card {n} of {total}', { n: i + 1, total: cards.length })} · ${c.topic}`;
    show.focus();
  }

  show.addEventListener('click', () => {
    back.hidden = false;
    rate.hidden = false;
    vote.hidden = false;
    show.hidden = true;
    rate.querySelector('.btn:not(.btn-secondary)')?.focus();
  });

  async function next(quality) {
    try {
      await invoke('card_review', { deck: deck.id, index: cards[i].index, quality });
      i++;
      render();
    } catch (e) {
      message.textContent = studyFailureText(e);
    }
  }

  render();
  return h(
    'section',
    { class: 'app-panel is-single' },
    h('div', { class: 'panel-head' }, h('h1', {}, deck.title), h('button', { type: 'button', class: 'btn btn-secondary', onClick: onDone }, t('Back'))),
    h('fieldset', { class: 'flashcard' }, meta, front, back, show, rate, vote, message),
  );
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

    const stopListening = listen('study-job', (e) => {
      if (e.kind !== 'deck') return;
      if (e.state === 'progress') {
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
          h('li', { class: 'material-card' }, h('div', { class: 'material-row' },
            h('span', { class: 'card-icon', html: icons.cards, 'aria-hidden': 'true' }),
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
