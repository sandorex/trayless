## trayless

The project started as small utility to use with dmenu/rofi for interacting with tray indicators but turned into a GUI menu like rofi

The `trayless` does the following:
- Act as host tray (keeps track of tray icons)
- Allows you to get information about them and trigger events like scroll, click and context menu items

The `trayless-gtk` gives you a GUI interface to interact with tray indicator and the context menu

### Build

#### CLI
The cli (`trayless`) requires no dependenices outside cargo ones

#### GUI
The gui (`trayless-gtk`) requires `gtk4` and `gtk4-layer-shell` libraries
