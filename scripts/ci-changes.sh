#!/bin/sh
# Usage: ci-changes.sh < changed-paths
# Reads one repo-relative path per line and prints "job=true" or "job=false" for each CI job, true
# when a changed path can affect what the job checks. A path no rule names runs every job.
set -eu

rust=false emu=false audit=false docs=false app=false libraries=false

all() { rust=true emu=true audit=true docs=true app=true libraries=true; }

while IFS= read -r path; do
	case $path in
	'') ;;
	README.md | LICENSE | .gitignore) ;;
	docs/theme/* | docs/book.toml) docs=true ;;
	docs/*) docs=true libraries=true ;;
	crates/mcpack/* | crates/mcglyph/* | crates/mcedit/* | crates/mcui/* | crates/mcemu/*)
		rust=true emu=true libraries=true
		;;
	crates/*) rust=true audit=true ;;
	kernel/* | lib/* | ui/* | app/* | probe/*) emu=true app=true libraries=true ;;
	libraries/*) rust=true libraries=true ;;
	release-please-config.json | .release-please-manifest.json) libraries=true ;;
	*) all ;;
	esac
done

for job in rust emu audit docs app libraries; do
	eval "echo $job=\$$job"
done
