#!/bin/sh
# Rebuild an emulator ROM image for a TI-84 Plus Silver Edition.
# Usage: tools/rebuild-rom.sh [workdir]   (default ~/ti84-rom)
set -eu

ROM8X_URL=https://gitlab.com/taricorp/rom8x.git
ROM8X_COMMIT=e8a44dc601e8f3bf86cb95178d4fe846be2fa79c
OS_PAGE=https://education.ti.com/en/software/details/en/B7DADA7FD4AA40CE9D7911B004B8C460/ti84plusoperatingsystem

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
say() { printf '==> %s\n' "$*"; }

case "${1:-}" in
-h | --help)
	sed -n '2,3p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

unset CDPATH GIT_DIR GIT_WORK_TREE
dir=${1:-$HOME/ti84-rom}
case $dir in
/*) ;;
*) dir=$PWD/$dir ;;
esac

probe=$dir
while [ ! -d "$probe" ]; do
	probe=$(dirname "$probe")
done
probe=$(cd -- "$probe" && pwd -P)
while :; do
	[ ! -e "$probe/.git" ] ||
		die "$dir is inside the repository at $probe; pick a directory outside any repository"
	[ "$probe" != / ] || break
	probe=$(dirname "$probe")
done

say "Checking prerequisites"
missing=
command -v git >/dev/null 2>&1 || missing="$missing
  git: install it from your package manager (xcode-select --install on macOS)"
command -v cc >/dev/null 2>&1 || missing="$missing
  a C compiler (cc): xcode-select --install on macOS, apt install build-essential on Debian or Ubuntu"
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
	missing="$missing
  sha256sum or shasum: part of coreutils or perl"
fi
[ -z "$missing" ] || die "missing prerequisites:$missing"

mkdir -p "$dir"
dir=$(cd -- "$dir" && pwd -P)
cd -- "$dir"

say "Fetching and building rom8x"
if [ ! -e rom8x-src ]; then
	git init -q rom8x-src
	git -C rom8x-src fetch -q --depth 1 "$ROM8X_URL" "$ROM8X_COMMIT"
	git -C rom8x-src checkout -q FETCH_HEAD
fi
[ "$(git -C rom8x-src rev-parse HEAD 2>/dev/null)" = "$ROM8X_COMMIT" ] ||
	die "$dir/rom8x-src is not rom8x at $ROM8X_COMMIT; remove it and rerun"
if [ -x rom8x ] && [ ! rom8x-src/rom8x.c -nt rom8x ]; then
	echo "rom8x already built, skipping"
else
	cc -Wall -Wextra -Os -o rom8x.tmp rom8x-src/rom8x.c
	mv rom8x.tmp rom8x
fi

say "Checking for the boot page dumps"
if [ ! -s D84PSE1.8xv ] || [ ! -s D84PSE2.8xv ]; then
	cat >&2 <<MSG
D84PSE1.8xv and D84PSE2.8xv are not in $dir.
They come from your own calculator:
  1. Send G84PSE1.8xp and G84PSE2.8xp to it with TI Connect CE or TiLP.
     They ship in the rom8x release package, or build them with spasm and rom8x-src/build.sh.
  2. On the calculator run Asm(G84PSE1) and then Asm(G84PSE2). Each creates an AppVar.
  3. Send the AppVars D84PSE1 and D84PSE2 back to this directory as .8xv files, then rerun this script.
MSG
	exit 1
fi

say "Checking for the OS upgrade file"
os=
n=0
for f in ./*.8xu ./*.8Xu ./*.8XU ./*.8xU; do
	[ -f "$f" ] || continue
	os=$f
	n=$((n + 1))
done
if [ "$n" -eq 0 ]; then
	die "no OS upgrade (.8xu) file in $dir. Download the TI-84 Plus OS 2.55MP from $OS_PAGE and save the .8Xu file here"
fi
[ "$n" -eq 1 ] || die "found $n OS upgrade files in $dir; leave only the one to use"

say "Building the ROM"
set -- 84PSE_v*.rom
if [ -f "$1" ]; then
	[ "$#" -eq 1 ] || die "found $# ROM images in $dir; leave only the one to keep"
	echo "$1 already exists, skipping"
else
	rm -rf rom-build
	mkdir rom-build
	(cd rom-build && ../rom8x 84PSE -1 ../D84PSE1.8xv -2 ../D84PSE2.8xv -u "../$os")
	set -- rom-build/84PSE_v*.rom
	[ "$#" -eq 1 ] && [ -s "$1" ] || die "rom8x produced no ROM"
	mv "$1" .
	rmdir rom-build
	set -- 84PSE_v*.rom
fi
rom=$1

if command -v sha256sum >/dev/null 2>&1; then
	sum=$(sha256sum "$rom")
else
	sum=$(shasum -a 256 "$rom")
fi
size=$(wc -c <"$rom" | tr -d ' ')
say "$dir/$rom"
echo "size:   $size bytes"
echo "sha256: ${sum%% *}"
