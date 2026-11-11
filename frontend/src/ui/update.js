// The update pill in the top bar and the small panel under it (not a modal). Rust finds the newer
// release once (src-tauri/src/update.rs); the page only shows the version it is told and asks to
// install. The state lives here, at module level: there is one pill.
import { h } from './dom.js';
import { invoke, listen, isTauri } from '../core/bridge.js';
import { HIDDEN, next } from '../core/update.js';
import { t } from '../core/i18n.js';

let state = HIDDEN;
let built = null;

const failureText = (code) =>
  code === 'working' ? t('Jay-AI is working. Try again when it finishes.') : code === 'busy' ? t('The update is already running.') : t('The update failed.');

function dispatch(event) {
  const was = state;
  state = next(state, event);
  if (state !== was) built?.render(was);
}

function build() {
  const pill = h('button', { type: 'button', class: 'status-pill update-pill', 'data-state': 'updates', 'aria-expanded': 'false', 'aria-controls': 'update-panel', hidden: true, onClick: () => dispatch({ type: 'open' }) });
  const title = h('h2', { id: 'update-title' });
  const what = h('div', { class: 'update-what' }, h('p', {}, t('Jay-S closes, installs the update and opens again.')), h('p', { class: 'hint' }, t("Your answers are saved as you go: an exam in progress continues where you left it. A mock test's clock keeps running.")));
  const progress = h('progress', { class: 'ai-progress', max: 100, value: 0 });
  const status = h('p', { class: 'update-status', 'aria-live': 'polite' });
  const install = h('button', { type: 'button', class: 'btn', onClick: start }, t('Install update'));
  const retry = h('button', { type: 'button', class: 'btn', onClick: start }, t('Try again'));
  const later = h('button', { type: 'button', class: 'btn btn-secondary', onClick: close }, t('Not now'));
  const panel = h('div', { id: 'update-panel', class: 'update-panel', role: 'region', 'aria-labelledby': 'update-title', tabindex: '-1', hidden: true }, title, what, progress, status, h('div', { class: 'button-row' }, install, retry, later));

  function render(was) {
    const { name } = state;
    const open = name !== 'hidden' && name !== 'available';
    const hadFocus = panel.contains(document.activeElement);
    pill.hidden = name === 'hidden';
    pill.textContent = t('Update to {version}', { version: state.version ?? '' });
    pill.setAttribute('aria-expanded', String(open));
    panel.hidden = !open;
    title.textContent = t('Jay-S {version}', { version: state.version ?? '' });
    what.hidden = name !== 'confirm';
    progress.hidden = name !== 'downloading';
    progress.value = state.percent ?? 0;
    status.textContent =
      name === 'downloading' ? t('Downloading the update… {percent}%', { percent: state.percent }) : name === 'installing' ? t('Installing: Jay-S closes now.') : name === 'error' ? failureText(state.code) : '';
    install.hidden = name !== 'confirm';
    retry.hidden = name !== 'error' || state.code === 'busy';
    later.hidden = name !== 'confirm' && name !== 'error';
    // The button that had focus may have just been hidden: keep focus in the panel, or give it back to the pill.
    if (hadFocus && name === 'available') pill.focus();
    else if (hadFocus && open) (name === 'error' && !retry.hidden ? retry : panel).focus();
    else if (open && was.name === 'available') install.focus();
  }

  function close() {
    dispatch({ type: 'close' });
  }

  async function start() {
    dispatch({ type: 'start' });
    if (state.name !== 'downloading') return;
    try {
      await invoke('update_install');
    } catch (e) {
      dispatch({ type: 'error', code: e?.code });
    }
  }

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && (state.name === 'confirm' || state.name === 'error')) close();
  });
  listen('update-download', (e) => {
    if (e.state === 'progress') dispatch({ type: 'progress', done: e.done, total: e.total });
    else if (e.state === 'error') dispatch({ type: 'error', code: e.error?.code });
  });
  return { pill, panel, render };
}

/** The pill and its panel, for the top bar; asks Rust once whether a newer version exists. */
export function createUpdatePill() {
  if (!built) {
    built = build();
    built.render(state);
    if (isTauri) invoke('update_check').then((version) => version && offerUpdate(version)).catch(() => {});
  }
  return [built.pill, built.panel];
}

/** Shows the pill for `version` (what the check's answer goes through). */
export function offerUpdate(version) {
  dispatch({ type: 'available', version });
}
