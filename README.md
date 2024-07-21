# Wisphaven

[Website](https://wisphaven.com/)

Wisphaven is a voxel village defense game that is currently in development.

This project has mostly been abandoned, but I'm making one more update to quickly turn it into a somewhat fun game before moving on.

Written in Rust using the Bevy game engine.

## How to run

Clone the project, have cargo installed, and run

`cargo run --release`

in the same folder as this file.

If you are developing, you can create `.cargo/config.toml` with the following to improve compile times:

```
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=/usr/bin/mold", "-Zshare-generics=y",]
```

Cranelift does not work very well with this project at the time of writing.

## Features

- Infinite, procedurally generated world
- Saving/loading terrain
- Scuffed multiplayer
- Basic combat
- Modular item, block, and crafting recipe systems

## Development

Initially, this project was going to be a town building game, but I fell out of love with that idea. I'm going to turn it into a more arcade survival game instead.

I will be posting development updates on my [YouTube channel](https://www.youtube.com/channel/UCsfEWFba7Zo1DPNHisczM-g)

## Contributing

I am currently not looking for contributions, but I'm open to suggestions! Feel free to put ideas in the issues.

## License

GPL3 - See LICENSE.txt

Wisphaven - a voxel adventure game.

Copyright (C) 2024 James Moore

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

Contact:

Email - jim (at) wisphaven.com
