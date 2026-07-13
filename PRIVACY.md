# Privacy Policy — server-start

**Effective date:** 2026-07-13

server-start is a Windows system tray application for starting, stopping, and
restarting local development servers. It is developed as an open-source
project; the complete source code is available in this repository.

## Summary

**server-start does not collect, transmit, or share any data. Ever.**

The application has no telemetry, no analytics, no crash reporting, no
update checks, and no network communication of any kind. It contains no
networking libraries capable of making outbound connections — you can verify
this in [`Cargo.toml`](Cargo.toml) and the source code.

## Data the application stores locally

All data stays on your machine and is created only as part of the
application's core function:

- **Configuration file** — `%APPDATA%\server-start\config.toml`, written by
  you, containing the commands, working directories, and ports of the dev
  servers you choose to manage.
- **Log files (optional)** — when a server's output mode is set to
  `logfile`, that server's console output is written to a local log file so
  you can read it. You control this per server in the config.
- **Registry entry (optional)** — enabling "Start with Windows" writes a
  single value to your user registry Run key
  (`HKEY_CURRENT_USER\...\Windows\CurrentVersion\Run`) so Windows launches
  the app at logon. Disabling the option removes it.

None of this data leaves your computer, and none of it is accessible to the
project maintainers.

## Local port inspection

To detect dev servers started outside the app, server-start reads the
Windows TCP listener table (a local operating-system API) to check which
processes are listening on the ports named in *your* configuration. This is
a read-only, local lookup. Nothing is connected to, probed over the network,
or transmitted anywhere.

## Third parties

server-start integrates no third-party services. There are no accounts, no
sign-ins, no ads, and no data processors.

## Children's privacy

The application collects no data from anyone, including children.

## Changes to this policy

If a future version ever changes any of the above (for example, adding an
optional update check), this document will be updated in the same release,
and the change will be listed in the release notes. The version history of
this file is publicly auditable in the repository's git history.

## Contact

Questions about this policy can be raised by
[opening an issue](https://github.com/paperhurts/server-start/issues) on
this repository.
