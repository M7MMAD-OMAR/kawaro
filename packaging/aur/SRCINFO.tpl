pkgbase = kawaro-bin
	pkgdesc = Local file converter with a honeycomb picker in your file manager (release package)
	pkgver = @VERSION@
	pkgrel = 1
	url = https://github.com/@REPO@
	arch = x86_64
	license = GPL-3.0-or-later
	depends = glibc
	depends = gcc-libs
	depends = fontconfig
	depends = freetype2
	optdepends = ffmpeg: audio, video and image actions
	optdepends = ghostscript: PDF actions
	optdepends = libreoffice-fresh: document actions
	optdepends = wl-clipboard: copied files on Wayland
	optdepends = curl: update checks
	optdepends = python-nautilus: Nautilus right click menu
	provides = kawaro=@VERSION@
	conflicts = kawaro
	noextract = kawaro_@VERSION@_amd64.deb
	options = !strip
	options = !debug
	source = kawaro_@VERSION@_amd64.deb::https://github.com/@REPO@/releases/download/v@VERSION@/kawaro_@VERSION@_amd64.deb
	sha256sums = @SHA@

pkgname = kawaro-bin
