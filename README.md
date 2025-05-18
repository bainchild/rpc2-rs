# RPC2: Bidirectional RPC for Roblox.
---
This workspace contains:
  - `server`: RPC2 server written in rust with plugin support.
  - `interface`: RPC2 plugin interface for writing/using plugins.
  - `*_plugin`: Provided plugins for the server.
---

The protocol works like [BloxstrapRPC](https://github.com/bloxstraplabs/bloxstrap/wiki/Integrating-Bloxstrap-functionality-into-your-game) for roblox to host communication,  
and for host to roblox it uses json files loaded from the content directory using `rbxasset://rpc2/<cachebuster><command>`, reading it using `TextService:GetFamilyInfo()`, which converts it to a table.  
The cachebuster is determined by the client and sent along with the command.
  
It used to use [EditableImage](https://robloxapi.github.io/ref/class/EditableImage.html)-s loaded from the content directory.  
Note that this does NOT allow arbitrary file access, as they have to be specific-format json files (or image files), and they have to be in a roblox content directory with a known path.

---

`TextService:GetFamilyInfoAsync(contentId)` will return a verified and reprocessed version of the json file specified by the contentId.
It has a large file length limit, which is incredibly useful as it deserializes faster than an image for the same data (up to 2mb has been tested)
`TextService:GetFamilyInfoAsync` was added in v517, march 2022, meaning it can be used in 2022 clients.

---
Host -> Roblox communication enables a lot of things, a few examples being:
  - mass data importing
  - cross-place authentication using binary blobs
  - cross-place settings(?)
  - cross-place starter scripts (for command systems like Cmdr)
  - live data input like midi or osc.
  - function polyfills

Another thing its useful for is communicating with the host computer  
without having to set up a webserver and (optionally) a domain, port forwarding and such.  
This simplifies API servers greatly, and is less work for the game developer and the user.  


<sub> This isn't in bloxstrap because I can't compile it on linux. </sub>
