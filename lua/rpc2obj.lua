local rpc2 = require("rpc2") -- something like
local new_instance = rpc2.RPC2OBJ_new_instance
local get_schema = rpc2.RPC2OBJ_get_schema
--
local objmt = {}
function objmt:__index(key)
   if rawget(self,"__wo")[key] then error("Attempt to read write-only property.") end
   if rawget(self,"__no")[key] then return rawget(self,key) end
   local v = assert(rawget(self,"index")(self,key))
   rawset(self,key,v)
   return v
end
function objmt:__newindex(key,value)
   if rawget(self,"__ro")[key] then error("Attempt to write read-only property.") end
   if rawget(self,"__no")[key] then return rawset(self,key,value) end
   assert(rawget(self,"__newindex")(self,key))
   rawset(self,key,value)
end
local function instiantate(schema,...)
   local s = assert(new_instance(schema,...))
   local no,ro,wo = {},{},{}
   rawset(s,"__no",no)
   rawset(s,"__ro",ro)
   rawset(s,"__wo",wo)
   for i,v in next, schema.properties do
      if v.NotRemote then
         no[i]=true
      end
      if v.ReadOnly then
         ro[i]=true
      elseif v.WriteOnly then
         wo[i]=true
      end
   end
   for i,v in next, schema.methods do
      s[i] = rpc2[v]
      no[i],ro[i] = true,true
   end
   setmetatable(s,objmt)
   return s
end
local mt = {}
function mt:__index(key)
   local r = rawget(self,key)
   if r==nil then
      r=assert(get_schema(key))
      rawset(self,key,r)
   end
   return function(...)
      return instiantate(r,...)
   end
end
return setmetatable({},mt)
