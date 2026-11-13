# AGENTS.md — plugin-transmission

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

Transmission as a first-party lemonfiber plugin, the torrent client that stands in
for qBittorrent where `F14-R31` proves `download.torrent` by a substitute:
`plugin.toml` and the adapter that speaks the `download.torrent@1` contract in
front of Transmission. The adapter is the Rust crate at the root, built on
`lemonfiber-adapter-kit` from the core at the commit `Cargo.toml` pins, into the
image the `Dockerfile` builds. `release.yml` publishes that image from every
commit on `main` and from the train's tag. The
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins this repository and how lemonfiber carries it.

The adapter asks Transmission's JSON-RPC 2.0 interface, `POST /transmission/rpc`,
with HTTP Basic and the session id Transmission hands out in a 409.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **Its proofs' recordings sit under `recordings/transmission/`**, and its
  starting files under `config/transmission/` (`ARCH-R271`).
- **The RPC account comes in through the environment.** The image's start-up
  turns RPC authentication off on every start unless `USER` and `PASS` are set,
  whatever `settings.json` says, so the manifest sets both from the minted
  secrets (`ARCH-R315`).
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **The reader is spec's `first-party-plugin` workflow's,** and lemonfiber
  records the recordings itself. There is no Python here.
- **No `unwrap`, `expect`, `panic` or indexing** outside tests. The lints in
  `Cargo.toml` are the core's.
- **Every value the core sends is held to its shape** before it is written into
  a call.
- **The adapter never writes its credential.** lemonfiber mints the RPC account
  and writes it into the adapter's `upstream.json` as `username` and `password`,
  as the adapter's `upstream` maps them (`ARCH-R315`, `ARCH-R317`); the adapter
  reads it for every call and never logs it. The one file the adapter writes is
  `categories.json`, the category names it was asked to file.
- **The core's crates move with the train.** The `rev` in `Cargo.toml` is the
  core commit `targets.toml` names, the same for every `lemonfiber-*` crate.
- **Moving Transmission's digest** means recording every proof's response again
  against the new image in the same change, moving `RELEASE` and `DIGEST` in
  `src/adapter.rs` with it, and committing the `proofs.json` the checks write.

## Checks

```
just ci
```

The gates that need nothing but this tree. The plugin verdict runs in CI.
