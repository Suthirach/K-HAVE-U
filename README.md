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

## ติดตั้งและอัปเดต (สำหรับผู้ใช้ทั่วไป)

โหลดไฟล์จากหน้า **Releases** ของ repo นี้ ไม่ต้องใช้ Terminal และไม่ต้องติดตั้ง Rust

| เครื่อง | ไฟล์ | วิธีติดตั้ง |
| --- | --- | --- |
| Mac (M1/M2/M3 และ Intel) | `K9s-Studio-Mac.dmg` | เปิดไฟล์ ดับเบิลคลิก **Install K9s Studio** → กด **ติดตั้ง** แอปจะเปิดขึ้นมาให้ใส่ key แล้วกด **ตกลง** (ครั้งแรกถ้ามีเตือน: คลิกขวา → Open) |
| Windows 10/11 (x64) | `K9s-Studio-Windows-Setup.exe` | ดับเบิลคลิก → Next → ใส่ key → Install → แอปเปิดขึ้นมาและติดตั้งเครื่องมือที่เหลือเอง (ถ้าเจอ SmartScreen: More info → Run anyway) |

- **เครื่องมือที่ต้องใช้ (kubectl, k9s, gcloud):** ถ้าเครื่องยังไม่มี แอปจะขึ้นแถบเหลือง กด **ติดตั้งให้อัตโนมัติ** แอปจะโหลดมาเก็บในโฟลเดอร์ของแอปเอง
- **อัปเดต:** ตอนเปิดแอปจะเช็กเวอร์ชันใหม่จาก Releases ถ้ามีจะขึ้นปุ่ม **อัปเดตเป็น vX** ที่แถบด้านบน กดแล้วแอปโหลดตัวใหม่ ติดตั้งทับ และเปิดใหม่ให้เอง 
- **ตั้งค่าครั้งแรก:** เปิดแอปครั้งแรกจะมีหน้า "ตั้งค่าเริ่มต้น" ให้วาง key (ไม่บังคับ) และติดตั้งเครื่องมือที่ขาด ก่อนไปหน้าเข้าสู่ระบบ ส่วน Windows มีช่องกรอก key ในหน้าต่างติดตั้ง (Setup.exe) ด้วย
- **ถ้า repo นี้เป็น private:** ที่หน้าล็อกอินมีช่อง "อัปเดตแอปจาก repo private" ให้วาง GitHub token แล้วกด **บันทึกและเช็ก** (สร้างที่ GitHub → Settings → Developer settings → Personal access tokens → *Fine-grained tokens* เลือกเฉพาะ repo `K-HAVE-U` และให้สิทธิ์ *Contents: Read-only* อย่างเดียว) token ถูกเก็บเป็นไฟล์ `update-settings.json` ในโฟลเดอร์ของแอป (Mac: `~/Library/Application Support/K9s Studio/`, Windows: `%LOCALAPPDATA%\K9s Studio\`) หรือตั้งตัวแปร `K9S_STUDIO_GITHUB_TOKEN` อย่าฝัง token ลงในตัวแอปที่แจกคนอื่น เพราะใครก็ดึงออกมาได้ ถ้าแจกให้หลายคนใช้ repo public ที่เก็บเฉพาะไฟล์ Release จะปลอดภัยกว่า

### ออกเวอร์ชันใหม่ (สำหรับผู้ดูแล)

1. แก้ `version` ใน `Cargo.toml` (เช่น `0.2.0`) แล้ว commit
2. `git tag v0.2.0 && git push origin v0.2.0` (tag ต้องตรงกับเลขใน `Cargo.toml`)
3. GitHub Actions ([.github/workflows/build.yml](.github/workflows/build.yml)) จะ build และสร้าง Release พร้อมไฟล์สองตัวด้านบนให้เอง ชื่อไฟล์ห้ามเปลี่ยน เพราะตัวอัปเดตในแอปโหลดตามชื่อนี้
4. ถ้า repo `rune` เป็น private ให้เพิ่ม secret `RUNE_TOKEN` (Personal Access Token ที่อ่าน repo นั้นได้) ใน Settings ของ repo นี้

### สคริปต์สำหรับนักพัฒนา (โฟลเดอร์ `scripts/`)

| ไฟล์ | ทำอะไร |
| --- | --- |
| `Mac-Run-for-Development.sh` | รันแอปจาก source (`cargo run`) พร้อมตั้ง PATH ให้ |
| `Mac-Build-and-Install-from-Source.command` | ดับเบิลคลิก: build จาก source แล้วติดตั้งลง /Applications |
| `Mac-Uninstall-K9s-Studio.command`, `Windows-Uninstall-K9s-Studio.bat` | ดับเบิลคลิกเพื่อลบแอป การตั้งค่า และเครื่องมือที่แอปโหลดเอง (ถามก่อนทุกครั้ง) ใน .dmg มีแอป **Uninstall K9s Studio** ให้ดับเบิลคลิก (ไม่เปิด Terminal) ถามขออนุญาตครั้งเดียว มีตัวเลือก ลบทั้งหมด / ลบเฉพาะแอป / ยกเลิก |
| `Mac-Install-Tools-Manual.command`, `Windows-Install-Tools-Manual.bat` | ติดตั้ง kubectl/k9s/gcloud ด้วยมือ (ปกติไม่ต้องใช้ เพราะแอปติดตั้งให้เอง) |
