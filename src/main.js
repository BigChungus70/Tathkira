import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './styles.css';

const i18n = window.TathkiraI18n;
const THEME_CHOICES = ['system', 'dark', 'light'];

let config = {
  texts: [],
  interval_minutes: 15,
  display_seconds: 8,
  language: 'en',
  position: 'bottom_right',
  theme: 'dark',
  launch_on_startup: false,
};

const byId = (id) => document.getElementById(id);
const statusEl = byId('status');
const appEl = document.querySelector('.app');
const languageSelect = byId('language-select');
const languageTrigger = byId('language-trigger');
const languageMenu = byId('language-menu');
const languageValue = byId('language-value');
let statusTimer;
let snoozeTimer;
let remaining = 0;
let wasSnoozed = false;

function setStatus(message) {
  statusEl.textContent = message;
  clearTimeout(statusTimer);
  statusTimer = setTimeout(() => {
    statusEl.textContent = '';
  }, 1800);
}

function setPoolBlur(active) {
  appEl.classList.toggle('pool-open', active);
}

function formatRemaining(seconds) {
  const pad = (value) => String(value).padStart(2, '0');
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  return hours > 0
    ? `${hours}:${pad(minutes)}:${pad(secs)}`
    : `${pad(minutes)}:${pad(secs)}`;
}

function updateSnoozeState(seconds) {
  remaining = seconds;
  const active = seconds > 0;
  const stateEl = byId('snooze-state');
  stateEl.classList.toggle('active', active);
  stateEl.textContent = active
    ? i18n.t('status.snoozeRemaining', { time: formatRemaining(seconds) })
    : i18n.t('status.notSnoozed');
  byId('snooze-cancel').disabled = !active;
  if (!active && wasSnoozed) {
    setStatus(i18n.t('status.snoozeEnded'));
  }
  wasSnoozed = active;
}

async function pollSnooze() {
  try {
    const seconds = await invoke('get_snooze');
    updateSnoozeState(seconds || 0);
  } catch (error) {
    setStatus(i18n.t('status.snoozeFailed', { error }));
  }
}

function scheduleSnoozePoll() {
  clearTimeout(snoozeTimer);
  snoozeTimer = setTimeout(async () => {
    await pollSnooze();
    scheduleSnoozePoll();
  }, 1000);
}

