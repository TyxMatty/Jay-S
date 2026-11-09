// node scripts/cdp-eval.mjs scripts/test-jayai.js — the Jay-AI section: status, the test button.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-jayai').click();
await wait(500);
const invoke = window.__TAURI_INTERNALS__.invoke;
const before = await invoke('ai_status');
const t0 = performance.now();
const reply = await invoke('ai_test');
const first = (performance.now() - t0) / 1000;
const t1 = performance.now();
const again = await invoke('ai_test');
const second = (performance.now() - t1) / 1000;
await wait(300);
return {
  before,
  reply,
  first,
  again,
  second,
  pill: document.querySelector('.status-pill').textContent,
  serverLine: document.querySelector('.app-panels .hint')?.textContent,
};
