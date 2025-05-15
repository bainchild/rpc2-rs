local rpc2 = require("rpc2")
local get_path = rpc2.rpc2.RPC2EV_get_path
local function new(scope)
   local path
   local ev = {
      checker=task.spawn(function()
         while true do
            task.wait(5)
            local s,p = get_path(scope)
            if not s then ev:Destroy();return end
            path=p
         end
      end),
      reader=task.spawn(function()
         while true do
            task.wait(.1)
            if path then

            end
         end
      end)
   }
   function ev:Destroy()
      task.cancel(self.checker)
      for i in next, self do self[i]=nil end
   end
   function ev:Connect(f)
      table.insert(self.handlers,f)
   end
   return ev
end
return new
