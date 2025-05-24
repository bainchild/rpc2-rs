pub trait RPC2BuiltinPlugin {
    fn get_name(&self) -> &'static str;
    fn get_filter(&self) -> &'static [&'static str];
    fn handle_message(&mut self, cmd: String, args: Vec<String>) -> Option<Vec<u8>>;
    fn cleanup(&mut self) {}
}
impl RPC2BuiltinPlugin for Box<dyn RPC2BuiltinPlugin> {
    fn get_name(&self) -> &'static str {
        self.as_ref().get_name()
    }
    fn get_filter(&self) -> &'static [&'static str] {
        self.as_ref().get_filter()
    }
    fn handle_message(&mut self, cmd: String, args: Vec<String>) -> Option<Vec<u8>> {
        self.as_mut().handle_message(cmd, args)
    }
    fn cleanup(&mut self) {
        self.as_mut().cleanup();
    }
}
