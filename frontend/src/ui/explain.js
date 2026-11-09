// "Explain this to me" (Materials): Jay-AI summarizes and explains a material in the language and
// depth the student picks, then asks them questions one at a time, checks each answer (with a
// follow-up when it's off), and answers their own questions. Rust does the asking
// (src-tauri/src/study.rs, jay_study::explain); everything from the model is shown as text.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, listen } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

/** Each language in its own name (it goes to Jay-AI as written). */
const LANGUAGES = ['Português (Brasil)', 'English', 'Español', 'Français', 'Deutsch', 'Italiano', '日本語', '中文', '한국어', 'Русский', 'العربية'];

export function createExplainView(material, onBack) {
  let current = null;
  let job = null;
  const message = h('p', { class: 'hint', role: 'status' });
  const progress = h('progress', { class: 'ai-progress', max: 1, value: 0, hidden: true });
  const result = h('div', { class: 'explain-result' });

  const languageSelect = h('select', { class: 'input', id: 'explain-language' }, LANGUAGES.map((l) => h('option', { value: l, selected: (language() === 'en' ? 'English' : 'Português (Brasil)') === l }, l)), h('option', { value: '' }, t('Other…')));
  const otherLanguage = h('input', { class: 'input', id: 'explain-other', maxlength: 40, placeholder: t('e.g. Nederlands, Tupi, Latin'), hidden: true });
  languageSelect.addEventListener('change', () => {
    otherLanguage.hidden = languageSelect.value !== '';
    if (!otherLanguage.hidden) otherLanguage.focus();
  });
  const level = h(
    'div',
    { class: 'segmented', role: 'radiogroup', 'aria-labelledby': 'explain-level-label' },
    [['simple', t('Very simple')], ['balanced', t('Balanced')], ['deep', t('In depth')]].map(([value, label]) => h('label', {}, h('input', { type: 'radio', name: 'explain-level', value, checked: value === 'balanced' }), label)),
  );
  const focus = h('input', { class: 'input', id: 'explain-focus', maxlength: 200, placeholder: t('e.g. the dark phase, for the ENEM') });
  const go = h('button', { type: 'button', class: 'btn' }, h('span', { class: 'btn-icon', html: icons.spark }), t('Explain with Jay-AI'));
  const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));

  const chosenLanguage = () => (languageSelect.value || otherLanguage.value.trim());

  go.addEventListener('click', async () => {
    message.textContent = '';
    if (!aiReady()) return (message.textContent = t('This needs Jay-AI: download it in the Jay-AI section.'));
    if (!chosenLanguage()) return (message.textContent = t('Write the language.'));
    try {
      await invoke('explain_build', { id: material.id, language: chosenLanguage(), level: level.querySelector('input:checked').value, focus: focus.value.trim() || null });
      go.disabled = true;
      stop.hidden = false;
      progress.hidden = false;
      progress.removeAttribute('value');
      message.textContent = t('Jay-AI is reading the material…');
    } catch (e) {
      message.textContent = e?.code === 'bad_language' ? t('Write just the name of the language.') : studyFailureText(e);
    }
  });

  job = listen('study-job', async (e) => {
    if (e.kind !== 'explain' || (e.id && e.id !== material.id)) return;
    if (e.state === 'progress') {
      const { done, total } = e.step;
      progress.max = total || 1;
      progress.value = done;
      message.textContent = done + 1 >= total ? t('Jay-AI is writing the explanation…') : t('Jay-AI is reading the material… part {n} of {total}', { n: done + 1, total: total - 1 });
      return;
    }
    go.disabled = false;
    stop.hidden = true;
    progress.hidden = true;
    if (e.state === 'done') {
      message.textContent = '';
      show(await invoke('explain_get', { id: material.id }));
    } else {
      message.textContent = studyFailureText(e.error);
    }
  });

  function show(x) {
    current = x;
    if (!x) return result.replaceChildren();
    result.replaceChildren(
      h('article', { class: 'explain-sheet' },
        h('p', { class: 'runner-meta' }, `${x.language} · ${{ simple: t('Very simple'), balanced: t('Balanced'), deep: t('In depth') }[x.level]}`),
        h('h2', {}, x.title),
        ...x.summary.map((p) => h('p', { class: 'explain-paragraph' }, p)),
        h('h3', {}, t('Key points')),
        h('ul', { class: 'key-points' }, x.key_points.map((k) => h('li', {}, k))),
        h('h3', {}, t('Concepts')),
        h('div', { class: 'concepts' }, x.concepts.map((c) => h('div', { class: 'concept' }, h('h4', {}, c.term), h('p', {}, c.explanation), h('p', { class: 'concept-example' }, h('strong', {}, t('Example: ')), c.example)))),
      ),
      quiz(x),
      askBox(),
    );
  }

  /** Jay-AI's questions, one at a time: answer, see the feedback, maybe a follow-up, next. */
  function quiz(x) {
    let i = 0;
    let asking = null;
    const box = h('fieldset', { class: 'quiz' });
    function render() {
      const q = x.questions[i];
      if (!q) {
        box.replaceChildren(cardTitle(icons.spark, t('Jay-AI asks you')), h('p', {}, t("That's all the questions. Ask your own below, or explain again in another way.")));
        return;
      }
      asking = q.question;
      const answer = h('textarea', { class: 'input quiz-answer', rows: 3, maxlength: 2000, id: 'quiz-answer', placeholder: t('Answer in your own words…') });
      const hint = h('p', { class: 'hint', hidden: true }, `${t('Hint')}: ${q.hint}`);
      const out = h('div', { 'aria-live': 'polite' });
      const send = h('button', { type: 'button', class: 'btn' }, t('Answer'));
      const next = h('button', { type: 'button', class: 'btn btn-secondary' }, i + 1 < x.questions.length ? t('Next question') : t('Finish'));
      next.addEventListener('click', () => {
        i++;
        render();
      });
      send.addEventListener('click', async () => {
        if (!answer.value.trim()) return answer.focus();
        send.disabled = true;
        out.replaceChildren(h('p', { class: 'hint' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Jay-AI is reading your answer…')));
        try {
          const c = await invoke('explain_check', { id: material.id, question: asking, answer: answer.value, language: x.language });
          const symbol = { right: '✓', partial: '½', wrong: '✗' }[c.verdict];
          const label = { right: t('Right!'), partial: t('Almost there.'), wrong: t('Not quite.') }[c.verdict];
          out.replaceChildren(
            h('div', { class: 'runner-feedback', 'data-tone': c.verdict }, h('span', { class: 'feedback-symbol', 'aria-hidden': 'true' }, symbol), h('strong', {}, label), h('p', {}, c.feedback)),
          );
          if (c.follow_up) {
            // A smaller question on the way to the answer: it becomes the one being answered.
            asking = c.follow_up;
            question.textContent = c.follow_up;
            question.classList.add('is-follow-up');
            answer.value = '';
            answer.focus();
          }
        } catch (e) {
          out.replaceChildren(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e)));
        }
        send.disabled = false;
      });
      const question = h('p', { class: 'quiz-question' }, q.question);
      box.replaceChildren(
        cardTitle(icons.spark, t('Jay-AI asks you')),
        h('p', { class: 'hint' }, t('Question {n} of {total}', { n: i + 1, total: x.questions.length })),
        question,
        h('label', { for: 'quiz-answer', class: 'visually-hidden' }, t('Your answer')),
        answer,
        hint,
        h('div', { class: 'button-row' }, send, h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => (hint.hidden = false) }, t('Show a hint')), next),
        out,
      );
    }
    render();
    return box;
  }

  /** The student's own questions about the material. */
  function askBox() {
    const input = h('textarea', { class: 'input quiz-answer', rows: 2, maxlength: 600, id: 'ask-input', placeholder: t('What didn\'t make sense? Ask anything about this material…') });
    const out = h('div', { class: 'ask-answers', 'aria-live': 'polite' });
    const send = h('button', { type: 'button', class: 'btn' }, t('Ask'));
    send.addEventListener('click', async () => {
      const q = input.value.trim();
      if (!q) return input.focus();
      send.disabled = true;
      const item = h('div', { class: 'ask-item' }, h('p', { class: 'ask-q' }, q), h('p', { class: 'hint' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Jay-AI is thinking…')));
      out.prepend(item);
      input.value = '';
      try {
        const a = await invoke('explain_ask', { id: material.id, question: q, language: current?.language ?? chosenLanguage() });
        item.lastChild.replaceWith(h('p', { class: 'ask-a' }, a));
      } catch (e) {
        item.lastChild.replaceWith(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e)));
      }
      send.disabled = false;
    });
    return h('fieldset', {}, cardTitle(icons.info, t('Ask Jay-AI')), h('label', { for: 'ask-input', class: 'visually-hidden' }, t('Your question')), input, h('div', { class: 'button-row' }, send), out);
  }

  invoke('explain_get', { id: material.id }).then(show).catch(() => {});

  const el = h(
    'section',
    { class: 'app-panel is-single explain' },
    h('div', { class: 'panel-head' }, h('h1', {}, material.name), h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { el.dispose(); onBack(); } }, t('Back'))),
    h(
      'fieldset',
      {},
      cardTitle(icons.spark, t('Explain this material')),
      h('p', { class: 'hint' }, t('Jay-AI summarizes and explains it in the language you choose, then asks you questions to see what stuck.')),
      h('div', { class: 'field' }, h('label', { for: 'explain-language' }, t('Language')), languageSelect, otherLanguage),
      h('div', { class: 'field' }, h('span', { class: 'field-label', id: 'explain-level-label' }, t('How')), level),
      h('div', { class: 'field' }, h('label', { for: 'explain-focus' }, t('Anything to focus on? (optional)')), focus),
      progress,
      message,
      h('div', { class: 'button-row' }, go, stop),
    ),
    result,
  );
  el.dispose = () => job?.then((un) => un());
  return el;
}
