# PVP Trade — Ürün ve Teknik Yol Haritası

> Sürüm: 0.1  
> Tarih: 29 Ağustos 2026  
> Durum: Faz 1 — Proof of Architecture geliştiriliyor
> Ana ağ: Solana  
> Doküman dili: Türkçe

## Güncel geliştirme özeti

- Battle yaşam döngüsü ve TypeScript referans modeli hazır.
- Anchor programı bulut CI üzerinde derleniyor.
- Altı ondalıklı settlement mint protokol yapılandırmasına sabitlendi.
- Kurucu ve rakip stake'leri ayrı, self-authorised PDA token vault'larına yatırılıyor.
- Açık battle iptalinde kurucu stake'i PDA imzasıyla iade ediliyor.
- Web uygulamasında Solana Wallet Standard devnet bağlantısı ile gerçek Create/Join instruction üretimi hazır.
- Kalıcı public program ID atandı; özel deployment keypair yalnızca yerel ve Git-ignore kapsamındaki `.anchor` klasöründe tutuluyor.
- LiteSVM custody entegrasyon testleri eşit deposit, ayrı vault, iptal iadesi, yanlış mint ve yetersiz bakiye senaryolarını kapsıyor.
- Manuel ve environment-secret korumalı devnet deployment workflow'u programı deploy edip protocol PDA'sını Circle devnet USDC ile initialize edecek şekilde hazır.
- Güncel dış engel: devnet deployer cüzdanı faucet rate-limit nedeniyle henüz fonlanmadı; program bu nedenle henüz deploy edilmedi.
- Sıradaki kritik iş: deployer'a devnet SOL sağlamak, deployment workflow'unu çalıştırmak ve iki cüzdanla canlı Create/Join smoke testi yapmak.

## 1. Ürün vizyonu

PVP Trade, eşit sermayeyle başlayan trader'ların belirli bir süre boyunca zincir üstünde yarıştığı bir PvP trading protokolüdür.

İlk ürün modeli:

- İki oyuncu eşit miktarda USDC yatırır.
- Her oyuncunun sermayesi program kontrollü, izole bir battle hesabında tutulur.
- Oyuncular izin verilen Solana tokenlarını DEX likiditesi üzerinden spot olarak trade eder.
- Maç sırasında ek para yatırılamaz ve para çekilemez.
- Süre sonunda bütün pozisyonlar USDC'ye settle edilir.
- Net USDC bakiyesi daha yüksek olan oyuncu, iki hesabın toplam kalan değerini protokol ücreti düşüldükten sonra alır.

Uzun vadeli vizyon:

- 1v1 düellolar
- Farklı süre ve sermaye ligleri
- Meme Arena ve Safe Arena
- 3v3 ve takım savaşları
- Eleme usulü turnuvalar
- Sezonlar, ligler ve klanlar
- Zincir üstü trader profili ve itibar sistemi
- İkinci ağ olarak Robinhood Chain üzerinde ayrı bir Arena
- Hukuki değerlendirme sonrasında utility veya düzenlemeye tabi gelir paylaşım tokenı

## 2. Kesinleşen temel kararlar

| Konu | Karar |
|---|---|
| İlk blockchain | Solana |
| İlk oyun modu | 1v1 |
| Varsayılan süre | 24 saat |
| Başlangıç ve settlement varlığı | Native Solana USDC |
| Trading türü | Spot DEX swap |
| Likidite erişimi | Jupiter Swap üzerinden Solana DEX'leri |
| Kullanıcı fonları | İzole ve program kontrollü PDA vault'larında |
| Token kapsamı | Güvenlik ve likidite koşullarını geçen SPL tokenlar |
| Meme coin desteği | Evet, ayrı risk kurallarıyla |
| MVP tokenı | Yok; önce ürün ve points/XP |
| Geliştirme deneyimi | Windows-first, PowerShell ve tarayıcı |
| Yerel Linux/WSL gereksinimi | Kullanıcı için olmayacak |

## 3. MVP dışında tutulanlar

Aşağıdaki özellikler ilk sürümde yapılmayacaktır:

