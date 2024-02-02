# HDR Snipping Tool

A Windows Snipping Tool alternative that better handles HDR displays and applications.

## Usage

Launch the app, then use the Print Screen Key ([customizable](https://docs.rs/livesplit-hotkey/0.7.0/livesplit_hotkey/enum.KeyCode.html)) to capture the screen the mouse is on.

This will open the screenshot window where you can tweak the gamma and alpha values to get the desired result.

Gamma controls the contrast of the image, and alpha controls the brightness of the image.

You can then select the area you want to capture, or press Enter or the Save and Close button to capture everything. You can press Esc to code without saving.

The screenshot is copied to your clipboard and saved as a png next to the executable.

## Goals

- Get correctly tone-mapped SDR screenshots from an HDR display.
- Provide similar features to the Windows Snipping Tool.

### Non-Goals

- Save HDR screenshots. This use case is mostly covered by Windows Game Bar.
- Built-in uploading to image-sharing websites.

## Limitations

- Imperfect tone mapping, colors, contrast, and brightness will look different, even on SDR displays.
- A single screenshot can only consist of one display.
- Windows only.
- No freeform mode or window mode.
- The mouse cursor may be included in the screenshot.

## Screenshot Comparison

### Windows Snipping Tool

![Windows example 1][win-example-1]

### HDR Snipping Tool

![HDR example 1][hdr-example-1]

[win-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/main/media/window-snipping-tool-example-1.png?raw=true "Windows snipping tool example showing a screenshot from Death Standing with blown out highlights"

[hdr-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/main/media/hdr-snipping-tool-example-1.png?raw=true "HDR snipping tool example showing the same screenshot from Death Stranding without the blown out highlights"

## Acknowledgments

Most windows-rs code has been adapted from [robminkh/screenshot-rs](https://github.com/robmikh/screenshot-rs).
