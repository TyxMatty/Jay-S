// The Jay-AI section (stage 2): pick the model, download it once (progress, Stop, resume), test it,
// stop it, remove it. Everything about the server stays in Rust (src-tauri/src/ai.rs).
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { createTabs } from './tabs.js';
import { invoke, listen, isTauri } from '../core/bridge.js';
import { onAiStatus, aiReady, setAiStatus } from '../core/ai.js';
import { t, language } from '../core/i18n.js';

const gb = (bytes) => new Intl.NumberFormat(language(), { maximumFractionDigits: 1 }).format(bytes / 1e9);

export function aiFailureText(error) {
  switch (error?.code) {
    case 'cancelled':
      return t('Stopped.');
    case 'mismatch':
      return t("The download didn't match the original file and was deleted. Try again.");
    case 'network':
      return t("Couldn't download: check the internet connection and try again.");
    case 'not_installed':
      return t('Download Jay-AI first (Jay-AI section).');
    case 'start':
      return t("Jay-AI couldn't start ({detail}).", { detail: error.detail ?? '' });
    case 'bad_reply':
      return t('Jay-AI answered something unexpected. Try again.');
    case 'busy':
      return t('A download is already running.');
    case 'web':
      return t('Only in the app.');
    default:
      return t("Something went wrong ({detail}).", { detail: error?.detail ?? String(error) });
  }
}

