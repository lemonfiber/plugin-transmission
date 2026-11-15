# Transmission for lemonfiber

**Downloads the torrents everything else asks for.**
Transmission is a torrent client lemonfiber can run in place of qBittorrent.
Without it: No torrent downloads.

## How lemonfiber carries it

lemonfiber installs Transmission with the adapter this repository holds, which
speaks lemonfiber's `download.torrent` contract in front of Transmission: a Rust
crate built on the core's adapter kit, into the image
`ghcr.io/lemonfiber/transmission-adapter`, published from every commit on `main`
and from each tag the release train cuts. lemonfiber reaches Transmission through
this adapter alone. The adapter asks Transmission's JSON-RPC interface at
`/transmission/rpc` for what it is downloading and seeding, stops and starts it,
sets its speed limits and their schedule, the port it takes peers on and the
folder it downloads into.
lemonfiber mints the RPC account the adapter signs in with and writes its name
and password, before Transmission first starts, into `rpc-username` and
`rpc-password` in Transmission's configuration directory, each readable by its
owner alone. The image's start-up reads them through `FILE__USER` and
`FILE__PASS` on every start and writes them into `settings.json`, where
Transmission keeps the password as a salted hash, so the RPC never answers
without the account. lemonfiber writes the same account into the adapter's
`upstream.json`.

Transmission holds a download folder and no categories: a curator files each
category in a folder of its own name under that folder. So filing a category sets
the download folder, and the adapter keeps the category names it was asked to file
in `categories.json` in its own configuration directory.

## What is here

| Path | What it is |
| --- | --- |
| `plugin.toml` | The plugin's manifest |
| `config/transmission/` | The `settings.json` Transmission starts with, which downloads under `/data`, and the two files the RPC account is written into |
| `recordings/transmission/` | The responses its proofs are held to |
| `src/` | The adapter |
| `conformance/download.torrent@1/` | The adapter's answer to each conformance case, in front of the pinned Transmission |
| `Dockerfile` | The adapter's image |
| `targets.toml` | The lemonfiber its checks run against, and the core the adapter is built on |
| `proofs.json` | What those checks said, committed |

## Checks

The plugin verdict is one job of spec's
[`first-party-plugin`](https://github.com/lemonfiber/spec/blob/main/.github/workflows/first-party-plugin.yml)
workflow: the manifest as the lemonfiber `targets.toml` names reads it, every
proof against its recording, and every pinned digest. The adapter is held to its
format, its lints, its tests, full line coverage and its dependency policy, and its
image runs as lemonfiber runs it, and `recordings` takes every proof and
conformance case again against the pinned Transmission. `just ci` runs the gates
that need nothing but this tree.

## Licence

The code and data in this repository are under the licence in `LICENSE`.
Transmission itself is under GPL-2.0-only or GPL-3.0-only and is not distributed
here: this repository names an image, it does not contain one.
