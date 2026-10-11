; GPUI Starter per-user installer, modeled on zed.iss (Zed's Inno Setup
; script, zed-industries/zed crates/zed/resources/windows/zed.iss).
; Decision record: docs/research/icons-and-windows.md (B1-B3).

#ifndef Version
#define Version "0.0.0"
#endif
; Passed by scripts/build-windows-installer.sh as a Windows path; the
; default reaches the repo root from packaging/windows/ for bare ISCC runs.
#ifndef RepoRoot
#define RepoRoot "..\\.."
#endif

[Setup]
; uuid5(DNS, "com.gpui-starter.app"): must never change, a different GUID
; installs a second app instead of upgrading in place.
AppId={{0DCC0A2C-DA4B-5462-97E5-882EB5929259}
AppName=GPUI Starter
AppVersion={#Version}
AppVerName=GPUI Starter {#Version}
AppPublisher=gpui-starter contributors
AppPublisherURL=https://github.com/freeoxide/gpui-starter
AppSupportURL=https://github.com/freeoxide/gpui-starter/issues
; {autopf} = %LOCALAPPDATA%\Programs under lowest privileges: per-user
; install dir stays writable by the updater's file swap without UAC.
DefaultDirName={autopf}\GPUI-Starter
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
DirExistsWarning=no
; A running exe locks itself; Restart Manager closes it before replacing.
CloseApplications=force
SetupIconFile={#RepoRoot}\assets\app-icon\gpui-starter.ico
OutputDir={#RepoRoot}\target\release
OutputBaseFilename=GPUI-Starter-{#Version}-x86_64-windows-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UninstallDisplayName=GPUI Starter
UninstallDisplayIcon={app}\gpui-starter.exe
; Signing is default-off; enable from CI with /DSignToolName=<name> plus
; /S<name>=<command> (see packaging/windows/README.md).
#if Defined(SignToolName)
SignTool={#SignToolName}
#endif

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#RepoRoot}\target\x86_64-pc-windows-msvc\release\gpui-starter.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Named gpui-starter so first launch rewrites THIS lnk with the toast AUMID
; (src/platform/toast_identity.rs); the value below already matches.
Name: "{autoprograms}\gpui-starter"; Filename: "{app}\gpui-starter.exe"; AppUserModelID: "com.gpui-starter.app"
Name: "{autodesktop}\gpui-starter"; Filename: "{app}\gpui-starter.exe"; Tasks: desktopicon; AppUserModelID: "com.gpui-starter.app"

; No [Registry]: the app self-registers gpui-starter:// in HKCU at first run
; (deep_link_registration.rs); a copy here adds only uninstall cleanup.
