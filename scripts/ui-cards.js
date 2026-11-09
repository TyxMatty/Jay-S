// Flashcards through the UI: open the first deck with cards due, flip the card (Space).
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
document.querySelector('#section-tab-flashcards').click();
for (let i = 0; i < 50 && !document.querySelector('#section-panel-flashcards .material-card'); i++) await wait(200);
[...document.querySelectorAll('#section-panel-flashcards .material-card .btn')].find((b) => b.textContent === 'Review' && !b.disabled).click();
for (let i = 0; i < 50 && !document.querySelector('.flip-card'); i++) await wait(200);
await wait(500);
const card = document.querySelector('.flip-card');
const before = card.dataset.flipped;
if (location.hash !== '#front') {
  card.focus();
  card.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
  await wait(700);
}
return { before, after: card.dataset.flipped, ratings: [...document.querySelectorAll('.rating')].filter((b) => !b.closest('[hidden]')).map((b) => b.textContent) };
