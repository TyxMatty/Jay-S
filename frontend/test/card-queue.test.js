import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requeueAt } from '../src/ui/card-queue.js';

test('a forgotten card returns after 3 others, or at the end', () => {
  assert.equal(requeueAt(0, 10), 4);
  assert.equal(requeueAt(7, 10), 10);
  assert.equal(requeueAt(9, 10), 10);
});
