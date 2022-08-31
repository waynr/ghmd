# ghmd - Gotta Have My Dots

`badm` is "But Another [Dotfiles](https://en.wikipedia.org/wiki/Hidden_file_and_hidden_directory) Manager".

## How it works

`ghmd` started off as an attempt to add support for multiple dotfile repos to [badm](https://github.com/jakeschurch/badm). Things got a little out of hand, so I have unofficially forked that repo. Very little of the original code or command line interface remains.

It's essentially a glorified bespoke/fancy `ln` tool. It moves files/directories from path A to path B then creates a symbolic link at path A pointing to path B.

Reasons you should use this tool:
* <crickets>

Reasons you shouldn't use this tool:

* I adapted it for my own use case, not yours.
* It's not written in your favorite programming language.
* I deleted all the tests that are in [badm](https://github.com/jakeschurch/badm).
* You have better things to do than sit in front of your computer moping over
  dot files lost to the great trash bin in the sky.

Dotfiles are tracked in `$HOME/.config/ghmd/config.toml` to enable all known dotfiles to be deployed in one swift command line call.

### Quick Demo

TODO (more like TODONT)

## Commands

* `badm stow <symlink_dir> <dotfiles_dir> <file>...`
  * Move each specified `<file>...` from `<symlink_dir>` to `<dotfiles_dir>`.
    * Fail `<symlink_dir>` is not a parent path of any `<file>...` paths.
  * Create a symlink pointing to the new location in `<dotfiles_dir>` from the old location in `<symlink_dir>`.
* `badm deploy <file>...`
  * Deploy symlinks to each `<file>...` to the configured `<symlink_dir>`.
* `badm restore <dotfiles_dir> <file>...`
  * Restore each specified `<file>...` from the specified `<dotfiles_dir>` to the configured `<symlink_dir>`.

## Roadmap

There is no roadmap. Only what you see before you.

## Contributing

Pull requests, issues/feature requests, and bug reports will be approached with trepidation and a mild sense of exhaustion!

## Similar Projects

- [badm](https://github.com/jakeschurch/badm)
- [GNU Stow](https://www.gnu.org/software/stow/)
- [YADM](https://www.yadm.io)

## License

This project is made available under the MIT license. See the [LICENSE](LICENSE) file for more information.
