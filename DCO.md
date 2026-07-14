# Developer Certificate of Origin (DCO)

Bebop is distributed under the **AGPL-3.0-or-later** license. To keep the chain of
authorship clean and machine-verifiable, every commit must be signed off with a
`Signed-off-by` line, certifying the DCO:

    Developer Certificate of Origin
    Version 1.1

    By making a contribution to this project, I certify that:

    (a) The contribution was created in whole or in part by me and I have the
        right to submit it under the open source license indicated in the file; or
    (b) The contribution is based upon previous work that, to the best of my
        knowledge, is covered under an appropriate open source license and I have
        the right to submit that work with modifications, whether created in whole
        or in part by me, under the same open source license; and
    (c) The contribution was provided directly to me by some other person who
        certified (a), (b) or (c) and I have not modified it.
    (d) I understand and agree that this project and the contribution are public and
        that a record of the contribution (including all personal information I
        submit with it) is maintained indefinitely and may be redistributed.

Sign off in your commit:

    git commit -s -m "feat: add warp-drive backend adapter"

The `-s` flag appends `Signed-off-by: Your Name <you@example.com>` automatically
(using your `git config user.name` / `user.email`).

TL;DR: **all commits must be `git commit -s`.** PRs with unsigned commits are not merged.
