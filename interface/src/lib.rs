use abi_stable::{
    library::{lib_header_from_path, LibraryError, RootModule},
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{ROption, RString, RVec},
    StableAbi,
};
use std::path::Path;
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = RPC2PluginRef)))]
#[sabi(missing_field(panic))]
pub struct RPC2Plugin {
    pub get_name: extern "C" fn() -> RString,
    // None for all events, Some(rvec![event_name, event_name]) for some
    pub get_event_mask: extern "C" fn() -> ROption<RVec<RString>>,
    // handle_message(command, rvec![arguments...])
    pub handle_message: extern "C" fn(RString, RVec<RString>) -> ROption<RVec<u8>>,
    #[sabi(last_prefix_field)]
    pub init: extern "C" fn(),
}

/// The RootModule trait defines how to load the root module of a library.
impl RootModule for RPC2PluginRef {
    abi_stable::declare_root_module_statics! {RPC2PluginRef}

    const BASE_NAME: &'static str = "example_library";
    const NAME: &'static str = "example_library";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

pub fn load_root_module_from_file(file: &Path) -> Result<RPC2PluginRef, LibraryError> {
    lib_header_from_path(file).and_then(|x| x.init_root_module::<RPC2PluginRef>())
}

pub trait MessageHandler {
    fn handle_message(self, command: RString, args: RVec<RString>) -> Option<RVec<u8>>;
}

pub type Response = RVec<u8>;
