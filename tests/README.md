# Tests

End-to-end smoke test for the chat surface. Hits the running HTTP API + WS — no UI driving — to confirm the happy path: register → create workspace → create channel → send via WS → receive via WS.

## Run

```bash
docker compose up -d postgres redis backend
cd tests
npm install
npx playwright test
```

Set `API_BASE` (default `http://localhost:8000`) and `WS_BASE` (default `ws://localhost:8000`) if your services run elsewhere.
