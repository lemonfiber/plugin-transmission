# Writing a lemonfiber plugin

**The plugin you copy.** One manifest, its recorded responses, and a workflow
running the commands a plugin is judged by, so the first thing you see is the
bar you will be held to, and it is already green.

This page is for plugin authors. The [README](../README.md) is for the people who
run your plugin: it has a `{placeholder}` for everything that is yours to fill
in.

## What a plugin is here

Data. A plugin is a `plugin.toml` and a directory of recorded responses, and
there is nowhere in the format to put code — contributed code is never executed,
and no setting, sandbox or grant can make it so. What you are writing is a
description: which image runs, what it can do, what must be true before it
installs, and what it adds to the diagnostics lemonfiber already runs.
lemonfiber reads that description and does the work, and it knows nothing about
your plugin that this file does not say.

That is what makes a stranger's plugin reviewable at all. There is nothing in it
to judge except declarations — and every declaration here is either checked by
the harness or demonstrated by a recording.

## What lemonfiber installs today

A manifest names what it needs from lemonfiber in `[requires].capabilities`, and
lemonfiber refuses to install one that asks for something it does not offer.
lemonfiber 0.16.0, the release `targets.toml` names, offers a plugin
`doctor.contribute` and nothing else. A plugin that adds a service, as this one
does, also asks for `service.add` and `service.health.http`, so lemonfiber
validates and proves it and then refuses to install it, naming both.

CI reports that as a notice, not a failure: `just ci` is green, and the `reader`
step prints `0.16.0 would not install this`. Say so at the top of your README
until lemonfiber offers what your plugin asks for.

## Start here

```sh
gh repo create my-plugin --template lemonfiber/plugin-template
cd my-plugin
just ci                                # green before you have changed anything
```

`just ci` turns this clone's git hooks on as its first step, which is what makes
`.githooks/commit-msg` say before a push what `commitlint`, `dco`, `spec-check`
and `attribution` would say after one.

Then, roughly in this order:

1. **`[plugin]`** — `id`, `name`, `description`, `upstream`, `license`. That
   `license` is the *upstream service's*, and has nothing to do with this
   repository's.
2. **`[[service]]`** — `image`, `digest`, `tag`, `port`, `bind`, and
   `config_path` if the image keeps its state somewhere other than `/config`.
   Pin the digest: the tag is the readable name for it and is never resolved.
3. **`health`** — start the image and time it, then write down what you
   measured rather than a number that looks safe.
4. **`provides`** — a core name out of lemonfiber's published vocabulary for
   each contracted thing your service does, and namespaced names of your own
   (`my-plugin:something`) for everything else.
5. **`[[claim]]`** — one for every core name, binding every probe that
   vocabulary declares for it.
6. **`[[proof]]`** and **`fixtures/`** — record real responses off the image at
   the digest you pinned, and delete the Kavita ones.
7. **`[[contribution]]`** — the checks the doctor should run for your service,
   each with a remedy. Namespaced with your plugin's id.
8. **`targets.toml`** — the lemonfiber release you validated and proved
   against, or, while what your plugin needs is on lemonfiber's `main` and in no
   release, that commit in full as `commit = "<40 hex>"`.
9. **`README.md`** — fill in every `{placeholder}`, delete what does not apply
   to your service, and check every sentence against your `plugin.toml`. It is
   what somebody deciding whether to install your plugin reads.

`just proofs` after each recording tells you whether what you wrote down
matches what you recorded. When something does not hold, lemonfiber names where
in the manifest it is and what was expected, and reports every violation in one
pass rather than the first.

