import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inTime, whenText } from '../src/ui/card-time.js';

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

test('when something happens, by the calendar', () => {
  const now = new Date(2026, 10, 11, 10, 0).getTime();
  const pt = (ms, at = now) => whenText(ms, 'pt-BR', at);
  assert.equal(pt(10 * MIN), 'em 10 minutos');
  assert.equal(pt(3 * HOUR), 'em 3 horas');
  assert.equal(pt(DAY), 'amanhã');
  assert.equal(pt(2 * HOUR, new Date(2026, 10, 11, 23, 0).getTime()), 'amanhã');
  assert.equal(pt(15 * DAY), 'em 2 semanas');
  assert.equal(pt(90 * DAY), 'em 3 meses');
});

test('and in English', () => {
  assert.equal(inTime(10 * MIN, 'en'), '10 minutes');
  assert.equal(inTime(DAY, 'en'), '1 day');
});
