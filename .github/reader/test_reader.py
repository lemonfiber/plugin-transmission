#!/usr/bin/env python3
"""What the reader writes down and fails on, given what lemonfiber said, and which
lemonfiber it asks.

lemonfiber is not asked here. Every verdict is lemonfiber's own, so what these
hold the reader to is the part that is not: that each verdict is written to
`proofs.json` as it was given, that failing as declared is kept apart from passed
and failed with the declaration it failed as, that the run fails on everything
but passed and failing as declared, that `targets.toml` names one release or one
commit and nothing else, and that a commit is held to `main`, or to a pull
request's head where the run takes one, built the way the train's gate builds its
candidate, and kept, and that an image's registry is called empty only where it
says so, and asked again where it did not answer.

The core a commit is built from is a git repository made here, and cargo is a
stand-in that writes a binary where cargo would and says where its target
directory is.

Stdlib unittest, no network. Run:  python3 -m unittest discover -s .github/reader
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import reader

CLAIMED = {
    "fixture": "fixtures/identity-anonymous.json",
    "constraint": "json",
    "place": "/MediaContainer/claimed",
    "held": "false",
    "reason": "Recorded from a server nobody has claimed; claiming one needs a plex.tv account.",
}


def answer(*checks: dict, proofs: tuple[dict, ...] = ()) -> dict:
    """What `lemonfiber plugin claims --json` says, cut to what the reader reads."""
    return {
        "against": "recordings",
        "proofs": list(proofs),
        "capabilities": [],
        "checks": list(checks),
        "refusals": [],
    }


def check(name: str, verdict: dict) -> dict:
    return {"id": name, "verdict": verdict}


def declared(**over) -> dict:
    return {"outcome": reader.AS_DECLARED, "declared": [{**CLAIMED, **over}]}


RELEASE = reader.Target("0.17.0", commit=False)
COMMIT = reader.Target("c" * 40, commit=True)


class Claimed(unittest.TestCase):
    def read(self, printed: dict) -> dict:
        """What the reader takes from `plugin claims --json` printing this."""
        answer = subprocess.CompletedProcess([], 0, stdout=json.dumps(printed), stderr="")
        with mock.patch.object(reader, "plugin", return_value=answer):
            return reader.claimed()

    def test_an_envelope_is_read_for_the_claims_it_carries(self):
        claims = {"refusals": [], "proofs": []}
        printed = {"api_version": 1, "kind": reader.CLAIMS_KIND, "data": claims}
        self.assertEqual(self.read(printed), claims)

    def test_claims_printed_bare_are_read_as_they_are(self):
        claims = {"refusals": [], "proofs": []}
        self.assertEqual(self.read(claims), claims)


class Proofs(unittest.TestCase):
    def setUp(self) -> None:
        box = tempfile.TemporaryDirectory()
        self.addCleanup(box.cleanup)
        self.report = pathlib.Path(box.name) / "proofs.json"
        self.target = RELEASE

    def proved(self, said: dict) -> tuple[int, str]:
        """`reader.py proofs` against an answer, as CI would print it."""
        out = io.StringIO()
        with (
            mock.patch.object(reader, "claimed", return_value=said),
            mock.patch.object(reader, "targeted", return_value=self.target),
            mock.patch.object(reader, "REPORT", self.report),
            mock.patch.object(sys, "argv", ["reader.py", "proofs"]),
            contextlib.redirect_stdout(out),
        ):
            code = reader.main()
        return code, out.getvalue()

    def written(self) -> list[dict]:
        return json.loads(self.report.read_text(encoding="utf-8"))["proofs"]

    def test_failing_as_declared_is_written_apart_and_fails_nothing(self):
        code, said = self.proved(answer(
            check("plex:claimed", declared()),
            check("plex:not-published", {"outcome": "passed"}),
        ))
        self.assertEqual(code, 0, said)
        claimed, published = self.written()
        self.assertEqual(claimed["outcome"], "failing-as-declared")
        self.assertEqual(claimed["declared"], [CLAIMED])
        self.assertEqual(
            claimed["detail"],
            "fails as declared on fixtures/identity-anonymous.json: json at /MediaContainer/claimed "
            "held false. Recorded from a server nobody has claimed; claiming one needs a plex.tv account.",
        )
        self.assertNotIn("declared", published)
        self.assertIn("  decl check  plex:claimed", said)
        self.assertIn("::notice::check plex:claimed fails as declared on fixtures/identity-anonymous.json", said)
        self.assertIn("1 passed, 1 failing as declared, 0 not, against the recordings.", said)

    def test_failing_as_declared_is_never_counted_as_passed(self):
        code, said = self.proved(answer(check("plex:claimed", declared())))
        self.assertEqual(code, 0, said)
        self.assertIn("0 passed, 1 failing as declared, 0 not", said)

    def test_a_declaration_about_the_answer_as_a_whole_names_no_place(self):
        whole = {"outcome": reader.AS_DECLARED, "declared": [{**CLAIMED, "constraint": "status", "held": "500"}]}
        del whole["declared"][0]["place"]
        code, said = self.proved(answer(check("plex:claimed", whole)))
        self.assertEqual(code, 0, said)
        self.assertIn("identity-anonymous.json: status held 500. Recorded", self.written()[0]["detail"])

    def test_every_declaration_an_assertion_failed_as_is_named(self):
        other = {**CLAIMED, "fixture": "fixtures/identity-other.json", "held": "null"}
        code, said = self.proved(answer(
            check("plex:claimed", {"outcome": reader.AS_DECLARED, "declared": [CLAIMED, other]}),
        ))
        self.assertEqual(code, 0, said)
        self.assertEqual(said.count("::notice::"), 2)
        self.assertEqual(self.written()[0]["declared"], [CLAIMED, other])

    def test_a_failure_beside_one_failing_as_declared_fails_the_run(self):
        code, said = self.proved(answer(
            check("plex:claimed", declared()),
            check("plex:no-anonymous-lan", {"outcome": "failed", "faults": ["status is 200, and it declares 401"]}),
        ))
        self.assertEqual(code, 1)
        self.assertIn("  FAIL check  plex:no-anonymous-lan", said)
        self.assertIn("0 passed, 1 failing as declared, 1 not", said)
        self.assertIn("::notice::check plex:claimed", said)

    def test_a_stale_declaration_is_a_failure_the_release_names(self):
        stale = {"outcome": "failed", "faults": ["passes on fixtures/identity-anonymous.json, which it declares it fails on"]}
        code, said = self.proved(answer(check("plex:claimed", stale)))
        self.assertEqual(code, 1)
        self.assertIn("  FAIL check  plex:claimed", said)
        self.assertEqual(self.written()[0]["outcome"], "failed")
        self.assertNotIn("declared", self.written()[0])

    def test_an_unproven_assertion_is_not_excused_by_a_declaration(self):
        unrun = {"outcome": "unproven", "why": "fixtures/identity-anonymous.json is not in the plugin"}
        code, said = self.proved(answer(check("plex:claimed", unrun)))
        self.assertEqual(code, 1)
        self.assertIn("  UNPR check  plex:claimed", said)
        self.assertIn("0 passed, 0 failing as declared, 1 not", said)

    def test_failing_as_declared_naming_no_declaration_writes_nothing(self):
        code, said = self.proved(answer(check("plex:claimed", {"outcome": reader.AS_DECLARED})))
        self.assertEqual(code, 1)
        self.assertIn("::error::the release said failing-as-declared and named no declaration", said)
        self.assertFalse(self.report.exists())

    def test_a_declaration_without_its_reason_writes_nothing(self):
        code, said = self.proved(answer(check("plex:claimed", declared(reason=""))))
        self.assertEqual(code, 1)
        self.assertIn("its declaration names no reason", said)
        self.assertFalse(self.report.exists())

    def test_a_declaration_that_is_not_a_table_names_everything_it_lacks(self):
        bare = {"outcome": reader.AS_DECLARED, "declared": ["fixtures/identity-anonymous.json"]}
        code, said = self.proved(answer(check("plex:claimed", bare)))
        self.assertEqual(code, 1)
        self.assertIn("names no fixture, constraint, held, reason", said)

    def test_a_report_naming_nothing_fails(self):
        code, said = self.proved(answer())
        self.assertEqual(code, 1)
        self.assertIn("0 passed, 0 failing as declared, 0 not", said)

    def test_the_report_names_the_release_that_answered(self):
        code, said = self.proved(answer(check("c", {"outcome": "passed"})))
        self.assertEqual(code, 0, said)
        self.assertEqual(json.loads(self.report.read_text(encoding="utf-8"))["lemonfiber"], "0.17.0")

    def test_the_report_names_the_commit_that_answered(self):
        self.target = COMMIT
        code, said = self.proved(answer(check("c", {"outcome": "passed"})))
        self.assertEqual(code, 0, said)
        self.assertEqual(json.loads(self.report.read_text(encoding="utf-8"))["lemonfiber"], COMMIT.name)

    def test_every_kind_is_written_in_the_order_the_report_keeps(self):
        said = answer(check("c", {"outcome": "passed"}), proofs=({"id": "p", "verdict": {"outcome": "passed"}},))
        said["capabilities"] = [{"name": "media.serve", "probes": [{"probe": "guarded", "verdict": {"outcome": "passed"}}]}]
        code, out = self.proved(said)
        self.assertEqual(code, 0, out)
        self.assertEqual(
            [(one["kind"], one["id"]) for one in self.written()],
            [("proof", "p"), ("probe", "media.serve/guarded"), ("check", "c")],
        )


class Schema(unittest.TestCase):
    """`reader.py manifest` holds the manifest's schema line to `targets.toml` (`F10-R17`)."""

    WANTED = "https://github.com/lemonfiber/lemonfiber/releases/download/v0.17.0/plugin-manifest.schema.json"
    COMMITTED = f"https://raw.githubusercontent.com/lemonfiber/lemonfiber/{COMMIT.name}/contract/plugin-manifest.schema.json"

    def setUp(self) -> None:
        box = tempfile.TemporaryDirectory()
        self.addCleanup(box.cleanup)
        self.manifest = pathlib.Path(box.name) / "plugin.toml"
        self.target = RELEASE

    def read(self, first_line: str | None) -> tuple[int, str]:
        """`reader.py manifest` on a manifest opening with `first_line`, the release refusing nothing."""
        body = "schema_version = 1\n"
        self.manifest.write_text(body if first_line is None else f"{first_line}\n{body}", encoding="utf-8")
        said = answer()
        said["vocabulary_version"], said["extension_points_version"] = 1, 1
        out = io.StringIO()
        with (
            mock.patch.object(reader, "claimed", return_value=said),
            mock.patch.object(reader, "targeted", return_value=self.target),
            mock.patch.object(reader, "MANIFEST", self.manifest),
            mock.patch.object(sys, "argv", ["reader.py", "manifest"]),
            contextlib.redirect_stdout(out),
        ):
            code = reader.main()
        return code, out.getvalue()

    def test_the_schema_of_the_targeted_release_passes(self):
        code, out = self.read(f"#:schema {self.WANTED}")
        self.assertEqual(code, 0, out)
        self.assertNotIn("::error", out)

    def test_the_schema_of_another_release_is_refused_naming_both(self):
        other = self.WANTED.replace("v0.17.0", "v0.16.0")
        code, out = self.read(f"#:schema {other}")
        self.assertEqual(code, 1, out)
        self.assertIn("::error file=plugin.toml,line=1::", out)
        self.assertIn(other, out)
        self.assertIn(self.WANTED, out)
        self.assertIn("1 refusal(s)", out)

    def test_a_schema_on_main_is_refused(self):
        code, out = self.read("#:schema https://raw.githubusercontent.com/lemonfiber/lemonfiber/main/contract/plugin-manifest.schema.json")
        self.assertEqual(code, 1, out)

    def test_a_schema_line_below_the_first_is_not_one_an_editor_reads(self):
        code, out = self.read(f"# a comment\n#:schema {self.WANTED}")
        self.assertEqual(code, 0, out)
        self.assertIn("::notice file=plugin.toml::", out)

    def test_naming_no_schema_says_which_line_would_and_refuses_nothing(self):
        code, out = self.read(None)
        self.assertEqual(code, 0, out)
        self.assertIn(f"::notice file=plugin.toml::an editor checks this manifest as it is typed once its first line is `#:schema {self.WANTED}`", out)
        self.assertIn("0 refusal(s)", out)

    def test_the_schema_committed_at_the_targeted_commit_passes(self):
        self.target = COMMIT
        code, out = self.read(f"#:schema {self.COMMITTED}")
        self.assertEqual(code, 0, out)
        self.assertNotIn("::error", out)

    def test_a_release_schema_beside_a_targeted_commit_is_refused_naming_both(self):
        self.target = COMMIT
        code, out = self.read(f"#:schema {self.WANTED}")
        self.assertEqual(code, 1, out)
        self.assertIn(f"targets.toml names lemonfiber at {COMMIT.name}, whose schema is {self.COMMITTED}", out)

    def test_the_schema_at_another_commit_is_refused(self):
        self.target = COMMIT
        code, out = self.read(f"#:schema {self.COMMITTED.replace(COMMIT.name, 'd' * 40)}")
        self.assertEqual(code, 1, out)