- Perpetual/futures ve kaldıraç
- Cross-chain battle
- 3v3 veya 10v10
- Seyirci bahisleri
- Permissionless şekilde her token contract'ını kabul etmek
- Protokol tokenı ve halka açık token satışı
- Pasif holder temettüsü
- DAO yönetimi
- Mobil native uygulama
- Kendi AMM veya DEX'imizi sıfırdan geliştirmek

## 4. Battle yaşam döngüsü

```text
OPEN
  ↓
FUNDED
  ↓
ACTIVE
  ↓
TRADING_LOCKED
  ↓
SETTLING
  ↓
RESOLVED ──→ CLAIMED
```

Alternatif son durumlar:

```text
OPEN ──→ CANCELLED
FUNDED ──→ REFUNDED
SETTLING ──→ DISPUTED / EMERGENCY_REFUND
```

### 4.1 Open

Battle kurucusu şu parametreleri belirler:

- Stake miktarı
- Süre
- Arena türü
- Kabul edilen token risk seviyesi
- Rakip türü: davetli veya açık lobby

### 4.2 Funded

- İki oyuncu aynı miktarda USDC yatırmıştır.
- İki ayrı battle vault oluşturulmuştur.
- Başlangıçtan sonra stake değiştirilemez.
- Belirli sürede başlamayan battle iptal ve iade edilebilir.

### 4.3 Active

- Oyuncular kendi vault'larındaki varlıkları Jupiter rotalarıyla takas eder.
- Bütün çıktılar aynı oyuncunun battle token hesaplarına dönmek zorundadır.
- Harici deposit, withdrawal veya yetkisiz transfer reddedilir.
- Swap fee, slippage ve işlem maliyetleri performansa dahil edilir.

### 4.4 Trading Locked

Önerilen ilk kural:

- Battle bitmeden kısa süre önce yeni alımlar durur.
- Oyuncular yalnızca USDC'ye dönmek için satış yapabilir.
- Kesin lock süresi prototip testlerinden sonra belirlenecektir.

### 4.5 Settling

- Oyuncuların kalan tokenları USDC'ye çevrilir.
- Aynı tokenı iki oyuncu da tutuyorsa mümkün olduğunda toplu satış ve oransal çıktı uygulanır.
- Satılamayan veya güvenli rota bulunamayan varlıklar için önceden ilan edilmiş acil durum kuralı kullanılır.
- Settlement tamamlanana kadar kullanıcı claim yapamaz.

### 4.6 Resolved

- Net USDC bakiyeleri karşılaştırılır.
- Protokol ücreti kesilir.
- Kalan toplam havuz kazanana atanır.
- Eşitlik toleransı içindeki sonuçlarda iade kuralı uygulanır.

## 5. Skor ve ödeme modeli

Başlangıç sermayeleri eşit olduğu için temel skor:

```text
net_equity = final_usdc_balance
```

Kullanıcıya gösterilecek performans:

```text
return_percent = (final_usdc_balance / initial_usdc_balance - 1) × 100
```

Kazanan ödemesi:

```text
gross_pool = player_a_final_usdc + player_b_final_usdc
protocol_fee = gross_pool × settlement_fee_rate
winner_payout = gross_pool - protocol_fee
```

Önemli kural: Trader'ın kârı protokol geliri değildir. Protokol geliri yalnızca açıkça tanımlanmış ücretlerden oluşur.

Henüz kesinleşmeyen parametreler:

- Settlement fee oranı
- Minimum ve maksimum stake
- Eşitlik toleransı
- Trading lock süresi
- Minimum emir büyüklüğü
- Bir oyuncunun aynı anda açabileceği maksimum battle sayısı

## 6. Solana program mimarisi

### 6.1 On-chain programlar/modüller

```text
pvp_trade_program
├── battle
│   ├── create_battle
│   ├── join_battle
│   ├── start_battle
│   ├── cancel_battle
│   └── emergency_pause
├── vault
│   ├── initialize_vault
│   ├── deposit_stake
│   ├── execute_swap
│   └── close_vault
├── registry
│   ├── add_token_policy
│   ├── update_token_policy
│   └── disable_token
├── settlement
│   ├── lock_trading
│   ├── begin_settlement
│   ├── settle_asset
│   ├── resolve_winner
│   └── claim_prize
└── fees
    ├── calculate_fee
    └── withdraw_protocol_fees
```

