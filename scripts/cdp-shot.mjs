// Saves a PNG of the running Jay-S page (only the app's page, never the screen), through the
// DevTools Protocol, like cdp-eval.mjs:
//
//   node scripts/cdp-shot.mjs <out.png> [port]
import { writeFileSync } from 'node:fs';

const [, , out, port = '9227'] = process.argv;
if (!out) {
  console.error('usage: node scripts/cdp-shot.mjs <out.png> [port]');
  process.exit(2);
}
const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === 'page' && /^https?:\/\/([^/]+:5178|tauri\.localhost)\//.test(t.url));
if (!page) throw new Error('no Jay-S page');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve) => ws.addEventListener('open', resolve, { once: true }));
ws.send(JSON.stringify({ id: 1, method: 'Page.captureScreenshot', params: { format: 'png' } }));
const reply = await new Promise((resolve) => ws.addEventListener('message', (e) => resolve(JSON.parse(e.data)), { once: true }));
ws.close();
if (!reply.result) throw new Error(JSON.stringify(reply.error));
writeFileSync(out, Buffer.from(reply.result.data, 'base64'));
console.log(out);
