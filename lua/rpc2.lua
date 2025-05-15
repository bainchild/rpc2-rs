-- usage: module.rpc2.external_func(external_args...) -> success, error | returns...
-- like a wrapped coroutine or a yieldable pcall
local timeout_period = 10
--
-- https://gist.github.com/metatablecat/1f6cd6f4495f95700eb1a686de4ebe5e
local base64 = (function()
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
	local INDEX = {[61] = 0, [65] = 0}

	for key, val in ipairs(SEQ) do
		-- memoization
		INDEX[string.byte(val)] = key
	end

	-- string.char has a MASSIVE overhead, its faster to precompute
	-- the values for performance
	for i = 0, 255 do
		local c = string.char(i)
		STRING_FAST[i] = c
	end

	local b64 = {}

	function b64.encode(str)
		local len = string.len(str)
		local output = table.create(math.ceil(len/4)*4)
		local index = 1

		for i = 1, len, 3 do
			local b0, b1, b2 = string.byte(str, i, i + 2)
			local b = bit32.lshift(b0, 16) + bit32.lshift(b1 or 0, 8) + (b2 or 0)

			output[index] = SEQ[bit32.extract(b, 18, 6)]
			output[index + 1] = SEQ[bit32.extract(b, 12, 6)]
			output[index + 2] = b1 and SEQ[bit32.extract(b, 6, 6)] or "="
			output[index + 3] = b2 and SEQ[bit32.band(b, 63)] or "="

			index = index + 4
		end

		return table.concat(output)
	end

	function b64.decode(hash)
		-- given a 24 bit word (4 6-bit letters), decode 3 bytes from it
		local len = string.len(hash)
		local output = table.create(len * 0.75)

		local index = 1
		for i = 1, len, 4 do
			local c0, c1, c2, c3 = string.byte(hash, i, i + 3)

			local b = 
				bit32.lshift(INDEX[c0], 18)
				+ bit32.lshift(INDEX[c1], 12)
				+ bit32.lshift(INDEX[c2], 6)
				+ (INDEX[c3])
			output[index] = STRING_FAST[bit32.extract(b, 16, 8)]
			output[index + 1] = c2 ~= "=" and STRING_FAST[bit32.extract(b, 8, 8)] or "="
			output[index + 2] = c3 ~= "=" and STRING_FAST[bit32.band(b, 0xFF)] or "="
			index = index + 3
		end
		return table.concat(output)
	end
	return b64
end)()
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
	local s,b = pcall(function()return game:GetService("TextService"):GetFamilyInfoAsync("rbxasset://"..path) end);
	if not s then return nil end
	local data = {base64.decode(b.Name)}
	for i,v in next, b.Faces do
		table.insert(data,base64.decode(v.Name))
	end
	return data
end
local https = game:GetService("HttpService")
local rpc2 = {}
local running = {}
local cache = 0
local mt = {__mode="k"}
function mt:__index(k)
	if rawget(self,k) then return rawget(self,k) end
	local f=function(...)
		if running[k] then repeat task.wait() until not running[k]; end
		running[k]=true
		local newargs = {...}
		for i,v in next, newargs do newargs[i]=tostring(v) end
		print("RPC2:"..https:JSONEncode({""..cache,k,unpack(newargs)}))
		local start = os.time()
		while true do
			local data = readdata("rpc2/"..cache..k);
			if data then
				print("RPC2:"..https:JSONEncode({""..cache,"__ACK",k}))
				running[k]=false
				cache=cache+1
				return unpack(https:JSONDecode(data[2]))
			end
			if os.time()-start >= timeout_period then
				running[k]=false
				cache=cache+1
				return false, "RPC request timed out."
			end
			task.wait()
		end
	end
	rawset(self,k,f)
	return f
end
setmetatable(rpc2,mt)
return {read_data=readdata,rpc2=rpc2}
