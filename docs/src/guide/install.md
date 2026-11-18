# Installing

Plug the calculator into your computer, then run this in a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/sean-reid/metacalc/main/scripts/install.sh | sh
```

No release has been published yet, so the command works from the first release on.

The script finds the latest release and downloads the app (`metacalc.8xk`), a small sender for your computer called `mcsend`, and a `SHA256SUMS` file. It checks both downloads against the sums, then runs `mcsend`, which sends the app over USB and prints its progress. It writes only to a temporary directory and removes it when it finishes. It never uses `sudo`. It runs on macOS (Apple silicon and Intel) and on Linux (x86_64).

## What you need

- A TI-84 Plus or TI-84 Plus Silver Edition, turned on.
- A USB cable from the calculator's own USB port to your computer.
- TI Connect CE quit. While it is open it holds the USB device, and `mcsend` says the calculator is busy.
- About three minutes. The app is large and the calculator takes a while to accept it.

On Linux, `mcsend` needs permission to open the USB device. If it says access was denied, add a udev rule for USB vendor `0451` or run the command from a root shell.

## If the app is already installed

The calculator refuses an app whose name is already on it, and `mcsend` says so. Delete the old copy first. Press `2nd` `+` for the memory menu, choose Mem Mgmt/Del, then Apps, select metacalc, and press `Del`. Then run the command again.

## Pinning a version

Set `METACALC_VERSION` to a release tag to install that release instead of the latest:

```sh
curl -fsSL https://raw.githubusercontent.com/sean-reid/metacalc/main/scripts/install.sh | METACALC_VERSION=v0.1.0 sh
```

## Installing by hand

Open the [latest release](https://github.com/sean-reid/metacalc/releases/latest) and download three files: `metacalc.8xk`, `SHA256SUMS`, and the `mcsend` archive for your system, named `mcsend-<tag>-<os>-<arch>.tar.gz`. Check them against the sums:

```sh
shasum -a 256 -c SHA256SUMS --ignore-missing
```

On Linux, use `sha256sum -c SHA256SUMS --ignore-missing`. Then unpack the archive and send the app:

```sh
tar -xzf mcsend-*.tar.gz
./mcsend-*/mcsend metacalc.8xk
```

`mcsend --help` lists its options. The archive also holds the licence files. `mcsend` links libusb statically under the LGPL-2.1, and the `NOTICE` file says where to get the source.
