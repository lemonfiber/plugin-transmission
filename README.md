<!--
  This README is for the people who run your plugin. Replace every {placeholder},
  delete what does not apply to your service, and delete this comment.

  It is written for a household service (`bind = "lan"`) that mounts the data
  root (`takes_data = true`). The notes in comments like this one say what to
  change for a loopback service or one that takes no data. Every sentence should
  stay true of the plugin you ship: check each one against your plugin.toml.

  If the lemonfiber release in targets.toml would not install your plugin (CI
  prints "would not install this"), say so at the top of this README, and name
  what it asks for that lemonfiber does not offer.

  How to write the plugin itself is in docs/development.md.
-->

> **This is the lemonfiber plugin template.** To write a plugin, start at
> [docs/development.md](docs/development.md). This README is the one the people
> who run your plugin read: fill in every `{placeholder}` and delete this note.

# {Service} for lemonfiber

**{What it does for the person running the stack, in one sentence. This is your
`description` in plugin.toml.}** This plugin adds {Service} to your lemonfiber
stack, where everyone on your home network can reach it.

Without it, {your `without_it` in plugin.toml}.

## What you get

- **{Service} {version}**, running as a service in your stack.
- **An address the household can remember:** `{hostname}.<your domain>`,
  through the stack's proxy. {Service} also answers on port {port} of this
  machine.
- **A link on the stack's dashboard**, in the {dashboard group} group.
- **{One line per thing the service does for the household.}**
- **{Number} more checks in `lemonfiber doctor`**, each with a fix when it fails:

<!-- One row per [[contribution]] at doctor.check: its title, and what it notices in plain words. -->

| Check | What it notices |
| --- | --- |
| {The check's title} | {What it notices, in plain words} |

## What it needs

- **A lemonfiber with the `lemonfiber plugin` commands.** lemonfiber 0.15.0 and
  earlier do not have them. `lemonfiber version` says which version you have.
- **A stack lemonfiber has already set up on this machine.** Run
  `lemonfiber setup` first if you have not. Installing a plugin is refused on a
  machine with no stack.
- **Port {port} free** on this machine.
- **Disk space** for the {Service} image, and for {Service}'s own data in
  `config/{service-id}` in your stack directory.
- **Access to `{registry}`** the first time {Service} starts, to download the
  image.
- **{Any account, licence or hardware it needs, or: No accounts anywhere else.}**

## Install

Get a copy of this repository, then ask lemonfiber what installing it would do.
`--dry-run` settles everything a real install settles and writes nothing:

```sh
git clone {repository URL}
lemonfiber plugin install ./{repository directory} --dry-run
lemonfiber plugin install ./{repository directory}
```

Keep the `./`: it marks a directory on this machine rather than the name of a
plugin in the catalogue.

When you install, lemonfiber:

1. Reads `plugin.toml` and checks all of it. If anything in it does not
   conform, it names every problem at once and changes nothing.
2. Writes {Service}'s container, its configuration directory, a
   `{hostname}` site for the proxy and a dashboard entry, and records each change
   as it makes it.
3. Starts {Service} and asks it the questions this plugin declares, to show
   {what the proofs establish, in plain words}.
4. Runs the stack's own checks before and after, and compares the two.
5. Records {Service} as installed only when all of that holds. If anything
   fails, it puts back everything it wrote, and your machine is as it was.

[Installing a plugin](https://docs.lemonfiber.app/plugins/installing-a-plugin/)
explains each step in more detail. `lemonfiber plugin installed` lists what is
installed, where each plugin came from, and how each of its services is reached.

## Set it up

1. **{The first thing to do after installing, such as creating the
   administrator account.}**
2. **{Where the library is.}** {Service} sees your data root at `/data`.
   {Which folder in it to point {Service} at.}

## What it changes on your machine

Everything below is in your stack directory, or is a setting of the {Service}
container lemonfiber writes:

<!--
  For a loopback service: the Network row is "Port {port} on `127.0.0.1`, so it
  is reachable from this machine only", and delete the Proxy row.
  For a service with `takes_data = false`: delete the Your library row.
-->

| What | Where |
| --- | --- |
| {Service}'s container | `compose/plugins/{plugin-id}.yml`, in a Compose profile of its own, `plugin-{plugin-id}` |
| {Service}'s own data | `config/{service-id}`, which {Service} sees as `{config_path}` |
| Your library | Your whole data root, which {Service} sees as `/data`, with permission to write |
| Network | Port {port} on the address your household services use (`LAN_BIND`, every interface unless you narrowed it) |
| Proxy | A `{hostname}` site in `config/caddy/Caddyfile` |
| Dashboard | A {Service} link in the {dashboard group} group of `config/homepage/services.yaml` |

A plugin cannot ask for more than this. lemonfiber writes the container itself,
and the plugin format has no field for another mount, another address or a line
of proxy configuration.

## What it sends anywhere

- **The image.** lemonfiber downloads {Service} from `{image}`, pinned to one
  exact build (`{digest}`). If you have switched off downloading with
  `LEMONFIBER_REACH_REGISTRY`, nothing is downloaded and the image has to be on
  this machine already.
- **Nothing else from this plugin.** {Keep this only while plugin.toml declares
  no [[recipe]], [[secret]] or [[override]].} It names no host outside your
  stack, captures no password or key, and changes no setting of the services
  lemonfiber bundles. Its doctor checks ask only your own {Service}.
- **{Service} itself.** {What the service itself connects to, if you know it
  and can show it. Otherwise: What {Service} connects to is {Service}'s own
  behaviour. This plugin does not configure it.}

## Update

Get the newer version of this repository, then update from it:

```sh
git -C {repository directory} pull
lemonfiber plugin update ./{repository directory} --dry-run
lemonfiber plugin update ./{repository directory}
```

The new version is checked and proved the way an install is. Your machine is on
the old version or the new one at every moment. If the new one does not hold,
lemonfiber puts the old one back and says which version you are on.
{Service}'s own data in `config/{service-id}` stays where it is.

## Remove

```sh
lemonfiber plugin remove {plugin-id} --dry-run
lemonfiber plugin remove {plugin-id}
```

Removing stops {Service} and takes out everything the install wrote: the
container, the proxy site and the dashboard link. If you have edited either of
those by hand since, lemonfiber refuses rather than overwrite your edit.

Two things stay:

- **{Service}'s own data** in `config/{service-id}`. lemonfiber does not delete
  a directory holding something it did not put there, and it lists what it left.
- **Your library.** Installing never wrote to your data root, so removing has
  nothing there to take back.

## Getting help

- **A question about lemonfiber:** ask on [Discord](https://discord.nightworks.io).
- **Something wrong with installing, updating, removing or the doctor checks:**
  open an issue on this repository. `lemonfiber support` shows what a support
  bundle would hold, with passwords and keys replaced;
  `lemonfiber support --write` writes it for you to attach. It sends nothing
  anywhere by itself.
- **Something wrong in {Service} itself:** {Service}'s own project, at
  `{upstream}`.

## Licence

{The licence of the plugin data in this repository.} {Service} itself is under
{upstream licence} and is not distributed here: this repository names an image,
it does not contain one.

## Working on this plugin

How the manifest is built, what each proof establishes, and how to run the
checks CI runs: [docs/development.md](docs/development.md).
