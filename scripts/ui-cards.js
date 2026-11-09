// Flashcards through the UI: open the first deck with cards due, show the answer.
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-flashcards').click();
for (let i = 0; i < 50 && !document.querySelector('#section-panel-flashcards .material-card'); i++) await wait(200);
const review = [...document.querySelectorAll('#section-panel-flashcards .material-card .btn')].find((b) => b.textContent === 'Review' && !b.disabled);
review.click();
for (let i = 0; i < 50 && !document.querySelector('.card-front'); i++) await wait(200);
const front = document.querySelector('.card-front').textContent;
[...document.querySelectorAll('.flashcard .btn')].find((b) => b.textContent === 'Show the answer').click();
await wait(300);
return { front, back: document.querySelector('.card-back').textContent, buttons: [...document.querySelectorAll('.flashcard .btn')].filter((b) => !b.closest('[hidden]')).map((b) => b.textContent) };
