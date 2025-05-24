mkdir temp
darklua process --format dense lua/rpc2.lua temp/rpc2.proc.lua
darklua process --format dense lua/rpc2_websocket.lua temp/rpc2_websocket.proc.lua
darklua process --format dense lua/rpc2_workspacefs.lua temp/rpc2_workspacefs.proc.lua
darklua process --format dense lua/loader_stub.lua temp/loader_stub.proc.lua
zip -j9 rpc2_lua.zip \
    lua/rpc2.lua \
    lua/rpc2_websocket.lua \
    lua/rpc2_workspacefs.lua \
    lua/loader_stub.lua \
    temp/rpc2.proc.lua \
    temp/rpc2_websocket.proc.lua \
    temp/rpc2_workspacefs.proc.lua \
    temp/loader_stub.proc.lua
cargo build --release --target x86_64-unknown-linux-gnu
tar -Jcf rpc2_release_linux.tar.xz \
    target/x86_64-unknown-linux-gnu/release/*.so \
    target/x86_64-unknown-linux-gnu/release/rpc2_server \
    target/x86_64-unknown-linux-gnu/release/plugin_debug_server
cargo build --release --target x86_64-pc-windows-gnu
zip -j9 rpc2_release_windows.zip \
    target/x86_64-pc-windows-gnu/release/*.dll \
    target/x86_64-pc-windows-gnu/release/plugin_debug_server.exe \
    target/x86_64-pc-windows-gnu/release/rpc2_server.exe
rm -r temp
