# AGENTS.md — plugin-door

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

A bundled part: Door, as the stack lemonfiber ships carries it. The
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins it and how lemonfiber carries it.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **One service, speaking no contract**, while the bundle assembles it. It
  declares `replaces` naming its own id, which is how it takes the id, the port
  and the hostname the bundle reserves for it (`ARCH-R222`).
- **Its claims' recordings sit under `recordings/door/`**, and its starting
  files, where it has any, under `config/door/` (`ARCH-R271`).
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **No Python here.** The reader is spec's `first-party-plugin` workflow's.
- **The image is built here** (`OPS-R90`). The `Dockerfile` builds it from
  its upstream's, and it runs under the posture its manifest declares (`ARCH-R250`).
  Its pin moves on the release app's rolling pull request.
- **The certificate its TLS port serves is declared, never made here.**
  `[service.certificate]` names its key and certificate beneath the
  configuration directory and the names it is made out to; lemonfiber makes the
  pair before the first start, and the image, recording and run proof checks
  are handed a throwaway one made the same way (`ARCH-R356`). No program this
  repository ships is run.
