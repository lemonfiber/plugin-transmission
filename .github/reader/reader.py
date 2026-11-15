#!/usr/bin/env python3
"""Ask the lemonfiber `targets.toml` names what this plugin comes to.

**This is CI harness, not plugin content.** Nothing under `.github/` is
installed, and lemonfiber never runs any of it (`F3-R6`).

Every verdict here is lemonfiber's own. `lemonfiber plugin claims` holds the
manifest to the schema, the capability vocabulary and the extension points its
build publishes, and runs every probe, proof and contributed check against the
recording it names (`F10-R3`, `F10-R4`). `lemonfiber plugin provenance` asks each
image's registry who vouched for it (`F3-R25`). This file fetches that release, or
builds that commit, asks it, and writes down what it said; it decides nothing a
manifest means.

    python3 .github/reader/reader.py reader     the build is the one named, and reads this plugin
    python3 .github/reader/reader.py manifest   nothing about the manifest is refused
    python3 .github/reader/reader.py proofs     every assertion holds on its recording, or fails as declared; writes proofs.json
    python3 .github/reader/reader.py image      every pinned digest resolves; what vouches for it

One verdict is reported and not failed on: an assertion failing as declared,
on a recording its manifest says it fails on, on the constraint it names and no
other (`F10-R13`). It is written to `proofs.json` apart from passed and failed,
with the recording, the constraint, what the answer held there and the reason,
and it is never counted as passed. Whether a failure is the one declared is the
release's to decide, as every other verdict is.

One refusal is reported and not failed on: a capability in `[requires]` that the
release does not offer a plugin. That is the release saying the plugin would not
be installed on it, which `targets.toml` does not claim; `F3-R21` is why the
refusal names the capability rather than a version, and `reader` prints it.

`targets.toml` names one of two things. A release, as `lemonfiber =
"X.Y.Z"`, is fetched once into `.lemonfiber/` beside the manifest and checked
against the digest published with it. A commit of lemonfiber, in full, as
`commit`, is what a plugin names before a release carries what it needs: it is
refused unless `main` holds it, or, where the run sets `LEMONFIBER_PULL_HEADS` to
`true` as a plugin's own checks do, the head of a lemonfiber pull request holds
it. It is then built once the way the release train's gate builds its candidate,
with the `dist` profile, the toolchain and lockfile the commit pins and every
submodule it embeds, and kept in `.lemonfiber/` under the commit. Either is asked
the same questions, and the report names the one that answered. The catalogue and
the release train set no such variable, and take no commit `main` does not hold.

Exit 0 = the answer is the one asked for, 1 = it is not, or lemonfiber could not
be asked.
"""

from __future__ import annotations

import hashlib
import io
import json
import os
import pathlib
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.request
from typing import NamedTuple

ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "plugin.toml"
TARGETS = ROOT / "targets.toml"
REPORT = ROOT / "proofs.json"
CACHE = ROOT / ".lemonfiber"
RELEASES = "https://github.com/lemonfiber/lemonfiber/releases/download"
CORE = "https://github.com/lemonfiber/lemonfiber.git"
COMMITTED = "https://raw.githubusercontent.com/lemonfiber/lemonfiber"
SCHEMA = "plugin-manifest.schema.json"
CONTRACT = "contract"
TRUNK = "main"
PROFILE = "dist"
DIRECTIVE = "#:schema "
FETCH_TIMEOUT_S = 120

# The envelope kind a build wrapping `plugin claims --json` names its answer by.
CLAIMS_KIND = "plugin-claims"

# The one refusal that is about the release rather than the manifest.
UNOFFERED = "is not something this build offers a plugin"

# The one verdict besides passed that does not fail a run, and what each
# declaration behind it has to say for the report to name it (`F10-R13`).
AS_DECLARED = "failing-as-declared"
DECLARED = ("fixture", "constraint", "held", "reason")

# What `targets.toml` names a release under, and a commit under.
RELEASE_KEY = "lemonfiber"
COMMIT_KEY = "commit"

# The variable a run sets to `true` to take a commit a lemonfiber pull request's head
# holds as well as one `main` holds, the refs those heads are fetched from, and where
# they are kept in the checkout.
PULL_HEADS_ENV = "LEMONFIBER_PULL_HEADS"
PULL_HEADS = "refs/pull/*/head"
PULLED = "refs/pulled"

