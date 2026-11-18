#!/bin/sh
# Usage: check-mcsend-links.sh binary
# Prints the dynamic dependencies of binary and fails on any that is not part of the base system:
# on Linux libc, libm, libgcc_s, libpthread, libdl, librt, the loader and libudev.so.1, on macOS
# libSystem, libobjc, libiconv and the system frameworks.
set -eu

[ $# -eq 1 ] || { echo "usage: check-mcsend-links.sh binary" >&2; exit 2; }
bin=$1

case $(uname -s) in
Darwin) libs=$(otool -L "$bin" | sed -n '2,$s/^[[:space:]]*\([^ ]*\) (.*$/\1/p') ;;
Linux) libs=$(ldd "$bin" | sed -n 's/^[[:space:]]*\([^ ]*\).*$/\1/p') ;;
*) echo "check-mcsend-links: unsupported OS" >&2; exit 1 ;;
esac

echo "$bin links:"
status=0
for lib in $libs; do
	echo "  $lib"
	case $lib in
	/usr/lib/libSystem* | /usr/lib/system/* | /usr/lib/libobjc* | /usr/lib/libiconv* | /System/Library/Frameworks/*) ;;
	linux-vdso.so.* | */ld-linux* | ld-linux* | libc.so.* | libm.so.* | libgcc_s.so.* | libpthread.so.* | libdl.so.* | librt.so.* | libudev.so.1) ;;
	statically | not) ;;
	*)
		echo "check-mcsend-links: $lib is not in the allow list" >&2
		status=1
		;;
	esac
done
exit $status
