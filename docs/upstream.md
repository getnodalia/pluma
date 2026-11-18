# Pluma upstream

| Field | Value |
|---|---|
| Upstream | `storytold/vectorcraft` |
| Last observed | `72190d6998829802ab91a70f2c133ec596c0c1c2` |
| Last evaluated | `72190d6998829802ab91a70f2c133ec596c0c1c2` |
| Last integrated | `72190d6998829802ab91a70f2c133ec596c0c1c2` (12480 commits since the last sync) on 2026-11-18 |
| Skipped | Graphite frontend and node graph, because Pluma now tracks VectorCraft |
| Adapted | desktop binary name `pluma` |
| Conflicting | `README.md` (kept ours), `AGENTS.md` (upstream plus our quality contract) |

Previous lineage was Graphite `68c3c759`. That tree is no longer the engine.

## How the sync works

Upstream's code arrives renamed: `scripts/rebrand.py` applies `.github/rebrand.toml` (VectorCraft
becomes Pluma, internal names included) to upstream's tree, and the weekly sync
(`.github/workflows/upstream-sync.yml`) records the result as one mirror commit: its only parent is
the previous mirror commit, its subject is `Upstream <sha> with this app's names`, and an
`Upstream-Commit:` trailer names the upstream commit it renames. The sync merges that commit into a
branch and opens a pull request; files listed in `.github/upstream-keep` keep this repository's
version. No mirror branch is pushed, so merge sync pull requests with a merge commit: a squash would
drop the mirror commit the next sync starts from.

The mirror starts at the renamed `eddf6766`, merged into `master` (with this repository's tree) by
the pull request that renamed the app. `python3 scripts/rebrand.py check` fails when an old name
comes back; a line that must keep one says `rebrand: keep`.