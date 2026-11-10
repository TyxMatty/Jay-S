import { test } from 'node:test';
import assert from 'node:assert/strict';
import { replyNotes } from '../src/ui/chat.js';
import { setLanguage } from '../src/core/i18n.js';

const reply = (kind, statuses) => ({ kind, sources: [], sentences: statuses.map((status) => ({ text: 'x', sources: [], status })) });

test('an app reply speaks of the guide, never of the student\'s sources', () => {
  setLanguage('pt-BR');
  const confirmed = replyNotes(reply('app', ['confirmed', 'unconfirmed']));
  assert.deepEqual([confirmed.badge, confirmed.banner], [['ok', 'Do guia do Jay-S'], null]);
  const none = replyNotes(reply('app', ['unconfirmed']));
  assert.deepEqual(none.badge, ['check', 'Não confirmado']);
  assert.equal(none.banner, 'O guia do Jay-S não explica isso. O mais perto que ele chega:');
  assert.ok(!/fontes/.test(none.banner));
});

test('a material reply keeps its own badge and banner', () => {
  setLanguage('pt-BR');
  assert.deepEqual(replyNotes(reply('material', ['confirmed'])).badge, ['ok', 'Das suas fontes']);
  assert.equal(replyNotes(reply('unknown', [])).banner, 'Nada nas fontes que você escolheu fala disso.');
});
