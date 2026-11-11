import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inTime } from '../src/ui/card-time.js';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

test('intervals read in the unit that fits, in Portuguese', () => {
  const pt = (ms) => inTime(ms, 'pt-BR');
  assert.equal(pt(10 * MIN), '10 minutos');
  assert.equal(pt(0), '1 minuto');
  assert.equal(pt(59 * MIN), '59 minutos');
  assert.equal(pt(HOUR), '1 hora');
  assert.equal(pt(23 * HOUR), '23 horas');
  assert.equal(pt(DAY), '1 dia');
  assert.equal(pt(13 * DAY), '13 dias');
  assert.equal(pt(15 * DAY), '2 semanas');
  assert.equal(pt(59 * DAY), '8 semanas');
  assert.equal(pt(60 * DAY), '2 meses');
  assert.equal(pt(90 * DAY), '3 meses');
});

test('and in English', () => {
  assert.equal(inTime(10 * MIN, 'en'), '10 minutes');
  assert.equal(inTime(DAY, 'en'), '1 day');
});
