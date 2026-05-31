# Maintainer: Mojahid <mi8238229@gmail.com>
pkgname=rataplay
_tag=v1.3.1 # Placeholder, will be replaced by CI
pkgver=1.3.1 # Placeholder, will be replaced by CI
pkgrel=1
pkgdesc="A high-performance Rust TUI for YouTube playback and management (Binary Release)"
arch=('x86_64')
url="https://github.com/mojahid8238/Rataplay"
license=('GPL3')
depends=('glibc' 'gcc-libs' 'openssl' 'mpv' 'yt-dlp')
provides=('rataplay')
options=('!strip' '!debug')
conflicts=('rataplay-git')

# Fetching the pre-compiled binary and metadata files
source=("rataplay::${url}/releases/download/${_tag}/rataplay"
	"LICENSE::${url}/raw/${_tag}/LICENSE"
	"rataplay.1::${url}/raw/${_tag}/man/rataplay.1")
#checksums for binary 
sha256sums=('ab775a68a71849fc15d2ec5bfa97d44533b6ad9d2bc08fdbd5e4abaedb6122a1'
            'e57f1c320b8cf8798a7d2ff83a6f9e06a33a03585f6e065fea97f1d86db84052'
            'SKIP')

package() {
  # Install the binary to /usr/bin/
  install -Dm755 "${srcdir}/rataplay" "${pkgdir}/usr/bin/rataplay"
  install -Dm644 "${srcdir}/LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
  install -Dm644 "${srcdir}/rataplay.1" "${pkgdir}/usr/share/man/man1/rataplay.1"
}