### 6.2 Ana hesaplar

| Hesap | Görev |
|---|---|
| `ProtocolConfig` | Global kurallar, yetkiler ve fee parametreleri |
| `Battle` | Oyuncular, süre, stake, durum ve sonuç |
| `PlayerVaultAuthority` | Oyuncunun izole vault PDA yetkisi |
| `PlayerTokenAccount` | Battle içindeki her token için program kontrollü ATA |
| `TokenPolicy` | Bir tokenın risk seviyesi ve işlem limitleri |
| `SettlementRecord` | Final dönüşümler, skor ve payout kaydı |
| `FeeVault` | Protokol ücretlerinin ayrı ve doğrulanabilir saklanması |

### 6.3 Yetki sınırları

Battle programı aşağıdaki şartları zincir üstünde doğrulamalıdır:

- Emri imzalayan kişi doğru oyuncudur.
- Battle `ACTIVE` durumundadır.
- Girdi tokenı oyuncunun battle hesabındadır.
- Çıktı token hesabı aynı battle vault'a aittir.
- Token aktif ve uygun risk kategorisindedir.
- Emir miktarı pozisyon ve likidite limitlerini aşmaz.
- Minimum output/slippage koruması vardır.
- Swap yalnızca izin verilen Jupiter programı ve doğrulanmış hesaplar üzerinden çalışır.
- İşlem başka bir cüzdana değer aktaramaz.

## 7. Jupiter trading katmanı

Jupiter kullanılmasının amaçları:

- Solana DEX'leri arasında iyi rota bulmak
- Çok sayıda SPL tokena erişmek
- Multi-hop ve split-route swap desteği
- Swap öncesi price impact verisi
- Program içinden CPI ile işlem yapabilmek
- İşlem takibi ve platform fee hesabı

Teknik yaklaşım:

1. Quote servisi Jupiter'dan rota alır.
2. Uygulama rotayı ve risk bilgisini kullanıcıya gösterir.
3. Kullanıcı emri imzalar.
4. Battle programı parametreleri doğrular.
5. Program, Jupiter Swap programını CPI ile çağırır.
6. Çıktı yalnızca battle token hesabına yazılır.
7. İşlem olayı indexer tarafından kaydedilir.

Jupiter mainnet-only olduğundan geliştirme üç katmanda yürütülecektir:

- LiteSVM ile unit ve durum makinesi testleri
- Mock swap programıyla yerel entegrasyon testleri
- Küçük tutarlı ve limitli mainnet Jupiter CPI doğrulaması

## 8. Token uygunluk ve meme coin güvenliği

### 8.1 Arena sınıfları

| Arena | Token kapsamı | Hedef kullanıcı |
|---|---|---|
| Safe Arena | Yüksek likiditeli, doğrulanmış tokenlar | Genel kullanıcı |
| Meme Arena | Risk filtresini geçen meme coinler | Yüksek risk isteyen trader |
| Open Arena | Daha geniş ama limitli evren | MVP sonrası deneysel kullanım |

### 8.2 Token uygunluk kontrolleri

Bir tokenın trade edilebilir olması için aşağıdakiler değerlendirilecektir:

- USDC veya SOL üzerinden geçerli Jupiter çıkış rotası
- Minimum ve zaman ağırlıklı likidite
- Battle büyüklüğüne göre kabul edilebilir price impact
- Token yaşı
- Alım ve satım simülasyonunun başarılı olması
- Mint authority ve freeze authority durumu
- Holder yoğunluğu
- Organik hacim ve işlem dağılımı
- Token Program veya Token-2022 kullanımı
- Transfer fee, transfer hook, rebase benzeri davranışlar
- Bilinen scam/honeypot/risk işaretleri

### 8.3 Pozisyon limiti

Token limiti sabit dolar değeri yerine likiditeyle ilişkili olmalıdır:

```text
max_position_value = min(
  arena_absolute_limit,
  time_weighted_liquidity × allowed_liquidity_ratio
)
```

İlk sürümde ayrıca oyuncu başına tutulabilecek farklı token sayısı sınırlandırılacaktır. Bu sınır settlement gas/compute maliyetini ve griefing riskini kontrol eder.

