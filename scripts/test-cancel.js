// node scripts/cdp-eval.mjs scripts/test-cancel.js — Stop reaches the request that is waiting its
// turn too. Two questions at once (Jay-AI answers one at a time, the second waits), Stop, and both
// must end "cancelled" at once; before, the waiting one forgot the Stop and ran whole. Then a new
// question must still be answered (the Stop doesn't stick).
const invoke = window.__TAURI_INTERNALS__.invoke;
const timed = async (question) => {
  const chat = await invoke('chat_new', { scope: {} });
  const t0 = performance.now();
  const seconds = () => Math.round((performance.now() - t0) / 100) / 10;
  try {
    const turn = await invoke('chat_ask', { id: chat.id, question });
    return { question, seconds: seconds(), kind: turn.reply.kind, sentences: turn.reply.sentences.length };
  } catch (e) {
    return { question, seconds: seconds(), error: e.code };
  } finally {
    await invoke('chat_delete', { id: chat.id });
  }
};
const running = timed('Explique em detalhes o que é a fotossíntese.');
await new Promise((r) => setTimeout(r, 300));
const waiting = timed('Explique em detalhes o que é a respiração celular.');
await new Promise((r) => setTimeout(r, 1500));
await invoke('job_cancel');
const stopped = await Promise.all([running, waiting]);
const after = await timed('Olá!');
return { stopped, after, ok: stopped.every((r) => r.error === 'cancelled') && !after.error };
