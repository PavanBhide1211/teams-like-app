# Frontend — Cowork Chat

Tauri 2 desktop shell + React 18 + TypeScript + Vite + Tailwind.

## Layout

```
frontend/
├── src-tauri/        Rust shell: tray icon, notifications, window
├── src/
│   ├── app/          providers, routing, top-level Shell
│   ├── features/     auth/, chat/
│   ├── shared/       api/, ws/, ui/
│   └── styles.css
├── package.json
└── tauri.conf.json
```

## Run

```bash
npm install
npm run tauri:dev      # boots Vite + opens the native window
```

Environment (Vite reads these at build time):

| Variable | Default | Purpose |
|---|---|---|
| `VITE_API_BASE` | `http://localhost:8000` | REST base URL |
| `VITE_WS_BASE`  | `ws://localhost:8000`   | WebSocket base URL |

## Build (release)

```bash
npm run tauri build
```

Produces native installers in `src-tauri/target/release/bundle/`.

## Memory / CPU profile (target)

| Metric | Target | Notes |
|---|---|---|
| Idle RAM | 50–100 MB | OS-native webview, no Chromium bundle |
| Active RAM (1 workspace, ~50 channels) | 120–180 MB | Virtualised lists keep this flat regardless of channel size |
| Binary size | ~10 MB | Stripped release |
| Cold start | < 800 ms | Tauri shell + cached HTML |
