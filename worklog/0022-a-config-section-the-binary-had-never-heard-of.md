---
number: 22
title: A config section the binary had never heard of
date: 2026-09-21
area: core, spec
files: crates/cairns-core/src/config.rs, crates/cairns/src/serve.rs
---

# 22. A config section the binary had never heard of

`[[link]]` from [[21]] worked here and did nothing on someone else's machine,
with no error either way. Two causes, and only one of them is interesting.

The dull one: a Homebrew tap is a git clone, and it is as old as the last
`brew update`. `brew upgrade cairns` against a stale tap reinstalls the version
the tap knows about. Mine did exactly that while I was reproducing the
report - I installed "0.5.0" and got 0.4.1.

## The one that matters

A cairns that predates `[[link]]` reads a config containing it, ignores the
section entirely, and prints `ok`.

```
$ cairns --version
cairns 0.4.1
$ cairns check
ok
```

That is serde's default: unknown fields are skipped. It is the right default
for an entry's front matter, where [[1]] deliberately passes unknown keys
through so a project can carry its own metadata. It is the wrong default for
configuration, because configuration is instructions - a key the program does
not recognise is an instruction it is not following, and silence is the worst
possible answer.

Every config struct now refuses what it does not recognise:

```
cairns.toml: TOML parse error at line 59, column 3
unknown field `lnik`, expected one of `spec_version`, `project`, `paths`,
`site`, `index`, `check`, `docs`, `area`, `publish`, `link`
```

which catches a typo and, from here on, catches a config written for a newer
cairns than the one running. It cannot fix 0.4.1 - that binary will go on
ignoring things quietly - so the first symptom of being too old is still the
one that was reported. From v0.5.1 the failure is loud.

`spec_version` was supposed to cover this and did not, because the rule says
additive changes do not bump it. That rule is right for the entry format, where
a new optional field genuinely does no harm to an old reader. For config it is
wrong, and the fix is the one above rather than a version bump on every added
section.

## Serve prints its version now

Twice today I have been confused by a `cairns serve` still running the binary
it was started with. It says which version it is at startup.

**Still unknown:** nothing.
