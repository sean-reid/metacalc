#!/bin/sh
# Usage: test-install.sh
# Builds the real mcsend, packs it with package-mcsend.sh for this machine, writes a real
# SHA256SUMS beside the fixture app in a directory laid out as a release download, and runs
# install.sh against it through a file:// URL, under sh and under dash when it is installed. Only
# the interrupt case shims curl and only the platform cases shim uname. With a calculator on USB
# the run that reaches mcsend is skipped, so nothing is ever sent to one.
set -u

root=$(cd "$(dirname "$0")/.." && pwd)
app=$root/crates/mcsend/tests/fixtures/metacalc.8xk
status=0

pass() { echo "ok $1"; }
fail() { echo "FAIL $1"; status=1; }

case $(uname -s) in Darwin) os=macos ;; Linux) os=linux ;; *) echo "test-install: unsupported OS" >&2; exit 1 ;; esac
case $(uname -m) in arm64 | aarch64) arch=arm64 ;; x86_64 | amd64) arch=x86_64 ;; *) echo "test-install: unsupported architecture" >&2; exit 1 ;; esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
scratch=$tmp/scratch
mkdir "$scratch"

digest() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# release TAG: a release download directory with the package, the app, and no SHA256SUMS yet.
release() {
	mkdir -p "$tmp/releases/$1"
	cp "$tmp/dist/mcsend-v1.0.0-$os-$arch.tar.gz" "$tmp/releases/$1/mcsend-$1-$os-$arch.tar.gz"
	cp "$app" "$tmp/releases/$1/metacalc.8xk"
}

# sums TAG: SHA256SUMS over what the release directory holds now.
sums() {
	(
		cd "$tmp/releases/$1" || exit 1
		for file in *; do echo "$(digest "$file")  $file"; done >"$tmp/sums"
		mv "$tmp/sums" SHA256SUMS
	)
}

cargo build --release --locked --manifest-path "$root/Cargo.toml" -p mcsend >&2 || { echo "test-install: cargo build failed" >&2; exit 1; }
sh "$root/scripts/package-mcsend.sh" v1.0.0 "$os" "$arch" "$tmp/dist" >/dev/null || { echo "test-install: packaging failed" >&2; exit 1; }
listing=$(tar -tzf "$tmp/dist/mcsend-v1.0.0-$os-$arch.tar.gz")
for entry in mcsend LICENSE COPYING AUTHORS NOTICE THIRD-PARTY; do
	if echo "$listing" | grep -qx "mcsend-v1.0.0-$os-$arch/$entry"; then pass "package holds $entry"; else fail "package is missing $entry"; fi
done
tar -xzf "$tmp/dist/mcsend-v1.0.0-$os-$arch.tar.gz" -C "$tmp"
pkg=$tmp/mcsend-v1.0.0-$os-$arch
if cmp -s "$pkg/LICENSE" "$root/LICENSE"; then pass "package LICENSE is the repo's"; else fail "package LICENSE differs from the repo's"; fi
if grep -q 'GNU LESSER GENERAL PUBLIC LICENSE' "$pkg/COPYING"; then pass "COPYING is the LGPL"; else fail "COPYING is not the LGPL"; fi
if grep -q 'libusb' "$pkg/AUTHORS" || grep -q 'Johannes Erdfelt' "$pkg/AUTHORS"; then pass "AUTHORS is libusb's"; else fail "AUTHORS is not libusb's"; fi
if grep -q 'statically links libusb' "$pkg/NOTICE" && grep -q 'Lesser General Public' "$pkg/NOTICE" && grep -q 'v1.0.0' "$pkg/NOTICE"; then
	pass "NOTICE names the licence and the tag"
else
	fail "NOTICE is incomplete"
fi

for crate in clap rusb libusb1-sys libc; do
	section=$(awk -v c="$crate" '/^=+$/ { on = 0 } $1 == c && /, licence: / { on = 1 } on' "$pkg/THIRD-PARTY" | wc -c | tr -d ' ')
	if [ "$section" -gt 500 ]; then pass "THIRD-PARTY has licence text for $crate"; else fail "THIRD-PARTY has $section bytes for $crate"; fi
