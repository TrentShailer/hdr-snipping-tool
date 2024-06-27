# Version 2 Deep Dive

Version 2 has been a large rewrite with a few goals:

- Perform tonemapping on your GPU.
- Balance resource usage and speed.
- Give me more control over depedencies.

## Tonemapping on GPU

Version 1 handled all tonemapping by processing the capture in parallel on the CPU. While it worked supprisingly well, this sort of parallel data processing is exactly what the GPU is made for, along with this, processing on the GPU has been a long standing area of programming I have wanted to learn.

Doing the processing on the GPU would help in a few main ways:

1. I could skip the conversion of the raw byte buffer of 16-bit floating point values Windows into 32-bit floating point values, this added time between the screenshot being pressed and the presentation of the application.
2. Shift memory usage from system memory to GPU memory.
3. Easier integration of local tonemapping techniques should I want to try any.
4. Speed up tonemapping, so changing tonemapping settings doesn't hit the CPU as hard or cause lag.

Deciding on what graphics API to use took some time and experimenting, the two options I considered was [wgpu](https://github.com/gfx-rs/wgpu) and [vulkano](https://github.com/vulkano-rs/vulkano).

WGPU is much more widely used in the Rust community and supports many different backends, however, due to being an abstraction over multiple underlying graphics APIs it has a few drawbacks. The main drawback was at the time, it had no support for pipeline overrideable constants or subgroup operations (these have been implemented as of WGPU v0.20.0). The other issue I had was that it is still a very new API based on the very new WebGPU specification, as a large part of the application would be running on the GPU this would add a lot of burden if I wanted to keep my application up-to-date with it's changes.

Vulkano is also not without it's own issues either, due to being less widely used there are fewer people working on it resulting in slower updates and fixes. Along with this, it doesn't support many Vulkan extensions which proved challenging when I wanted to view the GPU memory impact I was having. However, the Vulkan API itself is very solid and there are loads of resources available for working with Vulkan making learning it a lot easier.

I ultimately decided on Vulkano which for the mostpart was great but there were definitly frustrations with missing Vulkan extensions.

### Tradeoffs

Because the format of the capture I get from Windows is in 16-bit floating point, this means that this version will only work if your GPU/IGPU has support for 16-bit floating points and 16-bit floating point subgroup operations ([Required features and extensions](https://github.com/TrentShailer/hdr-snipping-tool/blob/version-2/crates/vulkan-instance/src/vulkan_instance/requirements.rs)). While this does mean some systems just won't be able to run version 2, running an HDR display and HDR games generally requires a decent GPU so it is unlikely to be a problem, and version 1 is still available.

While this does result in less CPU memory usage, it does eat a bit of GPU memory while editing an screenshot, but more on this in Resource Usage.

## Resource Usage

Because the application does nothing except listen for a screenshot key most of the time, an important goal for the rewrite was to consider resource usage and resource usage at idle throughout the process.

This involved having as few copies of the the HDR and tonemapped SDR capture as possible duing an active capture, this means reusing/sharing buffers between aspects. Along with trying to ensure that as many allocations as possible are freed after closing or saving a capture.

I still think there are improvments to be made to CPU memory but overall I am very happy with the changes.

*Resource usage and latency are hard to pin down as they will change depending on the display and hardware running the application.*

***This is not rigorous testing**, it is only the values reported by Windows at a single time on a single day. This should only serve as a point of comparison between the versions for my specific system.*

### Methodology

While not vigorous I still followed a methodology.

Version 2 tests were performed on commit [2cb3318330a777ea8d6d40cad31af8e35abced6a](https://github.com/TrentShailer/hdr-snipping-tool/tree/2cb3318330a777ea8d6d40cad31af8e35abced6a).

Version 1 tests were performed on commit [c68148c99ea093348c821f97697686ddd14c0692](https://github.com/TrentShailer/hdr-snipping-tool/tree/c68148c99ea093348c821f97697686ddd14c0692)

The tests were done on my system with an RTX 3060 12GB GPU, Ryzen 7 5700X CPU, and 32 GB Memory on my 3440x1440 monitor.

- Launch the application, wait for values to settle, record 'Hidden, Fresh Launch'.
- Press screenshot key to capture my monitor, wait for values to settle, record 'Active Capture'.
- Rapidly change tonemapping settings (hold buttons on v1, constantly scroll on v2), record a rough average of the value Windows reports.
- Cancel the capture, wait for values to settle, record 'Hidden, cancelled capture'.
- Close and reopen the application, take and save a fullscreen capture, wait for values to settle, record 'Hidden, saved capture'.

### Reference values

My display is 3440x1440, this means:

- A single copy of an HDR capture (8 bytes per pixel) consumes 39,628,800 bytes.
- A single copy of the SDR capture (4 bytes per pixel) consumes 19,814,400 bytes.

### System Memory

| Application State                | version 1 usage | version 2 usage |
|----------------------------------|-----------------|-----------------|
| Hidden, Fresh Launch             | ~27 MB          | ~37 MB          |
| Active Capture                   | ~150 MB         | ~53 MB          |
| Active Capture Changing Settings | ~160 MB         | ~53 MB          |
| Hidden, cancelled capture        | ~58 MB          | ~53 MB          |
| Hidden, saved capture            | ~71 MB          | ~71 MB          |

Increased idle memory usage after cancelling/saving a capture for the first time is to be expected. This is likely due to allocators holding onto memory allocations to reuse and storing the capture in the clipboard on save.

I've tested to ensure memory usage does not continue to increase significantly after the initial increase.

### CPU Load

| Application State                | version 1 usage | version 2 usage |
|----------------------------------|-----------------|-----------------|
| Fresh Launch                     | 0%              | 0%              |
| Active Capture                   | ~10%            | ~1%             |
| Active Capture Changing Settings | ~50%            | ~1%             |
| Hidden, cancelled capture        | 0%              | 0%              |
| Hidden, saved capture            | 0%              | 0%              |

Huge improvements to CPU load in version 2, especially when changing settings.

### GPU Load/Memory (version 2)

I can't easily directly measure my impact, however, I can *roughly* work out what my memory usage *should* be.

The swapchain usage will vary from system to system, I use your GPUs min number of images (usually 1 or 2) + 1 images in the swapchain and whatever the first surface format in the list is.

When a capture is taken there will be an initial bump in system and GPU memory usage while one-time operations are performed, I've tried to ensure memory allocated to these is freed properly.

On an active capture, there is one HDR capture stored in the GPU and one SDR capture stored in the GPU. The memory allocated to store these are done as dedicated allocations so *should* be freed when the application is hidden but vendors may do different things.

There are various other smaller allocations are also made but should have relatively minimal impact.

### Tradeoffs

In order to minimise resource usage at idle, a lot of the large memory allocations are only made when a capture is taken, then freed when you are done with the capture. However, large memory allocations take time and result in more latency while the allocations and writes are made, but more on this next.

## Latency

The latency of the application is what I use to refer to the time between the application recieving an event for the screenshot key being pressed and when the application has a tonemapped copy of the capture and is ready to present the editing screen to the user. It's more or less the time between the key press and the application appearing to the user.

Keeping this latency low is very important to improve how resposive the application feels to use.

In version 2 there are two main contributers to the latency:

- Getting the capture from windows.
- Performing one-time operation to prepare for tonemapping.

During development, on an unoptimised built I was seeing a total latency between 60-90ms for my 3440x1440 display.

This latency felt very resposive for me, and would likely improve in an optimised build. While it is likely possible to reduce the latency further, it would add a lot of complexity. Along with this, the latency fluctuates quite a lot and depends heavily on the size of the display. But this might be an area I return to later.

### Getting the capture from Windows

This operation adds some unavoidable latency while I wait on Windows, however, there are two major contributers that I have control over.

1. Copying the capture to the system memory.
2. Trimming additional padding Windows adds to the capture.

Windows stores the captures in a DirectX buffer, while I need them in a Vulkan buffer. The easiest way to do this is to copy the data out of the DirectX buffer to the system memory, then writing that data into a Vulkan buffer. However, this means the capture has to go from the GPU to the CPU then back to the GPU. I think it *may* be possible to directly copy the data over, however, it would likely involve some more unsafe memory operations and introduce more edge cases.

The next contributer is that Windows sometimes adds some blank data onto the end of each row in the capture to get the memory to align in the GPU. To remove this, I copy the data row-by-row rather than the entire block at once. I don't currently have any ideas on how I could improve this, and the copy operation is relatively fast.

During development, using an unoptimised built I was seeing 20-40ms of latency to get the capture from Windows.

### One-time operations

The main one-time operations:

- Writing to the buffer for thw HDR Capture.
- Finding the brightest component of the capture to use during tonemapping.

Writing to the HDR capture buffer was initially very slow, which pushed me to experiment with various ways of writing to the buffer, I eventually found that setting up a staging buffer on the CPU and using the `copy_from_slice` function was the fastest way to write the data, then copy the staging buffer to a GPU only buffer was the fastest way to get the data onto the GPU.

Finding the brightest component of the capture is not strictly neccecary, but it means that I can have an alpha value of 1.0 to always tonemap for the brightest part of the capture. This makes the inital tonemap of the capture pretty close to where you usually want it (if a little dark, as it revolves around the brightest part). However, efficently finding the maximum value in a large dataset is quite complicated.

The algorithm I used to find the maximum value on the GPU is adapted from kernel 4 from [Mark Harris' NVidia Webinar on Parallel Reduction in Cuda](https://developer.download.nvidia.com/assets/cuda/files/reduction.pdf). While this presentation is quite old after I was able to translate the CUDA to glsl I saw great results, then I played around with it for a bit and was able to take advantage of subgroups to improve my results further. While kernel 4 is not the most optimised version in the webinar, I was not able to get later versions working reliably due to I believe some CUDA behaviour that I missed implementing in my glsl/Vulkan version.

Overall this algorithm is very fast as I was able to get ~2-3ms times for the actual operation (ignoring intial setup and final read back) on the ~19.8 million values a capture of my display had.

Overall during development, the one-time operations contributed ~20-30ms of latency.

## Dependency Control

Version 1 handled all GUI by using the [imgui-rs](https://github.com/imgui-rs/imgui-rs) crate. While this crate works very well, I relied on their `imgui-winit-support` and `imgui-glow-renderer` crates, which locks me into whatever version of `winit` they support and restricts my usage of GPU APIs.

As `winit` receives very consistant updates and has recently had a major overhaul in public API in `v0.30`, I want to be able to stay up to date with their changes. However, this would require me to either make my own `imgui-winit-support` if I wanted to continue using `imgui` .

The next issue was around `imgui-glow-renderer`, which uses OpenGL. As I wanted to use the GPU to perform tonemapping operations this would either lock me into using OpenGL or have my application use two different graphics APIs.

This left me with a few options:

1. I could continue to use `imgui-rs` with my own `imgui-winit-support` and a custom imgui renderer with whatever graphics API I chose.
2. I could continue to use `imgui-rs` with my own `imgui-winit-support` and use their `imgui-glow-renderer` separate from the graphics API I chose.
3. I could drop `imgui` and create my own renderer integrated with my tonemapping library.

For me, option 2 seemed far too wasteful in running two graphics API instances, and in copying the tonemapped texture from the graphics API I chose over to OpenGL to render. Option 1 was definitly on the cards for a while, however, writing my own `imgui` renderer was one step too advanced for my current skill level and I found that using the buttons to control the tonemapping settings felt off. The interactive GUI and text rendering are the two reasons why I wanted to continue using `imgui`, however, with not liking the interative part of it I decided to try making my own non-imgui renderer to test the feasability.

### Tradeoffs

Writing a custom renderer means that any maintanance is a lot more complicated but does provide a good learning point for how renderers work.
