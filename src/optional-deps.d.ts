// Ambient declarations for OPTIONAL dependencies that are lazy-loaded and may be absent.
//
// better-auth (+ its adapters) and better-sqlite3 are only needed for the optional `bebop sync`
// multi-device path. They are dynamically imported inside auth.ts, so the rest of Bebop (boot,
// guard OS, loop, memory, tests) compiles and runs with ZERO heavy deps installed.
// These shims keep `tsc --noEmit` green when the optional packages are not present.

declare module 'better-auth' {
  const betterAuth: any;
  export { betterAuth };
  const _default: any;
  export default _default;
}

declare module 'better-auth/adapters/memory' {
  const memoryAdapter: any;
  export { memoryAdapter };
}

declare module 'better-auth/adapters/better-sqlite3' {
  const betterSqlite3: any;
  export { betterSqlite3 };
}

declare module 'better-sqlite3' {
  const Database: any;
  export default Database;
}
