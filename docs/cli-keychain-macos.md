# Where the CLI keeps its credentials

## By default: a file

`~/.config/portaki/credentials.json`, mode `0600`, in a `0700` directory.

- **Outside the repository** — a secrets file inside a working tree eventually gets committed, or
  swept up by a `git add -A`.
- **Written by atomic rename** — an interruption never leaves a truncated file, which would force
  you to log in again for an unrelated reason.
- **Never printed**, and `portaki logout` wipes it entirely.

Set `PORTAKI_CREDENTIALS_FILE` to change the path.

## What this file does not do

**It is not encrypted.** Two ideas keep coming up, and neither holds:

- *hash it* — impossible: a token has to be replayed as-is, and a digest cannot be replayed. What
  you would hash would no longer authenticate anything;
- *encrypt it* — that needs a key, and the key has to be stored somewhere on the same machine. The
  only right place for it is the keychain, the very thing we just walked away from. Scrambling the
  contents without a protected key protects nothing, it only looks like protection.

On a single-user machine, the protection that counts is the file permissions, and those are in
place. The access token lives fifteen minutes; the refresh token, seven days, and `portaki logout`
revokes it.

## Why it is no longer the keychain

The keychain was the right choice on paper: encrypted at rest, locked with the session. Its real
cost on macOS won out.

The keychain does not authorise *a file at a path*, it authorises a **code identity**. A rebuilt
binary does not have the same one: every `cargo install` produces a program it has never seen, and
"Always Allow" only ever covers that day's hash. A development loop that rebuilds therefore asks
for the login password on every pass.

A safeguard you run into a hundred times a day ends up being worked around — this one already was,
through the environment variable.

## Going back to the keychain

```sh
export PORTAKI_CREDENTIALS=keychain
```

Nothing was removed. If you do this on macOS and the dialog wears you down, the real answer is a
stable signing identity:

1. **Keychain Access ▸ Certificate Assistant ▸ Create a Certificate…**
   Name `Portaki Dev`, identity type `Self Signed Root`, certificate type `Code Signing`.
2. Install with `./scripts/install-cli.sh`, which signs the binary after installing it.

Check that the signature rests on the identity and not on a hash:

```sh
codesign -d -r- "$(command -v portaki)"
```

A requirement that mentions a `cdhash` means the signature stayed ad hoc, and the dialog will be
back.

## In CI

`PORTAKI_DEV_TOKEN` short-circuits everything: no file, no keychain. A build agent has neither, and
this variable wins over the rest — including over OIDC.
