# Codex auto-review setup

The `codex-after-ci.yml` workflow mentions `@codex review` after CI succeeds. It
needs a fine-grained personal access token (PAT) because Copilot ignores
comments from `github-actions[bot]`.

## Configure the token

1. Create a fine-grained PAT at <https://github.com/settings/personal-access-tokens/new>.
   - Scope it to the `ouankou/roup` repository.
   - Grant **Pull requests** and **Issues** read/write permissions. `Contents` read-only is optional.
2. Store the token as `CODEX_COMMENT_TOKEN` under **Settings → Secrets and variables → Actions**.

After saving the secret, new workflow runs will post comments from the token
owner. Open a pull request, wait for CI to finish, and confirm the comment shows
your account instead of the GitHub Actions bot.

## Troubleshooting

- Comment still appears from the bot → secret missing or misnamed.
- Workflow fails with “Resource not accessible by integration” → token lacks
  Issues permissions.
- Copilot does not answer → regenerate the token and update the secret (tokens
  expire on the date you selected).

Fine-grained PATs stay encrypted in Actions and can be rotated at any time. If
you prefer not to automate this flow, disable the workflow or comment
`@codex review` manually when needed.
