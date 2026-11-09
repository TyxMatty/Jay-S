// A 5-minute mock test through the UI: answer three, finish, wait for Jay-AI's analysis on screen.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-mock').click();
for (let i = 0; i < 50 && !document.querySelector('#mock-exam option[value]:not([value=""])'); i++) await wait(200);
document.querySelector('#mock-minutes').value = 5;
[...document.querySelectorAll('#section-panel-mock .btn')].find((b) => b.textContent.includes('Start the mock test')).click();
await wait(1000);
const timer = document.querySelector('.runner-timer')?.textContent;
for (let i = 0; i < 3; i++) {
  document.querySelectorAll('.runner-option input')[i % 5].click();
  await wait(300);
  [...document.querySelectorAll('.runner .btn')].find((b) => b.textContent === 'Next').click();
  await wait(300);
}
const dots = [...document.querySelectorAll('.runner-dot')].map((d) => d.dataset.state).join(',');
window.confirm = () => true;
[...document.querySelectorAll('.runner .btn')].find((b) => b.textContent === 'Finish').click();
await wait(1500);
const before = document.querySelector('.analysis')?.textContent.slice(0, 80);
for (let i = 0; i < 90 && document.querySelector('.analysis .spinner'); i++) await wait(1000);
return { timer, dots, score: document.querySelector('.result-score')?.textContent, before, after: document.querySelector('.analysis')?.textContent.slice(0, 300) };
