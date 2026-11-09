// node scripts/cdp-eval.mjs scripts/test-multi.js — an exam from two exams (ENEM + UEPG) and the content.
const invoke = window.__TAURI_INTERNALS__.invoke;
const { listen } = await import(performance.getEntriesByType('resource').map((e) => e.name).find((n) => n.includes('@tauri-apps_api_event')));
const materials = await invoke('materials_list');
const exams = materials.filter((m) => m.kind === 'exam');
const content = materials.filter((m) => m.kind === 'content').map((m) => m.id);
const info = {};
for (const e of exams) { const i = await invoke('exam_material_info', { id: e.id }); info[e.name] = { questions: i.questions, key: materials.find((m) => m.id === i.key)?.name ?? null }; }
const t0 = performance.now();
const done = new Promise((r) => listen('study-job', (ev) => ev.payload.kind === 'exam' && ev.payload.state !== 'progress' && r(ev.payload)));
await invoke('exam_build', { plan: { title: 'Várias provas', style: 'enem', content, exams: exams.map((e) => e.id), reuse: 8, new: 2 } });
const result = await done;
const out = { info, result: result.state, seconds: Math.round((performance.now() - t0) / 1000) };
if (result.state === 'done') {
  const e = (await invoke('exams_list')).find((x) => x.id === result.id);
  out.exam = { questions: e.questions, reused: e.reused, byAi: e.by_ai };
}
return out;
