# Privacy — ShareCursor

> ShareCursor is designed to work without an account or cloud service. The app communicates directly between paired computers on the same local network.

Last updated: August 29, 2026.

## The ShareCursor app

- **No account:** the app has no registration or sign-in system.
- **No product telemetry:** the app does not send usage analytics, crash reports, clipboard contents, input events, or transferred files to the ShareCursor project.
- **Local communication:** keyboard and mouse events, clipboard data, and files travel directly between paired computers over the local network.
- **Local configuration:** settings and the shared passphrase are stored on each computer. Protect the local account and configuration file with normal operating-system permissions.
- **Local discovery:** mDNS can advertise and discover ShareCursor peers on the local network. The shared passphrase is still required to authenticate a connection.

## Website analytics

The public website uses [Umami Cloud](https://umami.is/privacy) for aggregate, cookieless analytics. ShareCursor records page views and clicks on release links so the documentation and download flow can be improved. It does not use advertising trackers or create an account profile.

Requests to GitHub, Mintlify, and other external sites are governed by those services' privacy policies after you follow an external link.

## Data retention and requests

The ShareCursor project does not operate an application backend that receives user content. Website analytics are retained in the configured Umami Cloud account according to Umami's service and retention settings.

For a privacy question, open a [GitHub issue](https://github.com/phun333/ShareCursor/issues/new/choose). Do not put passwords, private files, or other sensitive information in a public issue.

## Changes

This page will be updated if the app begins collecting data or the website's analytics setup changes. Material changes will be recorded in the project changelog.
