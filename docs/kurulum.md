# Stok Takip — Kurulum

## Kullanıcılar için

Kurulum dosyaları GitHub'daki sürüm sayfasında bulunur.

| Sistem | Dosya | Not |
|---|---|---|
| Windows 10/11 | `Stok Takip_x.y.z_x64-setup.exe` | Çift tıklayın. İmzasız sürümde SmartScreen "Yine de çalıştır" sorabilir. |
| macOS 11+ | `Stok Takip_x.y.z_universal.dmg` | Uygulamayı Applications'a sürükleyin. İmzasız sürümde ilk açılışta sağ tık → Aç. |
| Ubuntu 22.04+ | `.deb` veya `.AppImage` | `sudo apt install ./stok-takip_x.y.z_amd64.deb` ya da AppImage'ı çalıştırılabilir yapıp açın. |

İlk açılışta yönetici hesabı oluşturulur. Veriler bilgisayarda saklanır:

- Windows: `%APPDATA%\com.guney.stoktakip\`
- macOS: `~/Library/Application Support/com.guney.stoktakip/`
- Linux: `~/.local/share/com.guney.stoktakip/`

Bu klasörde `stok.db` (veritabanı), `yedekler/` (günlük otomatik yedekler, son 14 gün) ve `images/` (ürün görselleri) bulunur.

## Geliştiriciler için

Gerekenler: Rust (stable, ≥ 1.85), Node.js 22+, platform araçları
(Windows: Visual Studio C++ Build Tools + WebView2; macOS: Xcode Command Line Tools;
Linux: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`).

```sh
cd ui && npm ci && cd ..
ui/node_modules/.bin/tauri dev      # geliştirme
cargo test --workspace              # Rust testleri
npm --prefix ui test                # arayüz testleri
ui/node_modules/.bin/tauri build    # bu platform için kurulum paketi
```

Sürüm yayınlamak: `git tag v0.1.0 && git push origin v0.1.0`. GitHub Actions üç platformun
paketlerini taslak sürüme yükler. macOS imzası için depo gizlerine `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
`APPLE_TEAM_ID` eklenir; yoksa paket imzasız üretilir.

Windows'ta "Akıllı Uygulama Denetimi" (Smart App Control) açıksa derleme sırasında yeni
üretilen test dosyaları ara sıra engellenebilir (`os error 4551`); komutu yeniden çalıştırmak yeterlidir.
