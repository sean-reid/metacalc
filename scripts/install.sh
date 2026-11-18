#!/bin/sh
# Usage: curl -fsSL https://raw.githubusercontent.com/sean-reid/metacalc/main/scripts/install.sh | sh
# Downloads mcsend and metacalc.8xk from the latest release, checks both against SHA256SUMS, and
# sends the app to a calculator on USB. METACALC_VERSION pins a tag and METACALC_BASE_URL replaces
# the releases download URL; with a base URL the version is required. Writes only to a temporary
# directory, which it removes on exit.
set -eu

repo=sean-reid/metacalc
builds="macos-arm64 macos-x86_64 linux-x86_64"
tmp=

say() { printf '%s\n' "$*"; }
die() { printf 'metacalc install: %s\n' "$*" >&2; exit 1; }

cleanup() {
	[ -z "$tmp" ] || rm -rf "$tmp"
}

platform() {
	case $(uname -s) in
	Darwin) os=macos ;;
	Linux) os=linux ;;
	*) os= ;;
	esac
	case $(uname -m) in
	arm64 | aarch64) arch=arm64 ;;
	x86_64 | amd64) arch=x86_64 ;;
	*) arch= ;;
	esac
	case " $builds " in
	*" $os-$arch "*) ;;
	*) die "there is no mcsend build for $(uname -s) on $(uname -m); it exists for macOS (arm64, x86_64) and Linux (x86_64)" ;;
	esac
}

curl_safe() {
	if [ -n "${METACALC_BASE_URL:-}" ]; then
		curl "$@"
	else
		curl --proto =https --tlsv1.2 "$@"
	fi
}

latest_tag() {
	url=$(curl_safe -fsSIL -o /dev/null -w '%{url_effective}' --url "https://github.com/$repo/releases/latest") ||
		die "could not reach github.com to find the latest release"
	case $url in
	*/releases/tag/*) say "${url##*/}" ;;
	*) die "no release has been published yet" ;;
	esac
}

fetch() {
	say "downloading $1"
	rc=0
	curl_safe -fsSL --retry 2 -o "$tmp/$1" --url "$base/$tag/$1" || rc=$?
	[ "$rc" -eq 0 ] && return 0
	if [ "$rc" -eq 22 ] && [ "$1" = SHA256SUMS ] && [ "$pinned" = no ]; then
		die "release $tag has no files attached yet; try again in a few minutes"
	fi
	die "could not download $base/$tag/$1; check that release $tag has it"
}

digest() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | cut -d' ' -f1
	else
		shasum -a 256 "$1" | cut -d' ' -f1
	fi
}

verify() {
	want=$(awk -v f="$1" '$2 == f { print $1 }' "$tmp/SHA256SUMS")
	[ -n "$want" ] || die "SHA256SUMS has no entry for $1"
	got=$(digest "$tmp/$1")
	[ "$got" = "$want" ] || die "$1 does not match its checksum in SHA256SUMS; nothing was sent"
}

main() {
	command -v curl >/dev/null 2>&1 || die "curl is required and was not found"
	command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 ||
		die "sha256sum or shasum is required to check the download and neither was found"
	platform

	base=${METACALC_BASE_URL:-https://github.com/$repo/releases/download}
	base=${base%/}
	tag=${METACALC_VERSION:-}
	pinned=yes
	if [ -z "$tag" ]; then
		pinned=no
		[ -z "${METACALC_BASE_URL:-}" ] || die "set METACALC_VERSION along with METACALC_BASE_URL"
		tag=$(latest_tag)
	fi

	tmp=$(mktemp -d) || die "could not make a temporary directory"
	trap cleanup EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM HUP

	say "installing metacalc $tag for $os-$arch"
	package=mcsend-$tag-$os-$arch.tar.gz
	fetch SHA256SUMS
	fetch "$package"
	fetch metacalc.8xk
	verify "$package"
	verify metacalc.8xk
	say "checksums match"
	tar -xzf "$tmp/$package" -C "$tmp" || die "could not unpack $package"

	say "sending metacalc.8xk to the calculator"
	status=0
	"$tmp/mcsend-$tag-$os-$arch/mcsend" "$tmp/metacalc.8xk" </dev/null || status=$?
	return "$status"
}

main "$@"
