# Governance & collaboration

Bebop is **AGPL-3.0-or-later** and open to collaboration. This document states how the project
is governed and how to get set up as a collaborator.

## License & DCO

- License: **AGPL-3.0-or-later** (see `LICENSE`). If you run a modified Bebop as a network
  service, you must offer its source to your users.
- All commits must be **signed off** (`git commit -s`) under the **Developer Certificate of
  Origin** (see `DCO.md`). No exceptions.

## How to contribute

1. Fork or branch off `main`.
2. Keep it green: `npm run boot`, `npm test` (159 falsifiable), `npm run typecheck`.
3. Open a PR using the template. CI runs boot + test + typecheck automatically.
4. A maintainer reviews; red-line areas (auth, money, RLS, migrations, bulk edits) need explicit
   human sign-off before merge.

## Roles

- **Maintainer** — merges, cuts releases, owns the guard-OS integrity. Currently the repo owner.
- **Collaborator (write)** — can push branches and open/merge PRs after review.
- **Contributor** — anyone via fork + PR.

## Recommended repository settings (owner action)

These keep the guard OS honest. They are owner-controlled on GitHub; documented here so any
future owner can apply them:

- **Branch protection on `main`:** require a passing CI status (`CI` workflow), require PR
  review (≥1), require signed commits, dismiss stale approvals on push.
- **Issues:** enabled. **Wiki:** disabled (docs live in-repo). **Merge commits:** allowed;
  all commits must still be DCO-signed.
- **Topics:** `ai-agent`, `coding-agent`, `autonomous-agent`, `typescript`, `agpl`,
  `post-quantum`, `vector-symbolic-architecture`, `self-hosted`, `deterministic`, `mesh`.

Owner command reference (needs a token with `repo:admin` / `admin:org` as applicable):

```bash
gh repo edit SyniakSviatoslav/bebop \
  --description "Bebop — your own coding agent. Deterministic guard OS, living memory, post-quantum identity, math-proven telemetry governor. AGPL-3.0." \
  --enable-issues --disable-wiki
gh api repos/SyniakSviatoslav/bebop/topics -X PUT \
  -f names[]=ai-agent -f names[]=coding-agent -f names[]=typescript -f names[]=agpl \
  -f names[]=post-quantum -f names[]=vector-symbolic-architecture -f names[]=autonomous-agent \
  -f names[]=self-hosted -f names[]=deterministic -f names[]=mesh
# Branch protection:
gh api repos/SyniakSviatoslav/bebop/branches/main/protection \
  -X PUT -f required_status_checks='{"strict":true,"contexts":["CI"]}' \
  -f enforce_admins=true -f required_pull_request_reviews='{"required_approving_review_count":1}' \
  -f require_signatures=true -f dismiss_stale_reviews=true
# Add a collaborator (write):
gh api repos/SyniakSviatoslav/bebop/collaborators/<handle> --method PUT -f permission=push
```

> The repo is **public** and the code is fully open. The only settings above that require an
> owner-action are GitHub-side permission/config toggles; the in-repo files (CI, templates, CoC,
> DCO, governance) are already in place and active.
