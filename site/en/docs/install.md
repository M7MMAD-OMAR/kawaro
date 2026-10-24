# Install

## Windows 10 and 11 (64 bit)

Download the MSI. It installs for your user and needs no administrator rights. A portable ZIP is also available. The build is unsigned, so [SmartScreen warns you](https://hive.sbarah.com/en/windows/).

## Linux (64 bit)

Download the DEB for Ubuntu and Debian, or the RPM for Fedora. Nautilus needs the `nautilus-python` package.

## Verify

Check every download against `SHA256SUMS`. The steps are on the [download page](https://hive.sbarah.com/en/download/#verify).

## Extra tools

Video and audio need FFmpeg. Documents and PDF need LibreOffice and Ghostscript. An action whose tool is missing does not appear in the hive. Run `sbarhive doctor` to see what was found.

## From source (Linux)

```
git clone https://github.com/M7MMAD-OMAR/sbarhive
cd sbarhive
./scripts/install.sh
```

Needs Rust 1.85 or newer. To remove it: `./scripts/uninstall.sh`.