### 8.4 Başlangıçta reddedilecek davranışlar

- Satılamayan tokenlar
- Güvenli olmayan transfer hook'ları
- Rebase tokenlar
- Belirsiz veya değişken transfer vergisi
- Dondurulmuş token hesapları
- Aşırı holder yoğunluğu
- Yetersiz çıkış likiditesi
- Programın okuyamadığı veya güvenle settle edemediği token extension'ları

## 9. Off-chain uygulama mimarisi

On-chain program fon ve kurallar için tek doğruluk kaynağıdır. Off-chain servisler kullanıcı deneyimi ve veri erişimi sağlar.

### 9.1 Web uygulaması

- Next.js ve TypeScript
- Solana wallet-standard bağlantısı
- Battle lobby
- Battle oluşturma ve katılma
- Trading terminali
- Canlı portföy ve PnL görünümü
- İşlem geçmişi
- Settlement ve claim ekranı
- Profil, XP ve leaderboard

### 9.2 API/Coordinator

- Jupiter quote koordinasyonu
- Token risk verilerinin toplanması
- Battle zamanlayıcılarının izlenmesi
- Settlement keeper işlerinin tetiklenmesi
- Bildirimler
- Rate limiting ve kötüye kullanım tespiti

Coordinator sonucu tek başına belirleyemez ve kullanıcı fonlarını transfer edemez.

### 9.3 Indexer ve veri katmanı

- Solana program event'lerini indeksleme
- PostgreSQL read model
- Redis üzerinden canlı battle state
- WebSocket ile frontend güncellemeleri
- Tekrar işlenebilir/idempotent event pipeline
- Zincir verisiyle periyodik reconciliation

## 10. Windows-first geliştirme standardı

Kullanıcı ve ana geliştirme akışı Linux/WSL gerektirmeyecektir.

### 10.1 Windows üzerinde çalışacak parçalar

- VS Code veya tercih edilen Windows editörü
- PowerShell
- Node.js frontend/backend
- PostgreSQL
- Tarayıcı cüzdanları
- Solana Playground
- Git ve CI kontrolü

### 10.2 Standart PowerShell komutları

Repo ilerledikçe aşağıdaki scriptler sağlanacaktır:

```text
scripts/setup.ps1
scripts/dev.ps1
scripts/test.ps1
scripts/build-program.ps1
scripts/deploy-devnet.ps1
scripts/check.ps1
```

Kullanıcıdan Bash, WSL veya Linux terminali açması istenmeyecektir.

### 10.3 On-chain build stratejisi

- Hızlı deneyler: Solana Playground
- Otomatik test/build: cloud CI
- Rust/Anchor bağımlılıkları: kilitli sürümler
- Mainnet öncesi: reproducible/verifiable build
- Deploy: manuel onay kapılı pipeline
- Mainnet anahtarları: repoya veya CI loglarına hiçbir zaman yazılmaz

## 11. Güvenlik modeli

### 11.1 Öncelikli tehditler

- Vault fonlarının başka adrese yönlendirilmesi
- Sahte veya değiştirilmiş Jupiter instruction/account listesi
- Token price/liquidity manipülasyonu
- Flash liquidity ile token uygunluk filtresini aşma
- Settlement sırasında MEV ve sandwich saldırısı
- Son blokta işlem sırası avantajı
- Satılamayan tokenla settlement'ı kilitleme
- Çok sayıda token hesabıyla compute griefing
- Admin anahtarının ele geçirilmesi
- Keeper/Coordinator kesintisi
- Aynı kişinin iki hesapla reward farming yapması

### 11.2 Kontroller

- PDA tabanlı izole vault
- Sıkı account ve program ID doğrulaması
- Token ve pozisyon limitleri
- Zaman ağırlıklı likidite kontrolleri
- Maksimum price impact ve slippage
- Oyuncu başına maksimum token sayısı
- Emergency pause ve dar kapsamlı kurtarma fonksiyonları
- Multisig admin
- Timelock ile kritik parametre değişiklikleri
- Event reconciliation
- Property/fuzz testleri
- LiteSVM ve Mollusk testleri
- Mainnet öncesi bağımsız akıllı sözleşme denetimi
- Açılıştan sonra bug bounty

