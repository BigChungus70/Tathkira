#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
  for config_dir in \
    /home/*/.config/com.tathkira.desktop \
    /home/*/.config/com.tathkira.app \
    /root/.config/com.tathkira.desktop \
    /root/.config/com.tathkira.app
  do
    if [ -d "$config_dir" ]; then
      rm -rf "$config_dir"
    fi
  done

  for autostart_file in \
    /home/*/.config/autostart/Tathkira.desktop \
    /root/.config/autostart/Tathkira.desktop
  do
    if [ -f "$autostart_file" ]; then
      rm -f "$autostart_file"
    fi
  done
fi

exit 0
