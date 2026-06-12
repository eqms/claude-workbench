---
phase: quick-260612-esh
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .github/workflows/release.yml
  - Cargo.toml
autonomous: true
requirements:
  - automate-homebrew-tap-update
must_haves:
  truths:
    - "After a v* tag is pushed, the Homebrew formula in eqms/homebrew-claude-workbench is updated automatically without manual intervention"
    - "The formula update job fails loudly (non-zero exit) if any of the 4 Homebrew assets is missing from the release"
    - "Private key material for the deploy key is never echoed to logs or committed to the repo"
    - "A workflow_dispatch input allows manually triggering the formula bump for an existing release tag (e.g. v0.96.0) to verify end-to-end without cutting a new release"
    - "The tap commit message follows the [CHG] convention: `[CHG] Update to vX.Y.Z`"
  artifacts:
    - path: ".github/workflows/release.yml"
      provides: "update-homebrew-tap job that runs after release job succeeds"
      contains: "update-homebrew-tap"
    - path: "Cargo.toml"
      provides: "version bumped to 0.96.1"
      contains: "version = \"0.96.1\""
  key_links:
    - from: "update-homebrew-tap job"
      to: "eqms/homebrew-claude-workbench"
      via: "SSH deploy key (TAP_DEPLOY_KEY secret)"
      pattern: "TAP_DEPLOY_KEY"
    - from: "formula updater script"
      to: "Formula/claude-workbench.rb"
      via: "sed replacements for 4 URLs + 4 sha256 values"
      pattern: "sha256"
---

<objective>
Automate the Homebrew formula bump that was previously done manually after each release (and was forgotten for v0.88–v0.95). Add an `update-homebrew-tap` job to release.yml that runs after the `release` job, downloads the 4 Homebrew-relevant release assets, computes their SHA256, rewrites Formula/claude-workbench.rb, and pushes the result to eqms/homebrew-claude-workbench via a deploy key.

Purpose: Eliminate the manual post-release step and prevent the tap from falling behind again.
Output: Extended release.yml with update-homebrew-tap job; Cargo.toml version bump to 0.96.1; deploy key provisioned on both repos.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/quick/260612-esh-automate-homebrew-formula-bump-in-github/260612-esh-PLAN.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Provision deploy key and store as Actions secret</name>
  <files>~/.ssh/tap-deploy-key (ephemeral, never committed)</files>
  <action>
Generate an Ed25519 SSH keypair with no passphrase, add the public key as a write-access deploy key on eqms/homebrew-claude-workbench, and store the private key as an Actions secret on eqms/claude-workbench. Delete the local key files afterwards.

Step-by-step (run with `gh` CLI, which already has admin rights on both repos):

1. Generate keypair to a temp directory:
   `ssh-keygen -t ed25519 -C "github-actions-tap-deploy@claude-workbench" -f /tmp/tap_deploy_key -N ""`

2. Add public key as write deploy key on the tap repo:
   `gh repo deploy-key add /tmp/tap_deploy_key.pub --repo eqms/homebrew-claude-workbench --title "claude-workbench CI tap bumper" --allow-write`

3. Store private key as secret on the workbench repo:
   `gh secret set TAP_DEPLOY_KEY --repo eqms/claude-workbench < /tmp/tap_deploy_key`

4. Wipe local key files:
   `rm -f /tmp/tap_deploy_key /tmp/tap_deploy_key.pub`

Verify: `gh repo deploy-key list --repo eqms/homebrew-claude-workbench` shows the new key; `gh secret list --repo eqms/claude-workbench` shows TAP_DEPLOY_KEY. Never echo or cat the private key at any point.
  </action>
  <verify>
    <automated>gh repo deploy-key list --repo eqms/homebrew-claude-workbench | grep "claude-workbench CI tap bumper" && gh secret list --repo eqms/claude-workbench | grep TAP_DEPLOY_KEY</automated>
  </verify>
  <done>Deploy key visible on tap repo with write access; TAP_DEPLOY_KEY secret visible (not its value) on workbench repo; no key files remain in /tmp.</done>
</task>

<task type="auto">
  <name>Task 2: Add update-homebrew-tap job to release.yml and bump Cargo.toml</name>
  <files>.github/workflows/release.yml, Cargo.toml</files>
  <action>