function applyTranslations() {
  document.querySelectorAll('[data-i18n]').forEach((element) => {
    element.textContent = i18n.t(element.dataset.i18n);
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach((element) => {
    element.placeholder = i18n.t(element.dataset.i18nPlaceholder);
  });
  document.querySelectorAll('[data-i18n-label]').forEach((element) => {
    const label = i18n.t(element.dataset.i18nLabel);
    element.title = label;
    element.setAttribute('aria-label', label);
  });
}

function languageLabel(code) {
  const language = i18n.languages().find((item) => item.code === code);
  return language ? language.label : code.toUpperCase();
}

function setLanguageValue() {
  languageValue.textContent = languageLabel(config.language);
}

function fillLanguages() {
  languageMenu.replaceChildren();
  i18n.languages().forEach((language) => {
    const option = document.createElement('button');
    option.type = 'button';
    option.className = 'select-option';
    option.dataset.language = language.code;
    option.setAttribute('role', 'option');
    option.textContent = language.label;
    option.addEventListener('click', () => selectLanguage(language.code));
    languageMenu.append(option);
  });
  setLanguageValue();
  syncLanguageSelection();
}

function syncLanguageSelection() {
  languageMenu.querySelectorAll('[data-language]').forEach((option) => {
    const selected = option.dataset.language === config.language;
    option.classList.toggle('selected', selected);
    option.setAttribute('aria-selected', String(selected));
  });
}

function closeLanguageMenu() {
  languageMenu.hidden = true;
  languageTrigger.setAttribute('aria-expanded', 'false');
}

function openLanguageMenu() {
  syncLanguageSelection();
  languageMenu.hidden = false;
  languageTrigger.setAttribute('aria-expanded', 'true');
  const selected = languageMenu.querySelector('.selected') || languageMenu.firstElementChild;
  if (selected) selected.focus();
}

async function selectLanguage(next) {
  closeLanguageMenu();
  if (next === config.language) return;
  try {
    await i18n.load(next);
    config.language = next;
    applyTranslations();
    updateSnoozeState(remaining);
    render();
    await persist();
  } catch (error) {
    await i18n.load(config.language);
    applyTranslations();
    updateSnoozeState(remaining);
    setStatus(String(error));
  }
}

function normalizedTheme(theme) {
  return THEME_CHOICES.includes(theme) ? theme : 'dark';
}

function applyTheme() {
  document.documentElement.dataset.theme = normalizedTheme(config.theme);
}

async function persist() {
  try {
    const latest = await invoke('get_config');
    const next = {
      ...latest,
      interval_minutes: config.interval_minutes,
      display_seconds: config.display_seconds,
      language: config.language,
      position: config.position,
      theme: normalizedTheme(config.theme),
      launch_on_startup: config.launch_on_startup,
    };
    await invoke('save_config', { config: next });
    config = next;
    setStatus(i18n.t('status.saved'));
  } catch (error) {
    setStatus(i18n.t('status.saveFailed', { error }));
  }
}

function bindNumericInput(id, min, max, key) {
  const input = byId(id);
  input.addEventListener('input', () => {
    const digits = input.value.replace(/[^\d]/g, '');
    if (digits !== input.value) input.value = digits;
  });
  input.addEventListener('change', async () => {
    const value = Number(input.value);
    if (!Number.isFinite(value)) {
      input.value = String(config[key]);
      return;
    }
    config[key] = Math.min(max, Math.max(min, Math.round(value)));
    input.value = String(config[key]);
    await persist();
  });
}

function render() {
  byId('pool-count').textContent = String(config.texts.length);
  byId('interval').value = String(config.interval_minutes);
  byId('display').value = String(config.display_seconds);
  byId('launch-on-startup').checked = config.launch_on_startup;
  setLanguageValue();
  syncLanguageSelection();
  const theme = normalizedTheme(config.theme);
  document.querySelectorAll('[data-theme-choice]').forEach((button) => {
    button.classList.toggle('active', button.dataset.themeChoice === theme);
  });
  applyTheme();

  document.querySelectorAll('[data-position]').forEach((button) => {
    button.classList.toggle('active', button.dataset.position === config.position);
  });
}

async function applySnooze(minutes) {
  const value = Number(minutes);
  if (!Number.isFinite(value) || value <= 0) return;
  const clamped = Math.min(1440, Math.max(1, Math.round(value)));
  try {
    await invoke('snooze', { minutes: clamped });
    await pollSnooze();
    setStatus(i18n.t('status.snoozed', { minutes: clamped }));
  } catch (error) {
    setStatus(i18n.t('status.snoozeFailed', { error }));
  }
}

function snooze() {
  return applySnooze(byId('snooze-minutes').value);
}

window.addEventListener('error', (event) => setStatus(String(event.message)));
window.addEventListener('unhandledrejection', (event) => setStatus(String(event.reason)));
byId('win-minimize').addEventListener('click', () => invoke('hide_main'));
byId('open-pool').addEventListener('click', () => invoke('show_pool'));
bindNumericInput('interval', 1, 1440, 'interval_minutes');
bindNumericInput('display', 1, 120, 'display_seconds');

byId('snooze-minutes').addEventListener('input', (event) => {
  const digits = event.target.value.replace(/[^\d]/g, '');
  if (digits !== event.target.value) event.target.value = digits;
});

document.querySelectorAll('[data-minutes]').forEach((button) => {
  button.addEventListener('click', () => applySnooze(button.dataset.minutes));
});

document.querySelectorAll('[data-position]').forEach((button) => {
  button.addEventListener('click', async () => {
    config.position = button.dataset.position;
    render();
    await persist();
  });
});

byId('snooze-apply').addEventListener('click', snooze);
byId('snooze-cancel').addEventListener('click', async () => {
  try {
    await invoke('cancel_snooze');
    await pollSnooze();
    setStatus(i18n.t('status.snoozeCancelled'));
  } catch (error) {
    setStatus(i18n.t('status.snoozeFailed', { error }));
  }
});

byId('quit').addEventListener('click', () => invoke('quit_app'));

byId('launch-on-startup').addEventListener('change', async (event) => {
  const enabled = event.target.checked;
  const previous = config.launch_on_startup;
  config.launch_on_startup = enabled;
  render();
  try {
    await persist();
    await invoke('set_autostart', { enabled });
  } catch (error) {
    config.launch_on_startup = previous;
    render();
    try {
      await persist();
    } catch (retryError) {
      setStatus(i18n.t('status.saveFailed', { error: retryError }));
    }
    setStatus(i18n.t('status.autostartFailed', { error }));
  }
});

languageTrigger.addEventListener('click', () => {
  if (languageMenu.hidden) openLanguageMenu();
  else closeLanguageMenu();
});

languageTrigger.addEventListener('keydown', (event) => {
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    openLanguageMenu();
  }
});

languageMenu.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    closeLanguageMenu();
    languageTrigger.focus();
    return;
  }
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
  event.preventDefault();
  const options = [...languageMenu.querySelectorAll('.select-option')];
  const current = options.indexOf(document.activeElement);
  const next = event.key === 'ArrowDown'
    ? Math.min(options.length - 1, current + 1)
    : Math.max(0, current - 1);
  if (options[next]) options[next].focus();
});

document.addEventListener('pointerdown', (event) => {
  if (!languageSelect.contains(event.target)) closeLanguageMenu();
});

document.querySelectorAll('[data-theme-choice]').forEach((button) => {
  button.addEventListener('click', async () => {
    config.theme = button.dataset.themeChoice;
    render();
    await persist();
  });
});

try {
  config = await invoke('get_config');
  applyTheme();
} catch (error) {
  setStatus(i18n.t('status.loadFailed', { error }));
}

try {
  await i18n.load(config.language);
  fillLanguages();
  applyTranslations();
} catch (error) {
  setStatus(String(error));
}

async function reloadFromBackend(showSaved) {
  try {
    config = await invoke('get_config');
    await i18n.load(config.language);
    fillLanguages();
    applyTranslations();
    updateSnoozeState(remaining);
    render();
    if (showSaved) setStatus(i18n.t('status.saved'));
  } catch (error) {
    setStatus(i18n.t('status.loadFailed', { error }));
  }
}

try {
  await listen('pool-changed', () => reloadFromBackend(true));
  await listen('window-shown', async (event) => {
    if (event.payload === 'pool') setPoolBlur(true);
    if (event.payload === 'main') {
      setPoolBlur(false);
      await reloadFromBackend(false);
    }
  });
  await listen('window-hidden', (event) => {
    if (event.payload === 'pool' || event.payload === 'main') setPoolBlur(false);
  });
} catch (error) {
  setStatus(String(error));
}

render();
await pollSnooze();
scheduleSnoozePoll();
