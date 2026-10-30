{
    "version": "@VERSION@",
    "description": "Local file converter with a honeycomb picker, Arabic and English. Files never leave your machine.",
    "homepage": "https://github.com/@REPO@",
    "license": "GPL-3.0-or-later",
    "architecture": {
        "64bit": {
            "url": "https://github.com/@REPO@/releases/download/v@VERSION@/kawaro-@VERSION@-windows-x64.zip",
            "hash": "@HASH@",
            "extract_dir": "kawaro-@VERSION@-windows-x64"
        }
    },
    "bin": [
        "kawaro.exe",
        "kawarod.exe"
    ],
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/@REPO@/releases/download/v$version/kawaro-$version-windows-x64.zip",
                "extract_dir": "kawaro-$version-windows-x64",
                "hash": {
                    "url": "https://github.com/@REPO@/releases/download/v$version/SHA256SUMS",
                    "regex": "$sha256\\s+$basename"
                }
            }
        }
    }
}
