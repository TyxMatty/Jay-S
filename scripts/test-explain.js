// node scripts/cdp-eval.mjs scripts/test-explain.js — explain the handout in English through the
// UI, then answer Jay-AI's first question with a half-right answer.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-materials').click();
for (let i = 0; i < 50 && !document.querySelector('.material-card .btn'); i++) await wait(200);
const card = [...document.querySelectorAll('.material-card')].find((c) => c.querySelector('.material-name')?.textContent === 'fotossintese-apostila.md');
[...card.querySelectorAll('.btn')].find((b) => b.textContent.includes('Explain')).click();
for (let i = 0; i < 50 && !document.querySelector('#explain-language'); i++) await wait(200);
document.querySelector('#explain-language').value = 'English';
const t0 = performance.now();
[...document.querySelectorAll('.explain .btn')].find((b) => b.textContent.includes('Explain with Jay-AI')).click();
for (let i = 0; i < 240 && !document.querySelector('.explain-sheet h2'); i++) await wait(1000);
const seconds = Math.round((performance.now() - t0) / 1000);
const out = {
  seconds,
  title: document.querySelector('.explain-sheet h2')?.textContent,
  summary: document.querySelector('.explain-paragraph')?.textContent.slice(0, 200),
  concepts: [...document.querySelectorAll('.concept h4')].map((h) => h.textContent),
  question: document.querySelector('.quiz-question')?.textContent,
};
document.querySelector('#quiz-answer').value = 'It happens in the chloroplasts.';
[...document.querySelectorAll('.quiz .btn')].find((b) => b.textContent === 'Answer').click();
for (let i = 0; i < 90 && !document.querySelector('.quiz .runner-feedback'); i++) await wait(1000);
out.feedback = document.querySelector('.quiz .runner-feedback')?.textContent.slice(0, 300);
out.followUp = document.querySelector('.quiz-question.is-follow-up')?.textContent ?? null;
return out;
