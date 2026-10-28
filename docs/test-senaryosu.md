# Uçtan uca test senaryosu

Her platformda kurulumdan sonra sırayla uygulanır. "Beklenen" sütunu tutmazsa adım başarısızdır.

| # | Adım | Beklenen |
|---|---|---|
| 1 | Uygulamayı kurup açın | Kurulum ekranı |
| 2 | İlk yöneticiyi oluşturun | Özet ekranı, boş durum mesajı |
| 3 | Ürünler → Yeni ürün "Kupa", varyant ekle: "Büyük", "Küçük" | Ürün sayfası, iki varyant, toplam 0 |
| 4 | "Büyük" için Giriş 10 | Mevcut 10 |
| 5 | Çıkış 4, firma seçili, "Baskı yapıldı" açık | Mevcut 6 |
| 6 | Çıkış 2, baskı kapalı | Mevcut 4 |
| 7 | Sayım / Düzeltme: sayılan 3 (önizleme −1) | Mevcut 3 |
| 8 | Özet | Giren 10, Çıkan 6, Düzeltme −1, Mevcut 3 |
| 9 | Varyantta kritik eşik 3 | Özette uyarı şeridi, satır vurgulu |
| 10 | Raporlar → CSV olarak kaydet, Excel'de açın | Türkçe karakterler doğru, baskılı 4 / baskısız 2 |
| 11 | Ürünü sil: "sil" yazın, sonra "SİL" | "sil" ile buton pasif, "SİL" ile aktif; ürün listeden kalkar |
| 12 | Çöp Kutusu | Kupa görünür, stok 3, kalan gün 30 |
| 13 | Geri getir | Özette Mevcut 3 |
| 14 | Tekrar sil → Çöp Kutusu → Kalıcı sil (kutu işaretsiz) | Kayıt çöpten kalkar |
| 15 | Hareketler (Yönetici) | Satırlar soluk, "Silinmiş ürün: Kupa" |
| 16 | Operatör kullanıcısıyla giriş → Hareketler | Kupa satırları görünmez; Çöp Kutusu menüde yok |
| 17 | Yönetici: başka bir ürünü "Geçmişiyle birlikte sil" ile kalıcı silin | O ürünün hareketleri hiçbir yerde yok |
| 18 | Ayarlar → Şimdi yedekle; yeni bir hareket girin; Yedekten geri yükle | Yeniden giriş; yeni hareket yok |
| 19 | Uygulamayı kapatıp açın | Veriler duruyor |

## Ölçümler

| Platform | Kurulum dosyası boyutu | Açılış süresi | Tarih | Sonuç |
|---|---|---|---|---|
| Windows 11 (x64) | _bkz. aşağı_ | | | |
| macOS | | | | |
| Ubuntu 22.04 | | | | |
