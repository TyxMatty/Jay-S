// node scripts/cdp-eval.mjs scripts/test-cards.js — stage 5: a deck from the handout, a review, a vote.
const invoke = window.__TAURI_INTERNALS__.invoke;
const { listen } = await import(performance.getEntriesByType('resource').map((e) => e.name).find((n) => n.includes('@tauri-apps_api_event')));
const handout = (await invoke('materials_list')).find((m) => m.name === 'fotossintese-apostila.md');
const t0 = performance.now();
const done = new Promise((resolve) => listen('study-job', (e) => e.payload.kind === 'deck' && e.payload.state !== 'progress' && resolve(e.payload)));
await invoke('deck_build', { title: 'Fotossíntese', materials: [handout.id], count: 8 });
const result = await done;
const out = { result, seconds: Math.round((performance.now() - t0) / 1000) };
if (result.state !== 'done') return out;
const due = await invoke('deck_due', { id: result.id });
out.cards = due.map((c) => ({ front: c.front, back: c.back, topic: c.topic }));
await invoke('card_review', { deck: result.id, index: due[0].index, quality: 4, again: false, ms: 0 });
out.dueAfter = (await invoke('deck_due', { id: result.id })).length;
try { await invoke('card_review', { deck: result.id, index: 0, quality: 9, again: false, ms: 0 }); out.badQuality = 'ACCEPTED?!'; } catch (e) { out.badQuality = e.code; }
out.vote = (await invoke('card_vote', { tooLong: true })).card_length;
return out;
