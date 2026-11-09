// node scripts/cdp-eval.mjs scripts/test-chat-ui.js — the chat screen itself, in the running app: a
// new conversation is created by sending the first question, shows up in the list, reopens from it
// and is deleted from it. Needs the Jay-AI installed (the answer comes from the real model).
const wait = async (what, test, seconds = 180) => {
  for (let i = 0; i < seconds * 4; i++) {
    const v = test();
    if (v) return v;
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`timeout: ${what}`);
};
const $ = (s) => document.querySelector(s);
const invoke = window.__TAURI_INTERNALS__.invoke;
const before = (await invoke('chats_list')).length;

[...document.querySelectorAll('.app-nav button, .app-nav a')].find((b) => b.textContent.includes('Jay-AI')).click();
await wait('the new-conversation screen', () => $('.chat-hero'));
$('.chat-side .btn').click();
const input = await wait('the pill', () => $('.chat-hero .chat-input'));
input.value = 'Em uma frase: o que é fotossíntese?';
await wait('send enabled', () => !$('.chat-hero .chat-send').disabled, 5);
$('.chat-hero .chat-send').click();

await wait('the conversation on screen', () => $('.chat .chat-question'));
const created = (await invoke('chats_list')).length - before;
const t0 = performance.now();
const answer = await wait('the answer', () => $('.chat-reply-head') && !$('.chat-waiting') && $('.chat-turn'));
const seconds = Math.round((performance.now() - t0) / 1000);
const row = await wait('the title in the list', () => $('.chat-item[aria-current="true"]'), 10);
const out = { created, seconds, question: $('.chat-question').textContent, answer: answer.textContent.slice(0, 200), listed: row.textContent, sendBack: !$('.chat-dock .chat-send').disabled };

// Back to a new conversation, then reopen the one just made from the list.
$('.chat-side .btn').click();
await wait('the new-conversation screen again', () => $('.chat-hero'));
out.noneCurrent = !$('.chat-item[aria-current="true"]');
[...document.querySelectorAll('.chat-item')].find((b) => b.textContent === out.listed).click();
await wait('the conversation reopened', () => $('.chat-title'));
out.reopened = { title: $('.chat-title').textContent, turns: document.querySelectorAll('.chat-turn').length };
if (window.KEEP_CHAT) return out;

// Delete it from the list (the confirmation answered here): the screen goes back to a new conversation.
const ask = window.confirm;
window.confirm = () => true;
$('.chat-item[aria-current="true"] + .chat-item-remove').click();
await wait('the new-conversation screen after deleting', () => $('.chat-hero'), 10);
window.confirm = ask;
out.left = (await invoke('chats_list')).length - before;
return out;
