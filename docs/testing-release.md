# Testing a release in Windows Sandbox

Windows Sandbox is a throwaway, clean copy of Windows: everything in it is deleted when you close it. Turn it on once in **Windows Features → Windows Sandbox** (needs a restart).

The test files live in `C:\tw-test`, outside the repo. Never commit them.

## 1. Prepare (on your PC)

1. Make the folder and a Sandbox config, `C:\tw-test\traywise.wsb`:
   ```xml
   <Configuration>
     <MappedFolders>
       <MappedFolder><HostFolder>C:\tw-test</HostFolder><ReadOnly>true</ReadOnly></MappedFolder>
     </MappedFolders>
     <AudioInput>Enable</AudioInput>
   </Configuration>
   ```
2. Double-click `traywise.wsb`. In the Sandbox, run `ipconfig` and note the **Default Gateway** (your PC, as the Sandbox sees it), for example `172.20.112.1`. Close the Sandbox.
3. Two test configs. Replace `HOST` with that address. They point the updater at your PC instead of GitHub, and make the update files. Plain `http` is allowed only in these test builds.

   `C:\tw-test\a.json`:
   ```json
   { "bundle": { "createUpdaterArtifacts": true },
     "plugins": { "updater": { "endpoints": ["http://HOST:8000/latest.json"], "dangerousInsecureTransportProtocol": true } } }
   ```
   `C:\tw-test\b.json`: the same, plus `"version": "0.1.1",` on the first line inside the braces.
4. Quit Traywise. In PowerShell, in the project folder, load your signing key for this window only:
   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\traywise.key" -Raw
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Net.NetworkCredential]::new('', (Read-Host -AsSecureString 'Key password')).Password
   ```
5. Build the "old" version and keep its installer:
   ```powershell
   pnpm tauri build --config C:\tw-test\a.json
   copy src-tauri\target\release\bundle\nsis\Traywise_0.1.0_x64-setup.exe C:\tw-test\
   ```
6. Build the "new" one and keep the installer and its signature:
   ```powershell
   pnpm tauri build --config C:\tw-test\b.json
   copy src-tauri\target\release\bundle\nsis\Traywise_0.1.1_x64-setup.exe* C:\tw-test\
   ```
7. Write `C:\tw-test\latest.json`. Paste the contents of `Traywise_0.1.1_x64-setup.exe.sig` as the signature:
   ```json
   { "version": "0.1.1",
     "platforms": { "windows-x86_64": {
       "signature": "PASTE .sig CONTENTS",
       "url": "http://HOST:8000/Traywise_0.1.1_x64-setup.exe" } } }
   ```
8. Serve the folder to the Sandbox (allow it through the firewall if asked):
   ```powershell
   python -m http.server 8000 --bind 0.0.0.0 --directory C:\tw-test
   ```

## 2. Checklist (in the Sandbox)

Open `traywise.wsb`. The folder is on the Sandbox's desktop.

**Install and first run**
- [ ] Run `Traywise_0.1.0_x64-setup.exe`. SmartScreen warns; "More info" → "Run anyway" works. No admin prompt.
- [ ] It's in the Start menu and in Settings → Apps → Installed apps (publisher Traywise, version 0.1.0).
- [ ] The tray icon appears. Left-click opens the panel and clicking elsewhere hides it. Right-click → Quit works.

**Every module** (switch each on in Settings)
- [ ] Volume mixer: play a sound in Edge and change its volume and mute.
- [ ] Audio devices: the output list shows; "Mute all microphones" and Ctrl+Alt+M work.
- [ ] Keep awake: on; `powercfg /requests` lists Traywise. Off; it's gone.
- [ ] System monitor: the numbers move while open.
- [ ] Quick toggles: dark mode, desktop icons, lock screen.

**Start with Windows**
- [ ] Switch it on. Task Manager → Startup apps lists Traywise as Enabled.

**Update**
- [ ] Quit and restart Traywise. Within a few seconds the panel shows "Traywise 0.1.1 is available".
- [ ] "Install & restart": a small progress window, then Traywise comes back by itself.
- [ ] Installed apps now shows 0.1.1, and settings and Start with Windows are kept.
- [ ] Switch "Check for updates" off, and on the next start no request reaches the server window on your PC.

**Uninstall**
- [ ] Uninstall from Installed apps with Traywise **running**, and tick "Delete the application data".
- [ ] None of these are left: `%LOCALAPPDATA%\Traywise`, `%APPDATA%\Traywise`, `%LOCALAPPDATA%\com.traywise.desktop`, the Start menu shortcut, the Installed apps entry, the Startup apps entry.
- [ ] Optional: reinstall, uninstall *without* the box ticked. `%APPDATA%\Traywise\settings.json` stays, and nothing else does.

Close the Sandbox (everything in it is deleted), stop the server (Ctrl+C), and delete `C:\tw-test` if you like. Its installers are signed with your key.
