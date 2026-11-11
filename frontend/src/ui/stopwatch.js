// Elapsed time that does not count while paused; the clock is injectable for tests.
export function stopwatch(now = () => performance.now()) {
  let sum = 0;
  let since = now();
  return {
    elapsed: () => sum + (since === null ? 0 : now() - since),
    pause() {
      if (since !== null) sum += now() - since;
      since = null;
    },
    resume() {
      if (since === null) since = now();
    },
  };
}

/** m:ss, or h:mm:ss past an hour. */
export function clockText(ms) {
  const s = Math.floor(ms / 1000);
  const pad = (n) => String(n).padStart(2, '0');
  const m = Math.floor(s / 60) % 60;
  return s >= 3600 ? `${Math.floor(s / 3600)}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}
