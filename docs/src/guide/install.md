# Installing

Plug the calculator into your computer, then run this in a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/sean-reid/metacalc/main/scripts/install.sh | sh
```

The script finds the latest release and downloads the app (`metacalc.8xk`), a small sender for your computer called `mcsend`, and a `SHA256SUMS` file. It checks both downloads against the sums, then runs `mcsend`, which sends the app over USB and prints its progress. It writes only to a temporary directory and removes it when it finishes. It never uses `sudo`. It runs on macOS (Apple silicon and Intel) and on Linux (x86_64). `mcsend` needs nothing installed beyond the system itself: it carries its own USB library, and on Linux it may also use libudev, which every systemd distribution has.

## What you need

- A TI-84 Plus or TI-84 Plus Silver Edition, turned on.
- A USB cable from the calculator's own USB port to your computer.
- TI Connect CE quit. While it is open it holds the USB device, and `mcsend` says the calculator is busy.
- About three minutes. The app is large and the calculator takes a while to accept it.

## If access is denied on Linux

`mcsend` needs permission to open the USB device. If it says access was denied, give your user that permission with a udev rule for USB vendor `0451`, then unplug and replug the cable:

```sh
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="0451", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/70-ti-calculator.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Do not pipe the installer into `sudo sh`. If you would rather not add a rule, use the manual route below and run only `mcsend` with `sudo`.

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
grep -e ' metacalc.8xk$' -e ' mcsend-.*-macos-arm64.tar.gz$' SHA256SUMS | shasum -a 256 -c
```

Change `macos-arm64` to `macos-x86_64` or `linux-x86_64` to match the archive you downloaded. On Linux, use `sha256sum -c` in place of `shasum -a 256 -c`. Every line must say OK. Then unpack the archive and send the app:

```sh
tar -xzf mcsend-*.tar.gz
./mcsend-*/mcsend metacalc.8xk
```

On Linux, if access is denied, run the last command with `sudo`.

`mcsend --help` lists its options. The archive also holds the licence files. `mcsend` links libusb statically under the LGPL-2.1, and the `NOTICE` file says where to get the source.
