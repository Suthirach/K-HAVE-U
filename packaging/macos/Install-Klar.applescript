-- "Install Klar" in the .dmg: one double-click copies the app to Applications and opens it.
-- The app then asks for the key and installs everything else itself.

-- macOS may run this installer from a random temporary folder (App Translocation) when the
-- .dmg was downloaded, so look for the app next to us first, then on the mounted disk image.
set myPath to POSIX path of (path to me)
set volumeDir to do shell script "dirname " & quoted form of myPath
set src to volumeDir & "/Klar.app"
set src to do shell script "if [ -d " & quoted form of src & " ]; then echo " & quoted form of src & "; else for d in /Volumes/*/; do if [ -d \"$d/Install Klar.app\" ] && [ -d \"$d/Klar.app\" ]; then echo \"${d}Klar.app\"; break; fi; done; fi"
if src is "" then
	display dialog "หาไฟล์ Klar ไม่เจอ" & return & return & "เปิดไฟล์ Klar-Mac.dmg ค้างไว้ แล้วลองใหม่ หรือลาก Klar ไปที่ Applications เอง" with title "Klar" buttons {"ตกลง"} default button "ตกลง" with icon caution
	return
end if
set dest to "/Applications/Klar.app"

display dialog "ติดตั้ง Klar ลงในเครื่องนี้?" with title "Klar" ¬
	buttons {"ยกเลิก", "ติดตั้ง"} default button "ติดตั้ง" cancel button "ยกเลิก"

try
	do shell script "pkill -x klar || true"
end try

try
	do shell script "rm -rf " & quoted form of dest & " && cp -R " & quoted form of src & " " & quoted form of dest
on error
	-- No write access to /Applications: install for this user only.
	set dest to (POSIX path of (path to home folder)) & "Applications/Klar.app"
	do shell script "mkdir -p \"$HOME/Applications\" && rm -rf " & quoted form of dest & " && cp -R " & quoted form of src & " " & quoted form of dest
end try

do shell script "xattr -dr com.apple.quarantine " & quoted form of dest & " || true"
do shell script "open " & quoted form of dest
