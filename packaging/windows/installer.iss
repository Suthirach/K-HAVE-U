; Inno Setup script: builds dist\Klar-Windows-Setup.exe (a normal click-through installer).
; The file name is fixed because the in-app updater downloads exactly this file.
; Build: ISCC.exe /DAppVersion=0.1.0 packaging\windows\installer.iss   (after `cargo build --release`)

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define AppName "Klar"

[Setup]
AppId={{B7A1D9C4-5E2F-4C3A-9D61-2F0C8A7E4B15}
AppName={#AppName}
AppVersion={#AppVersion}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
; Per-user install: no administrator prompt, and silent updates work without UAC.
PrivilegesRequired=lowest
OutputDir=..\..\dist
OutputBaseFilename=Klar-Windows-Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
SetupIconFile=..\..\assets\icon\klar.ico
; Shown on the first page of the wizard
InfoBeforeFile=README-Install.txt
UninstallDisplayIcon={app}\Klar.exe

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"

[Files]
Source: "..\..\target\release\klar.exe"; DestDir: "{app}"; DestName: "Klar.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\Klar.exe"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\Klar.exe"; Tasks: desktopicon

[Code]
var
  TokenPage: TInputQueryWizardPage;

procedure InitializeWizard;
begin
  TokenPage := CreateInputQueryPage(wpSelectDir,
    'Update key (optional)',
    'GitHub access token for automatic updates',
    'If Klar is published in a private GitHub repository, paste a read-only token here so the app can check for updates. Leave it empty if you do not need it. You can also add it later inside the app.');
  TokenPage.Add('GitHub token:', True);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Token, Dir: String;
begin
  if CurStep = ssPostInstall then
  begin
    Token := Trim(TokenPage.Values[0]);
    StringChangeEx(Token, '"', '', True);
    StringChangeEx(Token, '\', '', True);
    if Token <> '' then
    begin
      Dir := ExpandConstant('{localappdata}\Klar');
      ForceDirectories(Dir);
      SaveStringToFile(Dir + '\update-settings.json', '{"github_token":"' + Token + '"}', False);
    end;
  end;
end;

[Run]
; Normal install: offer to launch at the end. Silent install (in-app update): relaunch automatically.
Filename: "{app}\Klar.exe"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
Filename: "{app}\Klar.exe"; Flags: nowait; Check: WizardSilent
