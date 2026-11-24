# K9s Studio (k9s-ui)

แอปเดสก์ท็อปสำหรับดูสถานะ Kubernetes (GKE) แบบ Warm Dark Glass: Overview ของ namespace, Topology (AntV G6 / Classic), รายละเอียด service, Logs, ENV และเทอร์มินัล K9s ในตัว เขียนด้วย Rust บน UI framework [Rune](https://github.com/anomalyco/rune)

## Build บน Windows

### 1. ติดตั้งเครื่องมือ

| เครื่องมือ | ใช้ทำอะไร | ติดตั้ง |
| --- | --- | --- |
| Git | clone โค้ด | `winget install Git.Git` |
| Visual Studio Build Tools 2022 (workload **Desktop development with C++** + Windows 11/10 SDK) | linker / C compiler ของ MSVC | `winget install Microsoft.VisualStudio.2022.BuildTools` แล้วเลือก workload ใน Visual Studio Installer |
| Rust (rustup) | คอมไพเลอร์ | `winget install Rustlang.Rustup` |
| Microsoft Edge WebView2 Runtime | แสดงไดอะแกรม AntV G6 | มีมากับ Windows 10/11 อยู่แล้ว ถ้าไม่มี: `winget install Microsoft.EdgeWebView2Runtime` |

ไม่ต้องเลือกเวอร์ชัน Rust เอง: [rust-toolchain.toml](rust-toolchain.toml) กำหนดไว้แล้ว (`1.99.1`) rustup จะดาวน์โหลดให้อัตโนมัติตอน build ครั้งแรก

### 2. Clone โค้ดให้ `rune` อยู่ข้าง ๆ โปรเจกต์

[Cargo.toml](Cargo.toml) อ้างถึง Rune แบบ relative path (`../rune/...`) ทั้งสองโฟลเดอร์จึงต้องอยู่ในโฟลเดอร์แม่เดียวกัน:

```powershell
mkdir C:\dev; cd C:\dev
git clone https://github.com/anomalyco/rune.git
git clone https://github.com/Suthirach/K-HAVE-U.git
```

```text
C:\dev\
├── rune\
└── K-HAVE-U\
```

### 3. Build

เปิด **Developer PowerShell for VS 2022** (หรือ PowerShell ธรรมดาก็ได้ถ้า MSVC อยู่ใน PATH):

```powershell
cd C:\dev\K-HAVE-U
cargo build --release
```

ไฟล์ที่ได้: `target\release\k9s-ui.exe` (ฟอนต์ Plus Jakarta Sans และหน้า AntV G6 ฝังอยู่ใน exe แล้ว ก๊อปไปเครื่องอื่นได้ไฟล์เดียว)

รันระหว่างพัฒนา:

```powershell
cargo run
```

### 4. โปรแกรมที่ต้องมีตอนรัน

แอปเรียกคำสั่งเหล่านี้จาก `PATH`:

- `kubectl`: อ่านข้อมูล cluster (`winget install Kubernetes.kubectl`)
- `gcloud` + `gke-gcloud-auth-plugin`: ล็อกอิน Google / GKE ([Google Cloud SDK](https://cloud.google.com/sdk/docs/install) แล้วรัน `gcloud components install gke-gcloud-auth-plugin`)
- `k9s`: แท็บ K9s CLI (`winget install Derailed.k9s`)

ไดอะแกรม AntV G6 โหลดไลบรารี G6 จาก CDN (unpkg / jsDelivr) จึงต้องต่ออินเทอร์เน็ต ถ้าออฟไลน์ ให้สลับไปโหมด **Classic** ในหน้า Topology

### ปัญหาที่พบบ่อย

- **`link.exe` not found / `LNK1181`**: ยังไม่ได้ติดตั้ง workload *Desktop development with C++* หรือ Windows SDK
- **`failed to read ...\rune\crates\rune\Cargo.toml`**: โฟลเดอร์ `rune` ไม่ได้อยู่ข้าง `K-HAVE-U` (ดูข้อ 2)
- **ไดอะแกรมขึ้น "could not load"**: ไม่มีอินเทอร์เน็ต หรือไม่มี WebView2 Runtime

## ทดสอบ

```powershell
cargo test                       # unit tests ของ Rust
node tests\test_topology_g6.js   # ทดสอบหน้า AntV G6 (ต้องมี Node.js)
```

ภาพหน้าจอแบบ headless (debug build เท่านั้น):

```powershell
cargo build
target\debug\k9s-ui.exe --scene overview --size 1340x860 --scale 1 --snap overview.png
```

ซีนที่มี: `overview`, `topology`, `service`