done
if grep -q 'THIRD-PARTY' "$pkg/NOTICE"; then pass "NOTICE mentions THIRD-PARTY"; else fail "NOTICE does not mention THIRD-PARTY"; fi

release v1.0.0
sums v1.0.0

# v1.0.1: the package changed after the sums were written.
release v1.0.1
sums v1.0.1
printf 'x' >>"$tmp/releases/v1.0.1/mcsend-v1.0.1-$os-$arch.tar.gz"

# v1.0.2: the package is not attached.
release v1.0.2
sums v1.0.2
rm "$tmp/releases/v1.0.2/mcsend-v1.0.2-$os-$arch.tar.gz"

# v1.0.3: the app changed after the sums were written.
release v1.0.3
sums v1.0.3
printf 'x' >>"$tmp/releases/v1.0.3/metacalc.8xk"

# v1.0.4: SHA256SUMS holds only entries that look like the files.
release v1.0.4
printf '%s  metacalc.8xk.bak\n%s  mcsend-v1.0.4-%s-%s.tar.gz.sig\n' "$(digest "$app")" "$(digest "$app")" "$os" "$arch" >"$tmp/releases/v1.0.4/SHA256SUMS"

base=file://$tmp/releases

# calculator_attached: a USB device of vendor 0451 is present, found without opening anything.
calculator_attached() {
	if [ -d /sys/bus/usb/devices ]; then
		cat /sys/bus/usb/devices/*/idVendor 2>/dev/null | grep -qx 0451
	elif command -v ioreg >/dev/null 2>&1; then
		ioreg -p IOUSB -l -w0 2>/dev/null | grep -q '"idVendor" = 1105'
	else
		return 1
	fi
}

# What mcsend says with no calculator: on Linux without a USB bus (nothing under /dev/bus/usb) libusb cannot start.
expected="no TI-84 Plus found on USB"
if [ "$os" = linux ] && [ -z "$(ls -A /dev/bus/usb 2>/dev/null)" ]; then expected="this computer has no USB bus"; fi

# install SHELL VERSION [NAME=value...]: runs install.sh with output in $tmp/out and its exit status in $code.
install() {
	shell=$1 version=$2
	shift 2
	env TMPDIR="$scratch" METACALC_BASE_URL="$base" METACALC_VERSION="$version" "$@" "$shell" "$root/scripts/install.sh" >"$tmp/out" 2>&1 </dev/null
	code=$?
}

clean() {
	if [ -z "$(ls -A "$scratch")" ]; then pass "$1: nothing left in the temp directory"; else fail "$1: the temp directory kept $(ls "$scratch")"; fi
}

# tools DIR NAME...: DIR holds links to the named tools and to the ones install.sh needs besides curl and a checksum tool.
tools() {
	dir=$1
	mkdir -p "$dir"
	shift
	for tool in uname mktemp tar gzip rm mkdir grep sed awk cut tr basename dirname cat ls env sleep "$@"; do
		path=$(command -v "$tool") && ln -sf "$path" "$dir/$tool"
	done
}

shells="sh"
command -v dash >/dev/null 2>&1 && shells="sh dash"
nocalc="" want_exit=0
if ! calculator_attached; then
	nocalc=$("$pkg/mcsend" "$app" 2>&1 </dev/null)
	want_exit=$?
fi

for sh_name in $shells; do
	shell=$(command -v "$sh_name")
	name="[$sh_name]"

	if calculator_attached; then
		echo "note $name: a TI calculator is on USB, so the run that reaches mcsend is skipped and nothing is sent"
	else
		install "$shell" v1.0.0
		if [ "$want_exit" -ne 0 ] && [ "$code" = "$want_exit" ] && grep -qxF "$nocalc" "$tmp/out" && grep -q "$expected" "$tmp/out"; then
			pass "$name mcsend's message and exit status $code come through"
		else
			fail "$name expected exit $want_exit and the line '$nocalc', got exit $code: $(cat "$tmp/out")"
		fi
		if grep -q "installing metacalc v1.0.0 for $os-$arch" "$tmp/out" && grep -q 'checksums match' "$tmp/out"; then pass "$name prints what it does"; else fail "$name progress lines are missing"; fi
		clean "$name sent"
	fi

	if ! calculator_attached; then
		cat "$root/scripts/install.sh" | env TMPDIR="$scratch" METACALC_BASE_URL="$base" METACALC_VERSION=v1.0.0 "$shell" >"$tmp/out" 2>&1
		code=$?
		if [ "$want_exit" -ne 0 ] && [ "$code" = "$want_exit" ] && grep -qxF "$nocalc" "$tmp/out"; then
			pass "$name the script piped to the shell on stdin runs to the end"
		else
			fail "$name piped script: exit $code: $(cat "$tmp/out")"
		fi
		clean "$name piped script"
	fi

	install "$shell" v1.0.1
	if [ "$code" -ne 0 ] && grep -q 'does not match its checksum' "$tmp/out" && grep -q "mcsend-v1.0.1-$os-$arch.tar.gz" "$tmp/out" && ! grep -q 'no TI-84 Plus' "$tmp/out"; then
		pass "$name a corrupted package is refused"
	else
		fail "$name corrupted package: exit $code: $(cat "$tmp/out")"
	fi
	clean "$name corrupted package"

	install "$shell" v1.0.3
	if [ "$code" -ne 0 ] && grep -q 'metacalc.8xk does not match its checksum' "$tmp/out" && ! grep -q 'no TI-84 Plus' "$tmp/out"; then
		pass "$name a corrupted app is refused"
	else
		fail "$name corrupted app: exit $code: $(cat "$tmp/out")"
	fi
	clean "$name corrupted app"

	install "$shell" v1.0.4
	if [ "$code" -ne 0 ] && grep -q 'SHA256SUMS has no entry for mcsend-v1.0.4' "$tmp/out"; then
		pass "$name look-alike entries do not count"
	else
		fail "$name look-alike entries: exit $code: $(cat "$tmp/out")"
	fi
	clean "$name look-alike entries"

	install "$shell" v1.0.2
	if [ "$code" -ne 0 ] && grep -q "could not download .*mcsend-v1.0.2-$os-$arch.tar.gz" "$tmp/out"; then
		pass "$name a missing package is a plain message"
	else
		fail "$name missing package: exit $code: $(cat "$tmp/out")"
	fi
	clean "$name missing package"

	install "$shell" v9.9.9
	if [ "$code" -ne 0 ] && grep -q 'could not download .*SHA256SUMS' "$tmp/out"; then
		pass "$name a missing release is a plain message"
	else
		fail "$name missing release: exit $code: $(cat "$tmp/out")"
	fi
	clean "$name missing release"

	install "$shell" "" METACALC_VERSION=
	if [ "$code" -ne 0 ] && grep -q 'set METACALC_VERSION' "$tmp/out"; then
		pass "$name a base URL without a version is refused"
	else
		fail "$name base URL without a version: exit $code: $(cat "$tmp/out")"
	fi

	for platform in "Haiku x86_64" "Linux aarch64" "Darwin ppc"; do
		kernel=${platform% *} machine=${platform#* }
		mkdir -p "$tmp/shim-uname"
		printf "#!/bin/sh\ncase \"\$1\" in -s) echo %s ;; -m) echo %s ;; *) echo %s ;; esac\n" "$kernel" "$machine" "$kernel" >"$tmp/shim-uname/uname"
		chmod +x "$tmp/shim-uname/uname"
		install "$shell" v1.0.0 PATH="$tmp/shim-uname:$PATH"
		if [ "$code" -ne 0 ] && grep -q "there is no mcsend build for $kernel on $machine" "$tmp/out" && [ "$(wc -l <"$tmp/out" | tr -d ' ')" = 1 ]; then
			pass "$name $kernel on $machine is one plain sentence"
		else
			fail "$name $kernel on $machine: exit $code: $(cat "$tmp/out")"
		fi
	done
	clean "$name unsupported platform"

	rm -rf "$tmp/bin-nocurl" "$tmp/bin-nosum"
	tools "$tmp/bin-nocurl" sha256sum shasum
	install "$shell" v1.0.0 PATH="$tmp/bin-nocurl"
	if [ "$code" -ne 0 ] && grep -q 'curl is required' "$tmp/out"; then pass "$name missing curl is a plain message"; else fail "$name missing curl: exit $code: $(cat "$tmp/out")"; fi
	tools "$tmp/bin-nosum" curl
	install "$shell" v1.0.0 PATH="$tmp/bin-nosum"
	if [ "$code" -ne 0 ] && grep -q 'sha256sum or shasum is required' "$tmp/out"; then pass "$name missing checksum tool is a plain message"; else fail "$name missing checksum tool: exit $code: $(cat "$tmp/out")"; fi
	clean "$name missing tools"

	mkdir -p "$tmp/bin-latest"
	cat >"$tmp/bin-latest/curl" <<SHIM
#!/bin/sh
echo "\$*" >>"$tmp/curl.log"
case "\$*" in
*url_effective*) echo "\$MC_SHIM_URL"; exit 0 ;;
*) exit 22 ;;
esac
SHIM
	chmod +x "$tmp/bin-latest/curl"
	for url in https://github.com/sean-reid/metacalc/releases https://github.com/sean-reid/metacalc/releases/tag/v1.0.0; do
		rm -f "$tmp/curl.log"
		env -u METACALC_BASE_URL -u METACALC_VERSION TMPDIR="$scratch" MC_SHIM_URL="$url" PATH="$tmp/bin-latest:$PATH" "$shell" "$root/scripts/install.sh" >"$tmp/out" 2>&1 </dev/null
		code=$?
		case $url in
		*/tag/*)
			if [ "$code" -ne 0 ] && grep -q 'installing metacalc v1.0.0' "$tmp/out" && grep -q 'release v1.0.0 has no files attached yet; try again in a few minutes' "$tmp/out"; then
				pass "$name the latest tag is read from the redirect and an unfinished release is said plainly"
			else
				fail "$name latest tag: exit $code: $(cat "$tmp/out")"
			fi
			;;
		*)
			if [ "$code" -ne 0 ] && grep -q 'no release has been published yet' "$tmp/out"; then pass "$name no release is a plain message"; else fail "$name no release: exit $code: $(cat "$tmp/out")"; fi
			;;
		esac
		if [ "$(grep -c -e '--proto =https --tlsv1.2' "$tmp/curl.log")" = "$(wc -l <"$tmp/curl.log" | tr -d ' ')" ]; then
			pass "$name every curl call is limited to https"
		else
			fail "$name a curl call is not limited to https: $(cat "$tmp/curl.log")"
		fi
	done
	clean "$name latest tag"

	rm -rf "$tmp/bin-slow" "$tmp/started"
	mkdir "$tmp/bin-slow"
	mkfifo "$tmp/started"
	printf '#!/bin/sh\necho started >"%s"\nsleep 2\n' "$tmp/started" >"$tmp/bin-slow/curl"
	chmod +x "$tmp/bin-slow/curl"
	env TMPDIR="$scratch" METACALC_BASE_URL="$base" METACALC_VERSION=v1.0.0 PATH="$tmp/bin-slow:$PATH" "$shell" "$root/scripts/install.sh" >"$tmp/out" 2>&1 </dev/null &
	pid=$!
	read -r _ <"$tmp/started"
	kill -TERM "$pid"
	wait "$pid"
	code=$?
	if [ "$code" = 143 ]; then pass "$name terminated mid-download exits 143"; else fail "$name terminated mid-download exited $code"; fi
	clean "$name terminated"
done

exit $status
