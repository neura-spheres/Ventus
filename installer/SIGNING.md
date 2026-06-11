# Signing Ventus installers with SignPath Foundation

Unsigned installers trigger the Windows SmartScreen "Windows protected your PC - unknown
publisher" prompt on download. We won't buy a certificate, so we use **SignPath Foundation**,
which gives open-source projects free code signing with a CA-trusted certificate. The private
key stays on SignPath's HSM; signing happens in CI, not locally.

What gets signed: **`Ventus-Setup-<version>.exe`** (the installer). The in-app auto-updater
(`src/updater.rs`) downloads that file and runs it `/VERYSILENT`, so signed updates roll out
in place with no manual reinstall.

The release workflow is `.github/workflows/release.yml`: it builds the exe, builds the
installer with Inno Setup, submits the installer to SignPath, then publishes the signed
installer to a GitHub Release for the pushed tag.

> SmartScreen reputation with an OV certificate builds up over downloads; the very first signed
> releases may still warn until reputation accrues, but the publisher name shows instead of
> "unknown publisher." An EV certificate would clear it instantly but is not free.

---

## One-time setup (maintainer only)

These steps need a human with SignPath + GitHub admin access. The agent cannot do them.

1. **Apply to SignPath Foundation** at https://signpath.org/ (open-source program) for the
   `neura-spheres/Ventus` repo. Wait for approval.

2. **Connect the SignPath GitHub App** to `neura-spheres/Ventus` (SignPath uses it to verify
   signing requests originate from this repo's Actions).

3. **In the SignPath dashboard, create:**
   - a **Project** (note its slug, e.g. `ventus`),
   - a **Signing Policy** for releases (note its slug, e.g. `release-signing`),
   - an **Artifact Configuration** for a single `.exe` file with Authenticode signing
     (note its slug, e.g. `installer`).

4. **Generate a CI API token** in SignPath.

5. **Add GitHub repository secrets** (Settings -> Secrets and variables -> Actions -> Secrets):

   | Secret | Value |
   | --- | --- |
   | `SIGNPATH_API_TOKEN` | the CI API token from step 4 |
   | `SIGNPATH_ORGANIZATION_ID` | your SignPath organization GUID |

6. **Add GitHub repository variables** (same page -> Variables):

   | Variable | Value |
   | --- | --- |
   | `SIGNPATH_PROJECT_SLUG` | project slug from step 3 |
   | `SIGNPATH_SIGNING_POLICY_SLUG` | signing-policy slug from step 3 |
   | `SIGNPATH_INSTALLER_ARTIFACT_CONFIG_SLUG` | installer artifact-configuration slug from step 3 |

   The workflow targets the SignPath cloud at `https://app.signpath.io`. If your organization is
   on a regional or self-hosted instance, change `connector-url` in the signing step of
   `.github/workflows/release.yml`.

---

## Cutting a release

1. Bump `version:` in `config.yaml`.
2. Commit.
3. Tag and push: `git tag v1.0.20 && git push origin v1.0.20`
   (the tag must equal `v` + the `config.yaml` version, or the workflow fails on purpose).
4. The workflow builds, signs the installer via SignPath, and publishes the Release.

---

## Test checklist: clean second machine with old Ventus installed

Verify the in-place auto-update before announcing a release.

1. On a second PC that already has an **older** Ventus installed, open it and note open tabs /
   bookmarks.
2. Trigger the in-app update (or wait for the check). It downloads `Ventus-Setup-<new>.exe` and
   applies it silently.
3. After the app relaunches, confirm:
   - [ ] No SmartScreen "unknown publisher" prompt during the download/run (publisher name shows).
   - [ ] App reopens on its own (auto-update relaunch worked).
   - [ ] Bookmarks, history, settings, and session are preserved.
