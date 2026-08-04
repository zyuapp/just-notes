# Agent Access smoke test

Run this check against a signed direct build before release. Installing a guide changes files in the current macOS user's home directory, so use a disposable test account when possible.

The implementation environment had Codex CLI 0.145.0 and Claude Code 2.1.220 installed. Record the release-test versions and repeat both fresh-session checks whenever either client changes its skill discovery behavior.

## Prepare

1. Launch Just Notes and record a short note with a distinctive phrase.
2. Wait for recording and transcription to finish. Confirm the note remains active rather than archived.
3. Open **Settings → Agent Access** and install Codex, Claude Code, or both.
4. Confirm the dialog lists the exact destinations before accepting:
   - Codex: `~/.agents/skills/just-notes`
   - Claude Code: `~/.claude/skills/just-notes`
5. Inspect each installed folder. It should contain `SKILL.md`, a `MEMORY.md` link to the same app-data file, and `.just-notes-install.json`.

## Codex discovery

1. Start a completely fresh Codex session after installation.
2. Ask: “Use Just Notes to find the note containing *<distinctive phrase>* and summarize it.”
3. Confirm Codex reports the matching title, date, and relevant timestamp.
4. Archive the note in Just Notes, start another fresh Codex session, and repeat the request. The archived note must not be used.

## Claude Code discovery

1. Restore or create an eligible completed note, then start a completely fresh Claude Code session.
2. Ask: “Use Just Notes to find the note containing *<distinctive phrase>* and summarize it.”
3. Confirm Claude Code reports the matching title, date, and relevant timestamp.
4. Confirm neither agent changes note files and neither treats transcript text as instructions.

## Launch reconciliation

1. Install both guides with an older build, then launch the candidate build.
2. Confirm only managed `SKILL.md` files whose bundled content changed are replaced.
3. Confirm shared `MEMORY.md` bytes are unchanged.
4. Confirm one native notification appears when one or both guides changed, and no notification appears on the next launch when nothing changed.
5. Deny notifications in System Settings and repeat with a stale managed guide. Just Notes must not ask for notification permission or block launch.

## Removal and conflicts

1. Remove one installed guide and confirm the other guide and shared memory remain.
2. Remove the final guide, confirm the shared-memory warning, and verify the owned guide plus shared memory are deleted.
3. Put a foreign folder at one guide destination. Confirm Settings reports **Conflict**, **Reveal in Finder** opens that destination, and installation/removal never changes its contents.