class Targets(unittest.TestCase):
    """`targets.toml` names one release or one commit of `main`, in full."""

    def setUp(self) -> None:
        box = tempfile.TemporaryDirectory()
        self.addCleanup(box.cleanup)
        self.targets = pathlib.Path(box.name) / "targets.toml"
        patched = mock.patch.object(reader, "TARGETS", self.targets)
        patched.start()
        self.addCleanup(patched.stop)

    def named(self, body: str) -> reader.Target:
        self.targets.write_text(body, encoding="utf-8")
        return reader.targeted()

    def refused(self, body: str) -> str:
        with self.assertRaises(reader.Unasked) as said:
            self.named(body)
        return str(said.exception)

    def test_a_release(self):
        self.assertEqual(self.named('lemonfiber = "0.17.0"\n'), RELEASE)

    def test_a_commit(self):
        self.assertEqual(self.named(f'commit = "{COMMIT.name}"\n'), COMMIT)

    def test_neither_is_refused(self):
        self.assertEqual(self.refused(""), "targets.toml names neither a lemonfiber release nor a commit")

    def test_both_are_refused_naming_both(self):
        said = self.refused(f'lemonfiber = "0.17.0"\ncommit = "{COMMIT.name}"\n')
        self.assertEqual(said, f"targets.toml names the release '0.17.0' and the commit '{COMMIT.name}'; a plugin targets one")

    def test_a_short_commit_is_refused(self):
        self.assertIn("'cccccccc', which is not a full commit", self.refused('commit = "cccccccc"\n'))

    def test_a_branch_is_not_a_commit(self):
        self.assertIn("'main', which is not a full commit", self.refused('commit = "main"\n'))

    def test_a_commit_is_described_as_main_at_it(self):
        self.assertEqual(str(COMMIT), f"lemonfiber at {COMMIT.name}")
        self.assertEqual(str(RELEASE), "0.17.0")