# A commit as git names one in full. A short one names whatever it happens to
# match on the day.
FULL_COMMIT = re.compile(r"\A[0-9a-f]{40}\Z")

# What this runs on, and the name the release gives the build for it.
BUILDS = {
    ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("Darwin", "arm64"): "aarch64-apple-darwin",
    ("Darwin", "x86_64"): "x86_64-apple-darwin",
}


# How often a registry that did not answer is asked in all, how long the first wait
# before asking again is, and what each wait is multiplied by for the next.
REGISTRY_ATTEMPTS = 4
REGISTRY_FIRST_WAIT_S = 5
REGISTRY_BACKOFF = 3

# What a registry says where it holds no such image, tag or digest. Any other failure
# is a registry that did not answer: a rate limit, a server error, the network.
ABSENT = re.compile(r"not found|manifest unknown|name unknown", re.IGNORECASE)


class Unasked(RuntimeError):
    """lemonfiber could not be fetched, built or asked, so nothing was decided."""


class Unanswered(RuntimeError):
    """A registry did not say whether it holds a reference, however often it was asked."""


class Target(NamedTuple):
    """The lemonfiber `targets.toml` names: a release, or a commit."""

    name: str
    commit: bool

    def __str__(self) -> str:
        return f"lemonfiber at {self.name}" if self.commit else self.name


def targeted() -> Target:
    """The release or the commit `targets.toml` names, and nothing else (`REPO-R56`)."""
    read = tomllib.loads(TARGETS.read_text(encoding="utf-8"))
    release, commit = read.get(RELEASE_KEY), read.get(COMMIT_KEY)
    if release is not None and commit is not None:
        raise Unasked(f"{TARGETS.name} names the release {release!r} and the commit {commit!r}; a plugin targets one")
    if commit is not None:
        if not isinstance(commit, str) or not FULL_COMMIT.match(commit):
            raise Unasked(f"{TARGETS.name} names the commit {commit!r}, which is not a full commit")
        return Target(commit, commit=True)
    if not isinstance(release, str) or not release:
        raise Unasked(f"{TARGETS.name} names neither a lemonfiber release nor a commit")
    return Target(release, commit=False)


def build() -> str:
    named = BUILDS.get((platform.system(), platform.machine()))
    if named is None:
        raise Unasked(f"no lemonfiber build is published for {platform.system()} {platform.machine()}")
    return named


def fetched(url: str) -> bytes:
    with urllib.request.urlopen(url, timeout=FETCH_TIMEOUT_S) as answer:
        return answer.read()


def binary() -> pathlib.Path:
    """The `lemonfiber` `targets.toml` names, fetched or built the first time it is asked for."""
    target, named = targeted(), build()
    held = CACHE / target.name / named / "lemonfiber"
    if held.is_file():
        return held
    if target.commit:
        built(target.name, named, held)
    else:
        released(target.name, named, held)
    held.chmod(held.stat().st_mode | stat.S_IXUSR)
    return held


def released(version: str, named: str, held: pathlib.Path) -> None:
    """The release's `lemonfiber`, fetched and checked against its published digest."""
    archive = f"lemonfiber-{named}.tar.xz"
    base = f"{RELEASES}/v{version}/{archive}"
    body = fetched(base)
    published = fetched(f"{base}.sha256").decode("utf-8").split()[0]
    if hashlib.sha256(body).hexdigest() != published:
        raise Unasked(f"{archive} from v{version} is not the file its published digest names")
    with tarfile.open(fileobj=io.BytesIO(body), mode="r:xz") as unpacked:
        member = unpacked.extractfile(f"lemonfiber-{named}/lemonfiber")
        if member is None:
            raise Unasked(f"{archive} from v{version} holds no lemonfiber")
        held.parent.mkdir(parents=True, exist_ok=True)
        held.write_bytes(member.read())


def ran(*command: str, at: pathlib.Path) -> str:
    """One step of building a commit, its output, or why it failed."""
    done = subprocess.run(command, cwd=at, capture_output=True, text=True, check=False)
    if done.returncode != 0:
        lines = done.stderr.strip().splitlines()
        raise Unasked(f"{' '.join(command[:2])} failed building lemonfiber: {lines[-1] if lines else f'it exited {done.returncode}'}")
    return done.stdout


def pull_heads_taken() -> bool:
    """Whether this run takes a commit a lemonfiber pull request's head holds."""
    return os.environ.get(PULL_HEADS_ENV) == "true"


