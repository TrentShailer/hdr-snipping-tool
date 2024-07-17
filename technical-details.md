# Technical Details

## High level

The goal of this application is to take a screenshot of some HDR content, then tonemap it to be SDR so that is can be shared and viewed on non-HDR displays.

This problem can be broken down into two main parts, getting the HDR capture, then tonemapping it to SDR.

I also do custom rendering for the UI as it allows me to easily reuse resources from the tonemapping and was a good point of learning. However, I will not go into details on the rendering here.

## In Depth

### Getting the capture

After starting, the application waits for the screenshot key which kicks off everything.

The first step is to work out what display the user wants the capture of. This is done by using the Windows API to enumerate the displays, then find the one that the mouse is inside.

Next we need to aquite a capture of the display, this is done using the Windows API's framepool. Using the framepool we request a capture with the display we found and the pixel format we want the capture in. As we want the an HDR capture, we request the pixel format `DXGI_FORMAT_R16G16B16A16_FLOAT` which stores each component of RGBA as a 16-bit float in Windows' scRGB format.

The scRGB format encodes the RGB components with a linear gamma, this means that `(2.0, 2.0, 2.0)` is twice as bright as `(1.0, 1.0, 1.0)`. In this color space the value `(1.0, 1.0, 1.0)` maps to D65 white with a brightness of 80 nits.

Microsoft has some good articles on [screen capture](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture) and how they [process color](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range) at a system level.

The Windows API stores this capture in a GPU local (no direct CPU access) DirectX Texture 2D. To get this capture onto the CPU, the data on the GPU is copied to a CPU accessable buffer which is then read into a 1-D array on the CPU. However, in order to achieve memory alignment on the GPU, Windows *may* add some blank data onto the end of each row of the capture. To remove this, each 'row' in the 1-D array is itterated through and only the data we want is copied to a new array.

So, now what we have is a big array of bytes on the CPU that contains our HDR capture. The next step is to get the capture back onto the GPU, but this time in a Vulkan buffer rather than a DirectX buffer. I chose Vulkan was as the GPU API for this project as it has just been something I have wanted to learn for a while and this is a really good excuse, really any GPU API would work as long as it can handle 16-bit floats and GPU compute.

Now that the capture is in a Vulkan buffer on the GPU, we are ready to get into the tonemapping side of things. The information on how this is tonemapped is accurate as of v3.0.0.

### Tonemapping the capture

#### Preamble

Working out how to tonemap the capture such that the SDR version looks as similar as possible to the HDR version has been the hardest part of this project. They way I have approached tonemapping has gone through a lot of interations from v1.0.0 through to v3.0.0 along with many experiments with new ideas that led nowhere. There are two main problems with tonemapping HDR to SDR, the first is that the process is naturally lossy so there is no one to one mapping that can be done, the second is working out the exact method and values to use.

The current tonemapping technique I have as of v3.0.0 is very accurate. It is able to tonemap SDR content on an HDR or SDR display to look exactly the same as Windows Snipping Tool. For HDR content it is either able to tonemap most of the capture accurately but will blow out the highlights a little, or it will make most of the capture darker and correctly tonemap the highlights. Switching between these options (and anything inbetween) is controlled by the user.

#### The Problem

![Annotated HDR Screenshot](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/HDR%20Snipping%20Tool%20Annotated.jpg)

Above is an annotated HDR screenshot, as shown the bright parts of this capture are one or two orders of magnitude 'brighter' than most of the content in the capture.

Mapping linearly between $y=[0,1]$ and $x=[0,1000]\ nits$ will allocate a lot of our output values to a small portion of the image, tonemapping this way produces the image below.

![Linear Tonemap](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/Linear%20Tonemap.jpg)

Finding a function that maps between $x=[0,...]\ and\ y=[0,1]$ in a way that preserves how the image apears to me. The preservation of how the image appears has been the main problem throughout this project.

#### Gamma Compression

Gamma compression is a tonemapping method that uses the function $y=x^\gamma$ where $0 < \gamma < 1$. This function with a $\gamma=0.5$ looks like:

