// The bridge to src-tauri (Jay-E's core/files.js pattern). The Tauri modules load only in the app:
// in the browser (`npm run web`) every call is skipped and the page says "only in the app".

export const isTauri = '__TAURI_INTERNALS__' in globalThis;

const core = () => import('@tauri-apps/api/core');

/** Runs a Rust command. Rejects with `{ code, detail }` (src-tauri/src/library.rs `Failure`). */
export async function invoke(command, args) {
  if (!isTauri) throw { code: 'web', detail: null };
  return (await core()).invoke(command, args);
}

/** `callback(payload)` for every `event` Rust emits. */
export async function listen(event, callback) {
  if (!isTauri) return () => {};
  const api = await import('@tauri-apps/api/event');
  return api.listen(event, (e) => callback(e.payload));
}

/**
 * `callback(event)` for every study job event, and at once for the job already running (its last
 * progress), so a screen built after a job started still shows it. Returns the unlisten promise.
 */
export function listenJobs(callback) {
  const stop = listen('study-job', callback);
  if (isTauri) invoke('job_status').then((s) => s && callback(s)).catch(() => {});
  return stop;
}
