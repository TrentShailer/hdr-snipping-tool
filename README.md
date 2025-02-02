# HDR Snipping Tool

A Windows Snipping Tool alternative that doesn't overexpose HDR content.

> [!CAUTION] 
> ### This is a personal project
>
> Maintanance, bug fixes, new features, and support will only be provided when/if I feel like it.
>

## Installation

Requires Rust v1.84 or later.

`cargo install --git 'https://github.com/TrentShailer/hdr-snipping-tool.git' --locked`

## Usage

* Run the application.
* Take a screenshot using the screenshot key (default `PrintScr`).
  * This key can be changed in the config located in `%APPDATA%\Roaming\HDR Snipping Tool` or by using the `Open Config Directory` option in the tray icon.
  * Valid keys are found in the [global-hotkey docs](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/enum.Code.html).
* Use the `Enter` key to save the entire screenshot, or click and drag the mouse to save a portion of the screenshot.
* Use the `Escape` key to cancel a screenshot.
* After saving the file is saved to `Pictures\Screenshots` and copied to your clipboard.

## Goals

* Take screenshots of HDR content that isn't overexposed.
* Provide similar basic features to Windows Snipping Tool.
  * Cropping to a user-selected area of the screen.
  * Copying to clipboard.

### Non-Goals

* Save HDR versions of screenshots. This use case is mostly covered by Windows Game Bar or other applications.
* Built-in uploading to image-sharing websites.

## Limitations

* The screenshots will slightly clip highlights to preserve screenshot details.
* Screenshots can only be of one monitor at a time.
* Windows only.

## Screenshot Comparison

### Windows Snipping Tool

![Windows example 1][win-example-1]

### HDR Snipping Tool

![HDR example 1][hdr-example-1]

[win-example-1]: media/windows-snipping-tool-example.jpg "Windows Snipping Tool Screenshot of Death Standing that is overexposed hiding a mountain."
[hdr-example-1]: media/hdr-snipping-tool-example.jpg "HDR Snipping Tool Screenshot showing the same content as the Windows Snipping Tool but is not overexposed and shows the mountain."

## Thank You

* [ash](https://github.com/ash-rs/ash)
* [winit](https://github.com/rust-windowing/winit)
* [windows-rs](https://github.com/microsoft/windows-rs)
* [Vulkan-Profiles](https://github.com/KhronosGroup/Vulkan-Profiles)
