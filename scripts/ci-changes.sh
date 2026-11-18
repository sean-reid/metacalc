#!/bin/sh
# Usage: ci-changes.sh < changed-paths
# Reads one repo-relative path per line and prints "job=true" or "job=false" for each CI job, true
# when a changed path can affect what the job checks. A path no rule names runs every job. The
# libraries flag covers the jobs that use the built libraries, the site flag the site build alone.
set -eu

rust=false emu=false audit=false docs=false app=false libraries=false site=false

all() { rust=true emu=true audit=true docs=true app=true libraries=true site=true; }

while IFS= read -r path; do
	case $path in
	'') ;;
	README.md | LICENSE | .gitignore) ;;
	docs/src/guide/*) docs=true libraries=true ;;
	docs/*) docs=true site=true ;;
	crates/*/Cargo.toml) rust=true emu=true audit=true libraries=true ;;
	crates/mcpack/* | crates/mcglyph/* | crates/mcedit/* | crates/mcui/* | crates/mcemu/*)
		rust=true emu=true libraries=true
		;;
	crates/*) rust=true audit=true ;;
	kernel/* | lib/* | ui/* | app/* | probe/*) emu=true app=true libraries=true ;;
	libraries/*) rust=true libraries=true ;;
	release-please-config.json | .release-please-manifest.json) site=true ;;
	*) all ;;
	esac
done

[ $libraries = false ] || site=true
for job in rust emu audit docs app libraries site; do
	eval "echo $job=\$$job"
done
