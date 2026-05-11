# Security Notes

Operational security playbook for claude-workbench. This file tracks the
findings of the project audit and the remediation plan for each.

## Self-Update Supply-Chain Hardening (HIGH — open)

**Finding** (audit 2026-05-11): `src/update/install.rs` uses
`self_update::backends::github::Update` with default configuration. Downloads
are over HTTPS but there is no checksum or signature verification on the
binary archive. A compromised GitHub release asset (compromised account,
account takeover, supply-chain attack on a maintainer) would install an
arbitrary binary on every user's machine on the next auto-update — silently
and with the privileges of the running user.

Tar-slip protection is fully delegated to the `self_update` crate; if a
malicious archive entry contains `../` or absolute paths, defense depends
entirely on whatever guard that crate ships.

### Remediation Plan

The fix has two halves, both required:

#### Half 1: Sign release archives in CI

In `.github/workflows/release.yml`, before the `gh release upload` step:

1. Generate an ed25519 signing keypair **once**, on a developer workstation:
   ```bash
   # Use zipsign (matches self_update's `signatures` feature) or minisign.
   cargo install zipsign
   zipsign generate-keys --pubkey claude-workbench-pub.bin --privkey claude-workbench-priv.bin
   ```
2. Store `claude-workbench-priv.bin` as a base64-encoded GitHub Actions
   secret (e.g. `ZIPSIGN_PRIVATE_KEY`). Never commit the private key.
3. Commit `claude-workbench-pub.bin` to the repository at
   `signing/claude-workbench-pub.bin`.
4. In the release workflow, after building the platform archives:
   ```yaml
   - name: Sign archives
     env:
       ZIPSIGN_KEY_B64: ${{ secrets.ZIPSIGN_PRIVATE_KEY }}
     run: |
       echo "$ZIPSIGN_KEY_B64" | base64 -d > /tmp/key.bin
       for f in dist/claude-workbench-*.tar.gz dist/claude-workbench-*.zip; do
         zipsign sign tar /tmp/key.bin "$f"
       done
       rm -f /tmp/key.bin
   ```
5. Upload both the archive **and** the corresponding `.sig` sidecar file to
   the release.

#### Half 2: Verify signatures in the client

The `signatures` feature is already enabled on `self_update` in `Cargo.toml`.
What remains:

1. Embed the public key at compile time in `src/update/install.rs`:
   ```rust
   const RELEASE_VERIFYING_KEY: &[u8] =
       include_bytes!("../../signing/claude-workbench-pub.bin");
   ```
2. Configure the updater to require a verified signature:
   ```rust
   Update::configure()
       .repo_owner(REPO_OWNER)
       .repo_name(REPO_NAME)
       .bin_name(BIN_NAME)
       .target(target)
       .current_version(CURRENT_VERSION)
       .verifying_keys([RELEASE_VERIFYING_KEY])  // <-- new
       .show_download_progress(false)
       .show_output(false)
       .no_confirm(true)
       .build()
   ```
3. Bump the **major** binary version when this lands. Older binaries that
   self-update will still receive the new signed archive successfully (they
   simply won't verify the signature). Newer binaries will refuse any
   unsigned or wrongly-signed archive.

#### Rollout Order

The two halves must land **in this order**, never reversed:

1. Sign the next 2-3 releases first. Verification is _not_ enabled in the
   client, so older clients keep updating normally.
2. Once releases reliably ship `.sig` sidecars, ship a client release that
   enables `verifying_keys`. Document in RELEASE_NOTES that older signed
   archives are required from this version on.

If verification is enabled in the client before the release workflow signs,
**every existing user's auto-update will break** until they manually replace
the binary.

### Effort Estimate

- Half 1 (CI workflow): ~30 minutes once a keypair exists.
- Half 2 (client code): ~10 minutes plus regression test.
- Keypair generation + secret rotation policy: ~1 hour planning + setup.

---

## Browser/Editor Command Construction (MEDIUM — open)

**Finding**: `src/browser/opener.rs:83-106` (`open_file_with_browser`,
`open_file_with_editor`) reads `config.ui.browser` / equivalent as a free
string, splits on whitespace via a hand-rolled `split_command()`, and uses
the first token as the program. No allow-list, no path validation. Currently
low-risk because config is user-owned, but if any future code path derives
the field from PTY output, a URL, or a remote source, it becomes command
injection on every file-open.

**Mitigation** (cheap, do it):

- After splitting, validate that the first token resolves to either an
  absolute path or a basename in `$PATH` whose name matches
  `^[A-Za-z0-9_./-]+$`. Reject anything else.
- Document explicitly in the config schema that `browser` and `editor` must
  not be sourced from untrusted input.

---

## Shell Fallback in Dependency Probe (MEDIUM — open)

**Finding**: `src/setup/dependency_checker.rs:172-186` builds a shell command
string with `shlex::try_quote` (good, just migrated from `shell-escape`)
then passes it to `$SHELL -i -c "<cmd>"`. Static call sites today are safe.
The pattern is fragile — one careless caller passing PTY-derived text via
`args` and a `shlex` bug becomes shell injection.

**Mitigation**:

- Replace the `-i -c` shell string with a direct `Command::new(name).args(args)`
  invocation. The only reason to go through a shell is to resolve aliases or
  shell functions; for binary lookups (which is the entire purpose of this
  module), that is unnecessary.
- If shell-resolved binaries are required, gate the call behind an explicit
  allow-list of known dependency names.

---

## Predictable Temp File Path (MEDIUM — open)

**Finding**: `src/browser/pdf_export.rs:119` writes preview HTML to
`$TMPDIR/<stem>-<dd.mm.yyyy>.html`. The path is guessable. On a multi-user
system, a local attacker can pre-create the path as a symlink to a target
file and the write redirects to it.

**Mitigation**: Use `tempfile::Builder::new().prefix(stem).suffix(".html")
.tempfile_in(env::temp_dir())?`. The crate opens with `O_EXCL` and an
unpredictable suffix.

---

## Closed Findings

- **shell-escape unmaintained** (audit 2026-05-11): replaced with `shlex` 1.x
  in `src/app/pty.rs` and `src/setup/dependency_checker.rs`. Tracked in
  commit history for v0.89 release.
