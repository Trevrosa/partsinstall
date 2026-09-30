# partsinstall

Install applications packaged in compressed parts.

Can create start menu shortcuts if on Windows.

Supported archive types:

- 7z
- zip
- rar
- tgz

\<DESTINATION\> argument can be set from environment variable: `pinst_destination`

see also [combinefiles](https://github.com/Trevrosa/combinefiles]

## Usage

```sh
Usage: partsinstall.exe [OPTIONS] <NAME> <DESTINATION>

Arguments:
  <NAME>         Name of application in working directory to install
  <DESTINATION>  Destination of install [env: pinst_destination=D:\Games]

Options:
  -w, --working-dir <WORKING_DIR>  Working directory the tool will use
  -F, --force-threaded             Force the use of threads to combine files
  -T, --threads <THREADS>          Number of threads to use to combine files, if in multithreaded mode
  -d, --dry-run                    Only combine files, do not install
  -S, --no-shortcut                Do not create start menu shortcuts
  -F, --no-flatten                 Do not flatten installed directories
  -y, --no-interaction             Assume answer that continues execution without interaction on all prompts
  -h, --help                       Print help
  -V, --version                    Print version
```