## 12. Gelir modeli

Potansiyel protokol gelirleri:

- Settlement fee
- Jupiter/platform swap fee
- Premium battle veya turnuva giriş bedeli
- Sponsorlu ligler
- Klan ve profesyonel profil özellikleri
- B2B battle altyapısı

Gelir ilkeleri:

- Kullanıcı stake'i protokol hazinesi değildir.
- Trader kârı protokol geliri değildir.
- Bütün ücretler işlem öncesinde açıkça gösterilir.
- Battle fonları platform token likiditesini desteklemek için kullanılamaz.
- Fee vault ve şirket hazinesi muhasebesi ayrılır.

## 13. Points ve gelecekteki token

### 13.1 MVP: points/XP

- Devredilemez
- Parasal getiri vaadi yok
- Battle katılımı, kazanma, streak ve risk yönetimiyle kazanılır
- Sybil ve wash battle kontrollerine tabidir
- Gelecekte token garantisi vermez

### 13.2 Token değerlendirme kapısı

Token ancak aşağıdaki koşullardan sonra değerlendirilecektir:

- Ürün çalışan mainnet kullanımına ulaşmıştır.
- Gerçek ve sürdürülebilir protokol geliri vardır.
- Retention ve organik battle hacmi doğrulanmıştır.
- Türkiye ve hedef ülkeler için yazılı hukuki sınıflandırma alınmıştır.
- Tokenın utility veya gelir paylaşımı niteliği açıkça seçilmiştir.
- Kurucu payı, yatırımcı payı, vesting ve likidite planı kamuya açıklanabilir durumdadır.

Olası modeller:

1. Utility/governance token
2. Fee indirimi ve battle erişim tokenı
3. Aktif hizmet karşılığı staking reward
4. Buyback/burn modeli
5. Düzenlemeye tabi revenue-share/security token

Pasif holder temettüsü MVP kapsamı dışındadır ve hukuki onay olmadan tasarlanmayacaktır.

## 14. Aşamalı teslimat planı

Süreler ilk teknik spike sonrasında güncellenecek tahminlerdir.

### Faz 0 — Ürün ve protokol spesifikasyonu

Tahmini süre: 1–2 hafta

Teslimatlar:

- Battle Protocol Specification v0.1
- Account ve instruction şemaları
- Battle durum makinesi
- Fee ve payout kuralları
- Token eligibility kuralları
- Settlement edge-case matrisi
- Threat model v0.1
- Hedef ülke ve hukuki araştırma kapsamı

Çıkış kriteri:

- Kritik kurallarda açıklanmamış fon hareketi veya sonuç belirsizliği kalmaması

### Faz 1 — Proof of Architecture

Tahmini süre: 2–3 hafta

Teslimatlar:

- Anchor program iskeleti
- Battle ve iki PDA vault oluşturma
- Eşit mock USDC deposit
- Deposit sonrası withdrawal engeli
- Mock swap CPI
- Zaman ilerletmeli settlement testi
- Winner-take-all payout
- Kötü amaçlı instruction reddi
- Windows PowerShell script iskeleti
- Cloud CI build/test pipeline

Çıkış kriteri:

- Bütün kritik fon akışlarının otomatik testlerde geçmesi
- Hiçbir oyuncunun diğer vault'tan veya protokolden yetkisiz değer çıkaramaması

### Faz 2 — Jupiter entegrasyon prototipi

Tahmini süre: 2–4 hafta

Teslimatlar:

- Jupiter quote servisi
- Jupiter Swap CPI adapter
- Price impact ve slippage kontrolleri
- Platform fee hesabı
- Token hesaplarının güvenli oluşturulması
- Ana program ID/account doğrulaması
- Küçük tutarlı mainnet entegrasyon testi

Çıkış kriteri:

- Program kontrollü vault'tan swap yapılması
- Swap çıktısının yalnızca doğru vault'a dönebilmesi
- Yetkisiz destination ve değiştirilmiş route testlerinin reddedilmesi

### Faz 3 — Devnet/Test MVP web uygulaması

Tahmini süre: 4–6 hafta

Teslimatlar:

