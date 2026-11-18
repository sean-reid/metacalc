#!/bin/sh
# Usage: package-mcsend.sh tag os arch outdir
# Packs the release build of mcsend (target/release/mcsend, or $MCSEND_BIN) as
# outdir/mcsend-tag-os-arch.tar.gz with the repo's LICENSE, a NOTICE, the COPYING and AUTHORS
# files of the libusb that libusb1-sys bundles at the locked version, which the binary links
# statically under LGPL-2.1, and THIRD-PARTY, the licence expression and licence files of every
# crate the binary is built from. os is macos or linux, arch is arm64 or x86_64.
set -eu

[ $# -eq 4 ] || { echo "usage: package-mcsend.sh tag os arch outdir" >&2; exit 2; }
tag=$1 os=$2 arch=$3 outdir=$4
case $os in macos | linux) ;; *) echo "package-mcsend: os must be macos or linux, not $os" >&2; exit 2 ;; esac
case $arch in arm64 | x86_64) ;; *) echo "package-mcsend: arch must be arm64 or x86_64, not $arch" >&2; exit 2 ;; esac
case $tag in '' | */* | .*) echo "package-mcsend: bad tag '$tag'" >&2; exit 2 ;; esac

root=$(cd "$(dirname "$0")/.." && pwd)
manifest=$root/Cargo.toml

metadata=$(cargo metadata --format-version 1 --locked --manifest-path "$manifest" | tr ',' '\n')
bin=${MCSEND_BIN:-$(echo "$metadata" | sed -n 's/^"target_directory":"\(.*\)"$/\1/p' | head -n 1)/release/mcsend}
[ -x "$bin" ] || { echo "package-mcsend: $bin is not built; run cargo build --release --locked -p mcsend" >&2; exit 1; }

sys=$(echo "$metadata" | sed -n 's/^"manifest_path":"\(.*\/libusb1-sys-[0-9][^"\/]*\)\/Cargo.toml"$/\1/p' | head -n 1)
[ -n "$sys" ] || { echo "package-mcsend: libusb1-sys is not in the dependency graph" >&2; exit 1; }
version=${sys##*/libusb1-sys-}
for file in COPYING AUTHORS; do
	[ -f "$sys/libusb/$file" ] || { echo "package-mcsend: $sys/libusb/$file is missing" >&2; exit 1; }
done

name=mcsend-$tag-$os-$arch
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
host=$(rustc -vV | sed -n 's/^host: //p')
cargo metadata --format-version 1 --locked --manifest-path "$manifest" --filter-platform "$host" >"$stage/metadata.json"
sh "$root/scripts/check-mcsend-links.sh" "$bin" >&2 || exit 1
mkdir "$stage/$name"
python3 -I - "$stage/metadata.json" >"$stage/$name/THIRD-PARTY" <<'PY'
import json, os, sys

meta = json.load(open(sys.argv[1]))
packages = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
root = next(p["id"] for p in meta["packages"] if p["name"] == "mcsend")
seen, todo = set(), [root]
while todo:
    for dep in nodes[todo.pop()]["deps"]:
        if any(k["kind"] is None for k in dep["dep_kinds"]) and dep["pkg"] not in seen:
            seen.add(dep["pkg"])
            todo.append(dep["pkg"])

out = ["Licences of the crates mcsend is built from.\n"]
for pkg in sorted((packages[i] for i in seen), key=lambda p: (p["name"], p["version"])):
    directory = os.path.dirname(pkg["manifest_path"])
    files = sorted(
        f for f in os.listdir(directory)
        if f.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
        and os.path.isfile(os.path.join(directory, f))
    )
    if not files:
        sys.exit("THIRD-PARTY: %s %s has no licence file" % (pkg["name"], pkg["version"]))
    out.append("=" * 72)
    out.append("%s %s, licence: %s" % (pkg["name"], pkg["version"], pkg["license"] or "see the files below"))
    for f in files:
        text = open(os.path.join(directory, f), encoding="utf-8", errors="replace").read().strip()
        if not text:
            sys.exit("THIRD-PARTY: %s in %s %s is empty" % (f, pkg["name"], pkg["version"]))
        out.append("-" * 72)
        out.append(f)
        out.append("")
        out.append(text)
    out.append("")
print("\n".join(out))
PY
cp "$bin" "$stage/$name/mcsend"
cp "$root/LICENSE" "$sys/libusb/COPYING" "$sys/libusb/AUTHORS" "$stage/$name/"
cat >"$stage/$name/NOTICE" <<NOTICE
mcsend is MIT licensed; see LICENSE.

This binary statically links libusb, which is licensed under the GNU Lesser General Public
License, version 2.1 (COPYING, AUTHORS). libusb comes from the libusb1-sys crate, version $version.

The source for both is the commit tagged $tag of https://github.com/sean-reid/metacalc and the
libusb1-sys $version crate from https://crates.io/crates/libusb1-sys/$version. Running
"cargo build --release --locked -p mcsend" there rebuilds this binary, so you can change libusb and
relink.

THIRD-PARTY holds the licence expression and licence files of every other crate in the binary.
NOTICE
chmod 755 "$stage/$name/mcsend"
chmod 644 "$stage/$name/THIRD-PARTY" "$stage/$name/LICENSE" "$stage/$name/COPYING" "$stage/$name/AUTHORS" "$stage/$name/NOTICE"

mkdir -p "$outdir"
tar -czf "$outdir/$name.tar.gz" -C "$stage" "$name"
echo "$outdir/$name.tar.gz"
