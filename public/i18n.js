(function () {
  const dictionaries = new Map();
  let manifest = null;
  let strings = {};
  let current = null;

  async function getManifest() {
    if (!manifest) {
      const response = await fetch('locales/index.json');
      if (!response.ok) {
        throw new Error('Could not load locale manifest');
      }
      manifest = await response.json();
    }
    return manifest;
  }

  async function load(code) {
    const list = await getManifest();
    const fallback = list.languages.find((item) => item.code === list.default) || list.languages[0];
    const entry = list.languages.find((item) => item.code === code) || fallback;

    if (!entry) {
      throw new Error('No locales available');
    }

    if (!dictionaries.has(entry.code)) {
      const response = await fetch('locales/' + entry.file);
      if (!response.ok) {
        throw new Error('Could not load locale ' + entry.code);
      }
      dictionaries.set(entry.code, await response.json());
    }

    strings = dictionaries.get(entry.code);
    current = entry;
    document.documentElement.lang = entry.code;
    document.body.lang = entry.code;
    document.body.dir = entry.direction;
    return entry;
  }

  function t(key, params) {
    let value = strings;
    for (const part of key.split('.')) {
      if (value == null) {
        return key;
      }
      value = value[part];
    }
    if (typeof value !== 'string') {
      return key;
    }
    if (params) {
      for (const [name, replacement] of Object.entries(params)) {
        value = value.split('{' + name + '}').join(String(replacement));
      }
    }
    return value;
  }

  function languages() {
    return manifest ? manifest.languages : [];
  }

  window.TathkiraI18n = {
    load,
    t,
    languages,
    get current() {
      return current;
    },
  };
})();