- Wallet bağlantısı
- Battle lobby
- Create/join battle
- Trading terminali
- Canlı PnL ve portföy
- Battle timer
- Settlement ekranı
- Claim akışı
- Indexer, PostgreSQL ve WebSocket
- Safe Arena token listesi
- Points/XP temel sistemi

Çıkış kriteri:

- İki bağımsız kullanıcı baştan sona battle tamamlayabilmeli
- UI ve zincir durumu yeniden yüklemede tutarlı kalmalı
- Coordinator kapansa bile fonlar güvende kalmalı

### Faz 4 — Güvenlik ve kapalı mainnet alpha

Tahmini süre: 4–8 hafta

Teslimatlar:

- Threat model v1
- Property/fuzz testleri
- Keeper yedekliliği
- Reconciliation sistemi
- Emergency pause/refund tatbikatı
- Multisig ve timelock
- Token risk pipeline'ı
- Düşük stake limitli davetli alpha
- İzleme, alarm ve olay müdahale planı

Çıkış kriteri:

- Kritik/yüksek açık bulunmaması
- Settlement başarısının hedeflenen seviyeyi karşılaması
- Kullanıcı fonu ve muhasebe reconciliation farkının sıfır olması

### Faz 5 — Audit ve public beta

Tahmini süre: audit kapsamına göre belirlenecek

Teslimatlar:

- Bağımsız program audit'i
- Bulguların düzeltilmesi ve tekrar doğrulanması
- Bug bounty
- Safe Arena public beta
- Kademeli stake limit artışı
- Şeffaf fee ve sistem durumu sayfaları
- Kullanım şartları, risk açıklamaları ve gerekli uyum süreçleri

Çıkış kriteri:

- Audit'te açık kritik/yüksek bulgu kalmaması
- Operasyon ve hukuk onayları
- Kontrollü yük altında başarılı battle/settlement metrikleri

### Faz 6 — Meme Arena ve sosyal büyüme

Teslimatlar:

- Dinamik meme token registry
- Likiditeye bağlı pozisyon limitleri
- Trader profilleri
- Shareable battle sonuçları
- Streak, rank ve sezonlar
- Davet sistemi
- Klan altyapısının ilk sürümü

### Faz 7 — Turnuvalar ve takım modları

Teslimatlar:

- Farklı battle süreleri
- 8/16/32 kişilik bracket turnuvalar
- 3v3 takım battle prototipi
- Takım skoru ve payout kuralları
- Anti-collusion analizleri
- Sponsorlu ve özel ligler

### Faz 8 — Token kararı

Teslimatlar:

- Ürün ve gelir metrikleri değerlendirmesi
- Token gerekli mi raporu
- Hukuki görüş
- Token rights matrix
- Supply, allocation ve vesting taslağı
- Treasury ve likidite politikası
- Token launch için go/no-go kararı

### Faz 9 — Robinhood Chain Arena

Teslimatlar:

- EVM battle contract adapter
- Uniswap/aggregator trading katmanı
- Ayrı chain settlement
- Ortak profil ve leaderboard
- Uygunluk değerlendirmesi sonrasında RWA/Stock Token arena araştırması

## 15. Başarı metrikleri

### Teknik

- Başarılı settlement oranı
- Ortalama swap landing süresi
- Başarısız swap oranı
- Ortalama ve p95 settlement süresi
- Reconciliation farkı
- RPC/provider kesintisinde toparlanma süresi
- Battle başına compute ve işlem maliyeti

### Ürün

- Oluşturulan ve tamamlanan battle sayısı
- Battle fill rate
- İlk battle'ını tamamlayan kullanıcı oranı
- 7/30 günlük geri dönüş
- Kullanıcı başına battle sayısı
- Davet edilen rakip dönüşümü
- Safe Arena ve Meme Arena dağılımı

### Ekonomi

- Gerçek DEX hacmi
- Protokol fee geliri
- Kullanıcı başına edinme maliyeti
- Fee sonrası operasyon marjı
- Sybil/wash olarak işaretlenen hacim oranı

## 16. Ana riskler ve azaltma planı

