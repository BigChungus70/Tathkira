import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './styles.css';

const i18n = window.TathkiraI18n;

let config = {
  texts: [],
  interval_minutes: 15,
  display_seconds: 8,
  language: 'en',
  position: 'bottom_right',
  theme: 'dark',
};

const byId = (id) => document.getElementById(id);
const statusEl = byId('status');
let statusTimer;

window.addEventListener('error', (event) => setStatus(String(event.message)));
window.addEventListener('unhandledrejection', (event) => setStatus(String(event.reason)));
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

function fillLanguages() {
  const select = byId('language');
  select.replaceChildren();
  i18n.languages().forEach((language) => {
    const option = document.createElement('option');
    option.value = language.code;
    option.textContent = language.label;
    select.append(option);
  });
  select.value = config.language;
}

function applyTheme() {
  const theme = config.theme === 'light' ? 'light' : 'dark';
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
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
      theme: config.theme,
    };
    await invoke('save_config', { config: next });
    config = next;
    setStatus(i18n.t('status.saved'));
  } catch (error) {
    setStatus(i18n.t('status.saveFailed', { error }));
  }
}

function render() {
  byId('pool-count').textContent = String(config.texts.length);
  byId('interval').value = String(config.interval_minutes);
  byId('display').value = String(config.display_seconds);
  byId('theme').value = config.theme === 'light' ? 'light' : 'dark';
  applyTheme();

  document.querySelectorAll('[data-position]').forEach((button) => {
    button.classList.toggle('active', button.dataset.position === config.position);
  });
}

byId('win-minimize').addEventListener('click', () => invoke('hide_main'));
byId('open-pool').addEventListener('click', () => invoke('show_pool'));

byId('interval').addEventListener('change', async (event) => {
  const value = Number(event.target.value);
  if (!Number.isFinite(value)) return;
  config.interval_minutes = Math.min(1440, Math.max(1, Math.round(value)));
  event.target.value = String(config.interval_minutes);
  await persist();
});

byId('display').addEventListener('change', async (event) => {
  const value = Number(event.target.value);
  if (!Number.isFinite(value)) return;
  config.display_seconds = Math.min(120, Math.max(1, Math.round(value)));
  event.target.value = String(config.display_seconds);
  await persist();
});

document.querySelectorAll('[data-position]').forEach((button) => {
  button.addEventListener('click', async () => {
    config.position = button.dataset.position;
    render();
    await persist();
  });
});

document.querySelectorAll('[data-minutes]').forEach((button) => {
  button.addEventListener('click', () => {
    byId('snooze-minutes').value = button.dataset.minutes;
  });
});

async function snooze() {
  const minutes = Math.min(1440, Math.max(1, Math.round(Number(byId('snooze-minutes').value))));
  if (!Number.isFinite(minutes)) return;
  try {
    await invoke('snooze', { minutes });
    await pollSnooze();
    setStatus(i18n.t('status.snoozed', { minutes }));
  } catch (error) {
    setStatus(i18n.t('status.snoozeFailed', { error }));
  }
}

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
byId('language').addEventListener('change', (event) => {
  const select = event.target;
  const next = select.value;
  select.blur();
  setTimeout(async () => {
    try {
      await i18n.load(next);
      config.language = next;
      applyTranslations();
      updateSnoozeState(remaining);
      render();
      await persist();
    } catch (error) {
      select.value = config.language;
      await i18n.load(config.language);
      applyTranslations();
      updateSnoozeState(remaining);
      setStatus(String(error));
    }
  }, 0);
});

byId('theme').addEventListener('change', async (event) => {
  config.theme = event.target.value === 'light' ? 'light' : 'dark';
  applyTheme();
  await persist();
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
    if (event.payload === 'main') await reloadFromBackend(false);
  });
} catch (error) {
  setStatus(String(error));
}

render();
await pollSnooze();
scheduleSnoozePoll();
