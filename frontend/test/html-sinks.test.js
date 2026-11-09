// Text from a file or from the model only reaches the page as text (docs/ARQUITETURA.md, section 7,
// item 4). This is the search that blocks a release: a new place that takes HTML fails here and has
// to be reviewed before it joins the lists below.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';

const src = new URL('../src/', import.meta.url);
const files = readdirSync(src, { recursive: true }).filter((f) => f.endsWith('.js')).map((f) => f.replaceAll('\\', '/'));
const code = (file) => readFileSync(new URL(file, src), 'utf8').split('\n').filter((line) => !line.trim().startsWith('//')).join('\n');

// dom.js: the `html:` of h(). app-shell.js: the theme button's icon. viewer.js: the HTML the Rust
// writer made from a document, escaped there (jay_study::doc::to_html).
const SINKS = { 'ui/app-shell.js': 1, 'ui/dom.js': 1, 'ui/viewer.js': 1 };
// What `html:` may be given: an icon of the code itself, directly or through these variables.
const ICON = /^(icons\.\w+|icon|ICON\[m\.kind\]|(tab|p|z)\.icon)$/;

test('HTML enters the page only at the reviewed places', () => {
  assert.ok(files.length > 20, 'the page code was found');
  const found = {};
  for (const file of files) {
    const n = (code(file).match(/innerHTML|outerHTML|insertAdjacentHTML|document\.write|DOMParser|srcdoc/g) || []).length;
    if (n) found[file] = n;
  }
  assert.deepEqual(found, SINKS);
});

test('h() only takes the code\'s own icons as HTML', () => {
  let seen = 0;
  for (const file of files) {
    for (const [, value] of code(file).matchAll(/\bhtml:\s*([^,}]+)/g)) {
      seen += 1;
      assert.match(value.trim(), ICON, `${file}: html: ${value.trim()}`);
    }
  }
  assert.ok(seen > 20, 'the html: uses were found');
});
