# Stok Takip Programı — Tasarım Spesifikasyonu

**Tarih:** 2026-10-28
**Durum:** Taslak (kullanıcı onayı bekliyor)

## 1. Amaç

Profesyonel, basit kullanımlı, masaüstünde çalışan bir stok takip programı. İlk sürüm yerel (offline) çalışır; mimari, ileride WordPress (Vitrin / WooCommerce) ve herhangi bir web sitesiyle iki yönlü senkronizasyona, sunucu veritabanına ve çok kiracılı (herkesin kendi verisi) kullanıma geçişe hazır kurulur.

## 2. Kullanıcı gereksinimleri (kullanıcının ifadesiyle)

1. Masaüstü uygulaması.
2. WordPress'e entegre olacak; Vitrin veya WooCommerce ile kolay uyumlu; ayrıca istenen bir web sitesine kolayca bağlanabilecek.
3. Senkronizasyon iki yönlü.
4. Tek kullanıcı veya çoklu kullanıcı çalışabilmeli.
5. İlk kapsam: varyant, kritik stok uyarısı, raporlar (hangi firmaya kaç adet verildiği, baskı yapılıp yapılmadığı).
6. Ekranda yalnızca girilen stok, düşülen stok ve güncellenen stok (eklenen/çıkarılan) görünsün; başka şey kalabalık yapmasın.
7. Teknoloji: en etkili, güvenli, stabil ve hızlı seçenek; uygunsa Rust öncelikli. Arayüz estetik, hoş, kullanıcı dostu.
8. Her şey eklenebilir, düzenlenebilir, silinebilir; geçmiş her zaman görülebilir.
9. İleride: ürün/malzemeler görsel ve liste olarak görüntülenebilecek; nesne tabanlı yapı; sunucu veritabanı; herkesin kendi verileri. Her adım sonrakilerle uyumlu olmalı.
10. Windows, macOS ve Linux'ta çalışmalı.
11. Silme: onay için "SİL" yazılması gerekir; silinen ürün/varyant/firma **Çöp Kutusu**'na gider ve geri getirilebilir. Çöp kutusundakiler ayarlanan süre sonunda (varsayılan 30 gün) kalıcı silinir veya hemen kalıcı silinebilir. Stoğu olan ürün de silinebilir; geri getirilince stoğuyla döner.
12. Kalıcı silmede geçmiş varsayılan olarak **korunur**. Silme sırasında "Geçmişiyle birlikte sil" işaretlenirse ürünün hareketleri ve değişiklik geçmişi de silinir.
13. Silinmiş (çöpteki veya kalıcı silinmiş) ürünlere ait geçmiş kayıtları **yalnız Yönetici** görür, "Silinmiş ürün: <ad>" etiketiyle. Operatör bu kayıtları görmez.

## 3. Teknoloji kararı

| Katman | Seçim | Gerekçe |
|---|---|---|
| Uygulama kabuğu | **Tauri 2** (Rust) | Küçük kurulum boyutu, düşük RAM, izin (capability) tabanlı güvenlik modeli; Windows, macOS, Linux (her biri kendi işletim sisteminde derlenir; GitHub Actions ile otomatik). macOS'ta uyarısız kurulum için Apple geliştirici imzası gerekir. |
| İş mantığı | **Rust** (`stok-core`) | Bellek güvenliği, hız, derleme zamanında tip kontrolü |
| Veritabanı (aşama 1) | **SQLite** (sqlx, WAL modu) | Kurulumsuz, tek dosya, hızlı, yedeklenebilir |
| Veritabanı (aşama 3) | **PostgreSQL** (sqlx) | Aynı kütüphane; şema taşınabilir yazılır |
| Arayüz | **Svelte 5 + TypeScript + Vite + Tailwind CSS 4** | Hafif, hızlı, estetik bileşen geliştirmeye uygun |
| Parola | **argon2id** | Güncel öneri |
| Kimlikler | **UUID v7** | Zamana göre sıralanabilir; senkronizasyon ve sunucuya taşımada çakışmasız |

## 4. Veri modeli ilkeleri (geleceğe uyum)

- Her tabloda: `id` (UUID v7), `tenant_id`, `created_at`, `updated_at`, `row_version`, `deleted_at` (yumuşak silme).
- Stok bir **hareket defterinden** (ledger) hesaplanır. Hareket düzeltme/geri alma = ters kayıt (+ gerekirse yeni kayıt). Hareketler ve değişiklik geçmişi yalnız "Geçmişiyle birlikte sil" kalıcı silmesinde fiziksel silinir; bunun dışında veritabanı tetikleyicileri silmeyi engeller.
- Hareketler ürün/varyant/firma **adlarının anlık kopyasını** taşır → ürün kalıcı silinse de geçmiş okunabilir kalır.
- Silme iki aşamalıdır: `deleted_at` (çöp kutusu) → kalıcı silme (`purge`).
- Varyantın anlık stoku (`stock_qty`) aynı transaction içinde güncellenen bir önbellektir; defterle tutarlılığı test edilir.
- Hareketlerde `source` (`local` / `site`) ve benzersiz `idempotency_key` alanı vardır → ileride siteden gelen siparişler iki kez düşülmez.
- Ürün/varyant/firma değişiklikleri `audit_log` tablosuna önce/sonra JSON olarak yazılır.
- Zamanlar UTC, milisaniye tamsayı; miktarlar tamsayı (adet).

## 5. Kapsam ve aşamalar

**Aşama 1 — Çekirdek masaüstü (bu planın konusu):**
ürün + varyant, firma, stok giriş/çıkış/düzeltme, firmaya çıkışta baskı durumu, kritik stok uyarısı, firma bazlı rapor + CSV dışa aktarma, hareket geçmişi, değişiklik geçmişi, çöp kutusu + kalıcı silme, kullanıcılar ve roller (tek kullanıcı modu dahil), ürün görseli + liste/ızgara görünümü, yedekleme/geri yükleme, Windows/macOS/Linux kurulum paketleri.

**Aşama 2 — Site entegrasyonu (ayrı plan):**
`SiteConnector` arayüzü; WooCommerce REST API bağlayıcısı; Vitrin için küçük bir WordPress köprü eklentisi; genel JSON senkronizasyon protokolü (her siteye bağlanabilmek için). Vitrin şu an yalnız katalog; **sitede stok gösterilmez**. Program → site: ürün/varyant bilgisi. Site → program (ileride): siparişler; siparişe göre yeni stok girişi / yeni ürün siparişi yapılır. Masaüstü uygulaması dışarıdan istek kabul etmez; siteden **çeker** (cursor ile), siteye **iter**.

**Aşama 3 — Sunucu ve çok kiracılık (ayrı plan):**
PostgreSQL deposu, hesaplar, çoklu cihaz, bulut yedek.

## 6. Kapsam dışı (aşama 1)

Fiyat/maliyet, fatura/e-fatura, birden fazla depo, barkod. Şema bunlara engel olmayacak şekilde kurulur.

## 7. Kararlar (kullanıcı onaylı)

- K1: Windows, macOS, Linux.
- K2: "Baskı yapılıp yapılmadığı", firmaya yapılan **her çıkış hareketinde evet/hayır** bilgisidir (gerekirse not alanı).
- K3: Roller: **Yönetici** (her şey + kullanıcı yönetimi) ve **Operatör** (stok hareketi + görüntüleme; silme yok).
- K4: Silme onay kelimesi: **SİL**. Çöp kutusu varsayılan süresi 30 gün (ayarlanabilir).
- K5: Arayüz dili Türkçe; metinler tek dosyada (ileride çok dil).
