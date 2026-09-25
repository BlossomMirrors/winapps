#!/bin/bash
set -e

if rpm -q sangria >/dev/null 2>&1; then
    echo " *** Removing the sangria package *** "
    sudo dnf remove -y sangria
else
    echo " *** sangria is not installed, cleaning up leftovers anyway *** "
    sudo rm -f /usr/bin/sangria \
               /usr/share/applications/org.blossomos.sangria.desktop \
               /usr/share/metainfo/org.blossomos.sangria.metainfo.xml \
               /usr/share/icons/hicolor/scalable/apps/org.blossomos.sangria.svg
fi

rm -f ~/.local/share/applications/org.blossomos.sangria.desktop

echo " *** Dropping the .exe and .msi associations *** "
for type in \
    application/vnd.microsoft.portable-executable \
    application/x-msdownload \
    application/x-ms-dos-executable \
    application/x-dosexec \
    application/x-msi
do
    if [ "$(xdg-mime query default "$type" 2>/dev/null)" = "org.blossomos.sangria.desktop" ]; then
        xdg-mime default "" "$type" 2>/dev/null || true
    fi
done

update-desktop-database ~/.local/share/applications >/dev/null 2>&1 || true
rm -rf ~/.cache/ksycoca6* ~/.cache/icon-cache.kcache

echo
echo "The app library is untouched. To drop it as well:"
echo "  rm -rf ~/.local/share/sangria"
echo " *** Done *** "