Make two file changes:

**A) Cargo.toml — version bump**
Change `version = "0.96.0"` to `version = "0.96.1"` (PATCH; CI-only change). Also update the date in any header comment if present.

**B) .github/workflows/release.yml — three additions**

1. Add `workflow_dispatch` trigger with a `tag` input at the top of the `on:` block (alongside the existing `push.tags`):

```
  workflow_dispatch:
    inputs:
      tag:
        description: 'Release tag to update Homebrew formula for (e.g. v0.96.0)'
        required: true
        type: string
```

2. Add a top-level env var below the existing `CARGO_TERM_COLOR` env to derive the effective tag in both trigger modes:

Do NOT add a top-level env for the tag — instead, use `${{ github.ref_name || inputs.tag }}` inline in the new job (ref_name is empty on workflow_dispatch, inputs.tag is empty on push; one is always set).

3. Append the following new job after the `release:` job. The job must:
- Be named `update-homebrew-tap`
- Run on `ubuntu-latest`
- Declare `needs: release` — but with a conditional: the `release` job only runs on `push` (not `workflow_dispatch`, where no build/release happens). Use `if: always() && (needs.release.result == 'success' || github.event_name == 'workflow_dispatch')` to handle both triggers correctly.
- Actually: `needs` with a conditional is complex. Simpler: declare the job without `needs` on workflow_dispatch, but that requires two separate job definitions. Instead use the cleanest approach: declare `needs: [release]` and add `if: needs.release.result == 'success' || github.event_name == 'workflow_dispatch'`. This makes the job wait for `release` on push triggers (and only proceed if it succeeded), while running independently on workflow_dispatch.

Wait — `needs` on a job that didn't run results in the dependent job being skipped automatically. Use a different approach: define ONE job with no hard `needs` dependency, but gate it with the right condition. Use `needs: []` and rely on the trigger. Actually the cleanest pattern is: use `needs: [release]` with `if: always() && (needs.release.result == 'success' || github.event_name == 'workflow_dispatch')`. GitHub skips `release` on `workflow_dispatch` (it has no `workflow_dispatch` trigger path — wait, `release` job runs on any trigger because the `on` block is on the whole workflow). On `workflow_dispatch`, the `build` matrix won't run (it has no condition, so it WILL run). This is wrong — we want workflow_dispatch to SKIP build and ONLY run update-homebrew-tap.

Correct approach: add `if: github.event_name == 'push'` to the `build` job and to the `release` job. Then the `update-homebrew-tap` job uses `needs: [release]` with `if: always() && (needs.release.result == 'success' || github.event_name == 'workflow_dispatch')`.

So the final changes to the YAML structure are:
- Add `if: github.event_name == 'push'` to the `build` job
- Add `if: github.event_name == 'push'` to the `release` job  
- Append the new `update-homebrew-tap` job

**The update-homebrew-tap job body:**

