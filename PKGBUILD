# Maintainer: lstnbl<jialanxin1996@hotmail.com>
pkgname=fanyi
pkgver=0.1.0
pkgrel=1
arch=("x86_64")
package(){
    cd ..
    install -Dm 755 "target/release/${pkgname}" -t "${pkgdir}/usr/bin"
    ln -s  "$pkgdir/usr/bin/fanyi" "$pkgdir/usr/bin/fy"
}