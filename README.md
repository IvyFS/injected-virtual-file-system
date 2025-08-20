# IvyFS (Injected Virtual<sup>Y</sup> FileSystem)

IvyFS (Injected Virtual<sup>Y</sup> FileSystem) is a WIP Rust library for exposing virtual filesystems to target programs using process injection to patch low-level filesystem APIs.

# Acknowledgements

IvyFS draws heavily from [usvfs](https://github.com/ModOrganizer2/usvfs) in terms of goals and implementation, though does not aim for feature parity with usvfs. usvfs is available under the terms of the GPLv3 licence.

IvyFS uses the [Frida](https://www.frida.re/) reverse engineering kit for process injection and API patching. Frida is available under the terms of the wxWindows Library Licence, Version 3.1.
