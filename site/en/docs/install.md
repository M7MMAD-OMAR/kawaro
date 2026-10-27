# Install

## Windows 10 and 11 (64 bit)

Download the MSI. It installs for your user and needs no administrator rights. A portable ZIP is also available. The build is unsigned, so [SmartScreen warns you](https://kawaro.sbarah.com/en/windows/).

Windows shows the app under the fixed brand name Kawaro | كوارو: the Latin part comes first in every language and is never reordered for your language. In the file name of the Start menu shortcut the bar is its fullwidth form ｜, because Windows file names cannot contain |. The installed apps list (Add or remove programs) still shows Kawaro only, the name inside the MSI, because the tool that builds the MSI cannot write Arabic.

## Linux (64 bit)

Download the DEB for Ubuntu and Debian, or the RPM for Fedora. Nautilus needs the `nautilus-python` package.

## Verify

Check every download against `SHA256SUMS`. The steps are on the [download page](https://kawaro.sbarah.com/en/download/#verify).

## Extra tools

Video and audio need FFmpeg. Documents and PDF need LibreOffice and Ghostscript. An action whose tool is missing does not appear in the hive. Run `kawaro doctor` to see what was found.

## From source (Linux)

```
git clone https://github.com/M7MMAD-OMAR/kawaro
cd kawaro
./scripts/install.sh
```

Needs Rust 1.85 or newer. To remove it: `./scripts/uninstall.sh`.
