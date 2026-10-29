# Stok Takip

**[⬇ Windows için indir (setup.exe)](https://github.com/YSGN1/Inventory-Tracking/releases/latest)** —
sayfadaki `Stok.Takip_x.y.z_x64-setup.exe` dosyasını indirip çift tıklayın.

Küçük işletmeler için masaüstü stok takip programı. Windows, macOS ve Linux'ta çalışır; veriler
internete gönderilmez, bilgisayarınızda saklanır.

- Ürün ve varyantlar (beden, renk vb.), ürün görselleri, liste ve ızgara görünümü
- Stok girişi, çıkışı ve sayım düzeltmesi; her hareketin geçmişi
- Firmalara yapılan çıkışlar ve baskı durumu, firma bazlı rapor, CSV (Excel) dışa aktarma
- Kritik stok uyarısı
- Çöp kutusu: silinen kayıt geri getirilebilir; süre dolunca kalıcı silinir
- Günlük otomatik yedek, elle yedek alma ve geri yükleme
- Tek kişi kullanıyorsa giriş sormaz; istenirse yönetici/operatör rolleriyle çok kullanıcılı çalışır
- Yeni sürüm çıkınca uygulama içinden tek tıkla güncellenir

## Kurulum

[Sürümler](../../releases/latest) sayfasından işletim sisteminize uygun dosyayı indirin:

| Sistem | Dosya |
|---|---|
| Windows 10/11 | `Stok.Takip_x.y.z_x64-setup.exe` |
| macOS 11+ | `Stok.Takip_x.y.z_universal.dmg` |
| Ubuntu / Debian | `.deb` veya `.AppImage` |

Windows kurulumu yönetici izni istemez. Dosya imzalı olmadığı için SmartScreen uyarı verebilir:
**Ek bilgi → Yine de çalıştır**.

## Güncelleme

Uygulama açılışta yeni sürümü denetler. Yeni sürüm varsa sol menüde **"Yeni sürüm hazır"** yazar.
**Güncelle**'ye basınca verileriniz önce yedeklenir, sonra yeni sürüm kurulur ve uygulama yeniden
açılır. **Ayarlar → Güncellemeleri denetle** ile elle de denetlenebilir.

## Geliştirme

Gerekenler: [Rust](https://rustup.rs) (stable), Node.js 22+ ve
[Tauri ön koşulları](https://tauri.app/start/prerequisites/).

```sh
cd ui && npm ci && cd ..
ui/node_modules/.bin/tauri dev     # geliştirme modunda çalıştır
cargo test --workspace             # Rust testleri
npm --prefix ui test               # arayüz testleri
```

Proje yapısı:

| Klasör | İçerik |
|---|---|
| `crates/stok-core` | İş kuralları (veritabanından bağımsız) |
| `crates/stok-db` | SQLite veritabanı, şema ve sorgular |
| `src-tauri` | Masaüstü kabuğu ve uygulama komutları |
| `ui` | Arayüz (SvelteKit + Tailwind) |
| `docs` | Tasarım, plan, kurulum ve yayınlama notları |

Yayınlama adımları için [docs/kurulum.md](docs/kurulum.md) dosyasına bakın.

## Lisans

[MIT](LICENSE)
