# Codex Auto-Review Setup Guide

The `codex-after-ci.yml` workflow automatically posts `@codex review` comments on pull requests after successful CI runs. For this to work properly, you need to set up a Personal Access Token (PAT).

## Why PAT is Required

GitHub Copilot (`@codex`) only responds to mentions from actual users, not from bot accounts like `github-actions[bot]`. To make the workflow post comments on your behalf (so Copilot responds), we need to authenticate with a PAT instead of the default `GITHUB_TOKEN`.

## Setup Instructions

### Step 1: Create a Fine-Grained Personal Access Token

1. Go to GitHub Settings → Developer settings → Personal access tokens → Fine-grained tokens
   - Direct link: https://github.com/settings/personal-access-tokens/new

2. Configure the token:
   - **Token name**: `ROUP Codex Auto-Review`
   - **Expiration**: 90 days (or custom)
   - **Repository access**: Only select repositories → Choose `ouankou/roup`
   - **Permissions**:
     - Repository permissions:
       - **Pull requests**: Read and write ✅
       - **Issues**: Read and write ✅ (PR comments use issues API)
       - **Contents**: Read only (optional, for context)

3. Click **Generate token**

4. **IMPORTANT**: Copy the token immediately (starts with `github_pat_...`) - you won't see it again!

### Step 2: Add Token to Repository Secrets

1. Go to repository Settings → Secrets and variables → Actions
   - Direct link: https://github.com/ouankou/roup/settings/secrets/actions

2. Click **New repository secret**

3. Create secret:
   - **Name**: `CODEX_COMMENT_TOKEN` (exact name - workflow expects this)
   - **Secret**: Paste your PAT token

4. Click **Add secret**

### Step 3: Verify Setup

After adding the secret, the workflow will automatically use it. To test:

1. Open a pull request
2. Wait for CI to complete successfully
3. Check if `@codex review` comment appears (should be posted by your account, not `github-actions[bot]`)
4. Copilot should respond with a review shortly after

## Troubleshooting

### Comment posted by `github-actions[bot]` instead of you

- **Cause**: Secret not configured or wrong name
- **Fix**: Verify secret name is exactly `CODEX_COMMENT_TOKEN` (case-sensitive)

### Workflow fails with "Resource not accessible by integration"

- **Cause**: Token lacks required permissions
- **Fix**: Regenerate token with **Issues: Read and write** permission (PR comments use issues API)

### Copilot doesn't respond to comment

- **Cause**: Comment posted by bot account, not user
- **Fix**: Ensure PAT is configured correctly (see above)

### Token expired

- **Cause**: Fine-grained tokens expire after set duration
- **Fix**: Create a new token and update the `CODEX_COMMENT_TOKEN` secret

## Security Notes

- ✅ **Fine-grained tokens** are safer than classic PATs (scoped to specific repos and permissions)
- ✅ Tokens are stored as encrypted secrets (not visible in logs or workflow files)
- ✅ Workflow only runs on `workflow_run` trigger (can't be triggered by external PRs)
- ⚠️ Token grants write access to PRs/issues - protect the repository accordingly
- 🔄 Rotate tokens periodically (every 90 days recommended)

## Alternative: GitHub App (Advanced)

For organizations or stricter security requirements, you can create a GitHub App instead:

1. Create GitHub App with PR comment permissions
2. Install app on repository
3. Use app authentication in workflow
4. App comments appear as bot, but can be configured to appear as user

This is more complex but provides better audit logs and fine-grained control. See GitHub Actions documentation for details.

## Fallback Behavior

If `CODEX_COMMENT_TOKEN` is not configured:

- ❌ Workflow will **fail** with authentication error
- ℹ️ You can manually comment `@codex review` on PRs to trigger reviews
- 💡 To disable auto-review, delete or disable the `codex-after-ci.yml` workflow

## Current Status

- **Workflow**: `.github/workflows/codex-after-ci.yml`
- **Required secret**: `CODEX_COMMENT_TOKEN`
- **Triggers**: After successful CI on pull requests only
- **Posts**: `@codex review` comment on behalf of token owner