def holds(commit: str, *refs: str, at: pathlib.Path) -> bool:
    """Whether any of `refs` holds `commit`, which a commit git has not got is not."""
    holding = subprocess.run(
        ["git", "for-each-ref", "--count=1", "--contains", commit, "--format=%(refname)", *refs],
        cwd=at, capture_output=True, text=True, check=False,
    )
    return holding.returncode == 0 and holding.stdout.strip() != ""


def unheld(commit: str, source: pathlib.Path) -> str | None:
    """Why the commit may not be built: `main` does not hold it, nor, where this run takes
    them, does any lemonfiber pull request's head."""
    if holds(commit, f"refs/remotes/origin/{TRUNK}", at=source):
        return None
    if not pull_heads_taken():
        return f"{TARGETS.name} names the commit {commit}, which lemonfiber's {TRUNK} does not hold"
    ran("git", "fetch", "--quiet", "origin", f"+{PULL_HEADS}:{PULLED}/*", at=source)
    if holds(commit, PULLED, at=source):
        return None
    return f"{TARGETS.name} names the commit {commit}, which neither lemonfiber's {TRUNK} nor the head of any of its pull requests holds"


def built(commit: str, named: str, held: pathlib.Path) -> None:
    """lemonfiber built at a commit, the way the release train's gate builds its
    candidate, and kept under the commit.

    The commit is held to `main`, or to a pull request's head where this run takes
    one, before anything is built. The checkout is made outside this repository, so
    no cargo configuration here reaches the build, and is thrown away once the
    binary is kept; cargo's own target directory, wherever it is configured to be,
    is not. The binary is moved into place whole, so a build that stops part-way
    leaves nothing that reads as kept.
    """
    with tempfile.TemporaryDirectory() as scratch:
        source = pathlib.Path(scratch) / "lemonfiber"
        ran("git", "clone", "--quiet", "--no-checkout", "--filter=blob:none", CORE, str(source), at=pathlib.Path(scratch))
        refused = unheld(commit, source)
        if refused is not None:
            raise Unasked(refused)
        ran("git", "checkout", "--quiet", "--detach", commit, at=source)
        ran("git", "submodule", "update", "--init", "--recursive", "--depth", "1", at=source)
        print(f"building {Target(commit, commit=True)} for {named}; it is kept, so this happens once.", file=sys.stderr)
        building = subprocess.run(
            ["cargo", "build", "--locked", "--profile", PROFILE, "--target", named, "-p", "lemonfiber"],
            cwd=source, check=False,
        )
        if building.returncode != 0:
            raise Unasked(f"lemonfiber at {commit} did not build for {named}; cargo exited {building.returncode}")
        target_dir = json.loads(ran("cargo", "metadata", "--format-version", "1", "--no-deps", at=source))["target_directory"]
        held.parent.mkdir(parents=True, exist_ok=True)
        kept = held.with_name(f"{held.name}.part")
        shutil.copyfile(pathlib.Path(target_dir) / named / PROFILE / "lemonfiber", kept)
        kept.replace(held)


def plugin(*asked: str) -> subprocess.CompletedProcess:
    """One question to `lemonfiber plugin`, about this directory."""
    return subprocess.run(
        [str(binary()), "plugin", *asked, str(ROOT)],
        capture_output=True, text=True, check=False,
    )


def claimed() -> dict:
    """What `lemonfiber plugin claims --json` says, out of its envelope where the build
    answers in one, or why it said nothing."""
    answer = plugin("claims", "--json")
    try:
        said = json.loads(answer.stdout)
    except ValueError:
        raise Unasked(f"lemonfiber read nothing here: {answer.stderr.strip()}") from None
    return said["data"] if said.get("kind") == CLAIMS_KIND else said


def reader() -> int:
    """The build is the one `targets.toml` names, and it reads this plugin.

    A release says its own version, and saying another means the download is not
    that release. A commit's build says the version its workspace declares, which
    names no commit, so it is shown rather than compared: what says it is that
    commit is that it was built from it and is kept under it.
    """
    target = targeted()
    said = subprocess.run([str(binary()), "--version"], capture_output=True, text=True, check=False)
    if target.commit:
        print(f"::notice::targets.toml names {target}, built here; it says {said.stdout.strip()!r}")
    elif said.stdout.strip() != f"lemonfiber {target.name}":
        print(f"::error::targets.toml names {target}, and the binary says {said.stdout.strip()!r}")
        return 1
    read = claimed()
    print(plugin("claims").stdout)
    for refusal in read["refusals"]:
        if UNOFFERED in refusal["message"]:
            print(f"::notice::{target} would not install this: {refusal['message']}")
    return 0


