# Maintainer: Your Name <your.email@example.com>
pkgname=rataplay
_pkgname=Rataplay
pkgver=r8.038166d
pkgrel=1
pkgdesc="A high-performance Rust TUI orchestrating yt-dlp and mpv (Renamed to vivid-tui)"
arch=('x86_64' 'aarch64')
url="https://github.com/mojahid8238/Rataplay"
license=('GPL3')
depends=('glibc' 'gcc-libs' 'openssl' 'mpv' 'yt-dlp')
makedepends=('cargo' 'git')
provides=('rataplay')
conflicts=('rataplay')
# We remove 'vivid' from conflicts because we are renaming the binary, 
# so you can keep the original vivid installed.
source=("git+$url.git")
sha256sums=('SKIP')

pkgver() {
  cd "$_pkgname"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

prepare() {
  cd "$_pkgname"
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd "$_pkgname"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --all-features
}

check() {
  cd "$_pkgname"
  # Optional: run tests
  # cargo test --frozen --all-features
}

package() {
  cd "$_pkgname"
  
  install -Dm755 "target/release/rataplay"   
  # Install docs
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
