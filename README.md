## mneseq

![](https://github.com/yunmikun2/mneseq/workflows/Code%20Quality/badge.svg)

Generates mnemonic sequences from random bytes from stdin.

### How does it work

When encoding, the program uses 2048-word dictionary. Each word
encodes 11 bits of byte stream provided in stdin. In case the byte
stream is divisible by 11 without remainder, we add additional `of`
word before the last word. That way we can decode it back.

### Example

```sh
$ openssl rand 12 | mnesec
blur-snow-connect-guilt-valid-canyon-inside-square-anxiety
```

We can decode encoded sequence back with `-d` option.

```sh
$ echo blur-snow-connect-guilt-valid-canyon-inside-square-anxiety \
    | mnesec -d \
    | mnesec
blur-snow-connect-guilt-valid-canyon-inside-square-anxiety
```

### Installation

Run `cargo build --release` and put `./target/release/mnesec`
whereever you want.

#### Arch Linux

You can get the package from the AUR.

```
$ yay -S mnesec
```
