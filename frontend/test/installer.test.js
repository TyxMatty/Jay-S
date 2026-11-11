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

test('the release asset is the Jay-I setup, built from a pinned Jay-I', () => {
  const workflow = readFileSync(new URL('../../.github/workflows/installer.yml', import.meta.url), 'utf8');
  for (const [, ref] of workflow.matchAll(/^\s*-?\s*uses:\s*\S+@(\S+)/gm)) {
    assert.match(ref, /^[0-9a-f]{40}$/, `every action is pinned by full SHA, got ${ref}`);
  }
  const pin = workflow.match(/repository: TyxMatty\/Jay-I\s+ref: (\S+)/);
  assert.ok(pin, 'the workflow checks out TyxMatty/Jay-I');
  assert.match(pin[1], /^[0-9a-f]{40}$/);
  assert.match(workflow, /jay-i\/scripts\/build-setup\.ps1/);
  assert.match(workflow, /15 \* 1024 \* 1024/);
  // Only the setup is uploaded: the hub takes any asset ending in _x64-setup.exe.
  const upload = workflow.match(/upload-artifact@[\s\S]*?\n\s+path: (\S+)/);
  assert.ok(upload, 'the workflow uploads an artifact');
  assert.doesNotMatch(upload[1], /bundle\/nsis/);
  assert.match(upload[1], /^setup-out\//);
  // The updater (the jay-i crate) and the setup come from the same Jay-I commit.
  const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
  const dep = cargo.match(/^jay-i\s*=.*$/m);
  assert.ok(dep, 'Cargo.toml depends on jay-i');
  assert.ok(dep[0].includes(`rev = "${pin[1]}"`), `Cargo.toml jay-i rev must be ${pin[1]}`);
});