export function createJayAiPanel() {
  const message = h('p', { class: 'hint', role: 'status' });
  const progress = h('progress', { class: 'ai-progress', max: 1, value: 0, hidden: true });
  const choices = [
    ['standard', t('Standard'), t('Qwen3.5 4B · {size} GB · better questions. Best with a graphics card or 16 GB of memory.')],
    ['light', t('Lightweight'), t('Qwen3.5 2B · {size} GB · for computers with little memory; simpler questions.')],
  ];
  const radios = choices.map(([value]) => h('input', { type: 'radio', name: 'ai-model', value, onChange: () => choose(value) }));
  const sizes = choices.map(() => h('span'));
  const states = choices.map(() => h('span', { class: 'badge' }));
  const options = h(
    'div',
    { class: 'ai-models', role: 'radiogroup', 'aria-label': t('Model') },
    choices.map(([value, label], i) =>
      h('label', { class: 'ai-model' }, radios[i], h('span', { class: 'ai-model-text' }, h('strong', {}, label), h('span', { class: 'hint' }, sizes[i])), states[i]),
    ),
  );

  const download = h('button', { type: 'button', class: 'btn', disabled: !isTauri, onClick: install }, h('span', { class: 'btn-icon', html: icons.download }), t('Download'));
  const stop = h('button', { type: 'button', class: 'btn btn-secondary', hidden: true, onClick: () => invoke('ai_cancel') }, t('Stop'));
  const remove = h('button', { type: 'button', class: 'btn btn-secondary', onClick: removeModel }, t('Remove model'));
  const test = h('button', { type: 'button', class: 'btn', onClick: runTest }, h('span', { class: 'btn-icon', html: icons.spark }), t('Test Jay-AI'));
  const unload = h('button', { type: 'button', class: 'btn btn-secondary', onClick: () => invoke('ai_stop') }, t('Free the memory now'));
  const serverLine = h('p', { class: 'hint' });
  const testOut = h('div', { class: 'ai-test', hidden: true });

  let current = null;

  function render(s) {
    current = s;
    choices.forEach(([value, , text], i) => {
      const m = s[value];
      radios[i].checked = s.choice === value;
      sizes[i].textContent = text.replace('{size}', gb(m.size));
      states[i].dataset.status = m.ready ? 'ok' : m.partial ? 'check' : 'off';
      states[i].textContent = m.ready ? t('Downloaded') : m.partial ? t('{n}% downloaded', { n: Math.floor((m.partial / m.size) * 100) }) : t('Not downloaded');
    });
    const ready = aiReady(s);
    download.hidden = ready || s.downloading;
    stop.hidden = !s.downloading;
    progress.hidden = !s.downloading;
    remove.hidden = !s[s.choice].ready && !s[s.choice].partial;
    for (const r of radios) r.disabled = s.downloading;
    test.disabled = !ready || s.server === 'loading' || s.server === 'working';
    unload.hidden = s.server === 'off';
    serverLine.textContent = {
      off: t('Off: it starts by itself when needed, and uses no memory now.'),
      loading: t('Loading the model…'),
      ready: t('Ready. It leaves the memory 2 minutes after the last use.'),
      working: t('Working…'),
    }[s.server];
  }

  async function choose(value) {
    try {
      setAiStatus(await invoke('ai_set_choice', { choice: value }));
    } catch (e) {
      message.textContent = aiFailureText(e);
    }
  }

  async function install() {
    message.textContent = '';
    try {
      await invoke('ai_install');
    } catch (e) {
      message.textContent = aiFailureText(e);
    }
  }

  async function removeModel() {
    if (!confirm(t('Remove the downloaded model? Jay-AI stops working until you download it again.'))) return;
    try {
      setAiStatus(await invoke('ai_remove_model', { choice: current.choice }));
    } catch (e) {
      message.textContent = aiFailureText(e);
    }
  }

  async function runTest() {
    testOut.hidden = true;
    message.textContent = '';
    try {
      const r = await invoke('ai_test');
      const fmt = (n) => new Intl.NumberFormat(language(), { maximumFractionDigits: 1 }).format(n);
      testOut.replaceChildren(
        h('p', {}, h('strong', {}, t('Capital: ')), String(r.reply.capital ?? '')),
        h('p', {}, String(r.reply.curiosidade ?? '')),
        h('p', { class: 'hint' }, r.load_seconds ? t('Loaded in {load} s, answered in {s} s.', { load: fmt(r.load_seconds), s: fmt(r.seconds) }) : t('Answered in {s} s.', { s: fmt(r.seconds) })),
      );
      testOut.hidden = false;
    } catch (e) {
      message.textContent = aiFailureText(e);
    }
  }

  listen('ai-download', (e) => {
    if (e.state === 'progress') {
      progress.max = e.total;
      progress.value = e.done;
      message.textContent = e.what === 'runtime' ? t('Downloading the engine… {done} of {total} MB', { done: Math.round(e.done / 1e6), total: Math.round(e.total / 1e6) }) : t('Downloading the model… {done} of {total} MB', { done: Math.round(e.done / 1e6), total: Math.round(e.total / 1e6) });
    } else if (e.state === 'done') {
      message.textContent = t('Jay-AI is ready.');
    } else {
      message.textContent = aiFailureText(e.error);
    }
  });
  onAiStatus(render);

  const modelPane = h(
    'div',
    { class: 'jayai-pane' },
    h(
      'fieldset',
      {},
      cardTitle(icons.spark, t('Model')),
      h('p', { class: 'hint' }, t('Jay-AI runs on this computer. It is downloaded once, checked against the original (SHA-256), and after that nothing you study leaves the computer.')),
      options,
      progress,
      message,
      h('div', { class: 'button-row' }, download, stop, remove),
    ),
    h(
      'fieldset',
      {},
      cardTitle(icons.info, t('Memory')),
      serverLine,
      h('div', { class: 'button-row' }, test, unload),
      testOut,
    ),
  );
  // Chat first: the model is set up once, the conversation is what the section is for.
  const chatPane = h('div', { class: 'jayai-pane' });
  const tabs = createTabs({
    idPrefix: 'jayai',
    label: t('Jay-AI'),
    className: 'material-tabs',
    items: [
      { key: 'chat', label: h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: icons.spark }), t('Chat')), panel: chatPane },
      { key: 'model', label: h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: icons.settings }), t('Model')), panel: modelPane },
    ],
    onSelect: (key) => key === 'chat' && !chatPane.firstChild && import('./chat.js').then((m) => chatPane.replaceChildren(m.createChatPanel())),
  });
  tabs.select('chat');
  return h('section', { class: 'app-panel is-single' }, h('h1', {}, t('Jay-AI')), tabs.list, chatPane, modelPane);
}
