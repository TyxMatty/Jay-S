// Exams (stage 3): build an exam (a new one from content, or questions reused from an exam mixed
// with new ones), list them, and take one in exam mode or learning mode (ui/runner.js). The
// building runs in Rust with Jay-AI (src-tauri/src/study.rs) and reports through `study-job`.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, listen, listenJobs, isTauri } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { createRunner, createResult } from './runner.js';
import { studyFailureText } from './study-text.js';
import { createTabs } from './tabs.js';
import { createReadyMade } from './ready-made.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short', timeStyle: 'short' }).format(ms);
const fmt = (n) => new Intl.NumberFormat(language(), { maximumFractionDigits: 1 }).format(n);

/**
 * The finished attempts, newest first, each opening its result; one Jay-AI hasn't analyzed says
 * whether it is being analyzed right now. Shared by Exams and Mock test. `box` is shown only when
 * there is something to list.
 */
export function createHistory(views, message, box) {
  const list = h('ul', { class: 'material-list' });
  let analyzing = false;
  async function load() {
    if (!isTauri) return;
    const done = (await invoke('attempts_list')).filter((a) => a.finished).sort((a, b) => b.started - a.started);
    const state = (a) =>
      a.analyzed ? t('analyzed by Jay-AI') : analyzing ? h('span', {}, h('span', { class: 'spinner', 'aria-hidden': 'true' }), ' ', t('Jay-AI is analyzing…')) : t('waiting for Jay-AI');
    list.replaceChildren(
      ...done.map((a) =>
        h('li', { class: 'material-card' }, h('div', { class: 'material-row' },
          h('div', { class: 'material-main' },
            h('h3', { class: 'material-name' }, a.title),
            h('p', { class: 'material-meta' }, [t('{p} of {total}', { p: fmt(a.points ?? 0), total: a.total }), a.mode === 'learn' ? t('Learning mode') : a.time_limit ? t('Mock test') : t('Exam mode'), date(a.started)].join(' · '), ' · ', state(a)),
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
    if (box) box.hidden = done.length === 0;
    return done.length;
  }
  // An analysis starting or ending changes the marks.
  const stop = listen('study-job', (e) => {
    if (e.kind !== 'analysis') return;
    analyzing = e.state === 'progress';
    load();
  });
  if (isTauri) invoke('job_status').then((j) => j?.kind === 'analysis' && ((analyzing = true), load())).catch(() => {});
  return { list, load, dispose: () => stop.then((un) => un()) };
}

/** A section that shows one view at a time (the list, a runner, a result). */
export function createViews(home) {
  const host = h('div', { class: 'lazy-panel' });
  let current = null;
  const show = (view) => {
    current?.dispose?.();
    current = view;
    host.replaceChildren(view);
    host.closest('.app-panels')?.scrollTo?.(0, 0);
  };
  return {
    host,
    home: () => show(home()),
    run(started) {
      show(createRunner(started, async (finished) => {
        const [attempt, questions] = await invoke('attempt_result', { id: finished.id });
        show(createResult(attempt, questions, { onBack: () => show(home()) }));
      }, () => show(home())));
    },
    async result(id) {
      const [attempt, questions] = await invoke('attempt_result', { id });
      show(createResult(attempt, questions, { onBack: () => show(home()) }));
    },
  };
}

function builder({ onBuilt, onError }) {
  const title = h('input', { class: 'input', id: 'exam-title', value: t('My exam'), maxlength: 120 });
  const style = h('select', { class: 'input', id: 'exam-style' });
  const styleHint = h('p', { class: 'hint' });
  const mode = h(
    'div',
    { class: 'segmented', role: 'radiogroup', 'aria-label': t('How') },
    h('label', {}, h('input', { type: 'radio', name: 'exam-mode', value: 'new', checked: true }), t('New, from my content')),
    h('label', {}, h('input', { type: 'radio', name: 'exam-mode', value: 'reuse' }), t('Reuse exams')),
  );
  // Several exams at once: each with what Jay-S found in it (questions, its own answer key).
  const examList = h('div', { class: 'check-list', role: 'group', 'aria-label': t('Exams to reuse') });
  const examInfo = h('p', { class: 'hint' });
  const reuse = h('input', { class: 'input', type: 'number', id: 'exam-reuse', min: 1, max: 200, value: 10 });
  const fresh = h('input', { class: 'input', type: 'number', id: 'exam-new', min: 0, max: 50, value: 10 });
  const contentList = h('div', { class: 'check-list', role: 'group', 'aria-label': t('Content') });
  const progress = h('progress', { class: 'ai-progress', max: 1, value: 0, hidden: true });
  const message = h('p', { class: 'hint', role: 'status' });
  const build = h('button', { type: 'button', class: 'btn' }, h('span', { class: 'btn-icon', html: icons.spark }), t('Build the exam'));
  const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));
  const reuseFields = h(
    'div',
    { class: 'field', hidden: true },
    h('span', { class: 'field-label' }, t('Exams to reuse (one or more)')),
    examList,
    examInfo,
    h('label', { for: 'exam-reuse' }, t('How many of their questions, in all')),
    reuse,
  );
  let styles = [];
  let materials = [];
  /** exam id → what exam_material_info said. */
  const infos = new Map();

  const currentMode = () => mode.querySelector('input:checked').value;
  const chosenExams = () => [...examList.querySelectorAll('input:checked')].map((i) => i.value);
  const render = () => {
    reuseFields.hidden = currentMode() !== 'reuse';
    const s = styles.find((x) => x.id === style.value);
    styleHint.textContent = s ? t(s.description) : '';
  };
  mode.addEventListener('change', render);
  style.addEventListener('change', render);

  /** The questions of the checked exams, in all, and the style most of them have. */
  function summarize() {
    const picked = chosenExams().map((id) => infos.get(id)).filter(Boolean);
    const total = picked.reduce((n, i) => n + i.questions, 0);
    const sums = picked.reduce((n, i) => n + i.sum, 0);
    examInfo.textContent = picked.length ? t('{n} questions in all from {exams} exams', { n: total, exams: picked.length }) : '';
    reuse.max = total || 1;
    reuse.value = Math.min(total, Math.max(Number(reuse.value) || 0, 10)) || 1;
    if (total) style.value = sums > total / 2 ? 'uepg' : 'enem';
    render();
  }

  async function describe(row, id) {
    const line = row.querySelector('.check-note');
    try {
      const info = await invoke('exam_material_info', { id });
      infos.set(id, info);
      const key = info.key ? materials.find((m) => m.id === info.key)?.name : null;
      line.textContent = [t('{n} questions found', { n: info.questions }), key ? t('answer key: {name}', { name: key }) : t('no answer key: Jay-AI answers them (marked as such)')].join(' · ');
    } catch (e) {
      line.textContent = studyFailureText(e);
    }
    summarize();
  }

  async function refresh() {
    [styles, materials] = await Promise.all([invoke('styles_list'), invoke('materials_list')]);
    style.replaceChildren(...styles.map((s) => h('option', { value: s.id }, s.name)));
    const exams = materials.filter((m) => m.kind === 'exam' || m.kind === 'both');
    infos.clear();
    examList.replaceChildren(
      ...(exams.length
        ? exams.map((m) => {
            const input = h('input', { type: 'checkbox', value: m.id });
            const row = h('label', { class: 'check-row' }, input, h('span', { class: 'check-text' }, h('span', {}, m.name), h('span', { class: 'check-note hint' })));
            input.addEventListener('change', () => (input.checked && !infos.has(m.id) ? describe(row, m.id) : summarize()));
            return row;
          })
        : [h('p', { class: 'hint' }, t('Attach an exam in Materials first'))]),
    );
    const contents = materials.filter((m) => m.kind === 'content' || m.kind === 'both');
    contentList.replaceChildren(
      ...(contents.length
        ? contents.map((m) => h('label', { class: 'check-row' }, h('input', { type: 'checkbox', value: m.id, checked: true }), m.name))
        : [h('p', { class: 'hint' }, t('No content yet: attach it in Materials.'))]),
    );
    render();
  }

  function running(on) {
    build.disabled = on;
    stop.hidden = !on;
    progress.hidden = !on;
  }

  build.addEventListener('click', async () => {
    message.textContent = '';
    const reusing = currentMode() === 'reuse';
    const plan = {
      title: title.value.trim() || t('My exam'),
      style: style.value,
      content: [...contentList.querySelectorAll('input:checked')].map((i) => i.value),
      exams: reusing ? chosenExams() : [],
      reuse: reusing ? Number(reuse.value) || 0 : 0,
      new: Math.max(0, Number(fresh.value) || 0),
    };
    if (reusing && !plan.exams.length) return (message.textContent = t('Check at least one exam to reuse.'));
    if (plan.new > 0 && !plan.content.length) return (message.textContent = t('Pick some content for the new questions, or set them to 0.'));
    if (plan.new > 0 && !aiReady()) return (message.textContent = t('New questions need Jay-AI: download it in the Jay-AI section.'));
    try {
      await invoke('exam_build', { plan });
      running(true);
      progress.removeAttribute('value');
      message.textContent = t('Reading the materials…');
    } catch (e) {
      message.textContent = studyFailureText(e);
      // Already building (this screen was rebuilt meanwhile): show that job instead.
      if (e?.code === 'busy') invoke('job_status').then((s) => s && onJob(s));
    }
  });

  function onJob(e) {
    if (e.kind !== 'exam') return;
    if (e.state === 'progress') {
      running(true);
      const s = e.step;
      if (s.step === 'writing' || s.step === 'solving') {
        progress.max = s.total || 1;
        progress.value = s.done;
      } else {
        progress.removeAttribute('value');
      }
      message.textContent = {
        reading: t('Reading the materials…'),
        solving: t('Jay-AI is answering the questions without a key… {done} of {total}', { done: s.done, total: s.total }),
        tagging: t('Jay-AI is naming the topics…'),
        writing: t('Jay-AI is writing and checking question {n} of {total}…', { n: s.done + 1, total: s.total }),
      }[s.step];
      return;
    }
    running(false);
    if (e.state === 'done') {
      message.textContent = t('Ready!');
      onBuilt(e.id);
    } else {
      message.textContent = studyFailureText(e.error);
      onError?.(e.error);
    }
  }

  // A build started before this screen existed (another tab, Back from an exam) shows here too.
  const stopListening = listenJobs(onJob);

  refresh();
  const el = h(
    'fieldset',
    { class: 'builder' },
    cardTitle(icons.exam, t('New exam')),
    h('div', { class: 'field' }, h('label', { for: 'exam-title' }, t('Name')), title),
    h('div', { class: 'field' }, h('label', { for: 'exam-style' }, t('Style')), style, styleHint),
    h('div', { class: 'field' }, h('span', { class: 'field-label' }, t('How')), mode),
    reuseFields,
    h('div', { class: 'field' }, h('span', { class: 'field-label' }, t('Content for new questions (one or more)')), contentList, h('label', { for: 'exam-new' }, t('How many new questions (Jay-AI writes and checks each one)')), fresh),
    progress,
    message,
    h('div', { class: 'button-row' }, build, stop),
  );
  el.refresh = refresh;
  el.dispose = () => stopListening.then((un) => un());
  return el;
}

export function createExamsPanel() {
  const views = createViews(home);
  function home() {
    const list = h('ul', { class: 'material-list' });
    const going = h('ul', { class: 'material-list' });
    const message = h('p', { class: 'hint', 'data-tone': 'error', role: 'alert' });
    const make = builder({ onBuilt: () => load() });
    make.hidden = true;
    // A build in progress: open the builder so its progress shows.
    invoke('job_status').then((j) => j?.kind === 'exam' && (make.hidden = false)).catch(() => {});
    const newButton = h('button', { type: 'button', class: 'btn', disabled: !isTauri, onClick: () => { tabs.select('mine'); make.hidden = !make.hidden; if (!make.hidden) make.refresh(); } }, h('span', { class: 'btn-icon', html: icons.exam }), t('New exam'));

    async function start(id, mode) {
      try {
        views.run(await invoke('attempt_start', { exam: id, mode, minutes: null }));
      } catch (e) {
        message.textContent = studyFailureText(e);
      }
    }

    async function load() {
      if (!isTauri) return;
      const [exams, attempts] = await Promise.all([invoke('exams_list'), invoke('attempts_list')]);
      list.replaceChildren(
        ...exams.map((e) =>
          h(
            'li',
            { class: 'material-card' },
            h(
              'div',
              { class: 'material-row' },
              h('span', { class: 'card-icon', html: icons.exam, 'aria-hidden': 'true' }),
              h('div', { class: 'material-main' }, h('h3', { class: 'material-name' }, e.title), h('p', { class: 'material-meta' }, [e.style.toUpperCase(), t('{n} questions', { n: e.questions }), e.reused ? t('{n} reused', { n: e.reused }) : null, e.by_ai ? t('{n} answered by Jay-AI', { n: e.by_ai }) : null, date(e.created)].filter(Boolean).join(' · '))),
              h('div', { class: 'material-actions' },
                h('button', { type: 'button', class: 'btn', onClick: () => start(e.id, 'exam') }, t('Take the exam')),
                h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => start(e.id, 'learn') }, t('Learning mode')),
                h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Delete {name}', { name: e.title }), title: t('Delete'), html: icons.trash, onClick: async () => {
                  if (!confirm(t('Delete the exam "{name}"? Your results stay.', { name: e.title }))) return;
                  await invoke('exam_delete', { id: e.id }).catch((err) => (message.textContent = studyFailureText(err)));
                  load();
                } }),
              ),
            ),
          ),
        ),
      );
      const open = attempts.filter((a) => !a.finished);
      going.replaceChildren(
        ...open.map((a) =>
          h('li', { class: 'material-card' }, h('div', { class: 'material-row' },
            h('div', { class: 'material-main' }, h('h3', { class: 'material-name' }, a.title), h('p', { class: 'material-meta' }, [a.mode === 'learn' ? t('Learning mode') : t('Exam mode'), t('{n} of {total} answered', { n: a.answered, total: a.total }), date(a.started)].join(' · '))),
            h('button', { type: 'button', class: 'btn', onClick: async () => {
              try {
                views.run(await invoke('attempt_resume', { id: a.id }));
              } catch (err) {
                message.textContent = studyFailureText(err);
              }
            } }, t('Continue')),
            h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('Discard {name}', { name: a.title }), title: t('Discard'), html: icons.trash, onClick: async () => {
              if (!confirm(t('Discard this attempt? Your answers are lost.'))) return;
              await invoke('attempt_delete', { id: a.id }).catch((err) => (message.textContent = studyFailureText(err)));
              load();
            } }),
          )),
        ),
      );
      goingBox.hidden = open.length === 0;
      empty.hidden = exams.length > 0;
    }

    const empty = h('div', { class: 'empty-state' }, h('span', { class: 'card-icon', html: icons.exam, 'aria-hidden': 'true' }), h('p', {}, t('A new exam from your content in the style you pick (ENEM, UEPG, UEM…), or the questions of your exam mixed with your content. Answer it right here, and see the result at the end or each answer at once in learning mode.')), h('p', { class: 'hint' }, isTauri ? t('Start with "New exam".') : t('Only in the app.')));
    const goingBox = h('div', { hidden: true }, h('h2', { class: 'section-title' }, t('In progress')), going);
    const resultsBox = h('div', { hidden: true }, h('h2', { class: 'section-title' }, t('Results')));
    const history = createHistory(views, message, resultsBox);
    resultsBox.append(history.list);
    load();
    history.load();
    const mine = h('div', {}, make, goingBox, empty, list, resultsBox);
    const ready = createReadyMade({ onOpen: (id) => start(id, 'exam'), onChange: () => load() });
    const tabs = createTabs({
      idPrefix: 'exams',
      label: t('Kinds of exams'),
      className: 'material-tabs',
      items: [
        { key: 'mine', label: h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: icons.exam }), t('My exams')), panel: mine },
        { key: 'ready', label: h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: icons.download }), t('Ready-made exams')), panel: ready },
      ],
    });
    tabs.select('mine');
    const section = h('section', { class: 'app-panel is-single' }, h('div', { class: 'panel-head' }, h('h1', {}, t('Exams')), newButton), message, tabs.list, mine, ready);
    section.dispose = () => {
      make.dispose?.();
      ready.dispose();
      history.dispose();
    };
    return section;
  }
  views.home();
  return views.host;
}
