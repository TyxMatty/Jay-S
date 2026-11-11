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

const midnight = (ms) => new Date(ms).setHours(0, 0, 0, 0);

/** When something `ms` from `now` happens, in words ("em 10 minutos", "amanhã", "em 2 semanas"). */
export function whenText(ms, lang, now = Date.now()) {
  const rtf = new Intl.RelativeTimeFormat(lang, { numeric: 'auto' });
  if (ms < HOUR) return rtf.format(Math.max(1, Math.round(ms / MIN)), 'minute');
  const days = Math.round((midnight(now + ms) - midnight(now)) / DAY);
  if (days === 0) return rtf.format(Math.round(ms / HOUR), 'hour');
  return days >= 60 ? rtf.format(Math.round(days / 30), 'month') : days >= 7 ? rtf.format(Math.round(days / 7), 'week') : rtf.format(days, 'day');
}
