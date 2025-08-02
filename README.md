```brainfuck
                    .=./:)  ,===.   .==. ,===.  ,===.    ____     .=======.    
                    \8.-.') |8@8R\  |88| |@8Y/  |8@8|  .xX888b`.  |@8^Y^88B\   
                    /8`-'8\ |88,8R\ |@8| |88|   |@8Y' /x88'  \8b\ |8( ' )8@|   
                     `-'`"` |@8|\_Y\|88| |@8| _ |88|  |888|  /@8| |(_ o _)8/   
                     .===.  |8Y_( )Y\&8| |8&Y( )Y8@|     _.-`@88| |8(_,_).' __
                     |8@8|  |&(_ o _)8@| \Y(_ o._)Y/  .X@88^8888| |8&|\Y\  |@8|
                     |88&|  |8L(_,_)\88|  \Y(_,_)Y/   |88_( )_88| |X8| \Y`'@8Y/
                     |Y8R|  |8@|    |@8|   \Y@88Y/    \Y(_ o _)Y/ |8@|  \&88Y/
                     '==='  '=='    '=='    `===`      'Y(_,_)Y'  ''='   `'='  
```

<h3 align="center">Invar</h3>

> [!TIP]
> For a quick start, acquire an Invar binary, and just run it.
> Thanks to `clap`, it will suggest all implemented subcommands and actions.
> It should be fairly straightforward from there on.

**Invar** is a CLI management tool for modded (or not) Minecraft servers. This project has the following goals:

- Allowing you to *declaratively*[^1] build and configure Minecraft modpacks, which includes managing mods, resourcepacks, shaderpacks and datapacks[^2], while treating their configuration files as first-class citizens. I aim to implement fetching component data from both the [Modrinth][modrinth] and [CurseForge][curseforge] APIs.
- Providing an automated setup of a [Docker][docker] container with your modded minecraft server, powered by [`itzg/minecraft-server`][itzg-minecraft-server] and `docker compose`, with ~~configurable automatic backups[^3]~~ and maintenance.
- Being as user-friendly and informative as possible and allowing you to organize and categorize your managed `components`, so you don't get lost in them while playing around with hundreds of mods at the same time.

I try to design **Invar** to be as rugged, industrial-grade and flexible as possible, but it's still niche. It's very far away from things like [Folia][folia] and such. **Invar** can be made to fit other use cases, but it is made to be a good tool for managing small-ish private servers with lots of modded content, large worlds and extensive configuration.

Features include (this list will very likely get out of sync from the actual development...):

- [x] Modpack creation, in the form of a `git` repository
- [x] Source-code-style iteration & development of said modpacks:  
  **Flexible component system**
  - [x] Dependency-, version- & loader-aware [Modrinth][modrinth] interface
  - [x] Local components for cases where [Modrinth][modrinth] can't help or you've decided to go full custom
  - [ ] Version control automation to help you debug, bisect and keep track of things
  - [x] Pack export in `.mrpack` format
- [x] (Almost) automagical dedicated server creation & management
- [x] CLI completions
- [ ] Something else?

[^1]: By "declaratively", I mean having everything built from plaintext metadata (like in [`packwiz`](https://packwiz.infra.link)) and being tightly integrated with the Git VCS. One of the design concepts of this tool is to treat modpacks and servers as software source code and deployments, respectively.
[^2]: There still may be a need for a datapack loader mod.
[^3]: This feature was ripped out of Invar for now - backups should be handled by something actually good at backups, like the filesystem itself.

[modrinth]: https://modrinth.com
[curseforge]: https://curseforge.com/minecraft
[folia]: https://papermc.io/software/folia
[docker]: https://www.docker.com
[itzg-minecraft-server]: https://docker-minecraft-server.readthedocs.io/en/latest
