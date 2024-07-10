# Technical Details

After starting, the application waits for the screenshot key which kicks off everything.

The first step is to work out what display the user wants the capture of, this is done by using the Windows API to enumerate the displays, then find the one that the mouse is inside.

Next we need to aquite a capture of the display, this is done using the Windows API's framepool. Using the framepool we request a capture with the display we found and the pixel format we want the capture in. As we want the an HDR capture, we request the pixel format `DXGI_FORMAT_R16G16B16A16_FLOAT` which stores each component of RGBA as a 16-bit float.

Microsoft has some good articles on [screen capture](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture) and how they [process color](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range) at a system level.

The Windows API stores this capture in a GPU local (no direct CPU access) DirectX Texture 2D. To get this capture onto the CPU, the data on the GPU is copied to a CPU accessable buffer which is then read into a 1-D array on the CPU. However, in order to achieve memory alignment on the GPU, Windows *may* add some blank data onto the end of each row of the capture. To remove this, each 'row' in the 1-D array is itterated through and only the data we want is copied to a new array.

So, now what we have is a big array of bytes on the CPU that contains our HDR capture. The next step is to get the capture back onto the GPU, but this time in a Vulkan buffer rather than a DirectX buffer. I chose Vulkan was as the GPU API for this project as it has just been something I have wanted to learn for a while and this is a really good excuse, really any GPU API would work as long as it can handle 16-bit floats and GPU compute.

Now that the capture is in a Vulkan buffer on the GPU, we are ready to get into the tonemapping side of things. The information on how this is tonemapped is accurate as of v2.3.2, however I am looking into changing the process to get more accurate results so this may have changed.

The tonemapping method I use, is called [gamma correction](https://en.wikipedia.org/wiki/Gamma_correction), the equation is basically $sdr=\alpha(\frac{hdr}{hdr\ max})^\gamma$, where $hdr\ max$ is the largest value in the capture, $\alpha$ controls the brightness, and $\gamma$ controls the contrast. You can [view the curve here](https://www.desmos.com/calculator/b8o698ounb).

Therefore, before we can tonemap, we must first find the maximum value. This is done by using a GPU reduction, the specific algorithm I used is adapted from kernel 3 in this [Nvidia presention](https://developer.download.nvidia.com/assets/cuda/files/reduction.pdf), but has been modified to make use of subgroups, and translated to GLSL.

The algorithm finds the maximum of $1024 \times {subgroup\ size}$ values in a single invocation. So $\frac{n\ input\ values}{1024 \times {subgroup\ size}}$ invocations need to be done on the first pass to cover the entire input. However, each invocation produces a maximum value, so the reduction needs to be performed again on the output of the first reduction and so on until only a single value remains.

Now that we have the maximum and the $\gamma$ is obtained from the settings file, all we need is the $\alpha$ value. With the $\alpha$ set to $1.0$ the tonemapper will tonemap for the brightest part of the capture. However, the brightest part of the capture is often extremely bright, such as the sun, this underexposes the output and produces a dark result. Therefore, the $\alpha$ value needs to be adjusted to overexpose enough such that the core content is exposed correctly. This step is where Windows Snipping Tool gets it wrong, they over expose or just clip everything above a certain value and cause all the highlights to take over the image.

I obtain the $\alpha$ value by setting a desired mid-point of the tonemapping curve, the $hdr$ value where $sdr=0.5$, as the mid-point of the tonemapping curve is about where the main content should be. However, this just shifts the value problem down to having a correct mid-point value. Through my testing I found that a mid-point value of $~0.875$ produced a good alpha for game and regular HDR windows content. This value is picked mostly from trial and error and is definitly something I want to improve and back up with a reason in the future.

The $\alpha$ is obtained by reversing the tonemapping equation such that it solves for $\alpha=\frac{sdr}{(\frac{hdr}{hdr\ max})^\gamma}$ with an $sdr=0.5$, $hdr=midpoint=0.875$, $\gamma$ from the settings, and ${hdr\ max}$ from the GPU reduction.

Finally, the image can be tonemapped by passing each RGB value through the tonemapping equation with the parameters we have found and writing the result to an output image to be displayed to the user. From here the user can tweak the $\alpha$ and $\gamma$ with changes happening in real time to get the output they want should the default values be incorrect. Then they can select an area of the image they want to save and it is written to their clipboard along with a file.

Alongside all the app logic, I also built a custom renderer for the application as it let me share resources with the tonemapper. Making this custom renderer was a challenge on it's own, especially for rendering text, but isn't something that I will go into further here.

The tonemapper produces really good results but isn't 100% accurate, this is evident when compared to Windows Snipping Tool when capturing non-HDR content on an HDR display. This comes down to either incorrect input parameters for the tonemapper and/or missing a step as I don't fully understand how everything works on Window's end at this point. Improving the tonemapper accuracy is something I want to work on, however, it's current state is almost perfect and does everything I need it to.