```yaml
  update-homebrew-tap:
    name: Update Homebrew Formula
    runs-on: ubuntu-latest
    needs: [release]
    if: always() && (needs.release.result == 'success' || github.event_name == 'workflow_dispatch')
    steps:
      - name: Determine release tag
        id: tag
        run: |
          if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
            echo "tag=${{ inputs.tag }}" >> $GITHUB_OUTPUT
          else
            echo "tag=${{ github.ref_name }}" >> $GITHUB_OUTPUT
          fi

      - name: Download Homebrew assets and compute SHA256
        id: hashes
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          TAG="${{ steps.tag.outputs.tag }}"
          VERSION="${TAG#v}"
          echo "Updating formula for $TAG (version $VERSION)"

          # Four Homebrew targets (no Windows)
          TARGETS=(
            "aarch64-apple-darwin"
            "x86_64-apple-darwin"
            "aarch64-unknown-linux-gnu"
            "x86_64-unknown-linux-gnu"
          )

          # Download each asset and compute sha256
          for TARGET in "${TARGETS[@]}"; do
            ASSET="claude-workbench-${TARGET}.tar.gz"
            echo "Downloading $ASSET ..."
            gh release download "$TAG" \
              --repo eqms/claude-workbench \
              --pattern "$ASSET" \
              --dir /tmp/assets
            if [ ! -f "/tmp/assets/$ASSET" ]; then
              echo "ERROR: Asset $ASSET not found in release $TAG"
              exit 1
            fi
            SHA=$(sha256sum /tmp/assets/$ASSET | awk '{print $1}')
            echo "sha256_${TARGET//-/_}=$SHA" >> $GITHUB_OUTPUT
            echo "$TARGET -> $SHA"
          done

      - name: Checkout tap repo
        uses: actions/checkout@v4
        with:
          repository: eqms/homebrew-claude-workbench
          ssh-key: ${{ secrets.TAP_DEPLOY_KEY }}
          path: tap

      - name: Rewrite formula
        run: |
          TAG="${{ steps.tag.outputs.tag }}"
          VERSION="${TAG#v}"
          FORMULA="tap/Formula/claude-workbench.rb"

          # Verify all four sha256 values are non-empty
          SHA_AARCH64_DARWIN="${{ steps.hashes.outputs.sha256_aarch64_apple_darwin }}"
          SHA_X86_64_DARWIN="${{ steps.hashes.outputs.sha256_x86_64_apple_darwin }}"
          SHA_AARCH64_LINUX="${{ steps.hashes.outputs.sha256_aarch64_unknown_linux_gnu }}"
          SHA_X86_64_LINUX="${{ steps.hashes.outputs.sha256_x86_64_unknown_linux_gnu }}"

          for SHA in "$SHA_AARCH64_DARWIN" "$SHA_X86_64_DARWIN" "$SHA_AARCH64_LINUX" "$SHA_X86_64_LINUX"; do
            if [ -z "$SHA" ] || [ ${#SHA} -ne 64 ]; then
              echo "ERROR: Invalid SHA256 value: '$SHA'"
              exit 1
            fi
          done

          # Replace version in all 4 download URLs
          # Pattern: /releases/download/vX.Y.Z/ -> /releases/download/<TAG>/
          sed -i "s|/releases/download/v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/|/releases/download/${TAG}/|g" "$FORMULA"

          # Replace sha256 values — formula has exactly 4 sha256 lines in this order:
          # aarch64-apple-darwin, x86_64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu
          # Use awk to replace the Nth occurrence of a sha256 line
          awk -v s1="$SHA_AARCH64_DARWIN" \
              -v s2="$SHA_X86_64_DARWIN" \
              -v s3="$SHA_AARCH64_LINUX" \
              -v s4="$SHA_X86_64_LINUX" \
              'BEGIN{n=0}
               /^[[:space:]]*sha256 "/ {
                 n++
                 if(n==1) { sub(/"[a-f0-9]{64}"/, "\"" s1 "\"") }
                 if(n==2) { sub(/"[a-f0-9]{64}"/, "\"" s2 "\"") }
                 if(n==3) { sub(/"[a-f0-9]{64}"/, "\"" s3 "\"") }
                 if(n==4) { sub(/"[a-f0-9]{64}"/, "\"" s4 "\"") }
               }
               { print }' "$FORMULA" > "$FORMULA.tmp" && mv "$FORMULA.tmp" "$FORMULA"

          echo "Formula after update:"
          cat "$FORMULA"

          # Sanity check: exactly 4 sha256 lines present
          SHA_COUNT=$(grep -c 'sha256 "' "$FORMULA")
          if [ "$SHA_COUNT" -ne 4 ]; then
            echo "ERROR: Expected 4 sha256 lines, found $SHA_COUNT"
            exit 1
          fi

      - name: Commit and push formula
        run: |
          TAG="${{ steps.tag.outputs.tag }}"
          cd tap
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/claude-workbench.rb
          if git diff --cached --quiet; then
            echo "No changes to formula — already up to date for $TAG"
            exit 0
          fi
          git commit -m "[CHG] Update to ${TAG}"
          git push
          echo "Formula updated and pushed for $TAG"
```

After writing both file changes, run `cargo check` to ensure Cargo.toml is valid (no build needed).
  </action>
  <verify>
    <automated>grep 'version = "0.96.1"' /Users/picard/gitbase/workbench/Cargo.toml && grep 'update-homebrew-tap' /Users/picard/gitbase/workbench/.github/workflows/release.yml && grep 'workflow_dispatch' /Users/picard/gitbase/workbench/.github/workflows/release.yml && cargo check --manifest-path /Users/picard/gitbase/workbench/Cargo.toml 2>&1 | tail -3</automated>
  </verify>
  <done>Cargo.toml shows version 0.96.1; release.yml contains workflow_dispatch trigger, if: conditions on build and release jobs, and the full update-homebrew-tap job; cargo check passes.</done>
