;; bad-imports.wat — RED fixture for the empty-import property gate.
;;
;; This module imports a host function `env.host_secret()` — exactly the kind of
;; import the sovereign-core constraint forbids (anything outside the module is
;; reachable: clock/RNG/socket/syscall). `scripts/check-empty-imports.sh` builds
;; this fixture and asserts the gate REJECTS it (non-zero exit). If the gate ever
;; accepts a module with an import section, it is broken and must not pass CI.
;;
;; (Hand-written WAT so the fixture needs no separate Rust crate to compile.)

(module
  (import "env" "host_secret" (func $host_secret (result i32)))
  (func (export "run")
    (drop (call $host_secret)))
)
