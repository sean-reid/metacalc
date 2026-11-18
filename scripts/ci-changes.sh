#!/bin/sh
# Usage: ci-changes.sh < changed-paths
# Reads one repo-relative path per line and prints "job=true" or "job=false" for each CI job, true
# when a changed path can affect what the job checks. A path no rule names runs every job. The
# libraries flag covers the jobs that use the built libraries, the site flag the site build alone, and
# the installer flag the install script, the sender package and the release workflow.
set -eu

rust=false emu=false audit=false docs=false app=false libraries=false site=false installer=false

all() { rust=true emu=true audit=true docs=true app=true libraries=true site=true installer=true; }

while IFS= read -r path; do
	case $path in
	'') ;;
	README.md | .gitignore) ;;
	LICENSE) installer=true ;;
	docs/src/guide/*) docs=true libraries=true ;;
	docs/*) docs=true site=true ;;
	crates/mcsend/Cargo.toml) rust=true emu=true audit=true libraries=true installer=true ;;
	crates/*/Cargo.toml) rust=true emu=true audit=true libraries=true ;;
	crates/mcpack/* | crates/mcglyph/* | crates/mcedit/* | crates/mcui/* | crates/mcemu/*)
		rust=true emu=true libraries=true
		;;
	crates/mcsend/*) rust=true audit=true app=true installer=true ;;
	crates/*) rust=true audit=true ;;
	kernel/* | lib/* | ui/* | app/* | probe/*) emu=true app=true libraries=true ;;
	libraries/*) rust=true libraries=true ;;
	release-please-config.json | .release-please-manifest.json) site=true ;;
	scripts/install.sh | scripts/package-mcsend.sh | scripts/check-mcsend-links.sh | .github/workflows/release-please.yml) installer=true site=true ;;
	scripts/test-install.sh) installer=true ;;
	*) all ;;
	esac
done

[ $libraries = false ] || site=true
for job in rust emu audit docs app libraries site installer; do
	eval "echo $job=\$$job"
done
