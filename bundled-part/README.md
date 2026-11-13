> **This is the bundled-part variant of the lemonfiber plugin template.** How a
> part's repository is made from it is in the template's
> [docs/development.md](../docs/development.md#a-bundled-part). Delete this note.

# SABnzbd for lemonfiber

**Downloads from Usenet.**
SABnzbd is one of the parts the stack lemonfiber ships is built from.
Without it: No Usenet downloads.

## How lemonfiber carries it

The default bundle in
[lemonfiber-plugins](https://github.com/lemonfiber/lemonfiber-plugins) pins this
repository by a revision and the digests of its manifest and its tree, and
lemonfiber compiles that tree in.
lemonfiber reaches SABnzbd through its adapter, the image
`ghcr.io/lemonfiber/sabnzbd-adapter`, which speaks lemonfiber's `download.usenet`
contract in front of it. lemonfiber mints the key SABnzbd answers its API to,
writes it into SABnzbd's starting `sabnzbd.ini` and hands it to the adapter, and
to nothing else. Both images are pinned by digest and neither is built here, so
the release train never tags this repository, and its pin moves by a reviewed
pull request on the catalogue.

## What is here

| Path | What it is |
| --- | --- |
| `plugin.toml` | The part's manifest |
| `config/sabnzbd/` | The files SABnzbd starts with, written into its configuration directory |
| `recordings/sabnzbd/` | The responses its proofs are held to |
| `conformance/download.usenet@1/` | The adapter's answer to each conformance case, in front of the pinned SABnzbd |
| `targets.toml` | The lemonfiber its checks run against |
| `proofs.json` | What those checks said, committed |

## Checks

The plugin verdict is one job of spec's
[`first-party-plugin`](https://github.com/lemonfiber/spec/blob/main/.github/workflows/first-party-plugin.yml)
workflow: the manifest as the lemonfiber `targets.toml` names reads it, every
proof against its recording, every pinned digest, and every recording and
conformance case taken again from the pinned images. This repository holds no
Python of its own. `just ci` runs the gates that need nothing but this tree.

## Licence

The data in this repository is under the licence in `LICENSE`.
SABnzbd itself is under GPL-2.0-only and is not distributed here: this
repository names an image, it does not contain one.
