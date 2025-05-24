return function(rpc2)
   local rpc = rpc2.rpc
   local rpc_writefile,rpc_readfile = rpc.writefile,rpc.readfile
   local function writefile(file,data)
      assert(rpc_writefile(file,rpc2.b64_encode(data)))
   end
   local function readfile(file)
      local success,ret = rpc_readfile(file)
      assert(success,ret)
      return rpc2.b64_decode(ret)
   end
   local function wrap(f)
      return function(...)
         return assert(f(...))
      end
   end
   return {
      writefile=writefile,
      readfile=readfile,
      listfiles=wrap(rpc.listfiles),
      isfile=wrap(rpc.isfile),
      isfolder=wrap(rpc.isfolder),
      makefolder=wrap(rpc.makefolder),
      delfolder=wrap(rpc.delfolder),
      delfile=wrap(rpc.delfile),
   }
end
