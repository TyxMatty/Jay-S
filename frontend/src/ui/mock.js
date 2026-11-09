// Mock test (stage 4): one of the exams against the clock, always in exam mode; and the history of
// every finished attempt, each with its result and Jay-AI's analysis.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, listen, isTauri } from '../core/bridge.js';
import { t, language } from '../core/i18n.js';
import { createViews } from './exams.js';
import { studyFailureText } from './study-text.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short', timeStyle: 'short' }).format(ms);
const fmt = (n) => new Intl.NumberFormat(language(), { maximumFractionDigits: 1 }).format(n);
/** Minutes per question by style: ENEM's 5 h 30 min for 90 questions is ~3.5 min; UEPG's 4 h for 45 + essay, ~4. */
const PACE = { enem: 3.5, uepg: 4, uem: 4 };

export function createMockPanel() {
  const views = createViews(home);
  function home() {
    const exam = h('select', { class: 'input', id: 'mock-exam' });
    const minutes = h('input', { class: 'input', type: 'number', id: 'mock-minutes', min: 1, max: 600, value: 30 });
    const message = h('p', { class: 'hint', role: 'status' });
    const start = h('button', { type: 'button', class: 'btn', disabled: !isTauri }, h('span', { class: 'btn-icon', html: icons.timer }), t('Start the mock test'));
    const history = h('ul', { class: 'material-list' });
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
      const [list, attempts] = await Promise.all([invoke('exams_list'), invoke('attempts_list')]);
      exams = list;
      exam.replaceChildren(...(list.length ? list.map((e) => h('option', { value: e.id }, `${e.title} (${t('{n} questions', { n: e.questions })})`)) : [h('option', { value: '' }, t('No exams yet'))]));
      exam.dispatchEvent(new Event('change'));
      const done = attempts.filter((a) => a.finished);
      history.replaceChildren(
        ...done.map((a) =>
          h('li', { class: 'material-card' }, h('div', { class: 'material-row' },
            h('div', { class: 'material-main' },
              h('h3', { class: 'material-name' }, a.title),
              h('p', { class: 'material-meta' }, [t('{p} of {total}', { p: fmt(a.points ?? 0), total: a.total }), a.mode === 'learn' ? t('Learning mode') : a.time_limit ? t('Mock test') : t('Exam mode'), date(a.started), a.analyzed ? t('analyzed by Jay-AI') : t('waiting for Jay-AI')].join(' · ')),
            ),
            h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => views.result(a.id).catch((e) => (message.textContent = studyFailureText(e))) }, t('See the result')),
            h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete {name}', { name: a.title }), title: t('Delete'), html: icons.trash, onClick: async () => {
              if (!confirm(t('Delete this result?'))) return;
              await invoke('attempt_delete', { id: a.id }).catch((e) => (message.textContent = studyFailureText(e)));
              load();
            } }),
          )),
        ),
      );
      historyEmpty.hidden = done.length > 0;
    }
    // A finished analysis updates the "waiting for Jay-AI" marks.
    const stop = listen('study-job', (e) => e.kind === 'analysis' && e.state !== 'progress' && load());

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
      h('fieldset', {}, cardTitle(icons.info, t('Results')), historyEmpty, history),
    );
    el.dispose = () => stop.then((un) => un());
    return el;
  }
  views.home();
  return views.host;
}