#: cargo, as far as the reader asks it: `build` writes a binary where cargo would,
#: and `metadata` says where the target directory is. Every call is logged.
CARGO = """#!/bin/sh
echo "$*" >> "$CARGO_LOG"
case "$1" in
  build)
    shift
    while [ $# -gt 0 ]; do
      case "$1" in --target) target="$2"; shift ;; --profile) profile="$2"; shift ;; esac
      shift
    done
    [ -n "$CARGO_FAILS" ] && exit 101
    mkdir -p "$CARGO_TARGET/$target/$profile"
    git rev-parse HEAD > "$CARGO_TARGET/$target/$profile/lemonfiber"
    ;;
  metadata)
    printf '{"target_directory": "%s"}' "$CARGO_TARGET"
    ;;
esac
"""


def git(*args: str, at: pathlib.Path) -> str:
    return subprocess.run(["git", *args], cwd=at, check=True, capture_output=True, text=True).stdout.strip()


class Built(unittest.TestCase):
    """A commit is held to `main`, or to a pull request's head where the run takes one,
    built the way the train's gate builds its candidate, and kept under the commit."""

    BUILD = "x86_64-unknown-linux-gnu"

    def setUp(self) -> None:
        box = tempfile.TemporaryDirectory()
        self.addCleanup(box.cleanup)
        root = pathlib.Path(box.name)
        self.core = root / "core"
        self.core.mkdir()
        git("init", "--quiet", "--initial-branch", "main", at=self.core)
        for name, value in (("user.email", "core@example.invalid"), ("user.name", "core"), ("commit.gpgsign", "false")):
            git("config", name, value, at=self.core)
        git("commit", "--quiet", "--allow-empty", "-m", "on main", at=self.core)
        self.on_main = git("rev-parse", "HEAD", at=self.core)
        git("checkout", "--quiet", "-b", "elsewhere", at=self.core)
        git("commit", "--quiet", "--allow-empty", "-m", "not on main", at=self.core)
        self.elsewhere = git("rev-parse", "HEAD", at=self.core)
        git("commit", "--quiet", "--allow-empty", "-m", "a pull request's head", at=self.core)
        self.pulled = git("rev-parse", "HEAD", at=self.core)
        git("update-ref", "refs/pull/7/head", self.pulled, at=self.core)
        git("checkout", "--quiet", "main", at=self.core)

        tools = root / "bin"
        tools.mkdir()
        cargo = tools / "cargo"
        cargo.write_text(CARGO, encoding="utf-8")
        cargo.chmod(0o755)
        self.log = root / "cargo.log"
        self.cache = root / ".lemonfiber"
        self.targets = root / "targets.toml"
        env = {
            "PATH": f"{tools}{os.pathsep}{os.environ['PATH']}",
            "CARGO_LOG": str(self.log),
            "CARGO_TARGET": str(root / "shared-target"),
        }
        for patched in (
            mock.patch.dict(os.environ, env),
            mock.patch.object(reader, "CORE", str(self.core)),
            mock.patch.object(reader, "CACHE", self.cache),
            mock.patch.object(reader, "TARGETS", self.targets),
            mock.patch.object(reader, "build", return_value=self.BUILD),
        ):
            patched.start()
            self.addCleanup(patched.stop)
        os.environ.pop(reader.PULL_HEADS_ENV, None)

    def target(self, commit: str) -> None:
        self.targets.write_text(f'commit = "{commit}"\n', encoding="utf-8")

    def cargo(self) -> list[str]:
        return self.log.read_text(encoding="utf-8").splitlines() if self.log.exists() else []

    def taking(self, value: str) -> contextlib.AbstractContextManager:
        """The run with the pull-head variable set to `value`."""
        return mock.patch.dict(os.environ, {reader.PULL_HEADS_ENV: value})

    def test_a_commit_on_main_is_built_as_the_gate_builds_its_candidate_and_kept_under_it(self):
        self.target(self.on_main)
        with contextlib.redirect_stderr(io.StringIO()):
            held = reader.binary()
        self.assertEqual(held, self.cache / self.on_main / self.BUILD / "lemonfiber")
        self.assertEqual(held.read_text(encoding="utf-8").strip(), self.on_main)
        self.assertTrue(os.access(held, os.X_OK))
        self.assertIn(f"build --locked --profile dist --target {self.BUILD} -p lemonfiber", self.cargo())
        self.assertEqual([path.relative_to(self.cache) for path in self.cache.rglob("*") if path.is_file()],
                         [pathlib.Path(self.on_main, self.BUILD, "lemonfiber")])

    def test_a_kept_build_is_asked_without_building_again(self):
        self.target(self.on_main)
        with contextlib.redirect_stderr(io.StringIO()):
            reader.binary()
            builds = len(self.cargo())
            reader.binary()
        self.assertEqual(len(self.cargo()), builds)

    def test_a_commit_main_does_not_hold_is_refused_before_anything_is_built(self):
        self.target(self.elsewhere)
        with self.assertRaises(reader.Unasked) as said:
            reader.binary()
        self.assertEqual(str(said.exception), f"targets.toml names the commit {self.elsewhere}, which lemonfiber's main does not hold")
        self.assertEqual(self.cargo(), [])
        self.assertFalse(self.cache.exists())

    def test_a_pull_requests_head_is_refused_unless_the_run_takes_one(self):
        self.target(self.pulled)
        with self.assertRaises(reader.Unasked) as said:
            reader.binary()
        self.assertEqual(str(said.exception), f"targets.toml names the commit {self.pulled}, which lemonfiber's main does not hold")
        self.assertEqual(self.cargo(), [])

    def test_a_run_taking_pull_requests_heads_builds_a_commit_one_holds(self):
        for commit in (self.pulled, self.elsewhere):
            with self.subTest(commit=commit):
                self.target(commit)
                with self.taking("true"), contextlib.redirect_stderr(io.StringIO()):
                    held = reader.binary()
                self.assertEqual(held.read_text(encoding="utf-8").strip(), commit)

    def test_a_run_taking_pull_requests_heads_refuses_a_commit_none_holds(self):
        git("checkout", "--quiet", "-b", "unpulled", "main", at=self.core)
        git("commit", "--quiet", "--allow-empty", "-m", "on no pull request", at=self.core)
        unpulled = git("rev-parse", "HEAD", at=self.core)
        git("checkout", "--quiet", "main", at=self.core)
        self.target(unpulled)
        with self.taking("true"), self.assertRaises(reader.Unasked) as said:
            reader.binary()
        self.assertEqual(
            str(said.exception),
            f"targets.toml names the commit {unpulled}, which neither lemonfiber's main nor the head of any of its pull requests holds",
        )
        self.assertEqual(self.cargo(), [])

    def test_only_true_takes_pull_requests_heads(self):
        self.target(self.pulled)
        for value in ("1", "yes", "TRUE", ""):
            with self.subTest(value=value), self.taking(value), self.assertRaises(reader.Unasked):
                reader.binary()
        self.assertEqual(self.cargo(), [])

    def test_a_commit_the_core_never_had_is_refused_before_anything_is_built(self):
        nowhere = "d" * 40
        self.target(nowhere)
        with self.assertRaises(reader.Unasked) as said:
            reader.binary()
        self.assertIn(f"the commit {nowhere}, which lemonfiber's main does not hold", str(said.exception))
        self.assertEqual(self.cargo(), [])

    def test_a_build_that_fails_keeps_nothing(self):
        self.target(self.on_main)
        with (
            mock.patch.dict(os.environ, {"CARGO_FAILS": "1"}),
            contextlib.redirect_stderr(io.StringIO()),
            self.assertRaises(reader.Unasked) as said,
        ):
            reader.binary()
        self.assertEqual(str(said.exception), f"lemonfiber at {self.on_main} did not build for {self.BUILD}; cargo exited 101")
        self.assertFalse(self.cache.exists())

    def test_a_core_that_cannot_be_cloned_says_so(self):
        self.target(self.on_main)
        with mock.patch.object(reader, "CORE", str(self.core.parent / "nowhere")), self.assertRaises(reader.Unasked) as said:
            reader.binary()
        self.assertIn("git clone failed building lemonfiber:", str(said.exception))


