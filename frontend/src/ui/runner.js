// Taking an exam (stage 4), shared by Exams and Mock test: one question at a time with a navigator,
// X (one choice) or sum-type (check the propositions, the sum shows by itself); exam mode saves each
// answer and shows nothing until the end, learning mode grades each one at once. Rust keeps the
// answers (src-tauri/src/study.rs); everything from the exam is shown as text.
import { h } from './dom.js';
import { icons } from './icons.js';
import { invoke, listen } from '../core/bridge.js';
import { t, language } from '../core/i18n.js';
import { studyFailureText } from './study-text.js';

const fmt = (n) => new Intl.NumberFormat(language(), { maximumFractionDigits: 1 }).format(n);
const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

/** The marked answer as the page keeps it: a letter, or the set of proposition values. */
function sumOf(set) {
  return [...set].reduce((a, b) => a + b, 0);
}

/**
 * @param {{ attempt: object, questions: object[], graded: (object|null)[] }} started
 * @param {(attempt: object) => void} onFinished the finished attempt (with its result)
 */
export function createRunner(started, onFinished) {
  const { attempt, questions } = started;
  const learn = attempt.mode === 'learn';
  const answers = attempt.answers.slice();
  const graded = started.graded.slice();
  let current = Math.max(0, answers.findIndex((a) => a == null));
  let shownAt = performance.now();
  let finished = false;

  const status = h('p', { class: 'hint', role: 'status' });
  const timer = h('span', { class: 'runner-timer', hidden: !attempt.time_limit });
  const nav = h('div', { class: 'runner-nav', role: 'list', 'aria-label': t('Questions') });
  const body = h('div', { class: 'runner-question' });
  const prev = h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => go(current - 1) }, t('Previous'));
  const next = h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => go(current + 1) }, t('Next'));
  const finish = h('button', { type: 'button', class: 'btn', onClick: () => end(false) }, t('Finish'));

  function elapsed() {
    const ms = performance.now() - shownAt;
    shownAt = performance.now();
    return Math.round(ms);
  }

  function renderNav() {
    nav.replaceChildren(
      ...questions.map((q, i) => {
        const g = graded[i];
        const state = g ? (g.points >= 1 ? 'right' : g.points > 0 ? 'partial' : 'wrong') : answers[i] != null ? 'answered' : 'open';
        return h('button', { type: 'button', role: 'listitem', class: 'runner-dot', 'data-state': state, 'aria-current': i === current ? 'true' : null, 'aria-label': t('Question {n}', { n: i + 1 }), onClick: () => go(i) }, String(i + 1));
      }),
    );
  }

  async function save(i, marked) {
    try {
      const g = await invoke('attempt_answer', { id: attempt.id, index: i, marked, ms: elapsed() });
      answers[i] = marked;
      if (g) graded[i] = g;
      status.textContent = '';
    } catch (e) {
      status.textContent = studyFailureText(e);
      if (e?.code === 'time_up') end(true);
    }
    renderNav();
  }

  function feedback(q, g) {
    if (!g) return null;
    const right = g.points >= 1;
    return h(
      'div',
      { class: 'runner-feedback', 'data-tone': right ? 'right' : g.points > 0 ? 'partial' : 'wrong' },
      h('span', { class: 'feedback-symbol', 'aria-hidden': 'true' }, right ? '✓' : g.points > 0 ? '½' : '✗'),
      h('strong', {}, right ? t('Right!') : g.points > 0 ? t('Partly right ({p} of 1).', { p: fmt(g.points) }) : t('Not this time.')),
      ' ',
      t('Answer: {answer}', { answer: g.answer ?? '?' }),
      g.answer_by_ai ? h('span', { class: 'badge', 'data-status': 'check' }, t('answer by Jay-AI')) : null,
      g.explanation ? h('p', {}, g.explanation) : null,
    );
  }

  function renderQuestion() {
    const q = questions[current];
    const g = graded[current];
    const locked = learn && g != null;
    const header = h('p', { class: 'runner-meta' }, t('Question {n} of {total}', { n: current + 1, total: questions.length }), q.topic ? ` · ${q.topic}` : '');
    const passage = q.passage ? h('blockquote', { class: 'runner-passage' }, q.passage) : null;
    const stem = h('p', { class: 'runner-stem' }, q.stem);
    let options;
    let confirm = null;
    if (q.kind === 'choice') {
      const name = `q-${current}`;
      options = h(
        'div',
        { class: 'runner-options', role: 'radiogroup', 'aria-label': t('Choices') },
        q.options.map((o) => {
          const input = h('input', { type: 'radio', name, value: o.label, checked: answers[current] === o.label, disabled: locked });
          const mark = g && (o.label === g.answer ? 'right' : o.label === g.marked ? 'wrong' : null);
          return h('label', { class: 'runner-option', 'data-mark': mark }, input, h('span', { class: 'runner-label' }, o.label), h('span', {}, o.text));
        }),
      );
      options.addEventListener('change', (e) => {
        if (learn) {
          confirm.disabled = false;
        } else {
          save(current, e.target.value);
        }
      });
      if (learn && !locked) {
        confirm = h('button', { type: 'button', class: 'btn', disabled: true, onClick: async () => {
          const value = options.querySelector('input:checked')?.value;
          if (value) {
            await save(current, value);
            renderQuestion();
          }
        } }, t('Check'));
      }
    } else {
      const chosen = new Set(answers[current] ? q.options.map((o) => Number(o.label)).filter((v) => Number(answers[current]) & v) : []);
      const right = g ? Number(g.answer) : 0;
      const sum = h('p', { class: 'runner-sum', 'aria-live': 'polite' });
      const showSum = () => (sum.textContent = t('Sum: {sum}', { sum: String(sumOf(chosen)).padStart(2, '0') }));
      options = h(
        'div',
        { class: 'runner-options', role: 'group', 'aria-label': t('Propositions') },
        q.options.map((o) => {
          const v = Number(o.label);
          const input = h('input', { type: 'checkbox', value: o.label, checked: chosen.has(v), disabled: locked });
          input.addEventListener('change', () => {
            input.checked ? chosen.add(v) : chosen.delete(v);
            showSum();
            if (!learn) save(current, chosen.size ? String(sumOf(chosen)).padStart(2, '0') : null);
          });
          const mark = g && ((right & v) && (Number(g.marked) & v) ? 'right' : right & v ? 'missed' : Number(g.marked) & v ? 'wrong' : null);
          return h('label', { class: 'runner-option', 'data-mark': mark }, input, h('span', { class: 'runner-label' }, o.label), h('span', {}, o.text));
        }),
      );
      showSum();
      options.append(sum);
      if (learn && !locked) {
        confirm = h('button', { type: 'button', class: 'btn', onClick: async () => {
          if (!chosen.size) return (status.textContent = t('Mark at least one proposition.'));
          await save(current, String(sumOf(chosen)).padStart(2, '0'));
          renderQuestion();
        } }, t('Check'));
      }
    }
    body.replaceChildren(header, passage ?? '', stem, options, confirm ?? '', feedback(q, g) ?? '');
    prev.disabled = current === 0;
    next.disabled = current === questions.length - 1;
    renderNav();
  }

  function go(i) {
    if (i < 0 || i >= questions.length) return;
    // Time on a question counts when it's answered; moving away without answering drops it.
    elapsed();
    current = i;
    renderQuestion();
    body.querySelector('input')?.focus();
  }

  async function end(timeUp) {
    if (finished) return;
    const open = answers.filter((a) => a == null).length;
    if (!timeUp && open && !confirm(t('{n} questions are still blank. Finish anyway?', { n: open }))) return;
    finished = true;
    clearInterval(tick);
    try {
      onFinished(await invoke('attempt_finish', { id: attempt.id }));
    } catch (e) {
      finished = false;
      status.textContent = studyFailureText(e);
    }
  }

  let tick = null;
  if (attempt.time_limit) {
    const endsAt = attempt.started + attempt.time_limit * 1000;
    const update = () => {
      const left = Math.max(0, (endsAt - Date.now()) / 1000);
      timer.textContent = `${t('Time left')}: ${clock(left)}`;
      timer.dataset.tone = left < 300 ? 'low' : '';
      if (left <= 0) end(true);
    };
    update();
    tick = setInterval(update, 1000);
  }

  renderQuestion();
  const el = h(
    'section',
    { class: 'app-panel is-single runner' },
    h('div', { class: 'panel-head' }, h('h1', {}, attempt.title), h('span', { class: 'badge', 'data-status': learn ? 'ok' : 'check' }, learn ? t('Learning mode') : t('Exam mode')), timer),
    nav,
    h('div', { class: 'exam-sheet' }, body),
    status,
    h('div', { class: 'button-row runner-actions' }, prev, next, finish),
  );
  el.dispose = () => clearInterval(tick);
  return el;
}

