// Jay-AI's state for the whole page (src-tauri/src/ai.rs): one `ai-status` listener, many readers
// (the top bar's pill, the Jay-AI section, the buttons that need the model).
import { invoke, listen, isTauri } from './bridge.js';

let status = null;
const listeners = new Set();

function set(next) {
  status = next;
  for (const listener of listeners) listener(status);
}

if (isTauri) {
  listen('ai-status', set);
  invoke('ai_status').then(set);
}

/** Calls `listener(status)` now (if known) and on every change. Returns an unsubscribe function. */
export function onAiStatus(listener) {
  listeners.add(listener);
  if (status) listener(status);
  return () => listeners.delete(listener);
}

export const aiStatus = () => status;

/** The chosen model is downloaded, with its runtime: Jay-AI can work. */
export const aiReady = (s = status) => Boolean(s?.runtime && s[s.choice]?.ready);

export { set as setAiStatus };