class Reader(unittest.TestCase):
    """`reader.py reader` holds a release to the version it says, and shows what a commit's build says."""

    def asked(self, target: reader.Target, says: str) -> tuple[int, str]:
        said = answer()
        out = io.StringIO()
        version = subprocess.CompletedProcess([], 0, stdout=f"{says}\n", stderr="")
        claims = subprocess.CompletedProcess([], 0, stdout="claims\n", stderr="")
        with (
            mock.patch.object(reader, "targeted", return_value=target),
            mock.patch.object(reader, "binary", return_value=pathlib.Path("lemonfiber")),
            mock.patch.object(reader.subprocess, "run", return_value=version),
            mock.patch.object(reader, "claimed", return_value=said),
            mock.patch.object(reader, "plugin", return_value=claims),
            mock.patch.object(sys, "argv", ["reader.py", "reader"]),
            contextlib.redirect_stdout(out),
        ):
            code = reader.main()
        return code, out.getvalue()

    def test_a_release_saying_its_own_version_reads_the_plugin(self):
        code, out = self.asked(RELEASE, "lemonfiber 0.17.0")
        self.assertEqual(code, 0, out)

    def test_a_release_saying_another_version_is_refused(self):
        code, out = self.asked(RELEASE, "lemonfiber 0.16.0")
        self.assertEqual(code, 1, out)
        self.assertIn("::error::targets.toml names 0.17.0, and the binary says 'lemonfiber 0.16.0'", out)

    def test_a_commits_build_is_shown_rather_than_compared(self):
        code, out = self.asked(COMMIT, "lemonfiber 0.18.0")
        self.assertEqual(code, 0, out)
        self.assertIn(f"::notice::targets.toml names lemonfiber at {COMMIT.name}, built here; it says 'lemonfiber 0.18.0'", out)


