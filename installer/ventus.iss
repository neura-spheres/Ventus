#ifndef MyAppVersion
  #define MyAppVersion "1.0.15"
#endif

#define MyAppName      "Ventus"
#define MyAppPublisher "Ventus"
#define MyAppURL       "https://github.com/neura-spheres/Ventus"
#define MyAppExeName   "ventus.exe"
#define MyAppID        "Ventus"
#define MyAppDesc      "Focused desktop browser with AI built in"
#define MyAumid        "NeuraSpheres.Ventus"
#define MyHtmlProgID   "VentusHTML"
#define MyHttpProgID   "VentusURLHttp"
#define MyHttpsProgID  "VentusURLHttps"

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
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\extensions\ubol\*"; DestDir: "{app}\assets\extensions\ubol"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "Focused desktop browser with AI built in"; AppUserModelID: "{#MyAumid}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "Focused desktop browser with AI built in"; Tasks: desktopicon; AppUserModelID: "{#MyAumid}"

[Registry]
; App registration
Root: HKA; Subkey: "Software\{#MyAppID}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\{#MyAppID}"; ValueType: string; ValueName: "InstallPath"; ValueData: "{app}"
Root: HKA; Subkey: "Software\{#MyAppID}"; ValueType: string; ValueName: "Version"; ValueData: "{#MyAppVersion}"

; App Paths so Windows can launch the app by name and the Start Menu indexes it
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: "Path"; ValueData: "{app}"
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}"; ValueType: string; ValueName: ""; ValueData: "{#MyAppName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}"; ValueType: string; ValueName: "AppUserModelID"; ValueData: "{#MyAumid}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"""
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{#MyAppDesc}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".htm"; ValueData: "{#MyHtmlProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".html"; ValueData: "{#MyHtmlProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".shtml"; ValueData: "{#MyHtmlProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".xht"; ValueData: "{#MyHtmlProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".xhtml"; ValueData: "{#MyHtmlProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\UrlAssociations"; ValueType: string; ValueName: "http"; ValueData: "{#MyHttpProgID}"
Root: HKA; Subkey: "Software\Clients\StartMenuInternet\{#MyAppID}\Capabilities\UrlAssociations"; ValueType: string; ValueName: "https"; ValueData: "{#MyHttpsProgID}"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}"; ValueType: string; ValueName: ""; ValueData: "Ventus HTML Document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\Application"; ValueType: string; ValueName: "ApplicationCompany"; ValueData: "{#MyAppPublisher}"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\Application"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{#MyAppDesc}"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\Application"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\Application"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\Application"; ValueType: string; ValueName: "AppUserModelID"; ValueData: "{#MyAumid}"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHtmlProgID}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""--url"" ""%1"""
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}"; ValueType: string; ValueName: ""; ValueData: "Ventus HTTP URL"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\Application"; ValueType: string; ValueName: "ApplicationCompany"; ValueData: "{#MyAppPublisher}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\Application"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{#MyAppDesc}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\Application"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\Application"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\Application"; ValueType: string; ValueName: "AppUserModelID"; ValueData: "{#MyAumid}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHttpProgID}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""--url"" ""%1"""
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}"; ValueType: string; ValueName: ""; ValueData: "Ventus HTTPS URL"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\Application"; ValueType: string; ValueName: "ApplicationCompany"; ValueData: "{#MyAppPublisher}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\Application"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{#MyAppDesc}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\Application"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\Application"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\Application"; ValueType: string; ValueName: "AppUserModelID"; ValueData: "{#MyAumid}"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MyHttpsProgID}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""--url"" ""%1"""

[Run]
Filename: "{tmp}\MicrosoftEdgeWebview2Setup.exe"; Parameters: "/silent /install"; StatusMsg: "Checking WebView2 Runtime..."; Flags: waituntilterminated runhidden; Check: NeedsWebView2
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function HasWebView2Key(RootKey: Integer; Subkey: String): Boolean;
var
  Ver: String;
begin
  Result := RegQueryStringValue(RootKey, Subkey, 'pv', Ver) and (Ver <> '');
end;

function NeedsWebView2(): Boolean;
begin
  Result := not (
    HasWebView2Key(HKLM, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9C2A7C5}') or
    HasWebView2Key(HKCU, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9C2A7C5}') or
    HasWebView2Key(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9C2A7C5}') or
    HasWebView2Key(HKCU, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9C2A7C5}')
  );
end;
