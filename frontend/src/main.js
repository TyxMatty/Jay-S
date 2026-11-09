import { initTheme } from './ui/theme.js';
import { mountApp } from './ui/app-shell.js';
import { settings } from './store/settings.js';
import { setLanguage, resolveLanguage } from './core/i18n.js';

setLanguage(resolveLanguage(settings.get().language));
initTheme();
mountApp(document.getElementById('app'));

// The right-click menu and the browser shortcuts are off in release builds, natively (src-tauri/src/main.rs).
