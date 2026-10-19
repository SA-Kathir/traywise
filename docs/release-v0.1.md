# Release v0.1

Goal: the first public release of Traywise, on `github.com/SA-Kathir/traywise`, with an installer people can download and auto-updates for every later version.

Decisions (owner, 2026-10-19):
- **Owner:** the personal account SA-Kathir. Move to an organisation later if the project grows; GitHub keeps redirects for transferred repos.
- **Code signing:** v0.1 ships **unsigned**. The README explains the SmartScreen warning. The owner applies to the SignPath open-source programme in parallel, and signing starts once approved.
- **Auto-updates:** **included in v0.1**, so every installed copy can update itself from then on.

Read `CLAUDE.md` first. Same rules as before:
- Small, reviewable steps.
- Explain each one in plain terms.
- Commit only with the owner's OK.

**Nothing is published by Claude** (no repo creation, pushes, tags, releases or secrets) without the owner's explicit go-ahead in chat at that moment.

## Part 1: local prep (Claude can do all of this)

1. **README.md**, replacing the template:
   - what Traywise is, in one line
   - a screenshot or GIF of the panel (mark it as a placeholder if needed)
   - the module list with one line each
   - install steps, including the unsigned-SmartScreen note ("More info" → "Run anyway")
   - a privacy section (local only, no telemetry; what touches the network: only update checks)
   - requirements (Windows 11, WebView2 is built in)
   - build-from-source steps and the GPL-3.0 licence
   - It must not use Vorssaint's name or branding, except an optional "inspired by" credit.
2. **CONTRIBUTING.md** (how to build, the module pattern, small PRs) and **SECURITY.md** (how to report privately, using GitHub's private vulnerability reporting). *SECURITY.md was dropped (owner, 2026-10-19): it isn't required and the owner doesn't want to promise response times yet. Private reporting works without it.*
3. **Issue templates** (bug report, feature request) and a **pull request template**, all short.
4. **Installer (NSIS)**
   - Start menu shortcut and the uninstaller entry.
   - Version taken from tauri.conf.json.
   - Uninstall removes the start-with-Windows entry. It asks whether to delete `%APPDATA%\Traywise` settings, or leaves them and documents that.
   - Per-user install, no admin prompt, if Tauri's NSIS options allow it.
5. **Auto-updater**
   - Add `tauri-plugin-updater` (and the process plugin if a restart is needed).
   - Endpoint: `https://github.com/SA-Kathir/traywise/releases/latest/download/latest.json`.
   - Check on startup and then once a day, never while the panel is in use. Show "Update available → Install & restart" in the panel or settings; no forced updates.
   - The **public key** goes in tauri.conf.json. Leave a clearly marked placeholder until the owner generates the key pair (see Part 2).
   - A setting to turn update checks off.
6. **Release workflow** (`.github/workflows/release.yml`)
   - On a `v*` tag: build on a Windows runner with `tauri-apps/tauri-action`, sign the update artifacts with the secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and create a **draft** GitHub release with the installer and `latest.json`.
   - The owner publishes the draft by hand.
7. **Clean-machine test plan:** a short checklist in `docs/testing-release.md` for Windows Sandbox:
   - install, first run, every module, start with Windows
   - update from a test build
   - uninstall leaves nothing behind
8. **CHANGELOG.md** with a v0.1.0 entry.

## Part 2: owner-only steps (Claude explains, the owner does them)

These involve the owner's keys, accounts or publishing, so Claude must not do them itself:

1. **Generate the update-signing key** on your own machine: `pnpm tauri signer generate -w %USERPROFILE%\.tauri\traywise.key`. Choose a password. Store the key file and password in a password manager; losing them means existing installs can't update. Then give Claude the **public** key, which is safe to share.
2. **Create the GitHub repo** `SA-Kathir/traywise` as public, or give Claude the go-ahead to create it with `gh`. Then push.
3. **Add the two signing secrets** in the repo settings (Settings → Secrets and variables → Actions). Paste them yourself; never paste them into chat.
4. **Enable private vulnerability reporting** in the repo's security settings.
5. **Apply to SignPath** for free open-source code signing. That needs the public repo first.
6. **Tag `v0.1.0`**, check the draft release the workflow builds, then publish it.

## Done when

- Part 1 is complete, committed with the owner's OK, and CI passes.
- The installer and updater flow are verified in Windows Sandbox: a test build updates to a newer test build.
- The owner has done Part 2, and v0.1.0 is published.
