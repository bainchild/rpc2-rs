local json = require("cjson")
local pnge = require("pngencoder")
local function writedata(path,input)
   local out=assert(io.open("content/"..path,"wb"))
   if #input%4 ~= 0 then
      input=input..("\0"):rep(4-(#input%4))
   end
   local png = pnge(math.floor(#input/4),1,"rgba")
   png:write({input:byte(1,#input)}) -- might need to be changed for large data
   assert(png.done) -- should
   out:write(table.concat(png.output,''))
   out:close()
end
local path = assert((...),"usage: lua rpc2.lua path/to/log/file")
local lastone,waiting_ack = 0,{}
local funcs;funcs = {
   ["writefile"]=function(path,data)
      assert(path~=nil,"bad path")
      assert(data~=nil,"bad data")
      local file = assert(io.open("content/workspace/"..path,"wb"))
      file:write(data)
      file:close()
   end;
   ["readfile"]=function(path)
      assert(path~=nil,"bad path")
      local file = assert(io.open("content/workspace/"..path,"rb"))
      local data = file:read("*a")
      file:close()
      return data
   end;
}
while true do
   local logf = assert(io.open(path,"a+"))
   local enj = logf:seek("end")
   if enj~=lastone then
      print("size changed, checking...",enj,lastone)
      logf:seek("set",lastone)
      if lastone==0 then while logf:read("*l") do end end
      while true do
         local line = logf:read("*l")
         if line==nil then break end
         --print("linee",line)
         local cmd = line:match("%d+-%d+-%d+T%d+:%d+:%d+%.%d+Z,%d+%.%d+,%x+,%d+ %[FLog::Output%] RPC2:([^\n]+)")
         if cmd then
            print("> "..cmd)
            local suc,r = pcall(json.decode,cmd)
            if suc then
               if r[1]=="__ACK" then 
                  local func = r[2]
                  if func==nil then return end
                  if funcs[func] and waiting_ack[func] then
                     waiting_ack[func] = false
                     writedata("rpc2/"..func,"\0")
                  end
               elseif funcs[r[1] ] then
                  writedata("rpc2/"..r[1],json.encode({pcall(funcs[r[1] ],unpack(r,2))}))
                  waiting_ack[r[1] ] = true
               end
            end
         end
      end
      lastone=enj
   end
   logf:close()
   os.execute("sleep .4s")
end