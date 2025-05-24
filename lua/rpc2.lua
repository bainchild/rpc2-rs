---@diagnostic disable: deprecated
-- usage: module.rpc2.external_func(external_args...) -> success, error | returns...
-- like a wrapped coroutine or a yieldable pcall
local timeout_period = 10
--- localization for source processing
---@diagnostic disable: undefined-global, undefined-field
local table_concat,table_create,table_insert = table.concat,table.create,table.insert
local string_byte,string_char = string.byte,string.char
local bit32_lshift,bit32_extract,bit32_band = bit32.lshift,bit32.extract,bit32.band
local next = next
---@diagnostic enable: undefined-global
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
	for i = 0, 255 do
		local c = string_char(i)
		STRING_FAST[i] = c
	end

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
---@diagnostic disable: undefined-global
local https = game:GetService("HttpService")
local text = game:GetService("TextService")
local wait = (task and task.wait) or wait
---@diagnostic enable: undefined-global
-- local warncount = 99 -- first call that errors will always warn
local function readdata(path)
	-- local s,b = pcall(function()return game:GetService("AssetService"):CreateEditableImageAsync("rbxasset://"..path)end)
	-- if not s then return nil end
	-- local s2,data = pcall(function()return b:ReadPixelsBuffer(Vector2.zero,b.Size)end)
	-- if not s2 then
	-- 	warncount=warncount+1
	-- 	if warncount>=100 then
	-- 		warn("rpc2: Error reading pixels from EditableImage, is the API enabled?")
	-- 		warncount=0
	-- 	end
	-- 	return nil
	-- end
	-- return data
	local s,b = pcall(function()return text:GetFamilyInfoAsync("rbxasset://"..path) end);
	if not s then return nil end
	local data = {b64_decode(b.Name)}
	for _,v in next, b.Faces do
		table_insert(data,b64_decode(v.Name))
	end
	return data
end
local rpc2 = {}
local running = {}
local cache = 0
local mt = {__mode="k"}
function mt:__index(k)
	if rawget(self,k) then return rawget(self,k) end
	local f=function(...)
		local cc = ""..cache
		local path = cc..k
		if running[path] then return false, "internal error" end
		cache=cache+1
		running[path]=false
		--print("requesting "..path,running[path])
		local newargs = {...}
		for i,v in next, newargs do newargs[i]=tostring(v) end
		warn("RPC2:"..https:JSONEncode({cc,k,unpack(newargs)}))
		local start = os.time()
		while true do
			local data = running[path];
			if data then
				running[path]=nil
				-- print(unpack(data))
				return unpack(https:JSONDecode(data[2]))
			end
			if os.time()-start >= timeout_period then
				running[path]=nil
				return false, "RPC request timed out."
			end
			wait()
		end
	end
	rawset(self,k,f)
	return f
end
local shutdowns = {}
do
	local poller = task.spawn(function()
		while true do
			cache=cache+1
			local cc = ""..cache
			warn("RPC2:"..https:JSONEncode({cc,"__READY"}))
			task.wait(.1) -- it shouldn't even take .1, but this _should_ work
			local ready = readdata("rpc2/"..cc.."__READY")
			--print(ready,ready and ready[2])
			if ready then
				if #ready[2] > 2 then print(ready[2]) end
				for _,v in next, https:JSONDecode(ready[2]) do
					if running[v] == false then
						running[v] = readdata("rpc2/"..v)
					end
				end
				warn('RPC2:["","__READYACK"]')
			end
			task.wait(.1) -- maybe make shorter
		end
	end)
	table_insert(shutdowns,function()task.cancel(poller)end)
end
local function shutdown()
	for _,v in next, shutdowns do v() end
	warn('RPC2:["","__CLEANUP"]')
	shutdowns={}
end
-- game:BindToClose(shutdown)
setmetatable(rpc2,mt)
return {read_data=readdata,b64_encode=b64_encode,b64_decode=b64_decode,rpc=rpc2,shutdown=shutdown,before_shutdown=function(f) table_insert(shutdowns,f) end}
