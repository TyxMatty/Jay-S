// A flashcard interval as a short phrase ("10 minutos", "2 semanas"), in the interface language.
const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

/** `ms` from now as one whole unit: minutes < 1 h, hours < 1 day, days < 14, weeks < 60 days, then months. */
export function inTime(ms, lang) {
  const [unit, size] =
    ms < HOUR ? ['minute', MIN] : ms < DAY ? ['hour', HOUR] : ms < 14 * DAY ? ['day', DAY] : ms < 60 * DAY ? ['week', 7 * DAY] : ['month', 30 * DAY];
  const n = Math.max(1, Math.round(ms / size));
  return new Intl.NumberFormat(lang, { style: 'unit', unit, unitDisplay: 'long' }).format(n);
}
