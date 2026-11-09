// Exams (stage 3): build an exam (a new one from content, or questions reused from an exam mixed
// with new ones), list them, and take one in exam mode or learning mode (ui/runner.js). The
// building runs in Rust with Jay-AI (src-tauri/src/study.rs) and reports through `study-job`.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { invoke, listen, isTauri } from '../core/bridge.js';
import { aiReady } from '../core/ai.js';
import { t, language } from '../core/i18n.js';
import { createRunner, createResult } from './runner.js';
import { studyFailureText } from './study-text.js';

const date = (ms) => new Intl.DateTimeFormat(language(), { dateStyle: 'short', timeStyle: 'short' }).format(ms);

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
      }));
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
    h('label', {}, h('input', { type: 'radio', name: 'exam-mode', value: 'reuse' }), t('Reuse an exam')),
  );
  const examSelect = h('select', { class: 'input', id: 'exam-source' });
  const examInfo = h('p', { class: 'hint' });
  const reuse = h('input', { class: 'input', type: 'number', id: 'exam-reuse', min: 1, max: 200, value: 10 });
  const fresh = h('input', { class: 'input', type: 'number', id: 'exam-new', min: 0, max: 50, value: 10 });
  const contentList = h('div', { class: 'check-list', role: 'group', 'aria-label': t('Content') });
  const progress = h('progress', { class: 'ai-progress', max: 1, value: 0, hidden: true });
  const message = h('p', { class: 'hint', role: 'status' });
  const build = h('button', { type: 'button', class: 'btn' }, h('span', { class: 'btn-icon', html: icons.spark }), t('Build the exam'));
  const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('job_cancel') }, t('Stop'));
  const reuseFields = h('div', { class: 'field', hidden: true }, h('label', { for: 'exam-source' }, t('Exam to reuse')), examSelect, examInfo, h('label', { for: 'exam-reuse' }, t('How many of its questions')), reuse);
  let styles = [];
  let materials = [];
  let info = null;

  const currentMode = () => mode.querySelector('input:checked').value;
  const render = () => {
    reuseFields.hidden = currentMode() !== 'reuse';
    const s = styles.find((x) => x.id === style.value);
    styleHint.textContent = s ? t(s.description) : '';
  };
  mode.addEventListener('change', render);
  style.addEventListener('change', render);

  async function loadInfo() {
    info = null;
    examInfo.textContent = '';
    if (!examSelect.value) return;
    try {
      info = await invoke('exam_material_info', { id: examSelect.value });
      const key = info.key ? materials.find((m) => m.id === info.key)?.name : null;
      examInfo.textContent = [
        t('{n} questions found', { n: info.questions }),
        key ? t('answer key: {name}', { name: key }) : t('no answer key: Jay-AI answers them (marked as such)'),
      ].join(' · ');
      reuse.value = Math.min(info.questions, 20) || 1;
      reuse.max = info.questions || 1;
      if (info.sum > info.questions / 2) style.value = 'uepg';
      else if (info.questions) style.value = 'enem';
      render();
    } catch (e) {
      examInfo.textContent = studyFailureText(e);
    }
  }
  examSelect.addEventListener('change', loadInfo);

  async function refresh() {
    [styles, materials] = await Promise.all([invoke('styles_list'), invoke('materials_list')]);
    style.replaceChildren(...styles.map((s) => h('option', { value: s.id }, s.name)));
    const exams = materials.filter((m) => m.kind === 'exam' || m.kind === 'both');
    examSelect.replaceChildren(h('option', { value: '' }, exams.length ? t('Pick one') : t('Attach an exam in Materials first')), ...exams.map((m) => h('option', { value: m.id }, m.name)));
    const contents = materials.filter((m) => m.kind === 'content' || m.kind === 'both');
    contentList.replaceChildren(
      ...(contents.length
        ? contents.map((m) => h('label', { class: 'check-row' }, h('input', { type: 'checkbox', value: m.id, checked: true }), m.name))
        : [h('p', { class: 'hint' }, t('No content yet: attach it in Materials.'))]),
    );
    render();
  }

  build.addEventListener('click', async () => {
    message.textContent = '';
    const reusing = currentMode() === 'reuse';
    const plan = {
      title: title.value.trim() || t('My exam'),
      style: style.value,
      content: [...contentList.querySelectorAll('input:checked')].map((i) => i.value),
      exam: reusing ? examSelect.value || null : null,
      key: reusing ? info?.key ?? null : null,
      reuse: reusing ? Number(reuse.value) || 0 : 0,
      new: Math.max(0, Number(fresh.value) || 0),
    };
    if (reusing && !plan.exam) return (message.textContent = t('Pick the exam to reuse.'));
    if (plan.new > 0 && !plan.content.length) return (message.textContent = t('Pick some content for the new questions, or set them to 0.'));
    if (plan.new > 0 && !aiReady()) return (message.textContent = t('New questions need Jay-AI: download it in the Jay-AI section.'));
    try {
      await invoke('exam_build', { plan });
      build.disabled = true;
      stop.hidden = false;
      progress.hidden = false;
      progress.removeAttribute('value');
      message.textContent = t('Reading the materials…');
    } catch (e) {
      message.textContent = studyFailureText(e);
    }
  });

  listen('study-job', (e) => {
    if (e.kind !== 'exam') return;
    if (e.state === 'progress') {
      const s = e.step;
      if (s.step === 'writing' || s.step === 'solving') {
        progress.max = s.total || 1;
        progress.value = s.done;
      }
      message.textContent = {
        reading: t('Reading the materials…'),
        solving: t('Jay-AI is answering the questions without a key… {done} of {total}', { done: s.done, total: s.total }),
        tagging: t('Jay-AI is naming the topics…'),
        writing: t('Jay-AI is writing and checking question {n} of {total}…', { n: s.done + 1, total: s.total }),
      }[s.step];
      return;
    }
    build.disabled = false;
    stop.hidden = true;
    progress.hidden = true;
    if (e.state === 'done') {
      message.textContent = t('Ready!');
      onBuilt(e.id);
    } else {
      message.textContent = studyFailureText(e.error);
      onError?.(e.error);
    }
  });

  refresh();
  const el = h(
    'fieldset',
    { class: 'builder' },
    cardTitle(icons.exam, t('New exam')),
    h('div', { class: 'field' }, h('label', { for: 'exam-title' }, t('Name')), title),
    h('div', { class: 'field' }, h('label', { for: 'exam-style' }, t('Style')), style, styleHint),
    h('div', { class: 'field' }, h('span', { class: 'field-label' }, t('How')), mode),
    reuseFields,
    h('div', { class: 'field' }, h('span', { class: 'field-label' }, t('Content for new questions')), contentList, h('label', { for: 'exam-new' }, t('How many new questions (Jay-AI writes and checks each one)')), fresh),
    progress,
    message,
    h('div', { class: 'button-row' }, build, stop),
  );
  el.refresh = refresh;
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
    const newButton = h('button', { type: 'button', class: 'btn', disabled: !isTauri, onClick: () => { make.hidden = !make.hidden; if (!make.hidden) make.refresh(); } }, h('span', { class: 'btn-icon', html: icons.exam }), t('New exam'));

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
          )),
        ),
      );
      goingBox.hidden = open.length === 0;
      empty.hidden = exams.length > 0;
    }

    const empty = h('div', { class: 'empty-state' }, h('span', { class: 'card-icon', html: icons.exam, 'aria-hidden': 'true' }), h('p', {}, t('A new exam from your content in the style you pick (ENEM, UEPG, UEM…), or the questions of your exam mixed with your content. Answer it right here, and see the result at the end or each answer at once in learning mode.')), h('p', { class: 'hint' }, isTauri ? t('Start with "New exam".') : t('Only in the app.')));
    const goingBox = h('div', { hidden: true }, h('h2', { class: 'section-title' }, t('In progress')), going);
    load();
    return h('section', { class: 'app-panel is-single' }, h('div', { class: 'panel-head' }, h('h1', {}, t('Exams')), newButton), message, make, goingBox, empty, list);
  }
  views.home();
  return views.host;
}
