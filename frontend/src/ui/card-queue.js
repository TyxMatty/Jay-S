// Where a forgotten card goes back into the session queue: after at least 3 other cards, or at the
// end when fewer remain.
export function requeueAt(pos, length) {
  return Math.min(pos + 1 + 3, length);
}
