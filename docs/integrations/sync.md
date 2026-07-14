# Sync (optional)

Multi-device sync is **opt-in** and **self-hosted**. It is the one place Bebop runs a server,
and it uses [Better Auth](https://better-auth.com) — an optional, lazy-loaded dependency.

## Why optional

The default Bebop is a **single-user local CLI** with no auth server at all — fully offline,
fully native. `better-auth` is only pulled in when you run `bebop sync`. This keeps the core
install fast and portable (zero native builds).

## Run it

```bash
npm i -D better-auth      # one-time
bebop sync --port 8787
# ◈ Starting Bebop sync node (Better Auth, self-hosted) on :8787
#    No Supabase. No Fly. Your keys, your machine.
```

The node serves Better Auth's HTTP API from your own machine/infra. Signup:
`http://127.0.0.1:8787/sign-up`.

## Configuration

| Var | Meaning |
| --- | --- |
| `BEBOP_SYNC` | `1` to enable the sync server. |
| `BEBOP_SYNC_PORT` | Port (default 8787). |
| `BEBOP_SYNC_HOST` | Bind host (default 127.0.0.1). |
| `BEBOP_DB` | SQLite file for sync (optional; in-memory otherwise). |
| `BEBOP_AUTH_SECRET` | Session secret — generate a strong one for prod. |

## Security model

- The server is **fail-closed**: without `BEBOP_AUTH_SECRET` it won't issue sessions.
- Sessions are set with `secure`/`httpOnly`/same-site flags; CSRF origin is enforced (the
  sync server forwards the request `Origin` to Better Auth).
- `auth.test.ts` asserts (GREEN) signup+login round-trips and issues a session, and (RED)
  wrong-password / anonymous calls are rejected.

## Tests when the dep is absent

When `better-auth` isn't installed, the sync-server tests **skip cleanly** (detected
side-effect-free) so the rest of Bebop still installs and tests with zero heavy deps. Install
the dep to exercise the sync path.
