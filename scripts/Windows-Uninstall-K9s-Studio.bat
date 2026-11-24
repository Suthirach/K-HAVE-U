@echo off
chcp 65001 >nul
title K9s Studio - ลบการติดตั้ง
echo จะลบ K9s Studio ออกจากเครื่องนี้ใช่ไหม?
echo.
echo   Y = ลบทั้งหมด: ตัวแอป, การตั้งค่า/GitHub key, เครื่องมือที่แอปโหลดเอง
echo                   และ kubectl, k9s, gcloud ที่ติดตั้งด้วย winget
echo   A = ลบเฉพาะแอป (เก็บ kubectl, k9s, gcloud ไว้)
echo   N = ยกเลิก
echo.
echo ไม่ลบการล็อกอิน Google/GCP และ kubeconfig ของคุณ
choice /C YAN /N /M "เลือก (Y/A/N): "
set "CH=%errorlevel%"
if "%CH%"=="3" exit /b 0

taskkill /IM K9s-Studio.exe /F >nul 2>&1
if exist "%LOCALAPPDATA%\Programs\K9s Studio\unins000.exe" (
  "%LOCALAPPDATA%\Programs\K9s Studio\unins000.exe" /SILENT
)
rmdir /S /Q "%LOCALAPPDATA%\K9s Studio" >nul 2>&1

if "%CH%"=="1" (
  winget uninstall -e --id Kubernetes.kubectl
  winget uninstall -e --id Derailed.k9s
  winget uninstall -e --id Google.CloudSDK
)

echo.
echo ลบเรียบร้อยแล้ว
pause
