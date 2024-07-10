# HDR Snipping Tool

A Windows Snipping Tool alternative that handles HDR content better.

## How it works

See [version 2 deep dive](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/technical-details.md).

## Installation

Requires Rust v1.79 or later.

`cargo install --git 'https://github.com/TrentShailer/hdr-snipping-tool.git' --locked`

## Usage

Run the application.

The application storage directory is found in `%APPDATA%\Roaming\trentshailer\hdr-snipping-tool\data\` or by using the 'Open Storage Directory' option on the tray icon.

To take a capture press the screenshot key (default PrintSrc). This key can be changed in the config, found in the `hdr-config.toml` in the storage directory. Valid values can be found in the [global-hotkey docs](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/enum.Code.html), restarting the app is required.

Once the capture window appears you can use the up and down arrows or the scrollwheel to control the brightness of the output. The left and right arrows control the contrast. You can hold shift to change the tonemapping values faster.

Use enter to save the entire capture, or click and drag the mouse to save a portion of the capture.

Use escape to cancel a capture.

After saving the file is in the storage directory and saved in your clipboard.

## Goals

- Allow control over the HDR to SDR tonemapping of screenshots.
- Provide similar features to the Windows Snipping Tool.

### Non-Goals

- Save HDR versions of screenshots. This use case is mostly covered by Windows Game Bar or other applications.
- Built-in uploading to image-sharing websites.

## Limitations

- Imperfect tonemapping, colours, contrast, and brightness will look different even on SDR content.
- Screenshots can only be of one display at a time.
- Windows only.
- No freeform mode or window mode.

## Windows Snipping Tool Comparison

### Windows Snipping Tool

![Windows example 1][win-example-1]

### HDR Snipping Tool

![HDR example 1][hdr-example-1]

[win-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/windows-snipping-tool-example.jpg "Windows Snipping Tool Screenshot of Cyberpunk 2077 with blown out highlights."
[hdr-example-1]: https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/hdr-snipping-tool-example.jpg "HDR Snipping Tool Screenshot showing the same content as the Windows Snipping Tool but without any blown out highlights."
