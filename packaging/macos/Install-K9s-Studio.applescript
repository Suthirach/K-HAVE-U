-- "Install K9s Studio" in the .dmg: one double-click copies the app to Applications and opens it.
-- The app then asks for the key and installs everything else itself.

set myPath to POSIX path of (path to me)
set volumeDir to do shell script "dirname " & quoted form of myPath
set src to volumeDir & "/K9s Studio.app"
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
