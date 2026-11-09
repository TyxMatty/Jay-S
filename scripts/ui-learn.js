// Learning mode through the UI: a new attempt on the first exam, answer the first question wrong
// on purpose (when the key is known), check, and look.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-exams').click();
for (let i = 0; i < 50 && !document.querySelector('#section-panel-exams .material-card'); i++) await wait(200);
[...document.querySelectorAll('#section-panel-exams .material-card .btn-secondary')].find((b) => b.textContent === 'Learning mode').click();
for (let i = 0; i < 50 && !document.querySelector('.runner-option'); i++) await wait(200);
const inputs = document.querySelectorAll('.runner-option input');
inputs[inputs.length - 1].click();
await wait(200);
[...document.querySelectorAll('.runner .btn')].find((b) => b.textContent === 'Check').click();
await wait(900);
document.querySelector('.runner-feedback')?.scrollIntoView({ block: 'center' });
return { feedback: document.querySelector('.runner-feedback')?.textContent.slice(0, 80), marks: [...document.querySelectorAll('.runner-option')].map((o) => o.dataset.mark ?? '-').join(',') };