DIGEST = "sha256:" + "4e" * 32
MANIFESTED = f"""
[[service]]
image = "lscr.io/linuxserver/prowlarr"
tag = "2.6.5"
digest = "{DIGEST}"
"""
ABSENT = subprocess.CompletedProcess([], 1, stdout="", stderr=f"ERROR: lscr.io/linuxserver/prowlarr@{DIGEST}: not found\n")
LIMITED = subprocess.CompletedProcess([], 1, stdout="", stderr="ERROR: unexpected status: 429 Too Many Requests: toomanyrequests\n")
HELD = subprocess.CompletedProcess([], 0, stdout=f"{DIGEST}\n", stderr="")


class Image(unittest.TestCase):
    """`reader.py image` reports a digest the registry holds no such thing at as absent, and
    asks a registry that did not answer again, waiting longer each time, before failing
    the image as unchecked rather than absent."""

    def asked(self, *answers: subprocess.CompletedProcess) -> tuple[int, str, list, list]:
        box = tempfile.TemporaryDirectory()
        self.addCleanup(box.cleanup)
        manifest = pathlib.Path(box.name) / "plugin.toml"
        manifest.write_text(MANIFESTED, encoding="utf-8")
        vouched = subprocess.CompletedProcess([], 0, stdout="vouched\n", stderr="")
        out = io.StringIO()
        with (
            mock.patch.object(reader, "MANIFEST", manifest),
            mock.patch.object(reader.subprocess, "run", side_effect=list(answers)) as run,
            mock.patch.object(reader.time, "sleep") as sleep,
            mock.patch.object(reader, "plugin", return_value=vouched),
            mock.patch.object(sys, "argv", ["reader.py", "image"]),
            contextlib.redirect_stdout(out),
        ):
            code = reader.main()
        return code, out.getvalue(), [call.args[0][4] for call in run.call_args_list], [call.args[0] for call in sleep.call_args_list]

    def test_a_digest_its_tag_still_names_passes(self):
        code, out, asked, waits = self.asked(HELD, HELD)
        self.assertEqual(code, 0, out)
        self.assertIn(f"ok   lscr.io/linuxserver/prowlarr@{DIGEST}, which 2.6.5 names", out)
        self.assertEqual(asked, [f"lscr.io/linuxserver/prowlarr@{DIGEST}", "lscr.io/linuxserver/prowlarr:2.6.5"])
        self.assertEqual(waits, [])

    def test_a_digest_the_registry_holds_none_of_is_absent_at_once(self):
        code, out, asked, waits = self.asked(ABSENT)
        self.assertEqual(code, 1, out)
        self.assertIn(f"::error::lscr.io/linuxserver/prowlarr@{DIGEST} is not in its registry", out)
        self.assertEqual((len(asked), waits), (1, []))

    def test_a_rate_limit_that_lifts_is_asked_again_and_passes(self):
        code, out, asked, waits = self.asked(LIMITED, HELD, HELD)
        self.assertEqual(code, 0, out)
        self.assertNotIn("::error::", out)
        self.assertEqual((len(asked), waits), (3, [reader.REGISTRY_FIRST_WAIT_S]))

    def test_a_registry_that_never_answers_could_not_be_checked_and_is_not_called_absent(self):
        code, out, asked, waits = self.asked(*[LIMITED] * reader.REGISTRY_ATTEMPTS)
        self.assertEqual(code, 1, out)
        self.assertIn(
            f"::error::lscr.io/linuxserver/prowlarr@{DIGEST} could not be checked: its registry did not answer "
            f"in {reader.REGISTRY_ATTEMPTS} attempts (ERROR: unexpected status: 429 Too Many Requests: toomanyrequests)",
            out,
        )
        self.assertNotIn("is not in its registry", out)
        self.assertEqual(len(asked), reader.REGISTRY_ATTEMPTS)
        self.assertEqual(
            waits,
            [reader.REGISTRY_FIRST_WAIT_S * reader.REGISTRY_BACKOFF**at for at in range(reader.REGISTRY_ATTEMPTS - 1)],
        )

    def test_a_tag_the_registry_never_answers_for_could_not_be_checked(self):
        code, out, _, _ = self.asked(HELD, *[LIMITED] * reader.REGISTRY_ATTEMPTS)
        self.assertEqual(code, 1, out)
        self.assertIn("::error::lscr.io/linuxserver/prowlarr:2.6.5 could not be checked", out)
        self.assertNotIn("ok   ", out)

    def test_a_failure_that_says_nothing_names_its_exit(self):
        silent = subprocess.CompletedProcess([], 1, stdout="", stderr="")
        code, out, _, _ = self.asked(*[silent] * reader.REGISTRY_ATTEMPTS)
        self.assertEqual(code, 1, out)
        self.assertIn(f"in {reader.REGISTRY_ATTEMPTS} attempts (exit 1)", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)
