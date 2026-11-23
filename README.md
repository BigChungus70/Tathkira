# Tathkira

هذا البرنامج صدقة جارية عني وعن أبي وأمي وعموم المسلمين

Lightweight desktop thikr reminder that stays in the tray and shows periodic onscreen reminders.

- **Stack:** Tauri 2, Rust, plain HTML/CSS/JS, Vite.
- **Low RAM:** uses the OS webview instead of bundling Chromium like Electron.
- **Customizable:** thikr pool, reminder interval, on-screen duration, notification position, snoozing, dark/light theme, multiple language support, and launch-on-startup.
- **Multiplatform:** Windows, Linux (Debian and Portable AppImage for others).

## Development setup

Requirements:

- Node.js 24+
- Rust stable
- Tauri 2 system dependencies for your OS
- Windows: WebView2 runtime
- Linux: WebKit2GTK, GTK, AppIndicator, and build tools

Install dependencies:

```powershell
npm ci
```

Run the frontend only:

```powershell
npm run dev
```

Run the full desktop app in development mode:

```powershell
npm run tauri dev
```

Build the frontend:

```powershell
npm run build
```

Build the release app and installers:

```powershell
npm run tauri build
```

Check Rust code:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Generated outputs go to `dist/` and `src-tauri/target/`.

## Localizations

Additional localizations are welcome.

- `public/locales/index.json` registers available languages: `code`, `label`, `direction`, and `file`.
- `public/locales/en.json` is the reference translation file.
- Each locale file must keep the same nested key structure as `en.json`.
- Translate values only. Do not rename keys.
- Preserve placeholders such as `{error}` and `{minutes}` exactly.
- Add the new language to `index.json`, then open a pull request.

After review, proper localization additions can be pulled and merged.
