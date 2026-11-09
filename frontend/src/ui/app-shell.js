// The window, laid out like Jay-A's: a top bar (brand, Jay-AI pill, theme) over a side nav with the
// sections (Materials, Exams, Mock test, Flashcards, Jay-AI, Settings) as tabs.
import { h } from './dom.js';
import { icons } from './icons.js';
import { cardTitle } from './card-title.js';
import { createTabs } from './tabs.js';
import { toggleTheme, currentTheme } from './theme.js';
import { settings } from '../store/settings.js';
import { resolveStyle } from '../core/style.js';
import { t } from '../core/i18n.js';
import { onAiStatus, aiReady } from '../core/ai.js';
import { isTauri } from '../core/bridge.js';

/** Each language in its own name, so it can be found whichever one is showing. */
const LANGUAGES = [
  ['system', null],
  ['en', 'English'],
  ['pt-BR', 'Português (Brasil)'],
];

function createThemeToggle() {
  const button = h('button', { type: 'button', class: 'icon-btn' });
  const render = () => {
    const dark = currentTheme() === 'dark';
    button.innerHTML = dark ? icons.sun : icons.moon;
    button.setAttribute('aria-label', dark ? t('Switch to light mode') : t('Switch to dark mode'));
    button.title = button.getAttribute('aria-label');
  };
  button.addEventListener('click', toggleTheme);
  // The theme can also change from Settings or the OS.
  new MutationObserver(render).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  render();
  return button;
}

/**
 * Radios for the setting `key`, kept checked on its current value, also when it changes elsewhere
 * (the header's theme button). `renderOption(input, value, label)` builds each option's <label>.
 */
function radioGroup(key, attrs, options, renderOption) {
  const group = h(
    'div',
    { role: 'radiogroup', ...attrs },
    options.map(([value, label]) =>
      renderOption(h('input', { type: 'radio', name: key, value, onChange: () => settings.update({ [key]: value }) }), value, label),
    ),
  );
  const sync = () => {
    const current = key === 'style' ? resolveStyle(settings.get().style) : settings.get()[key];
    for (const input of group.querySelectorAll('input')) input.checked = input.value === current;
  };
  settings.subscribe(sync);
  sync();
  return group;
}

function createSettingsPanel() {
  // Each style option is a small preview drawn with that style's own tokens (styles/theme.css).
  const styleGroup = radioGroup(
    'style',
    { class: 'style-picker', 'aria-labelledby': 'style-label' },
    [['calm', t('Calm')], ['neon', t('Neon')], ['soft', t('Soft')]],
    (input, value, label) =>
      h(
        'label',
        { class: 'style-option' },
        input,
        h(
          'span',
          { class: 'style-preview', 'data-style-preview': value, 'aria-hidden': 'true' },
          h('span', { class: 'style-preview-tile' }, h('span', { class: 'style-preview-avatar' })),
          h('span', { class: 'style-preview-bar' }, h('span', {}), h('span', {}), h('span', { class: 'is-danger' })),
        ),
        h('span', { class: 'style-option-name' }, label),
      ),
  );
  const themeGroup = radioGroup(
    'theme',
    { class: 'segmented', 'aria-labelledby': 'theme-label' },
    [['system', t('System')], ['light', t('Light')], ['dark', t('Dark')]],
    (input, value, label) => h('label', {}, input, label),
  );
  const languageSelect = h(
    'select',
    { class: 'input', id: 'settings-language', 'aria-describedby': 'language-hint', onChange: (e) => settings.update({ language: e.target.value }) },
    LANGUAGES.map(([value, name]) => h('option', { value, selected: settings.get().language === value }, name ?? t('System'))),
  );

  return h(
    'section',
    { class: 'app-panel' },
    h('h1', {}, t('Settings')),
    h(
      'fieldset',
      {},
      cardTitle(icons.palette, t('Appearance')),
      h('div', { class: 'field' }, h('span', { class: 'field-label', id: 'style-label' }, t('Style')), styleGroup),
      h('div', { class: 'field' }, h('span', { class: 'field-label', id: 'theme-label' }, t('Theme')), themeGroup),
      h(
        'div',
        { class: 'field' },
        h('label', { for: 'settings-language' }, t('Language')),
        languageSelect,
        h('p', { class: 'hint', id: 'language-hint' }, t('Jay-S reloads to change the language.')),
      ),
    ),
    h(
      'fieldset',
      {},
      cardTitle(icons.info, t('About')),
      h(
        'div',
        { class: 'field' },
        h('p', { class: 'hint' }, t('Jay-S {version}', { version: __APP_VERSION__ })),
        h('p', { class: 'hint' }, t('Study with your own material. Jay-AI runs on this computer: your files never leave it.')),
      ),
    ),
  );
}

