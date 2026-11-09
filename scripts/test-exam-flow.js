// node scripts/cdp-eval.mjs scripts/test-exam-flow.js — stages 3–4 through the real commands:
// build an exam (5 reused from the ENEM with its key + 2 new from the handout), take it in learning
// mode and in exam mode, and check that no answer reaches the page early.
const invoke = window.__TAURI_INTERNALS__.invoke;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const { listen } = await import(performance.getEntriesByType('resource').map((e) => e.name).find((n) => n.includes('@tauri-apps_api_event') || n.includes('api/event')) ?? '/node_modules/@tauri-apps/api/event.js');
const materials = await invoke('materials_list');
const enem = materials.find((m) => m.name === '2023_PV_D1.pdf');
const key = materials.find((m) => m.name.startsWith('2023_GB'));
const handout = materials.find((m) => m.name === 'fotossintese-apostila.md');
const out = { kinds: materials.map((m) => [m.name, m.kind]) };
out.info = await invoke('exam_material_info', { id: enem.id });
const events = [];
const done = new Promise((resolve) => {
  listen('study-job', (e) => {
    events.push(e.payload.state === 'progress' ? e.payload.step.step : e.payload.state);
    if (e.payload.kind === 'exam' && e.payload.state !== 'progress') resolve(e.payload);
  });
});
const t0 = performance.now();
await invoke('exam_build', { plan: { title: 'ENEM + fotossíntese', style: 'enem', content: [handout.id], exam: enem.id, key: null, reuse: 5, new: 2 } });
const result = await done;
out.build = { result, seconds: Math.round((performance.now() - t0) / 1000), steps: [...new Set(events)] };
if (result.state !== 'done') return out;
const exam = (await invoke('exams_list')).find((e) => e.id === result.id);
out.exam = exam;
// Learning mode: graded at once, locked after.
const learn = await invoke('attempt_start', { exam: exam.id, mode: 'learn', minutes: null });
out.leaksInLearnStart = JSON.stringify(learn.questions).includes('"answer"') || JSON.stringify(learn.questions).includes('explanation');
const q0 = learn.questions[0];
out.learnFirst = await invoke('attempt_answer', { id: learn.attempt.id, index: 0, marked: q0.options[0].label, ms: 1500 });
try { await invoke('attempt_answer', { id: learn.attempt.id, index: 0, marked: q0.options[1].label, ms: 1 }); out.relock = 'CHANGED?!'; } catch (e) { out.relock = e.code; }
// Exam mode: nothing back until the end.
const ex = await invoke('attempt_start', { exam: exam.id, mode: 'exam', minutes: null });
const replies = [];
for (const [i, q] of ex.questions.entries()) replies.push(await invoke('attempt_answer', { id: ex.attempt.id, index: i, marked: q.options[i % q.options.length].label, ms: 2000 }));
out.examModeReplies = replies;
const finished = await invoke('attempt_finish', { id: ex.attempt.id });
out.score = [finished.result.points, finished.result.total];
out.topics = finished.result.topics;
out.sources = (await invoke('attempt_result', { id: ex.attempt.id }))[1].map((q) => q.topic);
// Wait for Jay-AI's analysis (sent automatically).
const analysis = await new Promise((resolve) => {
  listen('study-job', (e) => e.payload.kind === 'analysis' && e.payload.state !== 'progress' && e.payload.id === ex.attempt.id && resolve(e.payload));
  setTimeout(() => resolve('timeout'), 180000);
});
out.analysisEvent = analysis.state ?? analysis;
out.analysis = (await invoke('attempt_result', { id: ex.attempt.id }))[0].analysis;
return out;
