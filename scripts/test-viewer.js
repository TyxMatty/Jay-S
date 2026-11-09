// node scripts/cdp-eval.mjs scripts/test-viewer.js — Materials tabs, an exam's key choice, the PDF viewer.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const invoke = window.__TAURI_INTERNALS__.invoke;
location.hash = '';
document.querySelector('#section-tab-materials').click();
for (let i = 0; i < 50 && !document.querySelector('.material-tabs'); i++) await wait(200);
await wait(800);
const counts = [...document.querySelectorAll('.material-tabs [role=tab]')].map((t) => t.textContent);
document.querySelector('#materials-tab-exam').click();
await wait(800);
const examCards = [...document.querySelectorAll('#materials-panel-exam .material-card')].map((c) => ({ name: c.querySelector('.material-name')?.textContent, key: c.querySelector('.material-key select')?.selectedOptions[0]?.textContent }));
// Open the ENEM PDF in the viewer.
const enem = [...document.querySelectorAll('#materials-panel-exam .material-card')].find((c) => c.querySelector('.material-name')?.textContent === '2023_PV_D1.pdf');
enem.querySelector('.material-actions .btn-secondary').click();
for (let i = 0; i < 50 && !document.querySelector('.pdf-page'); i++) await wait(200);
await wait(3000);
const pages = [...document.querySelectorAll('.pdf-page img')];
const loaded = pages.filter((img) => img.getAttribute('src') && img.complete && img.naturalWidth > 0);
return { counts, examCards, pages: pages.length, withPicture: pages.filter((i) => i.getAttribute('src')).length, drawn: loaded.length, firstWidth: loaded[0]?.naturalWidth };
