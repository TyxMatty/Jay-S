// `node ml/fetch.mjs <folder>`: downloads the training set listed in ml/sources.csv into
// <folder>/<label>/<file> and prints `sha256  label/file` lines (the format of `sha256sum -c`).
// `node ml/fetch.mjs --check` only validates the list (no network). The folder must be outside the
// repository: features and labels are published, never the files (docs/ARQUITETURA.md, section 7).
// The files are untrusted data: this script only writes them, it never opens or runs them.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// exam_sum: an exam whose questions are mostly sum-type (UEM, UEPG, UFSC); for the app it is an exam.
const LABELS = new Set(['exam', 'exam_sum', 'content', 'both', 'key']);
const HOSTS = new Set([
  'download.inep.gov.br',
  'www.vestibular.uem.br',
  'cps.uepg.br',
  'www2.uepg.br',
  'antiga.coperve.ufsc.br',
  'www.fuvest.br',
  'vestibular.uenp.edu.br',
  'pt.wikipedia.org',
  'pt.wikibooks.org',
  'www.ufrgs.br',
  'www.gutenberg.org',
]);
const MAX_BYTES = 20 << 20;
// Lowercase ASCII only: no folder, no drive, nothing a shell or a CSV reader trips on.
const NAME = /^[a-z0-9][a-z0-9.-]*\.(pdf|txt)$/;
const AGENT = 'jay-s-dataset/0.1 (https://github.com/TyxMatty/Jay-S)';

/** One `label,file,license,url` line. Throws on anything that could write or fetch elsewhere. */
export function parseRow(line) {
  const fields = line.split(',');
  if (fields.length !== 4) throw new Error(`4 fields expected: ${line}`);
  const [label, file, license, url] = fields;
  if (!LABELS.has(label)) throw new Error(`label: ${line}`);
  if (!NAME.test(file) || file.includes('..')) throw new Error(`file name: ${line}`);
  if (!license.trim()) throw new Error(`license: ${line}`);
  const u = new URL(url);
  if (u.protocol !== 'https:' || !HOSTS.has(u.hostname) || u.port || u.username || u.password) {
    throw new Error(`url: ${line}`);
  }
  return { label, file, license, url };
}

export function parseList(text) {
  const rows = text.split(/\r?\n/).filter((l) => l.trim() && !l.startsWith('#')).slice(1).map(parseRow);
  const names = new Set(rows.map((r) => r.file));
  if (names.size !== rows.length) throw new Error('two rows with the same file name');
  return rows;
}

/** The body, refusing more than `max` bytes even when the server gives no length. */
async function body(res, max) {
  const chunks = [];
  let size = 0;
  for await (const chunk of res.body) {
    size += chunk.length;
    if (size > max) throw new Error(`more than ${max} bytes`);
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

/** What the file must look like for its extension: the same sniffing `extract::detect` does. */
export function looksRight(file, bytes) {
  if (file.endsWith('.pdf')) return bytes.subarray(0, 1024).includes('%PDF-');
  return bytes.length > 0 && !bytes.subarray(0, 4096).includes(0);
}

function selfCheck() {
  const ok = 'exam,enem-2023-d1.pdf,public exam,https://download.inep.gov.br/a.pdf';
  assert.equal(parseRow(ok).file, 'enem-2023-d1.pdf');
  assert.equal(parseRow(ok.replace('exam,', 'exam_sum,')).label, 'exam_sum');
  for (const bad of [
    'exam,../x.pdf,l,https://download.inep.gov.br/a.pdf',
    'exam,a/x.pdf,l,https://download.inep.gov.br/a.pdf',
    'exam,x..pdf,l,https://download.inep.gov.br/a.pdf',
    'exam,x.exe,l,https://download.inep.gov.br/a.pdf',
    'exam,Maria.pdf,l,https://download.inep.gov.br/a.pdf',
    'other,x.pdf,l,https://download.inep.gov.br/a.pdf',
    'exam,x.pdf,,https://download.inep.gov.br/a.pdf',
    'exam,x.pdf,l,http://download.inep.gov.br/a.pdf',
    'exam,x.pdf,l,https://example.com/a.pdf',
    'exam,x.pdf,l,https://download.inep.gov.br.example.com/a.pdf',
    'exam,x.pdf,l,https://user@download.inep.gov.br/a.pdf',
    'exam,x.pdf,l,https://download.inep.gov.br:8443/a.pdf',
    'exam,x.pdf,l,file:///C:/Windows/win.ini',
    'exam,x.pdf,l,https://download.inep.gov.br/a.pdf,extra',
  ]) {
    assert.throws(() => parseRow(bad), bad);
  }
  assert.throws(() => parseList(`h\n${ok}\n${ok}`));
  assert.ok(looksRight('a.pdf', Buffer.from('%PDF-1.7')));
  assert.ok(!looksRight('a.pdf', Buffer.from('<html>not found</html>')));
  assert.ok(!looksRight('a.txt', Buffer.from([0x4d, 0x5a, 0, 0])));
}

async function main() {
  selfCheck();
  const here = path.dirname(fileURLToPath(import.meta.url));
  const rows = parseList(readFileSync(path.join(here, 'sources.csv'), 'utf8'));
  // The three published lists must agree: same files, same labels (the set also has files that
  // were not downloaded, so sha256.txt and signals.csv are longer than sources.csv).
  const lines = (name) => readFileSync(path.join(here, name), 'utf8').trim().split(/\r?\n/);
  const hashed = new Map(lines('sha256.txt').map((l) => l.match(/^[0-9a-f]{64} {2}(\w+)\/(\S+)$/).slice(1, 3).reverse()));
  const signals = new Map(lines('signals.csv').slice(1).map((l) => l.split(',').slice(0, 2)));
  assert.deepEqual([...signals].sort(), [...hashed].sort(), 'signals.csv and sha256.txt differ');
  for (const [file, label] of hashed) assert.ok(NAME.test(file) && LABELS.has(label), file);
  for (const r of rows) assert.equal(hashed.get(r.file), r.label, r.file);
  const out = process.argv[2];
  if (out === '--check') return console.log(`${rows.length} sources ok, ${hashed.size} files in the set`);
  if (!out) throw new Error('usage: node ml/fetch.mjs <folder outside the repository> | --check');
  const root = path.resolve(out);
  const repo = path.resolve(here, '..');
  const rel = path.relative(repo, root);
  if (!rel.startsWith('..') && !path.isAbsolute(rel)) throw new Error('the folder must be outside the repository');

  let failed = 0;
  for (const { label, file, url } of rows) {
    const target = path.join(root, label, file);
    try {
      if (!existsSync(target)) {
        // No redirects: a listed host can't send the download somewhere else.
        const res = await fetch(url, { redirect: 'error', headers: { 'user-agent': AGENT }, signal: AbortSignal.timeout(180_000) });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const bytes = await body(res, MAX_BYTES);
        if (!looksRight(file, bytes)) throw new Error('not what the extension says');
        mkdirSync(path.dirname(target), { recursive: true });
        writeFileSync(target, bytes, { flag: 'wx' });
        await new Promise((r) => setTimeout(r, 1000)); // be gentle with the servers
      }
      console.log(`${createHash('sha256').update(readFileSync(target)).digest('hex')}  ${label}/${file}`);
    } catch (e) {
      failed += 1;
      console.error(`${label}/${file}: ${e.cause?.message ?? e.message}`);
    }
  }
  if (failed) process.exitCode = 1;
}

await main();