def schema_for(target: Target) -> str:
    """The manifest schema of what `targets.toml` names: published with a release
    (`F10-R1`), or committed at a commit."""
    if target.commit:
        return f"{COMMITTED}/{target.name}/{CONTRACT}/{SCHEMA}"
    return f"{RELEASES}/v{target.name}/{SCHEMA}"


def named_schema() -> str | None:
    """The schema the manifest's first line points an editor at, if it names one."""
    first = MANIFEST.read_text(encoding="utf-8").split("\n", 1)[0].strip()
    return first.removeprefix(DIRECTIVE).strip() if first.startswith(DIRECTIVE) else None


def schema_disagreement(target: Target) -> str | None:
    """Why the schema line and `targets.toml` disagree, where they do (`F10-R17`).

    A manifest naming no schema is told which line would let an editor check it,
    and is not refused for it: a plugin may be written without one.
    """
    wanted, named = schema_for(target), named_schema()
    if named is None:
        print(f"::notice file=plugin.toml::an editor checks this manifest as it is typed once its first line is `{DIRECTIVE}{wanted}`")
        return None
    if named == wanted:
        return None
    return f"the first line names the schema {named}, and targets.toml names {target}, whose schema is {wanted}"


def manifest() -> int:
    """Every refusal the release makes of the manifest, bar what it does not offer."""
    read = claimed()
    held = [one for one in read["refusals"] if UNOFFERED not in one["message"]]
    for refusal in held:
        print(f"::error file=plugin.toml::{refusal['location']}: {refusal['message']}")
    disagreement = schema_disagreement(targeted())
    if disagreement is not None:
        print(f"::error file=plugin.toml,line=1::{disagreement}")
        held.append({"location": "1", "message": disagreement})
    print(
        f"{len(held)} refusal(s) of the manifest, held to capability vocabulary generation "
        f"{read['vocabulary_version']} and extension points generation {read['extension_points_version']}."
    )
    return 1 if held else 0


def declarations(verdict: dict) -> list[dict]:
    """What a verdict failing as declared says failed, where, on what, and why.

    Refused rather than written down without it: a report naming a failure as
    declared and not the declaration is one nobody can check against the manifest.
    """
    found = verdict.get("declared")
    if not isinstance(found, list) or not found:
        raise Unasked(f"the release said {AS_DECLARED} and named no declaration")
    for one in found:
        lacking = [field for field in DECLARED if not isinstance(one, dict) or one.get(field) in (None, "")]
        if lacking:
            raise Unasked(f"the release said {AS_DECLARED} and its declaration names no {', '.join(lacking)}")
    return found


def as_declared(declared: dict) -> str:
    """One declaration an assertion failed as, as one line."""
    place = f" at {declared['place']}" if declared.get("place") else ""
    return (
        f"fails as declared on {declared['fixture']}: {declared['constraint']}{place} "
        f"held {declared['held']}. {declared['reason']}"
    )


def said(verdict: dict) -> str:
    """A verdict's reasons, as one line."""
    if verdict["outcome"] == AS_DECLARED:
        return "; ".join(as_declared(one) for one in declarations(verdict))
    reasons = [value for key, value in verdict.items() if key != "outcome"]
    flat = [str(each) for value in reasons for each in (value if isinstance(value, list) else [value])]
    return "; ".join(flat)


def written(one: dict) -> dict:
    """One assertion as the report keeps it; one failing as declared keeps its declarations."""
    verdict = one["verdict"]
    entry = {"id": one["id"], "kind": one["kind"], "outcome": verdict["outcome"], "detail": said(verdict)}
    if verdict["outcome"] == AS_DECLARED:
        entry["declared"] = declarations(verdict)
    return entry


