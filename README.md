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

<p align="center">
    <i>Not sure how good this CLI art is, but whatever (c) mxxntype.</i>
</p>

<h3 align="center">
    Invar
</h3>

**Invar** is a CLI management tool for modded (or not) Minecraft servers. This project has the following goals:

- Allowing you to *declaratively*[^1] build and configure Minecraft modpacks, which includes managing mods, resourcepacks, shaderpacks and datapacks[^2], while treating their configuration files as first-class citizens. I aim to implement fetching component data from both the [Modrinth](https://modrinth.com) and [CurseForge](https://curseforge.com/minecraft) APIs;
- Providing an automated setup of a [Docker](https://www.docker.com) container with your modded minecraft server, powered by [`itzg/minecraft-server`](https://docker-minecraft-server.readthedocs.io/en/latest) and `docker compose`, with configurable automatic backups[^3] and maintenance;
- Being as user-friendly and informative as possible and allowing you to organize and categorize your managed [components](TODO), so you don't get lost in them while playing around with hundreds of mods at the same time.

[^1]: By "declaratively", I mean having everything built from plaintext metadata (like in [`packwiz`](https://packwiz.infra.link)) and being tightly integrated with the Git VCS. One of the design concepts of this tool is to treat modpacks and servers as software source code and managed servers, respectively.
[^2]: There still may or may not be a need for a datapack loader mod. I'll update this when I get to implementing server-side and client-side datapack handling.
[^3]: It's unliky I will be hand-rolling some sophisticated backup system. This will probably be an equivalent of an automated `cp -r ./server .backups/` before you start the server and after it shuts down.
