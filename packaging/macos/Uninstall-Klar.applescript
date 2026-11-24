-- Uninstaller for Klar. Compiled into "Uninstall Klar.app" by build-app.sh.
-- One dialog asks for permission, then everything is removed.

set homePath to POSIX path of (path to home folder)
set appPath to "/Applications/Klar.app"
set userAppPath to homePath & "Applications/Klar.app"
set dataPath to homePath & "Library/Application Support/Klar"

set msg to "จะลบ Klar ออกจากเครื่องนี้ใช่ไหม?" & return & return & ¬
	"ลบทั้งหมด จะลบ:" & return & ¬
	"• ตัวแอป Klar" & return & ¬
	"• การตั้งค่า, GitHub key และเครื่องมือที่แอปโหลดมาเอง" & return & ¬
	"• kubectl, k9s, gcloud ที่ติดตั้งด้วย Homebrew" & return & return & ¬
	"ไม่ลบ: การล็อกอิน Google/GCP และ kubeconfig ของคุณ" & return & ¬
	"(โปรแกรมอื่นอาจใช้ kubectl/gcloud อยู่ ถ้าไม่แน่ใจเลือก \"ลบเฉพาะแอป\")"

set choice to button returned of (display dialog msg with title "Klar" with icon caution ¬
	buttons {"ยกเลิก", "ลบเฉพาะแอป", "ลบทั้งหมด"} default button "ยกเลิก" cancel button "ยกเลิก")

try
	do shell script "pkill -x klar || true"
end try
delay 1

do shell script "rm -rf " & quoted form of appPath & " " & quoted form of userAppPath & " " & quoted form of dataPath

if choice is "ลบทั้งหมด" then
	try
		do shell script "export PATH=/opt/homebrew/bin:/usr/local/bin:$PATH; if command -v brew >/dev/null 2>&1; then brew uninstall kubernetes-cli k9s >/dev/null 2>&1 || true; brew uninstall --cask gcloud-cli >/dev/null 2>&1 || true; fi"
	end try
end if

display dialog "ลบเรียบร้อยแล้ว" with title "Klar" buttons {"ตกลง"} default button "ตกลง"