def verdicts(read: dict) -> list[dict]:
    """Every assertion and what the release said of it, in the order the report keeps."""
    found = [{"id": one["id"], "kind": "proof", "verdict": one["verdict"]} for one in read["proofs"]]
    for capability in read["capabilities"]:
        found.extend(
            {"id": f"{capability['name']}/{probe['probe']}", "kind": "probe", "verdict": probe["verdict"]}
            for probe in capability["probes"]
        )
    found.extend({"id": one["id"], "kind": "check", "verdict": one["verdict"]} for one in read["checks"])
    return found


def proofs() -> int:
    """Every probe, proof and check on its recording, written down as `proofs.json`.

    The report is the record the release train reads (`OPS-R67`): which release
    or commit decided it, and what each assertion came to. Failing as declared is
    named and written down apart, and fails nothing (`F10-R13`). Anything else
    that did not pass fails, and a report naming nothing fails too (`F3-R5`).
    """
    read = claimed()
    found = verdicts(read)
    report = {
        "lemonfiber": targeted().name,
        "against": read["against"],
        "proofs": [written(one) for one in found],
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for entry in report["proofs"]:
        mark = {"passed": "  ok  ", AS_DECLARED: "  decl"}.get(entry["outcome"], "  " + entry["outcome"].upper()[:4])
        print(f"{mark} {entry['kind']:6} {entry['id']:<36} {entry['detail']}")
    for entry in report["proofs"]:
        for declared in entry.get("declared", []):
            print(f"::notice::{entry['kind']} {entry['id']} {as_declared(declared)}")
    passed = [entry for entry in report["proofs"] if entry["outcome"] == "passed"]
    excused = [entry for entry in report["proofs"] if entry["outcome"] == AS_DECLARED]
    failing = len(found) - len(passed) - len(excused)
    print(
        f"\n{len(passed)} passed, {len(excused)} failing as declared, {failing} not, "
        f"against the {read['against']}."
    )
    return 1 if failing or not found else 0


def resolves(reference: str) -> str | None:
    """The digest a reference names in its registry now, or None where the registry holds no such reference.

    A registry that fails without saying it holds none is asked again, waiting longer
    each time, and raises `Unanswered` once every attempt has failed.
    """
    wait = REGISTRY_FIRST_WAIT_S
    for attempt in range(1, REGISTRY_ATTEMPTS + 1):
        asked = subprocess.run(
            ["docker", "buildx", "imagetools", "inspect", reference, "--format", "{{.Manifest.Digest}}"],
            capture_output=True, text=True, check=False,
        )
        if asked.returncode == 0:
            return asked.stdout.strip()
        said = (asked.stderr.strip() or asked.stdout.strip()).splitlines()
        if ABSENT.search(" ".join(said)):
            return None
        if attempt < REGISTRY_ATTEMPTS:
            time.sleep(wait)
            wait *= REGISTRY_BACKOFF
    last = said[-1] if said else f"exit {asked.returncode}"
    raise Unanswered(f"{reference} could not be checked: its registry did not answer in {REGISTRY_ATTEMPTS} attempts ({last})")


def pinned_image(service: dict) -> tuple[bool, str]:
    """Whether one service's pinned digest resolves and its tag still names it, and what to say."""
    pinned = f"{service['image']}@{service['digest']}"
    try:
        held = resolves(pinned) == service["digest"]
        tagged = resolves(f"{service['image']}:{service['tag']}") if held else None
    except Unanswered as why:
        return False, str(why)
    if not held:
        return False, f"{pinned} is not in its registry"
    if tagged and tagged != service["digest"]:
        return False, f"{service['image']}:{service['tag']} names {tagged}, not the pinned digest"
    return True, f"{pinned}, which {service['tag']} names" if tagged else pinned


def image() -> int:
    """Every pinned digest resolves and its tag still names it (`F3-R8`), then who vouches (`F3-R25`)."""
    services = tomllib.loads(MANIFEST.read_text(encoding="utf-8")).get("service", [])
    faults = 0
    for service in services:
        held, said = pinned_image(service)
        print(f"  ok   {said}" if held else f"::error::{said}")
        faults += not held
    vouched = plugin("provenance")
    print(vouched.stdout + vouched.stderr)
    return 1 if faults or vouched.returncode != 0 else 0


ASKED = {"reader": reader, "manifest": manifest, "proofs": proofs, "image": image}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in ASKED:
        print(f"usage: reader.py {{{','.join(ASKED)}}}", file=sys.stderr)
        return 2
    try:
        return ASKED[sys.argv[1]]()
    except (Unasked, OSError) as why:
        print(f"::error::{why}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