/** A section that arrives in a later stage (docs/ARQUITETURA.md, section 8): its icon and what it will do. */
function comingPanel(icon, title, text, stage) {
  return h(
    'section',
    { class: 'app-panel' },
    h('h1', {}, title),
    h(
      'fieldset',
      { class: 'empty-state' },
      h('span', { class: 'card-icon', html: icon, 'aria-hidden': 'true' }),
      h('p', {}, text),
      h('p', { class: 'hint' }, t('Arrives in stage {n}.', { n: stage })),
    ),
  );
}

/** Jay-C's server pill, here for Jay-AI: not installed, off, loading, ready or working. Clicking it opens the Jay-AI section. */
function createStatusPill(onClick) {
  const pill = h('button', { type: 'button', class: 'status-pill', 'data-state': 'offline', onClick }, isTauri ? t('Jay-AI not installed') : t('Only in the app.'));
  onAiStatus((s) => {
    const [state, text] = s.downloading
      ? ['checking', t('Downloading Jay-AI…')]
      : !aiReady(s)
        ? ['offline', t('Jay-AI not installed')]
        : { off: ['idle', t('Jay-AI off')], loading: ['checking', t('Jay-AI loading…')], ready: ['online', t('Jay-AI ready')], working: ['updates', t('Jay-AI working…')] }[s.server];
    pill.dataset.state = state;
    pill.textContent = text;
  });
  return pill;
}

const tabLabel = (icon, text) => h('span', { class: 'tab-label' }, h('span', { class: 'tab-icon', html: icon }), text);

/**
 * A section whose code loads the first time it's opened (docs/ARQUITETURA.md, section 2: no section
 * costs memory before it's used). `load()` resolves to the section's element.
 */
function lazyPanel(load) {
  const panel = h('div', { class: 'lazy-panel' });
  let loaded = false;
  panel.open = () => {
    if (loaded) return;
    loaded = true;
    load().then((section) => panel.replaceChildren(section));
  };
  return panel;
}

export function mountApp(root) {
  const items = [
    {
      key: 'materials',
      label: tabLabel(icons.book, t('Materials')),
      panel: lazyPanel(() => import('./materials.js').then((m) => m.createMaterialsPanel())),
    },
    {
      key: 'exams',
      label: tabLabel(icons.exam, t('Exams')),
      panel: comingPanel(icons.exam, t('Exams'), t('A new exam from your content in the style you pick (ENEM, UEPG, UEM…), or the questions of your exam mixed with your content. Answer it right here, and see the result at the end or each answer at once in learning mode.'), 3),
    },
    {
      key: 'mock',
      label: tabLabel(icons.timer, t('Mock test')),
      panel: comingPanel(icons.timer, t('Mock test'), t('Take an exam against the clock, as long as you want, then see what you got right and what to review.'), 4),
    },
    {
      key: 'flashcards',
      label: tabLabel(icons.cards, t('Flashcards')),
      panel: comingPanel(icons.cards, t('Flashcards'), t('Cards from your content that adapt to you: short and direct or longer, and more of what you missed.'), 5),
    },
    {
      key: 'jayai',
      label: tabLabel(icons.spark, t('Jay-AI')),
      panel: lazyPanel(() => import('./jayai.js').then((m) => m.createJayAiPanel())),
    },
    { key: 'settings', label: tabLabel(icons.settings, t('Settings')), panel: createSettingsPanel() },
  ];
  const tabs = createTabs({
    idPrefix: 'section',
    label: t('Sections'),
    className: 'app-nav',
    vertical: true,
    items,
    onSelect: (key) => items.find((i) => i.key === key).panel.open?.(),
  });

  const header = h(
    'header',
    { class: 'topbar' },
    h('div', { class: 'brand' }, h('span', { class: 'brand-mark wolf-mark', 'aria-hidden': 'true' }), h('span', { class: 'brand-name' }, 'Jay', h('span', {}, '-'), 'S')),
    createStatusPill(() => tabs.select('jayai', { focus: true })),
    createThemeToggle(),
  );

  // A new language applies on a fresh page.
  settings.subscribe((next, prev) => {
    if (next.language !== prev.language) location.reload();
  });

  root.replaceChildren(header, h('div', { class: 'app-body' }, tabs.list, h('div', { class: 'app-panels' }, items.map((i) => i.panel))), h('p', { class: 'app-version' }, `Jay-S ${__APP_VERSION__}`));
  tabs.select('materials');
}
