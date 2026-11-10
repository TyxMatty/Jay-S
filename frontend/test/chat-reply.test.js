import { test } from 'node:test';
import assert from 'node:assert/strict';
import { replyNotes, goTarget, codeBlock, STARTERS } from '../src/ui/chat.js';
import { setLanguage, t } from '../src/core/i18n.js';

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

test('every starter suggestion has a Portuguese text', () => {
  setLanguage('pt-BR');
  for (const s of STARTERS) assert.notEqual(t(s), s);
});

test('the button to a section needs an app reply and one of our own keys', () => {
  const app = (go) => ({ ...reply('app', ['confirmed']), go });
  for (const key of ['materials', 'exams', 'mock', 'flashcards', 'settings']) assert.equal(goTarget(app(key)), key);
  for (const bad of ['../x', 'a'.repeat(100000), 7, { x: 1 }, 'jayai', 'toString', '__proto__', '', undefined, null]) assert.equal(goTarget(app(bad)), null);
  assert.equal(goTarget({ ...reply('material', ['confirmed']), go: 'exams' }), null);
});

// A tiny DOM: enough tree for `h()` to build into, so the test can look at what ends up where.
class FakeNode {}
class FakeText extends FakeNode {
  constructor(data) {
    super();
    this.data = data;
  }
  get textContent() {
    return this.data;
  }
}
class FakeElement extends FakeNode {
  constructor(tag) {
    super();
    this.tag = tag;
    this.children = [];
  }
  append(...nodes) {
    this.children.push(...nodes);
  }
  setAttribute() {}
  addEventListener() {}
  get textContent() {
    return this.children.map((c) => c.textContent).join('');
  }
}
globalThis.Node = FakeNode;
globalThis.document = { documentElement: {}, createElement: (tag) => new FakeElement(tag), createTextNode: (s) => new FakeText(s) };
const find = (el, tag) => (el.tag === tag ? [el] : []).concat((el.children ?? []).flatMap((c) => find(c, tag)));

test('code is shown as text in a code element, whatever it contains', () => {
  setLanguage('pt-BR');
  const payload = '</pre><script>alert(1)</script>';
  const block = codeBlock({ code: { language: 'python', text: payload } });
  const [code] = find(block, 'code');
  assert.equal(code.textContent, payload);
  assert.equal(find(block, 'script').length, 0);
  assert.equal(find(block, 'pre').length, 1);
  assert.match(block.textContent, /Copiar/);
  assert.match(block.textContent, /não testado/);
  // A language outside our list is not shown as a label.
  assert.doesNotMatch(codeBlock({ code: { language: '<img src=x>', text: 'x' } }).textContent, /img/);
});

test('no code block without a well-formed code', () => {
  for (const bad of [undefined, null, 'x', 7, [], {}, { text: 5 }, { text: '' }, { language: 'python' }, { text: ['a'] }]) assert.equal(codeBlock({ code: bad }), null, JSON.stringify(bad));
  assert.equal(codeBlock({}), null);
});

test('the code strings have a Portuguese text', () => {
  setLanguage('pt-BR');
  for (const s of ['Copy', 'Copied', 'Code written by Jay-AI, not tested: check it before using it.']) assert.notEqual(t(s), s);
});