</task>

<task type="auto">
  <name>Task 3: Commit and push to both remotes (no tag)</name>
  <files>.github/workflows/release.yml, Cargo.toml</files>
  <action>
Stage and commit the two changed files, then push to both remotes without creating a release tag (this is a CI-only change).

1. Stage files:
   `git add .github/workflows/release.yml Cargo.toml`

2. Commit with correct prefix:
   `git commit -m "[ADD] v0.96.1: automate Homebrew formula bump in release pipeline"`

3. Push to both remotes:
   `git push origin main`
   `git push upstream main`

Do NOT create or push a `v0.96.1` tag — a CI-only commit does not warrant a binary release. The new job will take effect on the next real release tag.

After pushing, verify the workflow appears on GitHub Actions:
   `gh workflow list --repo eqms/claude-workbench` — confirm "Release" workflow shows; `gh workflow view release.yml --repo eqms/claude-workbench` to confirm workflow_dispatch trigger is visible.

To do a dry-run end-to-end test without cutting a new release, trigger the formula bump manually against the existing v0.96.0 release:
   `gh workflow run release.yml --repo eqms/claude-workbench --field tag=v0.96.0`

Monitor via: `gh run list --repo eqms/claude-workbench --workflow=release.yml --limit 3`
  </action>
  <verify>
    <automated>git log --oneline -1 | grep "0.96.1" && git push --dry-run upstream main 2>&1 | grep -v "error"</automated>
  </verify>
  <done>Commit exists on main; both remotes updated; no v0.96.1 tag created; workflow_dispatch trigger confirmed visible on GitHub; optional manual dry-run against v0.96.0 succeeds (tap formula unchanged because already at v0.96.0, job exits cleanly with "already up to date").</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GH Actions → tap repo | Authenticated write via SSH deploy key stored as Actions secret |
| GH Actions → release assets | Read via GITHUB_TOKEN (same-repo, no elevation needed) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-esh-01 | Tampering | Formula/claude-workbench.rb | mitigate | SHA256 length check (must be 64 hex chars) + sanity count check (must be exactly 4 sha256 lines) before commit |
| T-esh-02 | Information Disclosure | TAP_DEPLOY_KEY secret | mitigate | Key set via stdin (`< /tmp/tap_deploy_key`), never echoed; local files wiped with `rm -f` immediately after |
| T-esh-03 | Spoofing | Release asset download | mitigate | `gh release download` uses GITHUB_TOKEN against same-repo — assets are verified by GitHub's release API |
| T-esh-04 | Denial of Service | Missing asset causes formula corruption | mitigate | Explicit `exit 1` if any asset file is absent after download; formula write is skipped entirely on error |
| T-esh-SC | Tampering | No new npm/pip/cargo installs in this plan | accept | No package installs; pure shell + existing `gh` CLI + standard utils |
</threat_model>

<verification>
Manual end-to-end test (run after Task 3):
```bash
gh workflow run release.yml --repo eqms/claude-workbench --field tag=v0.96.0
# Wait ~60s, then:
gh run list --repo eqms/claude-workbench --workflow=release.yml --limit 3
# Check tap repo after run:
gh api repos/eqms/homebrew-claude-workbench/commits/main --jq '.commit.message'
# Expected: "[CHG] Update to v0.96.0" or "No changes — already up to date" exit 0
```
</verification>

<success_criteria>
- workflow_dispatch on release.yml with `tag=v0.96.0` completes green in GitHub Actions
- tap repo commit history shows `[CHG] Update to vX.Y.Z` after a real v* tag push
- Formula contains correct SHA256 values (verifiable with `brew fetch eqms/claude-workbench/claude-workbench`)
- `cargo check` passes on version 0.96.1
- No key material appears in any git history or Actions log
</success_criteria>

<output>
Create `.planning/quick/260612-esh-automate-homebrew-formula-bump-in-github/260612-esh-01-SUMMARY.md` when done
</output>