/** The score as a ring filled to the share of points, colored by how it went (and said in words). */
function scoreHero(r, attempt) {
  const share = r.total ? r.points / r.total : 0;
  const tone = share >= 0.7 ? 'right' : share >= 0.4 ? 'partial' : 'wrong';
  const verdict = { right: t('Great result!'), partial: t('Halfway there.'), wrong: t('Lots to review: Jay-AI says where to start.') }[tone];
  return h(
    'div',
    { class: 'score-hero', 'data-tone': tone },
    h('div', { class: 'score-ring', style: `--share:${share}`, role: 'img', 'aria-label': t('{p} of {total}', { p: fmt(r.points), total: r.total }) }, h('span', {}, `${Math.round(share * 100)}%`)),
    h('div', {}, h('p', { class: 'result-score' }, t('{p} of {total}', { p: fmt(r.points), total: r.total })), h('p', { class: 'score-verdict' }, verdict), h('p', { class: 'hint' }, attempt.mode === 'learn' ? t('Learning mode') : attempt.time_limit ? t('Mock test') : t('Exam mode'))),
  );
}

/** The result of a finished attempt: score, per topic, per question, and Jay-AI's analysis. */
export function createResult(attempt, questions, { onBack }) {
  const r = attempt.result;
  const analysis = h('div', { class: 'analysis', 'aria-live': 'polite' });

  function renderAnalysis(a) {
    if (!a) {
      analysis.replaceChildren(h('p', { class: 'hint' }, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Your result went to Jay-AI. The analysis shows up here in a moment.')));
      return;
    }
    const list = (items) => h('ul', {}, (items ?? []).map((x) => h('li', {}, x)));
    analysis.replaceChildren(
      h('p', {}, a.summary ?? ''),
      a.strengths?.length ? h('h4', {}, t('What went well')) : '',
      a.strengths?.length ? list(a.strengths) : '',
      a.review?.length ? h('h4', {}, t('What to review')) : '',
      a.review?.length ? h('ul', {}, a.review.map((x) => h('li', {}, h('strong', {}, x.topic ?? ''), ': ', x.why ?? '', ' ', h('em', {}, x.tip ?? '')))) : '',
      a.next?.length ? h('h4', {}, t('Next steps')) : '',
      a.next?.length ? list(a.next) : '',
    );
  }

  renderAnalysis(attempt.analysis);
  const stop = attempt.analysis
    ? null
    : listen('study-job', async (e) => {
        if (e.kind !== 'analysis' || e.id !== attempt.id) return;
        if (e.state === 'done') {
          const [fresh] = await invoke('attempt_result', { id: attempt.id });
          renderAnalysis(fresh.analysis);
        } else if (e.state === 'error') {
          analysis.replaceChildren(h('p', { class: 'hint', 'data-tone': 'error' }, studyFailureText(e.error)));
        }
      });

  const topics = h(
    'ul',
    { class: 'topic-bars' },
    r.topics.map((tp) => {
      const share = tp.total ? tp.points / tp.total : 0;
      return h('li', { 'data-tone': share >= 0.7 ? 'right' : share >= 0.4 ? 'partial' : 'wrong' }, h('span', {}, tp.topic), h('span', { class: 'topic-bar', style: `--share:${share}` }), h('span', { class: 'hint' }, t('{p} of {total}', { p: fmt(tp.points), total: tp.total })));
    }),
  );
  const review = h(
    'ol',
    { class: 'review-list' },
    r.questions.map((g, i) => {
      const q = questions[i];
      const tone = g.points >= 1 ? 'right' : g.points > 0 ? 'partial' : 'wrong';
      return h(
        'li',
        { 'data-tone': tone },
        h('details', {},
          h('summary', {}, h('span', { class: 'review-mark' }, tone === 'right' ? '✓' : tone === 'partial' ? '½' : '✗'), ' ', (q?.stem ?? '').slice(0, 140)),
          q?.passage ? h('blockquote', { class: 'runner-passage' }, q.passage) : '',
          q ? h('p', {}, q.stem) : '',
          q ? h('ul', { class: 'review-options' }, q.options.map((o) => h('li', {}, h('strong', {}, o.label), ' ', o.text))) : '',
          h('p', {}, t('You marked: {marked}', { marked: g.marked ?? t('blank') }), ' · ', t('Answer: {answer}', { answer: g.answer ?? '?' }), g.answer_by_ai ? h('span', { class: 'badge', 'data-status': 'check' }, t('answer by Jay-AI')) : ''),
          g.explanation ? h('p', { class: 'hint' }, g.explanation) : '',
        ),
      );
    }),
  );
  const el = h(
    'section',
    { class: 'app-panel is-single result' },
    h('div', { class: 'panel-head' }, h('h1', {}, attempt.title), h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => { el.dispose(); onBack(); } }, t('Back'))),
    scoreHero(r, attempt),
    h('fieldset', {}, h('h3', { class: 'card-title' }, h('span', { class: 'card-icon', html: icons.spark, 'aria-hidden': 'true' }), t("Jay-AI's analysis")), analysis),
    h('fieldset', {}, h('h3', {}, t('By topic')), topics),
    h('fieldset', {}, h('h3', {}, t('Question by question')), review),
  );
  el.dispose = () => stop?.then?.((un) => un());
  return el;
}
