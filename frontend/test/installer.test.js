// What the installer may and may not do (docs/ARQUITETURA.md, section 7, "O instalador"): a change
// to tauri.conf.json or to the capabilities that widens it fails here before it reaches a release.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const json = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const conf = json('../src-tauri/tauri.conf.json');

test('the installer is per user and carries only the app', () => {
  assert.deepEqual(conf.bundle.targets, ['nsis']);
  // No admin: nothing is written outside the profile of who installs.
  assert.equal(conf.bundle.windows.nsis.installMode, 'currentUser');
  // Neither the model, the llama-server nor any file of the student ships in it (RNF-2, RNF-3).
  assert.equal(conf.bundle.resources, undefined);
  assert.equal(conf.bundle.externalBin, undefined);
  // No custom installer script: only Tauri's own template runs on the person's PC.
  assert.equal(conf.bundle.windows.nsis.template, undefined);
  assert.equal(conf.bundle.windows.nsis.installerHooks, undefined);
  // The hub compares this with the release tag.
  assert.match(conf.version, /^\d+\.\d+\.\d+$/);
});

test('the installed page talks to nothing outside the app', () => {
  const csp = conf.app.security.csp;
  assert.equal(csp['default-src'], "'self'");
  assert.equal(csp['script-src'], "'self'");
  assert.equal(csp['connect-src'], "'self' ipc: http://ipc.localhost");
  const permissions = json('../src-tauri/capabilities/default.json').permissions;
  for (const permission of permissions) {
    assert.match(String(permission), /^core:/, `only core permissions, got ${JSON.stringify(permission)}`);
  }
});
