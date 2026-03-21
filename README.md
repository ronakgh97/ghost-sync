### Quick rant/yap

[![Groudon](https://img.pokemondb.net/sprites/black-white/anim/normal/groudon.gif)](https://pokemondb.net/pokedex/groudon)
[![Rayquaza](https://img.pokemondb.net/sprites/black-white/anim/normal/rayquaza.gif)](https://pokemondb.net/pokedex/rayquaza)
[![Kyogre](https://img.pokemondb.net/sprites/black-white/anim/normal/kyogre.gif)](https://pokemondb.net/pokedex/kyogre)

Networking in game is super frustrating, even a simple relay server takes lots of time.
Forgot the actual game, now you are stuck with networking code chaos, forever and eternally, and you will never finish
your game, and you will never be satisfied, and you will never be proud of your work, and you will never be able to show
it to anyone or PLAY, and you will never be able to have fun at all......YOU WILL NEVER BE LOVED EVER....AGAIN....
and you will just be stuck in this endless loop of networking code forever.

**Dakara watashi ga anata o sukutte agemashou**

This will be a very simple AND opinionated game network libray not a daemon
(You will still need to embed it in a server wrapper or something)

> Note: This is very experimental.
> I am making this for learning networking and get familiar with tokio ecosystem,
> but I will try to make it as usable as possible

Checkout: [Examples](examples) for usage cases and [Docs](docs.md) for right way to use this library


Usage (I don't like release cycles and versioning, so just use local)

```toml
[dependencies]
game-server = { git = "https://github.com/ronakgh97/ghost-sync" }
```


TODO

Better lib design, currently its just a mess of functions and structs, need to refactor it into a more usable and
intuitive API
Add more examples, maybe a mini-game?
Experimental UDP support, maybe using QUIC?
Add tuned buffering and improve performance by lessen serialization and deserialization, where possible (Zero-copy,
etc.)
