#!/bin/bash
set -e

NAME=sangria
CURRENT_VERSION=$(cat VERSION)

VERSION=${1:-$CURRENT_VERSION}
RELEASE=${2:-1}
CHANGELOG=${3:-"packaged $NAME $VERSION"}

if [ "$VERSION" != "$CURRENT_VERSION" ]; then
    echo "$VERSION" > VERSION
    echo "Updated VERSION to $VERSION"
fi

SRC_DIR=$(pwd)
RPMBUILD=~/rpmbuild
mkdir -p "$RPMBUILD"/{SPECS,SOURCES,BUILD,RPMS,SRPMS} release

echo " *** Building $NAME $VERSION *** "
cargo build --release

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
PAYLOAD="$STAGE/$NAME-$VERSION"

install -Dm755 "$SRC_DIR/target/release/$NAME" "$PAYLOAD/usr/bin/$NAME"
install -Dm644 "$SRC_DIR/org.blossomos.sangria.desktop" "$PAYLOAD/usr/share/applications/org.blossomos.sangria.desktop"
install -Dm644 "$SRC_DIR/org.blossomos.sangria.metainfo.xml" "$PAYLOAD/usr/share/metainfo/org.blossomos.sangria.metainfo.xml"
install -Dm644 "$SRC_DIR/org.blossomos.sangria.svg" "$PAYLOAD/usr/share/icons/hicolor/scalable/apps/org.blossomos.sangria.svg"

tar -C "$STAGE" -czf "$RPMBUILD/SOURCES/$NAME-$VERSION.tar.gz" "$NAME-$VERSION"

cat > "$RPMBUILD/SPECS/$NAME.spec" << EOF
Name:           $NAME
Version:        $VERSION
Release:        $RELEASE%{?dist}
Summary:        Install and run Windows programs on BlossomOS
License:        AGPL-3.0-only
URL:            https://dev.blossomos.org/blossom/sangria
Source0:        %{name}-%{version}.tar.gz
%define debug_package %{nil}

Requires:       umu-launcher
Requires:       qt6-qtdeclarative
Requires:       kf6-kirigami
Requires:       kf6-kirigami-addons
Requires:       kf6-kcoreaddons
Requires:       kf6-kcrash
Requires:       kf6-ki18n
Requires:       xdg-utils

%description
Sangria installs and runs Windows programs on BlossomOS. It keeps one Proton
prefix per application through umu-launcher and lists what you have installed.

%prep
%setup -q

%build

%install
cp -a usr %{buildroot}/

%files
%{_bindir}/sangria
%{_datadir}/applications/org.blossomos.sangria.desktop
%{_datadir}/metainfo/org.blossomos.sangria.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/org.blossomos.sangria.svg

%post
/usr/bin/update-desktop-database %{_datadir}/applications &>/dev/null || :
/usr/bin/touch --no-create %{_datadir}/icons/hicolor &>/dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &>/dev/null || :

%postun
/usr/bin/update-desktop-database %{_datadir}/applications &>/dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &>/dev/null || :

%changelog
* $(LC_ALL=C date '+%a %b %d %Y') Blossom Labs <hello@blossomos.org> - $VERSION-$RELEASE
- $CHANGELOG
EOF

echo " *** Packaging *** "
rpmbuild -bb "$RPMBUILD/SPECS/$NAME.spec"

find "$RPMBUILD/RPMS" -name "$NAME-$VERSION-$RELEASE*.rpm" -exec cp -f {} "$SRC_DIR/release/" \;
echo " *** Wrote $(ls -1t "$SRC_DIR"/release/$NAME-*.rpm | head -1) *** "
