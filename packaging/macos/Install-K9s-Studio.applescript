-- "Install K9s Studio" in the .dmg: one double-click copies the app to Applications and opens it.
-- The app then asks for the key and installs everything else itself.

-- macOS may run this installer from a random temporary folder (App Translocation) when the
-- .dmg was downloaded, so look for the app next to us first, then on the mounted disk image.
set myPath to POSIX path of (path to me)
set volumeDir to do shell script "dirname " & quoted form of myPath
set src to volumeDir & "/K9s Studio.app"
set src to do shell script "if [ -d " & quoted form of src & " ]; then echo " & quoted form of src & "; else for d in /Volumes/*/; do if [ -d \"$d/Install K9s Studio.app\" ] && [ -d \"$d/K9s Studio.app\" ]; then echo \"${d}K9s Studio.app\"; break; fi; done; fi"
if src is "" then
	display dialog "หาไฟล์ K9s Studio ไม่เจอ" & return & return & "เปิดไฟล์ K9s-Studio-Mac.dmg ค้างไว้ แล้วลองใหม่ หรือลาก K9s Studio ไปที่ Applications เอง" with title "K9s Studio" buttons {"ตกลง"} default button "ตกลง" with icon caution
	return
end if
set dest to "/Applications/K9s Studio.app"

display dialog "ติดตั้ง K9s Studio ลงในเครื่องนี้?" with title "K9s Studio" ¬
	buttons {"ยกเลิก", "ติดตั้ง"} default button "ติดตั้ง" cancel button "ยกเลิก"

try
	do shell script "pkill -x k9s-ui || true"
end try

try
	do shell script "rm -rf " & quoted form of dest & " && cp -R " & quoted form of src & " " & quoted form of dest
on error
	-- No write access to /Applications: install for this user only.
	set dest to (POSIX path of (path to home folder)) & "Applications/K9s Studio.app"
	do shell script "mkdir -p \"$HOME/Applications\" && rm -rf " & quoted form of dest & " && cp -R " & quoted form of src & " " & quoted form of dest
end try

do shell script "xattr -dr com.apple.quarantine " & quoted form of dest & " || true"
do shell script "open " & quoted form of dest
