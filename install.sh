#!/bin/env bash
set -e

SRC_DIR=$(pwd)

if [ "$1" = "remove" ]; then
    echo "Use ./uninstall.sh to remove Sangria"
    exit 1
fi

echo " *** Unlocking rpm-ostree so the changes persist after restart *** "
sudo rpm-ostree usroverlay | true || true

sudo dnf install -y umu-launcher kf6-kirigami kf6-kirigami-addons

# build in a toolbox so compilers and devel packages never touch the system image
TOOLBOX=sangria-build
FEDORA=$(. /etc/os-release && echo "$VERSION_ID")

if ! toolbox list --containers | grep -qw "$TOOLBOX"; then
    echo " *** Creating the $TOOLBOX toolbox on Fedora $FEDORA *** "
    toolbox create -y -r "$FEDORA" "$TOOLBOX"
fi

echo " *** Installing build dependencies in the toolbox *** "
toolbox run -c "$TOOLBOX" sudo dnf install -y cargo rust rpm-build gcc-c++ \
    qt6-qtbase-devel qt6-qtdeclarative-devel \
    kf6-kcoreaddons-devel kf6-kcrash-devel kf6-ki18n-devel \
    kf6-kiconthemes-devel kf6-kconfigwidgets-devel kf6-kcmutils-devel

# ki18n 6.31 links its qml library to qt qml but only loads qt qml for static builds so the rust build gets an unknown library name
toolbox run -c "$TOOLBOX" sudo sed -i 's/^if (ON AND NOT ON)$/if (ON)/' /usr/lib64/cmake/KF6I18n/KF6I18nConfig.cmake

echo " *** Building *** "
# the shared home puts distrobox shims like ld in the path so keep only the toolbox system tools
toolbox run -c "$TOOLBOX" env PATH=/usr/local/bin:/usr/bin bash -c 'cd "$1" && ./release.sh' _ "$SRC_DIR"

RPM=$(ls -1t "$SRC_DIR"/release/sangria-*.rpm 2>/dev/null | head -1)
if [ -z "$RPM" ]; then
    echo "no RPM produced by release.sh, see build output above for the error"
    exit 1
fi

echo " *** Installing $(basename "$RPM") *** "
if rpm -q sangria >/dev/null 2>&1; then
    if [ "$(rpm -q --qf '%{VERSION}-%{RELEASE}' sangria)" = "$(rpm -qp --qf '%{VERSION}-%{RELEASE}' "$RPM")" ]; then
        sudo dnf reinstall -y "$RPM"
    else
        sudo rpm -Uvh --replacefiles "$RPM"
    fi
else
    sudo dnf install -y "$RPM"
fi

if [ -f ~/.local/share/applications/org.blossomos.sangria.desktop ]; then
    echo " *** Removing stale user copy of the desktop entry *** "
    rm -f ~/.local/share/applications/org.blossomos.sangria.desktop
fi

echo " *** Refreshing desktop and icon caches *** "
update-desktop-database ~/.local/share/applications >/dev/null 2>&1 || true
rm -rf ~/.cache/ksycoca6* ~/.cache/icon-cache.kcache

echo " *** Registering Sangria as the handler for .exe and .msi *** "
for type in \
    application/vnd.microsoft.portable-executable \
    application/x-msdownload \
    application/x-ms-dos-executable \
    application/x-dosexec \
    application/x-msi
do
    xdg-mime default org.blossomos.sangria.desktop "$type"
    printf '  %-52s -> %s\n' "$type" "$(xdg-mime query default "$type")"
done

echo " *** Done *** "
