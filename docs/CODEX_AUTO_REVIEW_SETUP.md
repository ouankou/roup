# Codex Auto-Review Setup

The `codex-after-ci.yml` workflow posts `@codex review` after successful CI runs. Because GitHub Apps cannot trigger Copilot, the
workflow needs a Personal Access Token (PAT) that belongs to a maintainer.

1. Create a fine-grained PAT at <https://github.com/settings/personal-access-tokens/new> with repository access to `ouankou/roup`
   and pull-request/issue read-write permissions.
2. Add the token as the `CODEX_COMMENT_TOKEN` secret under *Settings → Secrets and variables → Actions*.
3. Open a pull request and wait for CI; the comment should appear from your account and Copilot will respond shortly after.

If the workflow falls back to `github-actions[bot]`, double-check the secret name or regenerate the token with the required
permissions. Tokens expire according to the duration you choose—rotate them periodically.
