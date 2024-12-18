-- usage: rpc2.external_func(external_args...) -> success, error | returns...
-- like a wrapped coroutine or a yieldable pcall
local timeout_period = 10
--
local warncount = 99 -- first call that errors will always warn
local function readdata(path)
	local s,b = pcall(function()return game:GetService("AssetService"):CreateEditableImageAsync("rbxasset://"..path)end)
	if not s then return nil end
	local s2,data = pcall(function()return b:ReadPixelsBuffer(Vector2.zero,b.Size)end)
	if not s2 then
		warncount=warncount+1
		if warncount>=100 then
			warn("rpc2: Error reading pixels from EditableImage, is the API enabled?")
			warncount=0
		end
		return nil
	end
	return data
end
local https = game:GetService("HttpService")
local rpc2 = {}
local running = {}
local mt = {__mode="k"}
function mt:__index(k)
	if rawget(self,k) then return rawget(self,k) end
	local f=function(...)
		if running[k] then repeat task.wait() until not running[k]; end
		running[k]=true
		local narg = {...}
		for i,v in next, narg do narg[i]=tostring(v) end
		print("RPC2:"..https:JSONEncode({k,unpack(narg)}))
		local start = os.time()
		while true do
			local data = readdata("rpc2/"..k)
			if data and buffer.len(data)>1 and buffer.readstring(data,1,1)~="\0" then
				print("RPC2:"..https:JSONEncode({"__ACK",k}))
				running[k]=false
				return unpack(https:JSONDecode(buffer.tostring(data)))
			end
			if os.time()-start >= timeout_period then
				running[k]=false
				return false, "RPC request timed out."
			end
			task.wait()
		end
	end
	rawset(self,k,f)
	return f
end
setmetatable(rpc2,mt)
return rpc2
