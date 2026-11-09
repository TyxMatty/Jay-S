// Run inside the dev app with: node scripts/cdp-eval.mjs scripts/test-materials.js
// after attaching files by a second launch (`jay-s.exe a.pdf b.pdf`). Reports what the Materials
// section shows and checks hostile ids through the real commands.
const invoke = window.__TAURI_INTERNALS__.invoke;
const list = await invoke('materials_list');
const shown = [...document.querySelectorAll('.material-card')].map((c) => ({
  name: c.querySelector('.material-name')?.textContent,
  meta: c.querySelector('.material-meta')?.textContent,
  kind: c.querySelector('.material-kind')?.value,
  check: c.querySelector('.badge') ? !c.querySelector('.badge').hidden : null,
  state: c.className,
}));
const hostile = {};
for (const id of ['..', '../../x', 'C:\\Windows', 'CON', '0123456789ABCDEF0123456789ABCDEF', '0123456789abcdef0123456789abcdef']) {
  try {
    await invoke('materials_delete', { id });
    hostile[id] = 'DELETED?!';
  } catch (e) {
    hostile[id] = e.code;
  }
}
return { count: list.length, shown, hostile, kinds: list.map((m) => [m.name, m.kind, m.detected]) };
