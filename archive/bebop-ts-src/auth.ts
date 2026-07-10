// Bebop auth — Better Auth, self-hosted, optional, flag-gated.
//
// Design (RESEARCH §1.7 — zero-cloud determinism): auth is LOCAL and OPTIONAL. Bebop runs fully
// offline with no auth at all (the native single-user case). The Better Auth server only activates
// when the user WANTS multi-device sync, and runs it on THEIR machine/infra.
//
// No Supabase, no Fly, no third party.
//
// better-auth is loaded LAZILY (dynamic import) so the rest of Bebop — boot, guard OS, loop, memory,
// tests — has ZERO dependency on it. `npm install` stays fast and portable; the heavy auth stack is
// only pulled in the moment you actually run `bebop sync`. If it is missing, the error is clear.

export interface BebopAuthOptions {
  /** Base URL the sync server is reachable at (used for callbacks/cookies). */
  baseURL?: string;
  /** Secret for session signing. Falls back to a generated-per-process dev secret (NOT for prod). */
  secret?: string;
  /** When true, email+password auth is enabled (recommended for a self-hosted sync node). */
  emailAndPassword?: boolean;
  /** Optional path to a sqlite file; if omitted, an in-memory adapter is used. */
  dbFile?: string;
}

// better-auth is an OPTIONAL dependency resolved lazily, so its types are not present at typecheck
// time. We treat the instance as `any` to keep the rest of the codebase's contract intact.
export type BebopAuth = any;

/** Resolve a Better Auth database adapter — lazily so neither better-auth nor better-sqlite3 load
 *  unless a dbFile is actually requested. */
async function resolveAdapter(dbFile?: string) {
  if (dbFile) {
    // Lazy require so the native module is only loaded when actually requested.
    const Database = (await import('better-sqlite3')).default;
    const db = new Database(dbFile);
    const { betterSqlite3 } = await import('better-auth/adapters/better-sqlite3');
    return betterSqlite3(db);
  }
  const { memoryAdapter } = await import('better-auth/adapters/memory');
  return memoryAdapter(
    // The memory adapter does not auto-create its model tables; seed the standard Better Auth
    // models so reads/writes during signup/login have a home. Production uses dbFile (sqlite).
    { user: [], session: [], account: [], verification: [] },
  );
}

/** Build a self-hosted Better Auth instance. Throws a clear error if `better-auth` is not installed. */
export async function createBebopAuth(opts: BebopAuthOptions = {}): Promise<BebopAuth> {
  let betterAuth: any;
  try {
    betterAuth = (await import('better-auth')).betterAuth;
  } catch {
    throw new Error(
      'bebop sync needs the optional "better-auth" dependency.\n' +
        'Install it with:  npm i -g better-auth   (or add it to your project devDependencies)\n' +
        'Then run:  bebop sync',
    );
  }
  const secret = opts.secret ?? process.env.BEBOP_AUTH_SECRET ?? `dev-${Math.random().toString(36).slice(2)}`;
  const database = await resolveAdapter(opts.dbFile ?? process.env.BEBOP_DB);
  return betterAuth({
    basePath: '/', // sync endpoints at root (e.g. /sign-up/email) — clean for a self-hosted CLI node
    baseURL: opts.baseURL ?? process.env.BEBOP_SYNC_URL,
    secret,
    database,
    emailAndPassword: {
      enabled: opts.emailAndPassword ?? true,
      minPasswordLength: 12,
      autoSignIn: true,
    },
    session: {
      expiresIn: 60 * 60 * 24 * 7, // 7d
      updateAge: 60 * 60 * 24, // refresh daily
    },
    // Bebop is a CLI node: no social login by default. Add providers by passing them in opts if needed.
  });
}
