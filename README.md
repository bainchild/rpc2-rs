# RPC2: Bidirectional RPC for Roblox.
---
### NOTE: requires EditableImage API to be enabled or accessible.  
### You could technically use it on the server, but you don't have
### access to a server's logs and content directories usually.
---
This workspace contains:
  - `server`: RPC2 server written in rust with plugin support.
  - `interface`: RPC2 plugin interface for writing/using plugins.
  - `*_plugin`: Provided plugins for the server.
---

The protocol works like [BloxstrapRPC](https://github.com/bloxstraplabs/bloxstrap/wiki/Integrating-Bloxstrap-functionality-into-your-game) for roblox to host communication,  
and for host to client it uses [EditableImage](https://robloxapi.github.io/ref/class/EditableImage.html)-s loaded from the  
content directory using `rbxasset://rpc2/<command>`, and reading the pixels as a buffer, converting it to a string.  
Suprisingly, this doesn't cache when you call CreateEditableImageAsync with a string, allowing for fast polling.  
Note that this does NOT allow arbitrary file access, as they have to be image files, and they have to be in a roblox content directory.

---
Host -> Roblox communication enables a lot of things, a few examples being:
  - mass data importing
  - cross-place authentication using binary blobs
  - cross-place settings(?)
  - cross-place starter scripts (for command systems like Cmdr)
  - live data input like midi or osc.

Another thing its useful for is communicating with the host computer  
without having to set up a webserver and (optionally) a domain, port forwarding and such.  
This simplifies API servers greatly, and is less work for the game developer and the user.  


<sub> This isn't in bloxstrap because I can't compile it on linux. </sub>