**What a manifest may contain is not written down here.** It is
`plugin-manifest.schema.json`, generated from the types lemonfiber reads a
manifest with and kept at
[`contract/plugin-manifest.schema.json`](https://github.com/lemonfiber/lemonfiber/blob/main/contract/plugin-manifest.schema.json)
in lemonfiber's repository — the one document your editor, this repository's CI
and the catalogue all hold a manifest to. Point your editor at it and you get
completion and inline validation with nothing installed.

The harness is one artefact rather than two. `.github/reader/` lives here and
is copied byte for byte into every plugin repository and into the reviewed
catalogue, [`lemonfiber-plugins`](https://github.com/lemonfiber/lemonfiber-plugins),
whose `harness` job fails when its copy differs from this one. So you meet the
catalogue's bar in your own repository, before you ever open a pull request to
the catalogue.

## Why it describes something real

The template has to **validate and prove unmodified**
([authoring requirements](https://github.com/lemonfiber/spec/blob/main/10-functional/features/f-extensibility/f10-authoring.md)).
A template whose proofs are invented would pass that bar and teach an author to
invent proofs, so this one describes Kavita — a comics, manga and ebook reader — and
every recording under `fixtures/` came off that image at the digest
`plugin.toml` names.

Nothing here is a recommendation to install Kavita. It is here so that the
proofs are proofs.

## What is in it

| File | What it is |
|------|------------|
| `README.md` | What the people who run your plugin read. Placeholders until you fill them in. |
| `docs/development.md` | This page. |
| `plugin.toml` | The whole of what lemonfiber will act on. Identity, the service, the capability it claims, the proofs, the checks it contributes. |
| `fixtures/*.json` | Recorded responses, so everything provable is provable with no live instance anywhere |
| `targets.toml` | Which lemonfiber release, or which commit of its `main`, this is validated against. A CI fact, not a manifest one. |
| `proofs.json` | The outcome of every proof, which lemonfiber's release process re-reads. Generated; CI fails if it is stale. |
| `.github/reader/` | The CI harness. It fetches the lemonfiber release `targets.toml` names and checks it against its published digest, or builds the commit it names, and asks it `lemonfiber plugin claims` and `lemonfiber plugin provenance`. It describes no part of the format and decides no verdict. Not part of what an operator installs. |

## The shape, in the order the manifest carries it

**The service.** Which image runs, at which digest, on which port, and whether
that port is for administration on this machine or for the household. Not how the container is
assembled — lemonfiber writes that, which is what makes *what can this plugin
reach* answerable from the format rather than from the instance.

**What it can do.** `provides` carries two kinds of name and no third. A **core**
name comes from lemonfiber's published capability vocabulary and means the
contracted thing that vocabulary defines; something asking for it can be answered
by this plugin, which is what makes a plugin useful rather than merely readable.
A **namespaced** name — `kavita:opds` — is this plugin's own and is inert until
something asks for it.

**The claim.** A core capability is *demonstrated, not asserted*. The vocabulary
says what must be shown; the `[[claim]]` says where to ask it on this service and
what the answer must be, with a recording of that answer. Leave a probe unbound,
or bind one with a weaker expectation than the probe permits, and the manifest is
refused naming the probe.

A probe asserts what the *service* does, never what the operator's library
happens to hold. The catalogue probe here says "the answer reads as an array" and
nothing about how long it is — a probe demanding a non-empty one would refuse to
install on the machine of somebody who has not copied their books over yet. A
contributed **check** is the opposite and may say exactly that, because a check
reports on the stack rather than gating an install.

**The proofs.** What must hold before this is installed. Every one asserts a
body, never only a status — Docker publishes a port by putting a proxy in front
of it, and that proxy accepts a connection before knowing whether anything inside
is listening. One of the two proofs here exists only to record that: asked for a
path it does not implement, this service answers `200` and its single-page app.

**The contributions.** A row in a register lemonfiber already runs. The doctor
already runs checks independently, bounds each one, keeps `unverified` distinct
from `pass`, and carries a remedy on anything that does not pass; a contributed
check is another row in it, attributed to this plugin wherever it appears. No
code is contributed and none can be — there is nothing for contributed code to
*be*.

The check here and the `guarded` probe ask almost the same question, and the
difference is the point: a probe is asked once, at install; a check is asked
every time the doctor runs. An upgrade, a reconfiguration or a reverse proxy in
front of the service are all ways for it to stop refusing what it used to refuse.

## Running the checks

```sh
just ci                                                  # all of them, in CI's order
```

`just` lists the rest. Each is also a command:

```sh
python3 .github/reader/reader.py reader     # the build targets.toml names, and what it says here
python3 .github/reader/reader.py manifest   # every refusal it makes of the manifest
python3 .github/reader/reader.py proofs     # every assertion on its recording; writes proofs.json
python3 .github/reader/reader.py image      # the digests, their tags, what vouches for them
```

The first run fetches the release into `.lemonfiber/`, which git ignores, and
checks it against the digest published with it. A commit is built there instead,
once, with the toolchain and lockfile it pins, so the first run needs `git` and
`rustup` and takes minutes rather than seconds. Each command then asks it:
`lemonfiber plugin claims .` answers the first three and `lemonfiber plugin
provenance .` the last, and running either yourself gives the same answer in
full.

`reader.py proofs` writes `proofs.json` every time, and CI then compares the file
against the committed one, so a run that says everything passed can still be
refused by `git diff --exit-code proofs.json`.

A capability in `[requires]` that the release does not offer a plugin is the one
refusal reported without failing: it says the plugin would not be installed on
that release, which naming the release in `targets.toml` does not claim.

`just ci` is every gate CI runs over the contents of this repository. The jobs it
leaves out are named in the `justfile` beside the recipe, with what covers each.

SonarCloud's automatic analysis takes an issue ignore only from the project's own
settings on SonarCloud, never from a file in the repository. A repository made from
this one has `docker:S8431` ignored on `**/Dockerfile*` in its SonarCloud project when
the project is imported, because every image here is named by its tag and its digest
together (ADR-0025), which is what that rule reports.

## Recording a fixture

A recording is a moment, and it is reviewed like the manifest: a fixture nobody
can read is a place for something to hide. Record with the service running at the
digest `plugin.toml` names:

```sh
docker run --rm -p 5057:5000 \
  docker.io/jvmilazz0/kavita@sha256:b9c671586db2a6a688da3cb4b45f1319cca33b01e6e760c8bf3c19d60101bdf2
curl -si http://127.0.0.1:5057/api/health
```

The host port is deliberately not `5000`: macOS answers that one itself, with an
AirPlay receiver that returns `403` to everything, and a recording made against
it would be a recording of the wrong server. Then write down what state the
service was in when it answered:

```json
{
  "recorded_from": "docker.io/jvmilazz0/kavita@sha256:…",
  "note": "What this instance was, and why this response is worth keeping",
  "request":  { "method": "GET", "path": "/api/health" },
  "response": { "status": 200, "headers": { "content-type": "text/plain" },
                "json": null, "body_starts_with": "Ok" }
}
```

Moving the image pin means re-recording every fixture against the new image in
the same change. A recording that describes a different image is worse than no
recording.

## The recipe this does not declare, and what one looks like

`[[recipe]]` is in the format and is checked. It is **not** declared here, and the
reason is worth copying rather than the block: a manifest declaring one asks for
`recipe.run` by name, so a lemonfiber that cannot run recipes refuses it.
lemonfiber does not offer `recipe.run`, and a template an author copies should
not be one that installs nowhere.

What one looks like:

```toml
[requires]
capabilities = ["service.add", "service.health.http", "doctor.contribute", "recipe.run"]

[[secret]]
id  = "token"
of  = "kavita"
why = "Kavita answers a library being created only to a signed-in administrator."

[[recipe]]
id    = "adopt-the-library-the-stack-already-fills"
title = "Point it at the comics on disk instead of asking the operator to"
why   = "A reader with no library configured is a reader nobody can read anything in."

[[recipe.step]]
id      = "sign-in"
call    = { method = "POST", to = "kavita", path = "/api/account/login" }
expect  = { status = 200 }
capture = [{ name = "token", from = "json.token", origin = "stack-service" }]

[[recipe.step]]
id     = "create"
expect = { status = 200 }

[recipe.step.call]
method  = "POST"
to      = "kavita"
path    = "/api/library/create"
headers = { Authorization = "Bearer {{token}}" }

[[recipe.pair]]
value = "token"
to    = "kavita"
```

Three rules do the work, and the validator holds all three without running
anything. Every destination is a **name** — a service in this stack or a DNS name
outside it — never an address, a range or a bare host port, because a declared
address is a declared address wherever it points and this machine sits beside a
router's administration page. Every substitution refers to something an earlier
step captured. And every value that could reach a destination has a
`[[recipe.pair]]` behind it: a captured token going anywhere no pair permits is a
validation failure found before a call is made, not after two have landed.

The `[[secret]]` above is not one of the three, and it is not optional either:
every value a step captures has to be declared as a secret of the same `id`, and
lemonfiber refuses a manifest whose recipe takes out a value no `[[secret]]`
names. What installing a plugin commits the operator to holding is something they
read in the manifest, not something they find out afterwards.

The verbosity is the point. Three captures and two destinations are six possible
flows, most of which a plugin will never want — and the ones it does not declare
are the ones nobody has to wonder about afterwards.

## A bundled part

`bundled-part/` is the variant a first-party part of the stack lemonfiber ships
starts from: an upstream's image and the published adapter in front of it, no
Rust and no Python. It describes SABnzbd as the stack carries it, with its
starting file, its recordings and its adapter's conformance answers, and the
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins such a part and how lemonfiber carries it.

A part's repository is this repository with `bundled-part/` laid over its
root, and `registry/convert.py` in
[lemonfiber-plugins](https://github.com/lemonfiber/lemonfiber-plugins) writes it
from the part's description in the media stack:

- **Taken from `bundled-part/`:** `targets.toml`, `justfile`, `.gitignore`,
  `.github/dependabot.yml` and its workflows.
- **The part's own, in place of the variant's:** `plugin.toml`, `proofs.json`,
  `config/<id>/`, `recordings/<id>/` and `conformance/`, and a `README.md` and
  an `AGENTS.md` that say for the part what the variant's say for SABnzbd.
- **Kept from the root:** `.editorconfig`, `.githooks/`, `.markdownlint.jsonc`,
  `typos.toml`, `LICENSE`, `CLAUDE.md`, `.github/CODEOWNERS`,
  `.github/labeler.yml`, and the `labels` and `stale` workflows.
- **Dropped:** `fixtures/`, the root's `proofs.json`, `.github/reader/`,
  `ruff.toml`, `docs/`, `built-part/` and the `built-part` workflow, and the
  `bundle` workflow, which only a part that builds an image runs.

A workflow `bundled-part/` holds replaces the root's of the same name.

What changes from the plugin above:

- **The id is the upstream's service's.** `[plugin].id` is that service's id,
  so the container, `./config/<id>`, the settings and the journal keep the
  names the stack gave them.
- **It declares `replaces` naming its own id.** That is how a manifest takes the
  id, the port and the hostname the bundle reserves for the part (`ARCH-R222`);
  any other service taking one is refused.
- **Its starting files sit in `config/<id>/`**, at most 64 files and 1 MiB, with
  no symbolic link, and lemonfiber writes each into the service's configuration
  directory.
- **Its recordings sit in `recordings/<id>/`**, the paths the stack held them
  at, so the bundle's copy of the tree keeps every path.
- **It declares `reaches` and `asks_for`** on each service, as every
  first-party service does.
- **lemonfiber reaches it through its adapter.** The upstream's service names
  the API it answers in as `native`; the adapter `fronts` it, `speaks` the
  contract and `provides` the capability, and is handed the key lemonfiber
  mints for the upstream through a `[[secret]]` the upstream's starting file
  also names. The adapter's answers to the contract's conformance cases sit in
  `conformance/<contract>/`.
- **It names no profile and no form beyond its own `forms`.** The bundle places
  the part in a profile, and `forms` names exactly the forms whose closure holds
  it.
- **Its CI is spec's.** `plugin.yml` calls spec's `first-party-plugin`
  workflow, which takes this repository's reader at a pinned commit and runs
  `manifest`, `proofs` and `image` as one job, skipped where nothing it reads
  changed. CodeQL analyses the workflows alone, when a change touches
  `.github/`, and the Sonar gate is told no code changed.
- **It is never tagged.** No image is built here, so the catalogue pins a
  revision and the digest of its tree, and a person moves the pin.

The template's own `plugin` workflow judges `bundled-part/` with that same
workflow, so the variant holds as the repositories made from it do.

## A built part

`built-part/` is the variant for a part whose image lemonfiber builds from its
upstream's, because the upstream's image cannot run under the posture its
manifest declares as it is published. It describes Door, Caddy standing in
front of Jellyfin, whose binary carries a file capability that a container
holding no `NET_BIND_SERVICE` refuses to execute. The `Dockerfile` builds from
the digest the stack pins and takes that capability off.

`registry/convert.py --variant built-part` writes a part's repository from it as
it writes one from `bundled-part/`, laid over the root the same way, with these
differences:

- **The part's own as well:** `Dockerfile`, written as a `FROM` of the image,
  tag and digest the stack pins, to which the part adds what its posture lacks.
- **Its image is built and published here.** The `image` workflow calls spec's
  [`first-party-image`](https://github.com/lemonfiber/spec/blob/main/.github/workflows/first-party-image.yml):
  on a pull request it builds the image and runs it under the posture its
  manifest declares, failing with the container's logs where it does not come
  up healthy (`ARCH-R250`). On main it publishes `main-<12 hex>`, and on a tag
  the train cuts it publishes that tag, at the version `plugin.toml` declares
  (`OPS-R90`). Each published image is run under its posture again, and one
  rolling pull request through the release App sets the `digest` and `tag` of
  every service running it (`OPS-R91`).
- **A certificate is declared, never made by the part.** A service that serves
  TLS of its own declares `[service.certificate]`: its key and certificate
  beneath the configuration directory and the names it is made out to.
  lemonfiber makes the pair before the first start, and `first-party-image`,
  `plugin record` and a run proof hand the service a throwaway one made the
  same way (`ARCH-R356`). Door's TLS port serves the one it declares.
- **It is tagged.** A train tag's pin is on `release/image-pin`, and the
  `bundle` workflow asks the catalogue to pin the revision that pull request
  merges. A commit's pin is on `release/image-commit-pin`, which moves the
  manifest and never the bundle.
- **Until the first pin merges, the manifest names the upstream's image**, and
  the service whose id is the plugin's says how the image built here is run.
- **Dependabot also watches the `Dockerfile`**, so the upstream's digest moves
  by a reviewed pull request.

The repository needs a `release` environment admitting main and `v*` tags, the
release App installed, and `packages: write` on its image in GHCR.

The template's `plugin` workflow judges `built-part/` as it judges
`bundled-part/`, and its `built-part` workflow builds the variant's image and
runs it under its posture on every pull request; nothing is published from the
template.

## What is deliberately absent

`[[secret]]` and `[[override]]`, because capturing a value and changing a bundled
setting are recipe verbs, and lemonfiber runs no recipe.

A dashboard widget, because a widget reads a service's API with a credential, and
a credential needs a recipe. A dashboard panel or a command of its own, because
there is no point to declare either at. A minimum lemonfiber version, because a
manifest carries none — `[requires].capabilities` says what this needs, and an
unmet requirement is refused by naming the capability rather than a number that
cannot say which one.

## Licence

The plugin data in this repository is under the Hippocratic License 3.0
(HL3-CORE) — see [LICENSE](../LICENSE). Kavita itself is GPL-3.0-only and is not
distributed here; this repository names an image, it does not contain one.

`license` in `plugin.toml` is a fact about the upstream service and says nothing
about the repository that declares it. Copying this template does not copy a
licence decision: the copy is yours to license.