| Risk | Etki | İlk azaltma yaklaşımı |
|---|---|---|
| Meme coin manipülasyonu | Yanlış kazanan ve fon kaybı | Likiditeye bağlı limit, token yaşı, TWAP kontrolleri |
| Satılamayan token | Settlement kilidi | Ön simülasyon, extension filtresi, maksimum token sayısı |
| Jupiter/RPC kesintisi | İşlem ve settlement gecikmesi | Provider yedekliliği, tekrar deneme, emergency state |
| CPI/account validation açığı | Vault fon kaybı | Sıkı program/account doğrulaması, audit |
| MEV/sandwich | Skor ve payout adaletsizliği | Slippage, kontrollü landing, batch settlement |
| Son saniye avantajı | Oyun adaleti sorunu | Trading lock ve kesin işlem kabul kuralları |
| Hukuki sınıflandırma | Ürünün durdurulması veya kısıtlanması | Mainnet öncesi ülke bazlı hukuk görüşü ve limitli pilot |
| Tokenın menkul kıymet sayılması | Ağır uyum yükümlülüğü | Tokenı erteleme, rights matrix ve hukuki go/no-go |
| Wash battle/Sybil | Sahte hacim ve rewards sömürüsü | Points limitleri, davranış analizi, subvansiyon kontrolü |
| Admin anahtar riski | Protokol kontrol kaybı | Multisig, timelock, minimum admin yetkisi |

## 17. Açık ürün kararları

Kodlamadan veya ilgili fazdan önce karara bağlanması gerekenler:

- İlk stake katmanları: sabit mi, serbest mi?
- İlk settlement fee oranı nedir?
- 24 saat dışında hangi süreler açılacak?
- Son trading lock süresi kaç dakika olacak?
- Eşitlik toleransı ve iade modeli nedir?
- Oyuncu başına maksimum token sayısı kaç olacak?
- Safe ve Meme Arena likidite eşikleri nedir?
- Satılamayan tokenın final değeri sıfır mı, indirimli mi, acil iade mi?
- Settlement işlemlerinin maliyetini kim karşılar?
- Kullanıcının pozisyonları rakibine canlı gösterilecek mi?
- ProtocolConfig değişikliklerinde timelock süresi nedir?
- İlk hedef pazar ve kullanıcı ülkeleri hangileridir?
- Kapalı alpha stake üst limiti nedir?

## 18. İlk iki haftalık çalışma listesi

### Hafta 1

- Battle Protocol Specification v0.1 yazımı
- State ve instruction veri şemaları
- Bütün fon hareketlerinin diyagramı
- Settlement edge-case tablosu
- Token-2022 extension risk matrisi
- Windows-first repo ve CI tasarımı
- Program ID ve upgrade authority politikası

### Hafta 2

- Anchor program iskeleti
- `ProtocolConfig` ve `Battle` hesapları
- Create/join/cancel instruction'ları
- İki PDA vault ve mock USDC deposit
- LiteSVM temel testleri
- Zaman ilerletmeli battle testi
- Yetkisiz withdrawal negatif testleri

## 19. Referanslar

- [Solana geliştirici dokümantasyonu](https://solana.com/docs)
- [Anchor Framework](https://www.anchor-lang.com/docs)
- [Jupiter Developer Platform](https://developers.jup.ag/)
- [Jupiter development basics ve CPI](https://developers.jup.ag/docs/get-started/development-basics)
- [Solana Token Extensions](https://solana.com/docs/tokens/extensions)
- [Anchor LiteSVM testleri](https://www.anchor-lang.com/docs/testing/litesvm)
- [Anchor verifiable builds](https://www.anchor-lang.com/docs/references/verifiable-builds)
- [Solana Playground quick start](https://solana.com/docs/intro/quick-start)

## 20. Doküman yönetimi

Bu belge yaşayan bir yol haritasıdır.

- Kesinleşen kararlar 2. bölüme taşınır.
- Değişen varsayımlar tarih ve gerekçeyle güncellenir.
- Teknik uygulama ayrıntıları ayrı specification belgelerine bölünür.
- Bir faz, çıkış kriterleri karşılanmadan tamamlandı sayılmaz.
- Token, mainnet limiti ve hukuki kapsam gibi geri dönüşü zor kararlar ayrı go/no-go incelemesi gerektirir.
