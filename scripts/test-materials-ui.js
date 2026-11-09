// node scripts/cdp-eval.mjs scripts/test-materials-ui.js — kind change, text preview and delete,
// through the page's own buttons (confirm() answered yes for the test).
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const card = [...document.querySelectorAll('.material-card')].find((c) => c.querySelector('.material-name')?.textContent === 'fotossintese.md');
const out = {};
const select = card.querySelector('.material-kind');
select.value = 'both';
select.dispatchEvent(new Event('change'));
await wait(300);
const id = (await window.__TAURI_INTERNALS__.invoke('materials_list')).find((m) => m.name === 'fotossintese.md').id;
out.kindAfterChange = (await window.__TAURI_INTERNALS__.invoke('materials_list')).find((m) => m.id === id).kind;
card.querySelector('.material-actions .btn').click();
await wait(300);
const pre = card.querySelector('.material-text');
out.textShown = !pre.hidden && pre.textContent.startsWith('Fotossíntese');
out.textIsText = pre.children.length === 0;
const realConfirm = window.confirm;
window.confirm = () => true;
card.querySelector('.material-actions .icon-btn').click();
await wait(300);
window.confirm = realConfirm;
out.goneFromPage = !document.body.contains(card);
out.goneFromDisk = !(await window.__TAURI_INTERNALS__.invoke('materials_list')).some((m) => m.id === id);
return out;
