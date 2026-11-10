// Mock test (stage 4): one of the exams against the clock, always in exam mode; and the history of
// every finished attempt, each with its result and Jay-AI's analysis.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, isTauri } from '../core/bridge.js';
import { t } from '../core/i18n.js';
import { createViews, createHistory } from './exams.js';
import { studyFailureText } from './study-text.js';

/** Minutes per question by style: ENEM's 5 h 30 min for 90 questions is ~3.5 min; UEPG's 4 h for 45 + essay, ~4. */
const PACE = { enem: 3.5, uepg: 4, uem: 4 };

export function createMockPanel() {
  const views = createViews(home);
  function home() {
    const exam = h('select', { class: 'input', id: 'mock-exam' });
    const minutes = h('input', { class: 'input', type: 'number', id: 'mock-minutes', min: 1, max: 600, value: 30 });
    const message = h('p', { class: 'hint', role: 'status' });
    const start = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.timer }), t('Start the mock test'));
    const history = createHistory(views, message);
    let exams = [];

    exam.addEventListener('change', () => {
      const e = exams.find((x) => x.id === exam.value);
      if (e) minutes.value = Math.max(5, Math.round(e.questions * (PACE[e.style] ?? 3)));
    });

    start.addEventListener('click', async () => {
      if (!exam.value) return (message.textContent = t('Build an exam in Exams first.'));
      try {
        views.run(await invoke('attempt_start', { exam: exam.value, mode: 'exam', minutes: Math.max(1, Number(minutes.value) || 30) }));
      } catch (e) {
        message.textContent = studyFailureText(e);
      }
    });

    async function load() {
      if (!isTauri) return;
      const list = await invoke('exams_list');
      exams = list;
      exam.replaceChildren(...(list.length ? list.map((e) => h('option', { value: e.id }, `${e.title} (${t('{n} questions', { n: e.questions })})`)) : [h('option', { value: '' }, t('No exams yet'))]));
      exam.dispatchEvent(new Event('change'));
      historyEmpty.hidden = (await history.load()) > 0;
    }

    const historyEmpty = h('p', { class: 'hint' }, t('Your results show up here, each with what Jay-AI made of it.'));
    load();
    const el = h(
      'section',
      { class: 'app-panel is-single' },
      h('h1', {}, t('Mock test')),
      h(
        'fieldset',
        {},
        cardTitle(icons.timer, t('New mock test')),
        h('p', { class: 'hint' }, t('Take an exam against the clock, as long as you want, then see what you got right and what to review.')),
        h('div', { class: 'field' }, h('label', { for: 'mock-exam' }, t('Exam')), exam),
        h('div', { class: 'field' }, h('label', { for: 'mock-minutes' }, t('Time (minutes)')), minutes),
        message,
        h('div', { class: 'button-row' }, start),
      ),
      h('fieldset', {}, cardTitle(icons.info, t('Results')), historyEmpty, history.list),
    );
    el.dispose = history.dispose;
    return el;
  }
  views.home();
  return views.host;
}
