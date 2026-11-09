// node scripts/cdp-eval.mjs scripts/test-jobs.js — the two bugs of 2026-11-09:
// 1. the Exams screen rebuilt during a build shows its progress again;
// 2. "Free the memory now" during a build doesn't freeze the window, and stops the build.
const invoke = window.__TAURI_INTERNALS__.invoke;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const materials = await invoke('materials_list');
const exams = materials.filter((m) => m.kind === 'exam').map((m) => m.id);
const content = materials.filter((m) => m.kind === 'content').map((m) => m.id);
// A long build: two exams pooled + 5 new questions.
await invoke('exam_build', { plan: { title: 'Duas provas', style: 'enem', content, exams, reuse: 6, new: 5 } });
await wait(4000);
// 1. Rebuild the Exams screen (as Back from an exam does) and look for the progress.
document.querySelector('#section-tab-exams').click();
for (let i = 0; i < 50 && !document.querySelector('#section-panel-exams .builder'); i++) await wait(200);
const host = document.querySelector('#section-panel-exams .lazy-panel .lazy-panel') ?? document.querySelector('#section-panel-exams .lazy-panel');
await wait(1500);
const builder = document.querySelector('#section-panel-exams .builder');
const shown = { builderOpen: !builder.hidden, progressShown: !builder.querySelector('progress').hidden, message: builder.querySelector('.hint[role=status]').textContent, buildDisabled: builder.querySelector('.btn').disabled };
// 2. Free the memory mid-build; the page must keep answering (a timer keeps ticking).
let ticks = 0;
const timer = setInterval(() => ticks++, 100);
const t0 = performance.now();
const stopCall = invoke('ai_stop');
await wait(500);
const ticksWhileStopping = ticks;
await stopCall;
const stopSeconds = (performance.now() - t0) / 1000;
clearInterval(timer);
await wait(1500);
const status = await invoke('ai_status');
const job = await invoke('job_status');
return { shown, ticksIn500ms: ticksWhileStopping, stopSeconds, server: status.server, jobAfter: job, builderMessage: builder.querySelector('.hint[role=status]').textContent };