![Gamma Compression Curve](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/gamma-compression-curve.png)

This allocates more of the output to the lower values which is exactly what we want. The function can be modified to make the $y=1$ point equal to $x_{max}$. The new function $y=(\frac{x}{x_{max}})^\gamma$ lets us control the $x\ where\ y=1$.

The next problem is to work out a value for $\gamma$. For v1 and v2, through trial and error I found that a gamma of $\frac{1}{2}$ produced pretty good results but had two issues.

The first issue was that it still wasn't quite right, especially when tonemapping SDR content, the colors $\gamma=\frac{1}{2}$ produced looked off compared to what I saw, and importantly what Windows Snipping Tool Produced.

The second issue was that using $x_{max}=input\ max$ caused the result to be exposed for the brightest part of the capture. This caused most of the image to be under exposed.

However, this function serves as the basis to the function that is currently used.

#### sRGB Gamma Compression

Further researching into how Windows handles color space led me to the [DXGI color spaces](https://learn.microsoft.com/en-us/windows/win32/api/dxgicommon/ne-dxgicommon-dxgi_color_space_type#constants). My non-HDR display reported a color space `DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709`. This color space is described as:

> This is intended to be implemented with sRGB gamma (linear segment + 2.4 power), which is approximately aligned with a gamma 2.2 curve.

Reading into sRGB led me to [sRGB Transformation](https://en.wikipedia.org/wiki/SRGB#Transformation), which details converting from linear RGB to sRGB using a peicewise function containing a linear segment then a gamma compression with a $\gamma=\frac{1}{2.4}$.

$$
C_{sRGB}=\begin{cases}
12.92C_{linear}, & C_{linear}\le0.0031308\\
1.055(C_{linear}^\frac{1}{2.4})-0.055, & C_{linear} > 0.0031308
\end{cases}
$$

As mentioned earlier, the colorspace Windows encodes the capture with is scRGB which is encoded with a linear gamma. This means that the values I have are linear RGB.

Trying this new function with an $x_{max}=input\ maximum$ produced the curve below:

![sRGB Curve](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/srgb.png)

Comparing Windows Snipping Tool and HDR Snipping Tool using this function shows that it is able to correctly handle SDR content.

![SDR Comparison](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/SDR%20Comparison.jpg)

This curve also worked incredibly well for HDR content, however, the exposure problem was still present.

#### The Exposure Problem

One method to approach the exposure problem is to reduce the $x_{max}$ to some value othat than the capture maximum value. Then clamp the result such that values over $x_{max}$ are set to $1$.

Here's a capture exposing for the input maximum (~1000 nits in this case).

![Brightest Pixel](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/HDR%20Snipping%20Tool%20Brightest%20Pixel.jpg)

And here's a capture that I manually tweaked the $x_{max}$ such that the bloom around the sun matched what appeared to me (500 nits in this case).

![Custom x max](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/HDR%20Snipping%20Tool%20Custom.jpg)

Then here's a capture that exposes for something known as the displays SDR reference white (280 nits in this case).

![SDR White](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/HDR%20Snipping%20Tool%20SDR%20White.jpg)

Of these three images the 500 nits curve target matches the bloom around the sun, while the core of the image is darker that it appears to me. The 280 nits curve target makes the core of the image look the same as it appears to me, however, the bloom around the sun is too large.

There appears to be no single value that can tonemap for all cases, so this is where I've made the application open up to user input.

The SDR reference white for the display, is the value that SDR white is mapped to, in my case 280 nits/3.5 in scRGB space. Using this value seems to tonemap most of the capture regardless of the game properly, but does overexpose the highlights a bit. As a result I use this value as the default when a capture is taken.

While is does overexpose a little, it significantly better than what Windows Snipping Tool does as shown:

![Windows Snipping Tool](https://github.com/TrentShailer/hdr-snipping-tool/blob/gpu-version/media/Windows%20Snipping%20Tool%20HDR.jpg)

The curve target can be switched between SDR White, the display's maximum luminance, the input maximum, and custom where the user can control exactly where they want the curve target.
