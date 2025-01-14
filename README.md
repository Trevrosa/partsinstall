# partsinstall

Install applications packaged in compressed parts.

Can create start menu shortcuts if on Windows.

Supported archive types:

- 7z
- zip
- rar
- tgz

\<DESTINATION\> argument can be set from environment variable: `pinst_destination`

## Usage

```sh
Usage: partsinstall.exe [OPTIONS] <NAME> <DESTINATION>

Arguments:
  <NAME>         Name of or path to application to install
  <DESTINATION>  Destination of install [env: pinst_destination=]

Options:
  -S, --no-shortcut     Do not create start menu shortcuts
  -F, --no-flatten      Do not flatten installed directories
  -y, --no-interaction  Assume answer that continues execution without interaction on all prompts
  -h, --help            Print help
  -V, --version         Print version
```
