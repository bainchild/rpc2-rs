---@diagnostic disable: undefined-field, undefined-global
local next,assert,unpack = next,assert,unpack
local table_find,table_remove,table_concat,table_insert,table_create=table.find,table.remove,table.concat,table.insert,table.create
local string_byte=string.byte
local bit32_lshift,bit32_extract,bit32_band=bit32.lshift,bit32.extract,bit32.band
local websocket = {sockets={}}
-- https://gist.github.com/stravant/30dd9442cc9f24a938756192daf9e718
-- can be replaced by a bindableevent if not using the test environment
local signal = (function()
--------------------------------------------------------------------------------
--                  Simple Correct Signal Implementation                      --
-- This is the most straightforwards possible pure Lua implementation of a    --
-- Signal class that correctly implements all of the RBXScriptSignal          --
-- behavior (Connect, Disconnect, and Wait)                                   --
--------------------------------------------------------------------------------
local Signal = {}
Signal.__index = Signal

local Connection = {}
Connection.__index = Connection

function Connection.new(signal, handler)
	return setmetatable({
		_handler = handler,
		_signal = signal,
	}, Connection)
end

function Connection:Disconnect()
	local signal = self._signal
	local index = table.find(signal, self)
	if index then
		table.remove(signal, index)
	end
end

function Signal.new()
	return setmetatable({}, Signal)
end

function Signal:Connect(fn)
	local handler = Connection.new(self, fn)
	table.insert(self, handler)
	return handler
end

function Signal:DisconnectAll()
	table.clear(self)
end

function Signal:Fire(...)
	local handlersCount = #self
	local handlersCopy
	if handlersCount > 0 then
		handlersCopy = table.create(handlersCount)
		table.move(self, 1, handlersCount, 1, handlersCopy)
		for i = handlersCount, 1, -1 do
			task.spawn(handlersCopy[i]._handler, ...)
		end
	end
end

function Signal:Wait()
	local waitingCoroutine = coroutine.running()
	local cn;
	cn = self:Connect(function(...)
		cn:Disconnect()
		task.spawn(waitingCoroutine, ...)
	end)
	return coroutine.yield()
end

function Signal:Once(fn)
	local cn;
	cn = self:Connect(function(...)
		cn:Disconnect()
		fn(...)
	end)
	return cn
end

return Signal
end)()
--
local WebSocket = {}
WebSocket.__index = WebSocket
function WebSocket:Close()
   assert(websocket.initialized,"Not initialized!")
   if self.closed then return end
   self.OnClose:Fire(false)
   self.closed = true
   websocket.rpc_close(self.id)
   if table_find(websocket.sockets,self) then
      table_remove(websocket.sockets,table_find(websocket.sockets,self))
   end
end
local function close_from_server(ws)
   if ws.closed then return end
   ws.OnClose:Fire(true)
   ws.closed = true
   if table_find(websocket.sockets,ws) then
      table_remove(websocket.sockets,table_find(websocket.sockets,ws))
   end
