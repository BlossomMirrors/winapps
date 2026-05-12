# Windows application support

A Flatpak alias for [org.winehq.Wine](https://flathub.org/apps/org.winehq.Wine) under the BlossomOS package namespace (`org.blossomos.winapps`). It inherits Wine's binaries and extensions wholesale, adding only BlossomOS-branded metadata and the icon on top.

## Files

| File | Purpose |
|------|---------|
| `org.blossomos.winapps.yaml` | Flatpak manifest |
| `org.blossomos.winapps.metainfo.xml` | AppStream metadata (alias of `org.winehq.Wine`) |
| `org.blossomos.winapps.svg` | Wine icon |

## Building

```sh
flatpak-builder --force-clean build-dir org.blossomos.winapps.yaml
```

Install locally for testing:

```sh
flatpak-builder --user --install --force-clean build-dir org.blossomos.winapps.yaml
```

## Requirements

- `flatpak-builder`
- `org.winehq.Wine//stable-25.08` and its extensions available from Flathub
