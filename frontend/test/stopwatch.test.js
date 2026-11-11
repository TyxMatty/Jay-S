import { test } from 'node:test';
import assert from 'node:assert/strict';
import { stopwatch, clockText } from '../src/ui/stopwatch.js';

test('the stopwatch runs, pauses, resumes, and double calls are harmless', () => {
  let t = 1000;
  const w = stopwatch(() => t);
  t += 500;
  assert.equal(w.elapsed(), 500);
  w.pause();
  w.pause();
  t += 9000;
  assert.equal(w.elapsed(), 500);
  w.resume();
  w.resume();
  t += 250;
  assert.equal(w.elapsed(), 750);
});

test('clockText shows m:ss and h:mm:ss', () => {
  assert.equal(clockText(0), '0:00');
  assert.equal(clockText(65_900), '1:05');
  assert.equal(clockText(3_599_000), '59:59');
  assert.equal(clockText(3_725_000), '1:02:05');
});
