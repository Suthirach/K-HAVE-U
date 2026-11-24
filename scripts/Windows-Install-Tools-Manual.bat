@echo off
chcp 65001 >nul
title Klar - ติดตั้งเครื่องมือ
echo == Klar: ติดตั้งเครื่องมือที่ต้องใช้ (kubectl, k9s, gcloud) ==
where winget >nul 2>&1
if errorlevel 1 (
  echo ไม่พบ winget กรุณาอัปเดต "App Installer" จาก Microsoft Store แล้วรันใหม่
  pause
  exit /b 1
)

where kubectl >nul 2>&1 || winget install -e --id Kubernetes.kubectl --accept-source-agreements --accept-package-agreements
where k9s >nul 2>&1 || winget install -e --id Derailed.k9s --accept-source-agreements --accept-package-agreements
where gcloud >nul 2>&1 || winget install -e --id Google.CloudSDK --accept-source-agreements --accept-package-agreements

set "GCLOUD="
where gcloud >nul 2>&1 && set "GCLOUD=gcloud"
if not defined GCLOUD if exist "%LOCALAPPDATA%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd" set "GCLOUD=%LOCALAPPDATA%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"
if not defined GCLOUD if exist "%ProgramFiles(x86)%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd" set "GCLOUD=%ProgramFiles(x86)%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"

if defined GCLOUD (
  call "%GCLOUD%" components install gke-gcloud-auth-plugin --quiet
) else (
  echo ยังหา gcloud ไม่เจอ ให้ปิดหน้าต่างนี้ เปิดใหม่ แล้วรัน: gcloud components install gke-gcloud-auth-plugin
)

echo.
echo เรียบร้อย ปิดหน้าต่างนี้แล้วเปิด Klar.exe ได้เลย
echo (ถ้าเพิ่งติดตั้ง gcloud ให้เปิดโปรแกรมใหม่ เพื่อให้เห็น PATH ล่าสุด)
pause
