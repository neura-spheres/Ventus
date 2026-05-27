#ifndef MyAppVersion
  #define MyAppVersion "1.0.2"
#endif

#define MyAppName      "Ventus"
#define MyAppPublisher "Ventus"
#define MyAppURL       "https://github.com/neura-spheres/Ventus"
#define MyAppExeName   "ventus.exe"
#define MyAppID        "Ventus"

[Setup]
AppId={{8B3F2D91-6A4E-4C7B-9E1D-2F5A8C0B4E6D}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=Ventus-Setup-{#MyAppVersion}
SetupIconFile=..\assets\logo.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
CloseApplications=yes
CloseApplicationsFilter=*{#MyAppExeName}*
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\extensions\ubol\*"; DestDir: "{app}\assets\extensions\ubol"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "Focused desktop browser with AI built in"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "Focused desktop browser with AI built in"; Tasks: desktopicon

[Registry]
; App registration
Root: HKA; Subkey: "Software\{#MyAppID}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\{#MyAppID}"; ValueType: string; ValueName: "InstallPath"; ValueData: "{app}"
Root: HKA; Subkey: "Software\{#MyAppID}"; ValueType: string; ValueName: "Version"; ValueData: "{#MyAppVersion}"

; App Paths so Windows can launch the app by name and the Start Menu indexes it
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: "Path"; ValueData: "{app}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// WebView2 check removed - can be re-added with proper Inno Setup syntax if needed
