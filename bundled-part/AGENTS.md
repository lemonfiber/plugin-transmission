# AGENTS.md — plugin-sabnzbd

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

A bundled part: SABnzbd, as the stack lemonfiber ships carries it. The
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins it and how lemonfiber carries it.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **SABnzbd speaks no contract; its adapter does.** SABnzbd declares `native`
  and `replaces` naming its own id, which is how it takes the id, the port and
  the hostname the bundle reserves for it (`ARCH-R222`). `sabnzbd-adapter`
  fronts it and speaks `download.usenet@1`, with the key lemonfiber mints as
  `upstream-key`.
- **Its proofs' recordings sit under `recordings/sabnzbd/`**, its adapter's
  conformance answers under `conformance/download.usenet@1/`, and its starting
  files under `config/sabnzbd/` (`ARCH-R271`).
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **No Python here.** The reader is spec's `first-party-plugin` workflow's.
- **Neither image is built here**, so this repository is never tagged. Moving
  a digest means recording every response again against the new image in the
  same change, and committing the `proofs.json` the checks write.