end
-- taken from gist.github.com/metatablecat
local b64_decode,b64_encode
do
	local SEQ = {
		[0] = "A", "B", "C", "D", "E", "F", "G", "H",
		"I", "J", "K", "L", "M", "N", "O", "P",
		"Q", "R", "S", "T", "U", "V", "W", "X",
		"Y", "Z", "a", "b", "c", "d", "e", "f",
		"g", "h", "i", "j", "k", "l", "m", "n",
		"o", "p", "q", "r", "s", "t", "u", "v",
		"w", "x", "y", "z", "0", "1", "2", "3",
		"4", "5", "6", "7", "8", "9", "+", "/",
	}

	local STRING_FAST = {}
	local INDEX = {--[[[43]=62,[47]=63,]][61] = 0, [65] = 0}
	-- for i=65,90 do INDEX[i] = i-65 end
	-- for i=97,122 do INDEX[i] = i+(-97+25) end
	-- for i=48,57 do INDEX[i] = i+(-48+50) end
	for key, val in next,SEQ do
		-- memoization
		INDEX[string_byte(val)] = key
	end

	-- string.char has a MASSIVE overhead, its faster to precompute
	-- the values for performance
	-- for i = 0, 255 do
	-- 	local c = string_char(i)
	-- 	STRING_FAST[i] = c
	-- end

	function b64_encode(str)
   	local len = #str
   	local output = table_create(math.ceil(len/4)*4)
   	local index = 1

   	for i = 1, len, 3 do
   		local b0, b1, b2 = string_byte(str, i, i + 2)
   		local b = bit32_lshift(b0, 16) + bit32_lshift(b1 or 0, 8) + (b2 or 0)

   		output[index] = SEQ[bit32_extract(b, 18, 6)]
   		output[index + 1] = SEQ[bit32_extract(b, 12, 6)]
   		output[index + 2] = b1 and SEQ[bit32_extract(b, 6, 6)] or "="
   		output[index + 3] = b2 and SEQ[bit32_band(b, 63)] or "="

   		index = index + 4
   	end

   	return table_concat(output)
   end

   function b64_decode(hash)
   	-- given a 24 bit word (4 6-bit letters), decode 3 bytes from it
   	local len = #hash
   	local output = table_create(len * 0.75)

   	local index = 1
   	for i = 1, len, 4 do
   		local c0, c1, c2, c3 = string_byte(hash, i, i + 3)

   		local b =
   			bit32_lshift(INDEX[c0], 18)
   			+ bit32_lshift(INDEX[c1], 12)
   			+ bit32_lshift(INDEX[c2], 6)
   			+ (INDEX[c3])


   		output[index] = STRING_FAST[bit32_extract(b, 16, 8)]
   		output[index + 1] = c2 ~= "=" and STRING_FAST[bit32_extract(b, 8, 8)] or "="
   		output[index + 2] = c3 ~= "=" and STRING_FAST[bit32_band(b, 0xFF)] or "="
   		index = index + 3
   	end

   	return table_concat(output)
   end
end
function WebSocket:Send(data)
   assert(websocket.initialized,"Not initialized!")
   if self.closed then return end
   websocket.rpc_send(self.id,b64_encode(data))
end
--
function websocket.connect(url)
   assert(websocket.initialized,"Not initialized!")
   local id,err = websocket.rpc_open(url)
   if err=="Waiting on permission." then
      repeat task.wait(1); id,err = websocket.rpc_open(url) until id or err~="Waiting on permission."
   end
   assert(id,err)
   local new = setmetatable({id=id,OnMessage=signal.new(),OnClose=signal.new()},WebSocket)
   table_insert(websocket.sockets,new)
   return new
end
function websocket.init(rpc2)
   if websocket.initialized then return end
   websocket.initialized = true
   websocket.rpc2 = rpc2
   rpc2.before_shutdown(function()
      for _,v in next, websocket.sockets do
         v:Close()
      end
      websocket.sockets = {}
      task.cancel(websocket.poll_thread)
      websocket.rpc2 = nil
      websocket.rpc_open = nil
      websocket.rpc_close = nil
      websocket.rpc_send = nil
      websocket.rpc_poll = nil
   end)
   local rpc = rpc2.rpc
   websocket.rpc_open = rpc.websocket_open
   websocket.rpc_close = rpc.websocket_close
   websocket.rpc_send = rpc.websocket_send
   websocket.rpc_poll = rpc.websocket_poll
   websocket.poll_thread = task.spawn(function()
      while true do
         task.wait(.1)
         local ids = {}
         for _,v in next, websocket.sockets do
            table_insert(ids,v.id)
         end
         local s = assert(websocket.rpc_poll(unpack(ids)))
         for _,v in next, websocket.sockets do
            local evs = s[v.id]
            if evs then
               for _,v2 in next, evs do
                  local flags = v2[1]
                  local data = b64_decode(v2[2])
                  if flags[1] then
                     v.OnMessage:Fire(data)
                  end
                  if flags[4] or flags[5] then
                     close_from_server(v)
                     break
                  end
               end
            end
         end
      end
   end)
end
return websocket
