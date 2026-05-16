# Lessons

## 2026-04-03
- User lost their terminal when testing "Restart Terminals" — the app kills all PowerShell/Terminal processes indiscriminately, including the one the user is working in. Any feature that kills processes needs careful scoping.
- User naturally tried `[[reader]]` instead of `[[server]]` in TOML config — expected the bracket name to be the project identifier (like naming PowerShell tabs). TOML array-of-tables syntax is unintuitive. Sample config and error messaging must be very explicit about this. Don't assume users know TOML conventions.
- Left `eprintln!` in `start_all`/`stop_all` after replacing it everywhere else — missed the bulk operation paths. When replacing an error pattern, grep for ALL occurrences, not just the obvious ones.

## 2026-05-14
- Stashed pre-existing user edits to PROJECT_STATUS.md, popped them after branching, then edited the same section without first reading what was there — silently overwriting the user's edits. Recovery was possible from the dropped stash commit via `git fsck --unreachable`, and the overwritten content happened to match my edits, but this was luck. **Rule: after `git stash pop`, re-read any file before editing it.** "Never overwrite without backup" applies to working-tree edits too, not just file deletions.
- My build.rs comment claimed `winresource::compile()` was a no-op on non-Windows targets. It is not — it returns an error and my `.expect()` would have panicked on Linux CI. **Rule: don't write comments asserting third-party-crate behavior without reading the crate's source for that specific code path.** Documentation is often aspirational; source is truth.
