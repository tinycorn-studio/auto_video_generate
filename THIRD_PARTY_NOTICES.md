# Third-party runtime notices

## FFmpeg engine

Wavely offers an optional, user-initiated FFmpeg 9.0.2 download. The FFmpeg executable is not included in the Wavely installer. Wavely verifies each pinned archive with SHA-256 before extracting only the `ffmpeg` executable into the app's local data directory. Temporary archives are removed after installation.

FFmpeg is a project of the FFmpeg developers. Source code for FFmpeg 9.0.2 is available from the [FFmpeg source repository](https://github.com/FFmpeg/FFmpeg/tree/n9.0.2). FFmpeg is generally licensed under LGPL 2.1 or later, with optional GPL components. The selected platform builds include `libx264`, which is GPL-licensed; see the [FFmpeg licensing guidance](https://www.ffmpeg.org/legal.html) and the [FFmpeg license details](https://ffmpeg.org/doxygen/trunk/md_LICENSE.html). Each upstream build also contains third-party libraries with their own licenses.

### Windows x64

- Provider: [Gyan FFmpeg builds](https://www.gyan.dev/ffmpeg/builds/), a Windows build provider linked from the [FFmpeg download page](https://www.ffmpeg.org/download.html).
- Archive: `ffmpeg-9.0.2-essentials_build.7z`.
- Archive SHA-256: `4705843ccaaf54257c16ad90f3e952ece33c17df964ecf7bfdbb0f49c7171077`.
- Published checksum: [`.7z.sha256`](https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.7z.sha256).
- FFmpeg source revision associated with the build: [946fcce07b](https://github.com/FFmpeg/FFmpeg/commit/946fcce07b).

### macOS Intel x64

- Provider: [Martin Riedl's FFmpeg build server](https://ffmpeg.martin-riedl.de/).
- Archive: `ffmpeg.zip`, FFmpeg 9.0.2 for macOS amd64.
- Archive SHA-256: `7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4`.
- Published checksum: [`ffmpeg.zip.sha256`](https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip.sha256).
- Build scripts and license notes: [provider source](https://git.martin-riedl.de/ffmpeg/build-script).

### macOS Apple Silicon arm64

- Provider: [Martin Riedl's FFmpeg build server](https://ffmpeg.martin-riedl.de/).
- Archive: `ffmpeg.zip`, FFmpeg 9.0.2 for macOS arm64.
- Archive SHA-256: `c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924`.
- Published checksum: [`ffmpeg.zip.sha256`](https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip.sha256).
- Build scripts and license notes: [provider source](https://git.martin-riedl.de/ffmpeg/build-script).

The pinned URLs, versions, and archive digests are maintained in `src-tauri/src/lib.rs`. Before changing a provider or version, verify the archive digest, required filters and encoders, architecture, source/build information, and licenses for the exact binary. The hashes protect against corruption or an unexpected archive; they do not replace review of the publisher or its build provenance.
