# Lessons

## 2026-04-03
- User lost their terminal when testing "Restart Terminals" — the app kills all PowerShell/Terminal processes indiscriminately, including the one the user is working in. Any feature that kills processes needs careful scoping.
- User naturally tried `[[reader]]` instead of `[[server]]` in TOML config — expected the bracket name to be the project identifier (like naming PowerShell tabs). TOML array-of-tables syntax is unintuitive. Sample config and error messaging must be very explicit about this. Don't assume users know TOML conventions.
- Left `eprintln!` in `start_all`/`stop_all` after replacing it everywhere else — missed the bulk operation paths. When replacing an error pattern, grep for ALL occurrences, not just the obvious ones.
