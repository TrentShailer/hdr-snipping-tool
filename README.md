# HDR Snipping Tool

A Windows Snipping Tool alternative that better handles HDR displays and applications.

## Usage

Launch the app, then use the Print Screen Key ([customizable in hdr-config.toml, requires relaunch](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/enum.Code.html)) to capture the screen the mouse is on.

This will open the screenshot window where you can tweak the contrast and brightness values to get the desired result.

You can then select the area you want to capture, or press Enter/Save and Close to capture everything. You can press Esc to close without saving.

The screenshot is copied to your clipboard and saved as a png next to the executable.

## Goals

- Get correctly tone-mapped SDR screenshots from an HDR display.
- Provide similar features to the Windows Snipping Tool.

### Non-Goals

- Save HDR screenshots. This use case is mostly covered by Windows Game Bar.
- Built-in uploading to image-sharing websites.

## Limitations

- Imperfect tone mapping: colours, contrast, and brightness will look different, even of SDR context.
- A single screenshot can only consist of one display.
- Windows only.
- No freeform mode or window mode.

## Screenshot Comparison

### Windows Snipping Tool

![Windows example 1][win-example-1]

### HDR Snipping Tool

![HDR example 1][hdr-example-1]

[win-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/main/media/window-snipping-tool-example-1.png?raw=true "Windows snipping tool example showing a screenshot from Death Standing with blown out highlights"

[hdr-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/main/media/hdr-snipping-tool-example-1.png?raw=true "HDR snipping tool example showing the same screenshot from Death Stranding without the blown out highlights"

## Acknowledgments

Most windows-rs code has been adapted from [robminkh/screenshot-rs](https://github.com/robmikh/screenshot-rs).
