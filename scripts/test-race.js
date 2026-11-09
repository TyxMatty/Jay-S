// node scripts/cdp-eval.mjs scripts/test-race.js — answers fired all at once must all be saved.
const invoke = window.__TAURI_INTERNALS__.invoke;
const exam = (await invoke('exams_list'))[0];
const s = await invoke('attempt_start', { exam: exam.id, mode: 'exam', minutes: null });
await Promise.all(s.questions.map((q, i) => invoke('attempt_answer', { id: s.attempt.id, index: i, marked: q.options[0].label, ms: 10 })));
const list = await invoke('attempts_list');
const mine = list.find((a) => a.id === s.attempt.id);
await invoke('attempt_delete', { id: s.attempt.id });
return { questions: s.questions.length, saved: mine.answered };
