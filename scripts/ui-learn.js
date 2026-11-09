// Drives the Exams section like a student: learning mode on the first exam, answer, check.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-exams').click();
await wait(800);
const learn = [...document.querySelectorAll('.material-card .btn-secondary')].find((b) => b.textContent === 'Learning mode');
learn.click();
await wait(800);
document.querySelector('.runner-option input').click();
await wait(200);
[...document.querySelectorAll('.runner .btn')].find((b) => b.textContent === 'Check').click();
await wait(800);
return { feedback: document.querySelector('.runner-feedback')?.textContent?.slice(0, 120), dots: [...document.querySelectorAll('.runner-dot')].map((d) => d.dataset.state).join(',') };
