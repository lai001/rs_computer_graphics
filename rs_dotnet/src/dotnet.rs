#[cfg(windows)]
pub(crate) type HostfxrInitializeForRuntimeConfigFn =
    super::windows::dotnet::HostfxrInitializeForRuntimeConfigFn;

#[cfg(windows)]
pub(crate) type HostfxrCloseFn = super::windows::dotnet::HostfxrCloseFn;

#[cfg(windows)]
pub(crate) type HostfxrGetRuntimeDelegateFn = super::windows::dotnet::HostfxrGetRuntimeDelegateFn;

fn load_hostfxr_library() -> crate::error::Result<()> {
    #[cfg(windows)]
    return super::windows::dotnet::load_hostfxr_library();
}

fn get_entry_point_func<F>(
    config_path: String,
    assembly_path: String,
    type_name: String,
    method_name: String,
) -> crate::error::Result<*mut F> {
    #[cfg(windows)]
    return super::windows::dotnet::get_entry_point_func::<F>(
        config_path,
        assembly_path,
        type_name,
        method_name,
    );
}

pub fn load_and_get_entry_point_func<F>(
    config_path: String,
    assembly_path: String,
    type_name: String,
    method_name: String,
) -> crate::error::Result<*mut F> {
    load_hostfxr_library()?;
    let entry_point_func =
        get_entry_point_func::<F>(config_path, assembly_path, type_name, method_name);
    entry_point_func
}
