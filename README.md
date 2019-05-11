## Waybar Sway Workspace Output Manager

This project watches for Sway workspace events and writes Waybar custom module
text to one output file per Sway workspace. This way I can configure a custom
appearance for each Sway workspace in Waybar by configuring each one as a
separate custom module. Run the workspace output manager as a daemon and then
configure Waybar to tail the workspace output files:

```
"custom/sway-workspaces-1": {
  "exec": "tail --sleep-interval 0.6 -F ~/.cache/waybar-sway-workspaces/1 2>/dev/null",
  "on-click": "swaymsg workspace 1"
},
"custom/sway-workspaces-2": {
  "exec": "tail --sleep-interval 0.6 -F ~/.cache/waybar-sway-workspace-output-manager/2 2>/dev/null",
  "on-click": "swaymsg workspace 2"
},
...
```

Configure custom workspace modules up to the max number of Sway workspaces you
want to have tracked. Workspaces that don't exist yet will be ignored.

A config file must also be installed to control the workspace module colors.
The config file should live at `~/.config/waybar-sway-workspace-output-manager/config`
and should look like this, swapping in whatever colors you want:

```yaml
---
version: 0.1
focused_foreground_color: "#EC5F67"
minimum_workspace_count: 5
background_colors:
  - "#C594C5"
  - "#6699CC"
  - "#5FB3B3"
  - "#99C794"
  - "#FAC863"
  - "#F99157"
```

### Developing

```
cargo run
```

### Usage

```
./waybar-sway-workspace-output-manager
```
